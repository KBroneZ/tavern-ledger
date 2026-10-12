-- T-507 (T-401, D-085): community hero stats, opt-in only.
--
-- * Consent: public.community_consent, one row per account that ever used the
--   switch; no row means off, so every account, old or new, starts off and no
--   update can turn it on. Only the owner sets it, through
--   public.set_community_consent() (auth.uid()), and among client roles only
--   the owner reads it (the server's own roles can, as for every table). It
--   is not a profile column on purpose: public profiles are readable by
--   everyone, and whether someone shares their games with the community stats
--   is nobody else's business. A public profile does not imply it, and a
--   private profile can opt in (the aggregates show no player).
-- * Aggregates: a snapshot, private.community_hero_rows (per mode, period and
--   hero), rebuilt once a day by pg_cron from the finished games with a server
--   summary of the accounts that opted in at that moment, with the
--   leaderboards' impossible-value exclusions (D-082). Only whole days count:
--   games played up to yesterday (UTC), so each day's change mixes a whole
--   day of games, and the switch's changes all land at the same moment. Only
--   sums and counts are stored: no account id, no per-player row.
-- * Reads: public.community_heroes() returns only the heroes with enough games
--   from enough players (at least 50 from 5; nobody over half; the top two
--   together at most three quarters), and for all time only when the games
--   older than 30 days pass the same rule on their own (or there are none), so
--   the two periods cannot be subtracted to isolate a few games. Counts are
--   shown floored (games to tens, players to fives) and the averages and rates
--   rounded, so no one game can be worked back out of two readings. Nothing
--   is returned when the snapshot is older than 26 hours (a rebuild failed),
--   so a withdrawn consent never stays in the numbers past that.
-- * Opting out (or deleting the account) takes the player's games out at the
--   next daily rebuild; no stored row names them.
-- Rules and numbers: design notes.

-- ---------------------------------------------------------------------------
-- Consent
-- ---------------------------------------------------------------------------

-- The privacy policy version a consent was given under (its "Last updated"
-- date): raised when the policy's community section changes.
create function private.community_policy_version()
returns text language sql immutable set search_path = '' as $$ select '2026-10-11' $$;

revoke all on function private.community_policy_version() from public, anon, authenticated, service_role;

create table public.community_consent (
  user_id uuid primary key references public.profiles (user_id) on delete cascade,
  -- "Include my games in community hero stats". Off unless the owner turns it on.
  hero_stats boolean not null default false,
  -- When the owner last set it, and under which privacy policy (the record of
  -- their choice, GDPR Art. 7(1)).
  updated_at timestamptz not null default now(),
  policy_version text not null check (policy_version ~ '^[0-9]{4}-[0-9]{2}-[0-9]{2}$')
);

comment on table public.community_consent is
  'Opt-in to the community hero stats (T-507). No row means off. Set only by the owner through set_community_consent(); among client roles, read only by the owner.';

alter table public.community_consent enable row level security;
revoke all on public.community_consent from public, anon, authenticated;
grant select on public.community_consent to authenticated;

create policy community_consent_read_own on public.community_consent
  for select to authenticated
  using (user_id = (select auth.uid()));

-- No insert, update or delete policy or grant: writes go through the function
-- below, which can only touch the caller's own row.

-- Sets the caller's choice and returns it. Signed-in callers with a profile only.
create function public.set_community_consent(p_hero_stats boolean)
returns boolean
language plpgsql
security definer
set search_path = ''
as $$
declare
  uid uuid := auth.uid();
begin
  if uid is null or not exists (select 1 from public.profiles p where p.user_id = uid) then
    raise exception 'not signed in' using errcode = '42501';
  end if;
  if p_hero_stats is null then
    raise exception 'set_community_consent: true or false' using errcode = '22023';
  end if;
  insert into public.community_consent (user_id, hero_stats, updated_at, policy_version)
  values (uid, p_hero_stats, now(), private.community_policy_version())
  on conflict (user_id) do update set
    hero_stats = excluded.hero_stats,
    updated_at = excluded.updated_at,
    policy_version = excluded.policy_version;
  return p_hero_stats;
end;
$$;

revoke all on function public.set_community_consent(boolean) from public, anon;
grant execute on function public.set_community_consent(boolean) to authenticated;

-- ---------------------------------------------------------------------------
-- The snapshot
-- ---------------------------------------------------------------------------

-- Per mode, period and hero (skins under their base hero): the games played
-- with it, by how many players, the most games one player and the top two
-- players have with it, and the sums the rates are worked out from; then the
-- same for the games whose known hero choice offered it, and how many of
-- those picked it. Periods: '30d' (the 30 days before the rebuild's day),
-- 'old' (before that; never shown, only checked) and 'all' (both).
create table private.community_hero_rows (
  mode text not null check (mode in ('solo', 'duos')),
  period text not null check (period in ('30d', 'old', 'all')),
  hero text not null check (hero ~ '^[A-Za-z0-9_]{1,64}$'),
  games integer not null check (games >= 0),
  players integer not null check (players >= 0 and players <= games),
  top_player_games integer not null check (top_player_games >= 0 and top_player_games <= games),
  top_two_games integer not null check (top_two_games >= top_player_games and top_two_games <= games),
  place_sum integer not null check (place_sum >= games),
  top_half integer not null check (top_half >= 0 and top_half <= games),
  wins integer not null check (wins >= 0 and wins <= top_half),
  offered_games integer not null check (offered_games >= 0),
  offered_players integer not null check (offered_players >= 0 and offered_players <= offered_games),
  offered_top_player_games integer not null
    check (offered_top_player_games >= 0 and offered_top_player_games <= offered_games),
  offered_top_two_games integer not null
    check (offered_top_two_games >= offered_top_player_games and offered_top_two_games <= offered_games),
  picked_games integer not null check (picked_games >= 0 and picked_games <= offered_games and picked_games <= games),
  primary key (mode, period, hero)
);

-- When the snapshot was last rebuilt, and the last day it counts (one row).
create table private.community_state (
  id boolean primary key default true check (id),
  refreshed_at timestamptz not null,
  counted_until date not null,
  excluded_games integer not null
);

revoke all on private.community_hero_rows, private.community_state
  from public, anon, authenticated, service_role;

-- Thresholds, in one place for the rebuild, the reads, the tests and the docs.
-- A group of games shows only with at least 50 games, from at least 5
-- players, none of whom has more than half of them and no two of whom have
-- more than three quarters.
create function private.community_min_games()
returns integer language sql immutable set search_path = '' as $$ select 50 $$;
create function private.community_min_players()
returns integer language sql immutable set search_path = '' as $$ select 5 $$;

create function private.community_enough(p_games integer, p_players integer, p_top integer, p_top_two integer)
returns boolean
language sql
immutable
set search_path = ''
as $$
  select p_games >= private.community_min_games()
    and p_players >= private.community_min_players()
    and 2 * p_top <= p_games
    and 4 * p_top_two <= 3 * p_games;
$$;

-- How old the snapshot may be before the reads return nothing: a day and
-- two hours, so one late rebuild is fine and a failed one is not.
create function private.community_max_age()
returns interval language sql immutable set search_path = '' as $$ select interval '26 hours' $$;

revoke all on function private.community_min_games() from public, anon, authenticated, service_role;
revoke all on function private.community_min_players() from public, anon, authenticated, service_role;
revoke all on function private.community_enough(integer, integer, integer, integer)
  from public, anon, authenticated, service_role;
revoke all on function private.community_max_age() from public, anon, authenticated, service_role;

-- Rebuilds the snapshot in one transaction (readers see the old one until it
-- commits), from the accounts that opted in at the moment it reads them.
-- Counted: finished games (status 'ok', with a place, so the upload validator
-- passed them) with a server summary and a known hero, played up to
-- yesterday (UTC), with no impossible value (the leaderboards' rule; a game
-- dated after tomorrow is impossible). Returns the number of hero rows.
create function private.refresh_community_heroes()
returns integer
language plpgsql
security definer
set search_path = ''
as $$
declare
  n integer;
  n_excluded integer;
  today date := (now() at time zone 'utc')::date;
  last_day date := today - 1;
  since date := today - 30;
begin
  -- One rebuild at a time; an advisory lock, so reads never wait.
  perform pg_advisory_xact_lock(hashtext('private.refresh_community_heroes'));

  -- A second rebuild in the same transaction (tests) starts clean.
  if to_regclass('pg_temp.ch_games') is not null then
    drop table pg_temp.ch_games;
  end if;
  if to_regclass('pg_temp.ch_offers') is not null then
    drop table pg_temp.ch_offers;
  end if;

  create temp table ch_games on commit drop as
  select
    g.user_id,
    g.session,
    g.game_index,
    case g.game_type when 'GT_BATTLEGROUNDS' then 'solo' else 'duos' end as mode,
    coalesce(substring(g.hero_card_id from '^(.+)_SKIN_[A-Za-z0-9]+$'), g.hero_card_id) as hero,
    g.final_place::integer as place,
    g.final_place <= case g.game_type when 'GT_BATTLEGROUNDS' then 4 else 2 end as top_half,
    g.final_place = 1 as won,
    g.played_on,
    s.heroes_offered,
    private.leaderboard_game_plausible(s) and g.played_on <= today + 1 as plausible
  from public.games g
  join public.community_consent c on c.user_id = g.user_id and c.hero_stats
  join public.game_stats s
    on s.user_id = g.user_id and s.session = g.session and s.game_index = g.game_index
  where g.status = 'ok'
    and g.final_place is not null
    and g.hero_card_id is not null;

  select count(*) filter (where not plausible) into n_excluded from pg_temp.ch_games;
  -- Impossible games out; today's and tomorrow's wait for their day to end.
  delete from pg_temp.ch_games where not plausible or played_on > last_day;

  -- One row per game and hero its known choice offered (two skins of one
  -- hero count once), and whether the game was played with it.
  create temp table ch_offers on commit drop as
  select distinct g.user_id, g.session, g.game_index, g.mode, g.played_on,
    coalesce(substring(o.card from '^(.+)_SKIN_[A-Za-z0-9]+$'), o.card) as hero,
    coalesce(substring(o.card from '^(.+)_SKIN_[A-Za-z0-9]+$'), o.card) = g.hero as picked
  from pg_temp.ch_games g
  cross join lateral unnest(g.heroes_offered) as o(card);

  delete from private.community_hero_rows;

  with periods (period, first_day, last) as (
    values ('30d', since, last_day), ('old', null::date, since - 1), ('all', null::date, last_day)
  ),
  played as (
    select per.period, g.*
    from pg_temp.ch_games g
    join periods per
      on (per.first_day is null or g.played_on >= per.first_day) and g.played_on <= per.last
  ),
  offers as (
    select per.period, o.*
    from pg_temp.ch_offers o
    join periods per
      on (per.first_day is null or o.played_on >= per.first_day) and o.played_on <= per.last
  ),
  played_by as (
    select period, mode, hero, user_id, count(*) as games,
      sum(place) as place_sum,
      count(*) filter (where top_half) as top_half,
      count(*) filter (where won) as wins,
      row_number() over (partition by period, mode, hero order by count(*) desc) as rank
    from played
    group by period, mode, hero, user_id
  ),
  played_heroes as (
    select period, mode, hero,
      sum(games)::integer as games,
      count(*)::integer as players,
      max(games)::integer as top_player_games,
      (sum(games) filter (where rank <= 2))::integer as top_two_games,
      sum(place_sum)::integer as place_sum,
      sum(top_half)::integer as top_half,
      sum(wins)::integer as wins
    from played_by
    group by period, mode, hero
  ),
  offered_by as (
    select period, mode, hero, user_id, count(*) as games,
      count(*) filter (where picked) as picked,
      row_number() over (partition by period, mode, hero order by count(*) desc) as rank
    from offers
    group by period, mode, hero, user_id
  ),
  offered_heroes as (
    select period, mode, hero,
      sum(games)::integer as offered_games,
      count(*)::integer as offered_players,
      max(games)::integer as offered_top_player_games,
      (sum(games) filter (where rank <= 2))::integer as offered_top_two_games,
      sum(picked)::integer as picked_games
    from offered_by
    group by period, mode, hero
  )
  insert into private.community_hero_rows (
    mode, period, hero, games, players, top_player_games, top_two_games, place_sum, top_half, wins,
    offered_games, offered_players, offered_top_player_games, offered_top_two_games, picked_games
  )
  select coalesce(p.mode, o.mode), coalesce(p.period, o.period), coalesce(p.hero, o.hero),
    coalesce(p.games, 0), coalesce(p.players, 0), coalesce(p.top_player_games, 0),
    coalesce(p.top_two_games, 0), coalesce(p.place_sum, 0), coalesce(p.top_half, 0), coalesce(p.wins, 0),
    coalesce(o.offered_games, 0), coalesce(o.offered_players, 0),
    coalesce(o.offered_top_player_games, 0), coalesce(o.offered_top_two_games, 0),
    coalesce(o.picked_games, 0)
  from played_heroes p
  full join offered_heroes o using (period, mode, hero);
  get diagnostics n = row_count;

  insert into private.community_state (id, refreshed_at, counted_until, excluded_games)
  values (true, now(), last_day, n_excluded)
  on conflict (id) do update set
    refreshed_at = excluded.refreshed_at,
    counted_until = excluded.counted_until,
    excluded_games = excluded.excluded_games;
  return n;
end;
$$;

revoke all on function private.refresh_community_heroes() from public, anon, authenticated, service_role;

-- ---------------------------------------------------------------------------
-- Reads (anon and authenticated)
-- ---------------------------------------------------------------------------

-- True when the snapshot is recent enough to be shown.
create function private.community_fresh()
returns boolean
language sql
stable
security definer
set search_path = ''
as $$
  select coalesce(
    (select s.refreshed_at > now() - private.community_max_age() from private.community_state s),
    false
  );
$$;

revoke all on function private.community_fresh() from public, anon, authenticated, service_role;

-- The heroes of both modes and periods that pass the thresholds, in one read
-- (one snapshot): per mode, period, most games first. Games floored to tens
-- and players to fives (shown as "50+", "5+"), average place to 1 decimal,
-- top 4 (Solo) or top 2 (Duos) rate and win rate in whole percent. The pick
-- rate, with the games and players behind it floored the same way, only when
-- the games that offered the hero pass the same rule; null otherwise. An
-- all-time row (or its pick rate) needs the games older than 30 days to pass
-- the rule too, or to be none, whenever the 30-day one is shown. Nothing when
-- the snapshot is stale.
create function public.community_heroes()
returns table (
  mode text,
  period text,
  hero text,
  games integer,
  players integer,
  avg_place numeric,
  top_half_rate integer,
  win_rate integer,
  offered_games integer,
  offered_players integer,
  pick_rate integer
)
language sql
stable
security definer
set search_path = ''
as $$
  with checked as (
    select r.*,
      private.community_enough(r.games, r.players, r.top_player_games, r.top_two_games) as shown,
      private.community_enough(r.offered_games, r.offered_players, r.offered_top_player_games,
        r.offered_top_two_games) as offers_enough
    from private.community_hero_rows r
  ),
  -- A slice that can be published or that holds nothing.
  slice as (
    select c.mode, c.period, c.hero,
      c.games = 0 or c.shown as rows_ok,
      c.offered_games = 0 or c.offers_enough as offers_ok,
      c.shown as rows_shown,
      c.shown and c.offers_enough as pick_shown
    from checked c
  ),
  visible as (
    select c.*,
      c.shown and (c.period = '30d' or (
        not coalesce(recent.rows_shown, false) or coalesce(older.rows_ok, true)
      )) as row_visible,
      c.offers_enough and (c.period = '30d' or (
        not coalesce(recent.pick_shown, false) or coalesce(older.offers_ok, true)
      )) as pick_visible
    from checked c
    left join slice recent on recent.mode = c.mode and recent.period = '30d' and recent.hero = c.hero
    left join slice older on older.mode = c.mode and older.period = 'old' and older.hero = c.hero
    where c.period in ('30d', 'all')
  )
  select v.mode, v.period, v.hero,
    (v.games / 10 * 10)::integer,
    (v.players / 5 * 5)::integer,
    round(v.place_sum::numeric / v.games, 1),
    round(100.0 * v.top_half / v.games)::integer,
    round(100.0 * v.wins / v.games)::integer,
    case when v.pick_visible then (v.offered_games / 10 * 10)::integer end,
    case when v.pick_visible then (v.offered_players / 5 * 5)::integer end,
    case when v.pick_visible then round(100.0 * v.picked_games / v.offered_games)::integer end
  from visible v
  where v.row_visible and private.community_fresh()
  order by v.mode, v.period, v.games desc, v.hero;
$$;

comment on function public.community_heroes() is
  'Community hero stats (T-507): heroes per mode (solo, duos) and period (30d, all) with at least 50 games from at least 5 players who opted in, none over half, the top two at most three quarters; counts floored, values rounded. Self-reported data; no player is shown.';

-- One row: when the snapshot was built and the last day it counts (null
-- before the first rebuild), and whether it is recent enough to be shown.
create function public.community_heroes_state()
returns table (
  refreshed_at timestamptz,
  counted_until date,
  available boolean
)
language sql
stable
security definer
set search_path = ''
as $$
  select s.refreshed_at, s.counted_until, private.community_fresh()
  from (select 1) as one
  left join private.community_state s on true;
$$;

revoke all on function public.community_heroes() from public;
revoke all on function public.community_heroes_state() from public;
grant execute on function public.community_heroes() to anon, authenticated, service_role;
grant execute on function public.community_heroes_state() to anon, authenticated, service_role;

-- ---------------------------------------------------------------------------
-- Export: as in 20261011150000_leaderboards.sql, plus the community stats
-- switch (off when the account never set it). Account deletion removes the
-- consent row; the snapshot holds nothing per player, and the next rebuild
-- leaves the deleted games out.
-- ---------------------------------------------------------------------------

create or replace function public.export_my_data()
returns jsonb
language plpgsql
stable
security definer
set search_path = ''
as $$
declare
  uid uuid := auth.uid();
begin
  if uid is null then
    raise exception 'not signed in' using errcode = '42501';
  end if;

  return jsonb_build_object(
    'format', 'tavern-ledger-export',
    'format_version', 1,
    'exported_at', now(),
    'account', (
      select jsonb_build_object(
        'id', u.id,
        'email', u.email,
        'phone', u.phone,
        'created_at', u.created_at,
        'updated_at', u.updated_at,
        'email_confirmed_at', u.email_confirmed_at,
        'last_sign_in_at', u.last_sign_in_at,
        'app_metadata', u.raw_app_meta_data,
        'user_metadata', u.raw_user_meta_data
      )
      from auth.users u where u.id = uid
    ),
    'identities', coalesce((
      select jsonb_agg(jsonb_build_object(
        'provider', i.provider,
        'identity_data', i.identity_data,
        'created_at', i.created_at,
        'updated_at', i.updated_at,
        'last_sign_in_at', i.last_sign_in_at
      ) order by i.created_at)
      from auth.identities i where i.user_id = uid
    ), '[]'::jsonb),
    'sessions', coalesce((
      select jsonb_agg(jsonb_build_object(
        'created_at', s.created_at,
        'updated_at', s.updated_at,
        'not_after', s.not_after,
        'user_agent', s.user_agent,
        'ip', s.ip
      ) order by s.created_at)
      from auth.sessions s where s.user_id = uid
    ), '[]'::jsonb),
    -- Second factors (metadata only; secrets stay out).
    'mfa_factors', coalesce((
      select jsonb_agg(jsonb_build_object(
        'factor_type', f.factor_type,
        'friendly_name', f.friendly_name,
        'status', f.status,
        'created_at', f.created_at,
        'updated_at', f.updated_at
      ) order by f.created_at)
      from auth.mfa_factors f where f.user_id = uid
    ), '[]'::jsonb),
    'auth_events', coalesce((
      select jsonb_agg(jsonb_build_object(
        'created_at', a.created_at,
        'ip_address', a.ip_address,
        'payload', a.payload
      ) order by a.created_at)
      from auth.audit_log_entries a where (a.payload ->> 'actor_id' = uid::text
             or a.payload -> 'traits' ->> 'user_id' = uid::text)
    ), '[]'::jsonb),
    'profile', (
      select to_jsonb(p) from public.profiles p where p.user_id = uid
    ),
    -- The switch as it is now; updated_at and policy_version are null when
    -- it was never set.
    'community_consent', jsonb_build_object(
      'hero_stats', coalesce((
        select c.hero_stats from public.community_consent c where c.user_id = uid
      ), false),
      'updated_at', (
        select c.updated_at from public.community_consent c where c.user_id = uid
      ),
      'policy_version', (
        select c.policy_version from public.community_consent c where c.user_id = uid
      )
    ),
    'games', coalesce((
      select jsonb_agg(to_jsonb(g) order by g.session, g.game_index)
      from public.games g where g.user_id = uid
    ), '[]'::jsonb),
    'game_stats', coalesce((
      select jsonb_agg(to_jsonb(s) order by s.session, s.game_index)
      from public.game_stats s where s.user_id = uid
    ), '[]'::jsonb),
    'leaderboard_rows', coalesce((
      select jsonb_agg(jsonb_build_object(
        'mode', r.mode,
        'period', r.period,
        'board', r.board,
        'hero', r.hero,
        'value', r.value,
        'games', r.games
      ) order by r.mode, r.period, r.board, r.hero)
      from private.leaderboard_rows r where r.user_id = uid
    ), '[]'::jsonb),
    'files', coalesce((
      select jsonb_agg(jsonb_build_object(
        'bucket', o.bucket_id,
        'name', o.name,
        'created_at', o.created_at,
        'updated_at', o.updated_at,
        'metadata', o.metadata
      ) order by o.name)
      from storage.objects o
      where o.bucket_id = 'games' and (storage.foldername(o.name))[1] = uid::text
    ), '[]'::jsonb),
    'upload_events', coalesce((
      select jsonb_agg(e.at order by e.at)
      from private.upload_events e where e.user_id = uid
    ), '[]'::jsonb)
  );
end;
$$;

-- As in 20261011150000_leaderboards.sql, plus the consent row (the foreign
-- key would also remove it with the profile).
create or replace function public.delete_user_data(target uuid)
returns jsonb
language plpgsql
security definer
set search_path = ''
as $$
declare
  n_stats bigint;
  n_games bigint;
  n_profiles bigint;
  n_events bigint;
begin
  if exists (
    select 1 from storage.objects o
    where o.bucket_id = 'games' and (storage.foldername(o.name))[1] = target::text
  ) then
    raise exception 'files remain for this user; delete them first'
      using errcode = '55000';
  end if;

  delete from private.leaderboard_rows where user_id = target;
  delete from public.community_consent where user_id = target;
  delete from public.game_stats where user_id = target;
  get diagnostics n_stats = row_count;
  delete from public.games where user_id = target;
  get diagnostics n_games = row_count;
  delete from public.profiles where user_id = target;
  get diagnostics n_profiles = row_count;
  -- Auth audit entries have no foreign key to the user and would survive the
  -- auth user's deletion.
  delete from auth.audit_log_entries
  where payload ->> 'actor_id' = target::text
     or payload -> 'traits' ->> 'user_id' = target::text;
  get diagnostics n_events = row_count;

  return jsonb_build_object(
    'game_stats', n_stats, 'games', n_games, 'profiles', n_profiles, 'auth_events', n_events
  );
end;
$$;

revoke all on function public.delete_user_data(uuid) from public, anon, authenticated;
grant execute on function public.delete_user_data(uuid) to service_role;

-- ---------------------------------------------------------------------------
-- Schedule: once a day, at 04:05 UTC (after the sweep at 03:17).
-- ---------------------------------------------------------------------------

select cron.schedule('community-heroes-refresh', '5 4 * * *', 'select private.refresh_community_heroes()');

select private.refresh_community_heroes();
