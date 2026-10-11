-- T-506 (D-075, D-076, D-077, D-082): public leaderboards and the display
-- name filter.
--
-- * Display names (D-077): today's rule (the profiles.display_name check)
--   plus a small word list, private.name_blocklist, checked by a trigger on
--   every write of a new name (the account page, the API, the service role)
--   and again before a profile shows on a board, so a name that was set
--   before a word was added is hidden from the boards until it changes.
--   Making a profile private or clearing a name always works, whatever the
--   stored name.
-- * Leaderboards: a snapshot, private.leaderboard_rows, rebuilt every 10
--   minutes by pg_cron from the games of public profiles with an allowed
--   name. Only finished games (status 'ok', with a place) that passed the
--   upload validator and have a server summary count, and a game with a value
--   no real game can have is left out of every board. A player needs at least
--   20 such games in the mode and period (10 with the hero on the hero board).
--   Clients read it only through public.leaderboard(), which checks each
--   profile again as it reads (still public, name still allowed), so a profile
--   made private disappears at once, and its rows are deleted from the
--   snapshot at the same moment. Rows carry the player's site account id
--   (already public for a public profile), never an email, BattleTag or
--   rating (D-076: no rating on the website).
-- Rules and numbers: docs/research/leaderboards.md.

-- ---------------------------------------------------------------------------
-- Display name filter
-- ---------------------------------------------------------------------------

-- word: lower-case letters and digits, matched against the name's spellings
-- (below). 'part' matches anywhere in the name with the separators removed;
-- 'word' only a whole word of the name (split at spaces, '_', '.', '-') or the
-- whole name, for words that hide inside ordinary ones ("class", "Scunthorpe").
-- 'reserved' words would make a name look official.
create table private.name_blocklist (
  word text primary key check (word ~ '^[a-z0-9]{2,32}$'),
  match text not null check (match in ('part', 'word')),
  reason text not null check (reason in ('offensive', 'reserved'))
);

revoke all on private.name_blocklist from public, anon, authenticated, service_role;

insert into private.name_blocklist (word, match, reason) values
  -- Slurs and strong profanity, anywhere in the name.
  ('fuck', 'part', 'offensive'),
  ('shit', 'part', 'offensive'),
  ('cunt', 'part', 'offensive'),
  ('nigger', 'part', 'offensive'),
  ('nigga', 'part', 'offensive'),
  ('faggot', 'part', 'offensive'),
  ('retard', 'part', 'offensive'),
  ('whore', 'part', 'offensive'),
  ('slut', 'part', 'offensive'),
  ('bitch', 'part', 'offensive'),
  ('bastard', 'part', 'offensive'),
  ('motherf', 'part', 'offensive'),
  ('pussy', 'part', 'offensive'),
  ('penis', 'part', 'offensive'),
  ('vagina', 'part', 'offensive'),
  ('porn', 'part', 'offensive'),
  ('twat', 'part', 'offensive'),
  ('wanker', 'part', 'offensive'),
  ('tranny', 'part', 'offensive'),
  ('hitler', 'part', 'offensive'),
  ('killyourself', 'part', 'offensive'),
  ('molest', 'part', 'offensive'),
  -- Words that hide inside ordinary ones ("Fukuoka", "Ashkenazi", "Therapist",
-- "Class"): whole words only.
  ('fuk', 'word', 'offensive'),
  ('nazi', 'word', 'offensive'),
  ('rapist', 'word', 'offensive'),
  ('ass', 'word', 'offensive'),
  ('asshole', 'word', 'offensive'),
  ('arse', 'word', 'offensive'),
  ('cock', 'word', 'offensive'),
  ('cum', 'word', 'offensive'),
  ('tits', 'word', 'offensive'),
  ('fag', 'word', 'offensive'),
  ('dyke', 'word', 'offensive'),
  ('kike', 'word', 'offensive'),
  ('spic', 'word', 'offensive'),
  ('chink', 'word', 'offensive'),
  ('rape', 'word', 'offensive'),
  ('pedo', 'word', 'offensive'),
  ('paedo', 'word', 'offensive'),
  ('heil', 'word', 'offensive'),
  ('kys', 'word', 'offensive'),
  ('1488', 'word', 'offensive'),
  -- Names that would look official.
  ('blizzard', 'part', 'reserved'),
  ('tavernledger', 'part', 'reserved'),
  ('admin', 'word', 'reserved'),
  ('administrator', 'word', 'reserved'),
  ('moderator', 'word', 'reserved'),
  ('staff', 'word', 'reserved'),
  ('support', 'word', 'reserved'),
  ('official', 'word', 'reserved');

