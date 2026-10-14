-- T-572 (rest), DB-4, DB-5: a sturdier community threshold, a cleaner data
-- export and a sweep that scales.
--
-- Community stats. A group of games used to show with 5 players, however
-- those accounts came about: four accounts made for the purpose plus one real
-- opted-in player showed the real player's games as a "community" number.
-- Now only accounts that pass a trust bar count at all: the profile is at
-- least 7 days old. profiles.created_at is set by the server and no client
-- can write it; it is the cheapest signal the database already has and one
-- an upload cannot fake. Accounts made in one go have to wait a week before
-- they can count. Games of younger accounts are left out of every number
-- (the rebuild stores how many in community_state.untrusted_games), so the
-- player counts, the "nobody over half" rule and the checks between periods
-- all work on trusted accounts only. The number of players stays 5 (the
-- thresholds are in private.community_min_*): the beta has too few accounts
-- to show anything at a higher bar. Combinable slices stay as they were: the
-- only dimensions are mode, period and hero, a mode is never added up with
-- the other, and the all-time rows need the games older than 30 days to pass
-- the rule on their own, so no two readings subtract to one player. A hidden
-- row is just a missing row: the page already says there are not enough
-- players.
--
-- Export. The security log part of export_my_data() keeps the account's own
-- events as they are and, in an event someone else caused (an admin acting on
-- the account), drops the actor's id, name and sign-in method and the IP
-- address (the actor's).
--
-- Scale (DB-4). An index on games(storage_path) (the sweep and the file
-- retention join on it); a longer statement_timeout for the server's own
-- role (service_role, which the sweep's RPCs use through the API) so the
-- heavy steps do not hit the API's short limit; the same limit on each heavy
-- function's own settings (a function's SET does not lengthen the timeout of
-- the statement that calls it: Postgres arms that timer before the function
-- runs, so the role setting is what counts for API calls; the function
-- settings are kept for callers that apply them, and as a record);
-- pg_cron's run log kept 14 days.
-- CREATE OR REPLACE FUNCTION drops a function's settings: a later migration
-- that redefines one of them must set the timeout again
-- (supabase/tests/database/scale.test.sql checks).

-- ---------------------------------------------------------------------------
-- Community: the trust bar
-- ---------------------------------------------------------------------------

create function private.community_min_account_age()
returns interval language sql immutable set search_path = '' as $$ select interval '7 days' $$;

revoke all on function private.community_min_account_age() from public, anon, authenticated, service_role;

-- Games left out because the account is younger than that, at the last rebuild.
alter table private.community_state add column untrusted_games integer not null default 0;

-- As in 20261011180000_community_heroes.sql, plus the trust bar.
create or replace function private.refresh_community_heroes()
returns integer
language plpgsql
security definer
set search_path = ''
as $$
declare
  n integer;
  n_excluded integer;
  n_untrusted integer;
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

  -- Only accounts that pass the trust bar count: a profile at least the
  -- minimum age (the server's own clock, nobody can set it). Their games are
  -- out of every number, and out of the player counts the thresholds look at.
  select count(*) filter (where not exists (
    select 1 from public.profiles p
    where p.user_id = g.user_id and p.created_at <= now() - private.community_min_account_age()
  )) into n_untrusted from pg_temp.ch_games g;
  delete from pg_temp.ch_games g
  where not exists (
    select 1 from public.profiles p
    where p.user_id = g.user_id and p.created_at <= now() - private.community_min_account_age()
  );

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

  insert into private.community_state (id, refreshed_at, counted_until, excluded_games, untrusted_games)
  values (true, now(), last_day, n_excluded, n_untrusted)
  on conflict (id) do update set
    refreshed_at = excluded.refreshed_at,
    counted_until = excluded.counted_until,
    excluded_games = excluded.excluded_games,
    untrusted_games = excluded.untrusted_games;
  return n;
end;
$$;

revoke all on function private.refresh_community_heroes() from public, anon, authenticated, service_role;

-- ---------------------------------------------------------------------------
-- Export: no identity of anyone else in the security log
-- ---------------------------------------------------------------------------

-- As in 20261012200000_simulator_consent.sql, plus the security log rule above.
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
    -- The account's own events as they are. An event someone else caused (an
    -- admin acting on the account) loses who and from where: the actor's id,
    -- name and sign-in method, and the IP address (the actor's, not the
    -- owner's).
    'auth_events', coalesce((
      select jsonb_agg(jsonb_build_object(
        'created_at', a.created_at,
        'ip_address', case when a.payload ->> 'actor_id' = uid::text then a.ip_address end,
        'payload', case when a.payload ->> 'actor_id' = uid::text then a.payload::jsonb
          else a.payload::jsonb - 'actor_id' - 'actor_username' - 'actor_name' - 'actor_via_sso' end
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
    'simulator_consent', jsonb_build_object(
      'improve_simulator', coalesce((
        select c.improve_simulator from public.simulator_consent c where c.user_id = uid
      ), false),
      'updated_at', (
        select c.updated_at from public.simulator_consent c where c.user_id = uid
      ),
      'policy_version', (
        select c.policy_version from public.simulator_consent c where c.user_id = uid
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
    'file_retention_attempts', coalesce((
      select jsonb_agg(jsonb_build_object(
        'session', a.session,
        'game_index', a.game_index,
        'fails', a.fails,
        'last_at', a.last_at
      ) order by a.session, a.game_index)
      from private.file_retention_attempts a where a.user_id = uid
    ), '[]'::jsonb),
    'unlimited_history', (
      select jsonb_build_object('since', u.since)
      from private.unlimited_history u where u.user_id = uid
    ),
    'upload_events', coalesce((
      select jsonb_agg(e.at order by e.at)
      from private.upload_events e where e.user_id = uid
    ), '[]'::jsonb)
  );
end;
$$;

-- ---------------------------------------------------------------------------
-- Scale
-- ---------------------------------------------------------------------------

create index games_storage_path_idx on public.games (storage_path);

-- After every create or replace above and in earlier migrations.
alter function public.sweep_housekeeping() set statement_timeout = '120s';
alter function public.sweep_orphan_files(interval) set statement_timeout = '60s';
alter function public.file_retention_list(integer, integer) set statement_timeout = '60s';
alter function public.file_retention_check(text[], text[], integer) set statement_timeout = '60s';
alter function public.file_retention_mark(text[], text[]) set statement_timeout = '60s';
alter function public.game_stats_backfill_list(integer, integer) set statement_timeout = '60s';
alter function private.refresh_community_heroes() set statement_timeout = '120s';
alter function private.refresh_leaderboards() set statement_timeout = '120s';

-- The role the sweep's RPCs run as. Only clients' roles keep the short API
-- limits. If the migration role may not change it, the migration goes on.
do $$
begin
  alter role service_role set statement_timeout = '120s';
exception when insufficient_privilege then
  raise warning 'could not set service_role statement_timeout; set it in the dashboard (120s)';
end;
$$;

-- pg_cron keeps one row per run in cron.job_run_details and never deletes
-- them. Returns how many rows went.
create function private.purge_cron_log()
returns bigint
language plpgsql
set search_path = ''
as $$
declare
  n bigint;
begin
  delete from cron.job_run_details d where coalesce(d.end_time, d.start_time) < now() - interval '14 days';
  get diagnostics n = row_count;
  return n;
end;
$$;

revoke all on function private.purge_cron_log() from public, anon, authenticated, service_role;

-- Every day at 04:25 UTC, after the community rebuild (04:05).
select cron.schedule('cron-log-purge', '25 4 * * *', 'select private.purge_cron_log()');

select private.refresh_community_heroes();