-- The spellings a name is checked in: lower case after Unicode NFKC (so
-- full-width and other compatibility forms read as plain letters), as typed
-- and with digits read as letters (0 o, 1 i or l, 3 e, 4 a, 5 s, 7 t, 8 b,
-- 9 g). Look-alikes from other alphabets are not mapped: a basic filter.
create function private.name_spellings(p_name text)
returns text[]
language sql
immutable
set search_path = ''
as $$
  select array[n, translate(n, '01345789', 'oieastbg'), translate(n, '01345789', 'oleastbg')]
  from (select lower(normalize(p_name, nfkc)) as n) s;
$$;

-- True when no listed word is in the name; true for no name. Each spelling's
-- joined-up form and words are worked out once, then matched against the list.
create function private.display_name_allowed(p_name text)
returns boolean
language sql
stable
security definer
set search_path = ''
as $$
  -- Each spelling's forms once (materialized: otherwise the planner runs them
  -- inside the join, once per listed word), then matched against the list.
  with forms as materialized (
    select regexp_replace(v.n, '[ _.-]+', '', 'g') as joined,
      regexp_split_to_array(v.n, '[ _.-]+') as words
    from unnest(private.name_spellings(p_name)) as v(n)
  )
  select p_name is null or not exists (
    select 1
    from forms f
    join private.name_blocklist b on case b.match
      when 'part' then strpos(f.joined, b.word) > 0
      else b.word = any (f.words) or b.word = f.joined
    end
  );
$$;

revoke all on function private.name_spellings(text) from public, anon, authenticated, service_role;
revoke all on function private.display_name_allowed(text) from public, anon, authenticated, service_role;

-- Refuses a new name with a listed word. Only a changed name is checked, so
-- an old name that fails today never blocks making the profile private.
-- The message is a fixed key the account page recognises; nothing from the
-- word list goes back to the caller.
create function private.check_display_name()
returns trigger
language plpgsql
security definer
set search_path = ''
as $$
begin
  if tg_op = 'UPDATE' and new.display_name is not distinct from old.display_name then
    return new;
  end if;
  if not private.display_name_allowed(new.display_name) then
    raise exception 'display_name_not_allowed'
      using errcode = '23514', hint = 'Choose another display name.';
  end if;
  return new;
end;
$$;

revoke all on function private.check_display_name() from public, anon, authenticated, service_role;

create trigger profiles_display_name_allowed
  before insert or update of display_name on public.profiles
  for each row execute function private.check_display_name();

-- ---------------------------------------------------------------------------
-- The leaderboard snapshot
-- ---------------------------------------------------------------------------

-- One row per board, mode, period and player (and hero on the hero board).
-- value is rounded as the site shows it, so equal values share a standing.
create table private.leaderboard_rows (
  mode text not null check (mode in ('solo', 'duos')),
  period text not null check (period in ('7d', '30d', 'all')),
  board text not null check (board in (
    'avg_place', 'top_half_rate', 'win_rate', 'games', 'top_half_streak', 'hero_avg_place',
    'apm', 'gold_beyond_income', 'rolls', 'triples', 'biggest_board', 'most_golden'
  )),
  user_id uuid not null references public.profiles (user_id) on delete cascade,
  hero text check (hero ~ '^[A-Za-z0-9_]{1,64}$'),
  value numeric not null,
  games integer not null check (games >= 1),
  check ((board = 'hero_avg_place') = (hero is not null))
);

create index leaderboard_rows_board on private.leaderboard_rows (mode, period, board);
create index leaderboard_rows_user on private.leaderboard_rows (user_id);

-- When the snapshot was last rebuilt (one row).
create table private.leaderboard_state (
  id boolean primary key default true check (id),
  refreshed_at timestamptz not null,
  players integer not null,
  excluded_games integer not null
);

revoke all on private.leaderboard_rows, private.leaderboard_state
  from public, anon, authenticated, service_role;

-- Thresholds and limits, in one place for the refresh, the tests and the docs.
create function private.leaderboard_min_games()
returns integer language sql immutable set search_path = '' as $$ select 20 $$;
create function private.leaderboard_min_hero_games()
returns integer language sql immutable set search_path = '' as $$ select 10 $$;

-- A game the boards can count: no value beyond what a real game can have.
-- Board size and golden minions are capped at 7 by the game; the other limits
-- are far above anything seen (docs/research/leaderboards.md), so only a
-- broken or edited report reaches them. Unknown values do not exclude a game.
create function private.leaderboard_game_plausible(s public.game_stats)
returns boolean
language sql
immutable
set search_path = ''
as $$
  select coalesce(s.golden_max, 0) <= 7
    and coalesce(cardinality(s.final_board), 0) <= 7
    and coalesce(s.apm, 0) <= 100
    and coalesce(s.triples, 0) <= 50
    and coalesce(s.rolls, 0) <= 1000
    and coalesce(s.gold_beyond_income, 0) <= 5000
    and coalesce(s.board_stats_max, 0) <= 10000000
    and coalesce(s.rounds, 0) <= 100;
$$;

revoke all on function private.leaderboard_min_games() from public, anon, authenticated, service_role;
revoke all on function private.leaderboard_min_hero_games() from public, anon, authenticated, service_role;
revoke all on function private.leaderboard_game_plausible(public.game_stats)
  from public, anon, authenticated, service_role;

-- Rebuilds the snapshot in one transaction (readers see the old one until it
-- commits). The rows are worked out first into a temporary table; the stored
-- ones are only replaced at the end, so their row locks last a moment. Periods end today (UTC): the last 7 days are today and the six
-- before, by the day each game was played (the player's local day, so up to
-- tomorrow in UTC; a game dated later is impossible). Returns the number of
-- rows.
create function private.refresh_leaderboards()
returns integer
language plpgsql
security definer
set search_path = ''
as $$
declare
  n integer;
  n_players integer;
  n_excluded integer;
  min_games integer := private.leaderboard_min_games();
  min_hero integer := private.leaderboard_min_hero_games();
  today date := (now() at time zone 'utc')::date;
begin
  -- One rebuild at a time. An advisory lock, not a table lock: reads and a
  -- profile turning private (its trigger) never wait for a rebuild.
  perform pg_advisory_xact_lock(hashtext('private.refresh_leaderboards'));

  -- A second rebuild in the same transaction (tests) starts clean.
  if to_regclass('pg_temp.lb_games') is not null then
    drop table pg_temp.lb_games;
  end if;
  if to_regclass('pg_temp.lb_new') is not null then
    drop table pg_temp.lb_new;
  end if;
  create temp table lb_games on commit drop as
  select
    g.user_id,
    case g.game_type when 'GT_BATTLEGROUNDS' then 'solo' else 'duos' end as mode,
    coalesce(substring(g.hero_card_id from '^(.+)_SKIN_[A-Za-z0-9]+$'), g.hero_card_id) as hero,
    g.final_place::numeric as place,
    g.final_place <= case g.game_type when 'GT_BATTLEGROUNDS' then 4 else 2 end as top_half,
    g.final_place = 1 as won,
    g.played_on,
    g.session,
    g.game_index,
    s.apm,
    s.gold_beyond_income,
    s.rolls,
    s.triples,
    s.board_stats_max,
    s.golden_max,
    private.leaderboard_game_plausible(s) and g.played_on <= today + 1 as plausible
  from public.games g
  join public.profiles p on p.user_id = g.user_id
  join public.game_stats s
    on s.user_id = g.user_id and s.session = g.session and s.game_index = g.game_index
  where p.is_public
    and p.display_name is not null
    and private.display_name_allowed(p.display_name)
    and g.status = 'ok'
    and g.final_place is not null;

  select count(*) filter (where not plausible) into n_excluded from pg_temp.lb_games;

  create temp table lb_new on commit drop as
  with periods (period, since) as (
    values ('7d', today - 6), ('30d', today - 29), ('all', null::date)
  ),
  in_period as (
    select per.period, c.*
    from pg_temp.lb_games c
    join periods per on per.since is null or c.played_on >= per.since
  ),
  scoped as (
    select * from in_period where plausible
  ),
  totals as (
    select period, mode, user_id,
      count(*) as games,
      avg(place) as avg_place,
      count(*) filter (where top_half) as top_half,
      count(*) filter (where won) as wins,
      avg(apm) as apm, count(apm) as apm_games,
      avg(gold_beyond_income) as gold, count(gold_beyond_income) as gold_games,
      avg(rolls) as rolls, count(rolls) as rolls_games,
      avg(triples) as triples, count(triples) as triples_games,
      max(board_stats_max) as board, count(board_stats_max) as board_games,
      max(golden_max) as golden, count(golden_max) as golden_games
    from scoped
    group by period, mode, user_id
    having count(*) >= min_games
  ),
  runs as (
    -- Runs of top-half finishes in a row, in play order, within the period.
    -- A left-out game breaks a run: leaving it out never makes a run longer.
    select period, mode, user_id, counted,
      row_number() over (partition by period, mode, user_id order by played_on, session, game_index)
        - row_number() over (
            partition by period, mode, user_id, counted order by played_on, session, game_index
          ) as run
    from (select i.*, i.top_half and i.plausible as counted from in_period i) c
  ),
  streaks as (
    select period, mode, user_id, max(len) as longest
    from (
      select period, mode, user_id, run, count(*) as len
      from runs where counted
      group by period, mode, user_id, run
    ) r
    group by period, mode, user_id
  ),
  heroes as (
    select c.period, c.mode, c.user_id, c.hero, avg(c.place) as avg_place, count(*) as games
    from scoped c
    join totals t using (period, mode, user_id)
    where c.hero is not null
    group by c.period, c.mode, c.user_id, c.hero
    having count(*) >= min_hero
  )
  select t.mode, t.period, b.board, t.user_id, null::text as hero, b.value, b.games
  from totals t
  left join streaks st using (period, mode, user_id)
  cross join lateral (values
    ('avg_place', round(t.avg_place, 2), t.games),
    ('top_half_rate', round(100.0 * t.top_half / t.games, 1), t.games),
    ('win_rate', round(100.0 * t.wins / t.games, 1), t.games),
    ('games', t.games::numeric, t.games),
    ('top_half_streak', coalesce(st.longest, 0)::numeric, t.games),
    ('apm', round(t.apm, 2), t.apm_games),
    ('gold_beyond_income', round(t.gold, 2), t.gold_games),
    ('rolls', round(t.rolls, 2), t.rolls_games),
    ('triples', round(t.triples, 2), t.triples_games),
    ('biggest_board', t.board::numeric, t.board_games),
    ('most_golden', t.golden::numeric, t.golden_games)
  ) as b(board, value, games)
  -- A per-game number needs the same minimum of games that have it.
  where b.value is not null and b.games >= min_games
  union all
  select h.mode, h.period, 'hero_avg_place', h.user_id, h.hero, round(h.avg_place, 2), h.games
  from heroes h;

  -- The swap. The insert reads the profiles as they are now, so a profile
  -- that turned private while the rows were worked out is left out.
  delete from private.leaderboard_rows;
  insert into private.leaderboard_rows (mode, period, board, user_id, hero, value, games)
  select x.mode, x.period, x.board, x.user_id, x.hero, x.value, x.games
  from pg_temp.lb_new x
  join public.profiles p on p.user_id = x.user_id
  where p.is_public and p.display_name is not null;
  get diagnostics n = row_count;

  select count(distinct user_id) into n_players from private.leaderboard_rows;
  insert into private.leaderboard_state (id, refreshed_at, players, excluded_games)
  values (true, now(), n_players, n_excluded)
  on conflict (id) do update set
    refreshed_at = excluded.refreshed_at,
    players = excluded.players,
    excluded_games = excluded.excluded_games;
  return n;
end;
$$;

revoke all on function private.refresh_leaderboards() from public, anon, authenticated, service_role;

-- A profile made private, or whose name is cleared, leaves the snapshot at
-- once (the reads below also
-- check it, so this only keeps the stored copy as small as what is shown).
-- Rows a running rebuild holds are skipped, never waited for: that rebuild
-- deletes them itself, and its swap reads the profiles afresh. Only rows it
-- inserted from an earlier look at the profiles stay stored until the next
-- rebuild, hidden by the reads.
create function private.leaderboard_forget_private()
returns trigger
language plpgsql
security definer
set search_path = ''
as $$
begin
  delete from private.leaderboard_rows r
  where r.ctid in (
    select x.ctid from private.leaderboard_rows x
    where x.user_id = new.user_id
    for update skip locked
  );
  return new;
end;
$$;

revoke all on function private.leaderboard_forget_private() from public, anon, authenticated, service_role;

create trigger profiles_leaderboard_private
  after update of is_public, display_name on public.profiles
  for each row when (
    (old.is_public and not new.is_public)
    or (old.display_name is not null and new.display_name is null)
  )
  execute function private.leaderboard_forget_private();

-- ---------------------------------------------------------------------------
-- Reads (anon and authenticated)
-- ---------------------------------------------------------------------------

-- Every board of one mode and period: standing (equal values share it; lower
-- is better for the place boards, higher for the rest), the display name
-- read now, the hero on the hero board, the value and the games behind it.
-- Each profile is checked as it is read: public, with a name, and the name
-- allowed. Standings count only the rows shown. At most 50 rows per board,
-- listed by standing, then more games first, then account id (stable).
create function public.leaderboard(p_mode text, p_period text)
returns table (
  board text,
  standing integer,
  user_id uuid,
  display_name text,
  hero text,
  value numeric,
  games integer
)
language plpgsql
stable
security definer
set search_path = ''
as $$
begin
  if p_mode is null or p_mode not in ('solo', 'duos') then
    raise exception 'leaderboard: mode is solo or duos' using errcode = '22023';
  end if;
  if p_period is null or p_period not in ('7d', '30d', 'all') then
    raise exception 'leaderboard: period is 7d, 30d or all' using errcode = '22023';
  end if;
  return query
  -- Each player on these boards is checked once (public, named, name allowed):
  -- the players first, then the name check on their rows only.
  with ids as materialized (
    select distinct r.user_id as id
    from private.leaderboard_rows r
    where r.mode = p_mode and r.period = p_period
  ),
  checked as materialized (
    select p.user_id as id, p.display_name as name,
      private.display_name_allowed(p.display_name) as allowed
    from ids
    join public.profiles p on p.user_id = ids.id
    where p.is_public and p.display_name is not null
  ),
  shown as (
    select c.id, c.name from checked c where c.allowed
  )
  select x.board, x.standing, x.user_id, x.name, x.hero, x.value, x.games
  from (
    select r.board, r.user_id, sh.name, r.hero, r.value, r.games,
      (rank() over (partition by r.board order by o.score))::integer as standing,
      row_number() over (
        partition by r.board order by o.score, r.games desc, r.user_id, r.hero
      ) as n
    from private.leaderboard_rows r
    join shown sh on sh.id = r.user_id
    cross join lateral (
      select case when r.board in ('avg_place', 'hero_avg_place') then r.value else -r.value end
        as score
    ) o
    where r.mode = p_mode
      and r.period = p_period
  ) x
  where x.n <= 50
  order by x.board, x.n;
end;
$$;

comment on function public.leaderboard(text, text) is
  'Public leaderboards (T-506): every board of one mode (solo, duos) and period (7d, 30d, all), public profiles with an allowed display name and enough games only. Self-reported data.';

-- When the boards were last rebuilt; null before the first rebuild.
create function public.leaderboard_refreshed_at()
returns timestamptz
language sql
stable
security definer
set search_path = ''
as $$
  select s.refreshed_at from private.leaderboard_state s;
$$;

revoke all on function public.leaderboard(text, text) from public;
revoke all on function public.leaderboard_refreshed_at() from public;
grant execute on function public.leaderboard(text, text) to anon, authenticated, service_role;
grant execute on function public.leaderboard_refreshed_at() to anon, authenticated, service_role;

-- ---------------------------------------------------------------------------
-- Export: as in 20261011120000_game_stats.sql, plus the player's rows in the
-- leaderboard snapshot (worked out from the games already in the export).
-- Account deletion removes them with the profile (foreign key).
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

-- As in 20261011120000_game_stats.sql, plus the leaderboard rows first: a
-- rebuild locks them before it checks the profile, so deletion takes the same
-- order and the two never deadlock (the foreign key would also remove them).
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
-- Schedule: every 10 minutes (pg_cron replaces a job of the same name).
-- ---------------------------------------------------------------------------

select cron.schedule('leaderboards-refresh', '*/10 * * * *', 'select private.refresh_leaderboards()');

select private.refresh_leaderboards();
