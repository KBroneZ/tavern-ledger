-- Community stats count only trusted accounts (profile at least 7 days old):
-- accounts made in one go cannot make a group of players. Run: npx supabase test db
begin;
create extension if not exists pgtap with schema extensions;
select plan(11);

alter default privileges for role postgres grant execute on functions to public;

-- P1..P5 opted in, all with a hero H (10 games each, yesterday). P1 is old;
-- P2..P5 are sybil-like: made today. P6 is another young account.
create function pg_temp.pid(i int) returns uuid language sql immutable as $$
  select ('00000000-0000-4000-8000-' || lpad(to_hex(12288 + i), 12, '0'))::uuid
$$;
insert into auth.users (id, email)
select pg_temp.pid(i), 'ct' || i || '@example.test' from generate_series(1, 6) as i;
insert into public.community_consent (user_id, hero_stats, policy_version)
select pg_temp.pid(i), true, '2026-10-11' from generate_series(1, 6) as i;
update public.profiles set created_at = now() - interval '30 days' where user_id = pg_temp.pid(1);

create function pg_temp.seed(uid uuid, n int, hero text) returns int language sql as $$
  select count(*)::int from generate_series(1, n) as i,
  lateral public.upload_commit(
    uid, 'Hearthstone_2026_10_09_18_30_00', i, 'GT_BATTLEGROUNDS', 'ok', hero,
    ((i - 1) % 8) + 1, 253216, current_date - 1, '{}'::jsonb, 1791000000,
    repeat('a', 64), 2000, '0.1.0+r7', '{"rounds": 10}'::jsonb, 1
  ) as r
  where r = 'created';
$$;

select is((select sum(pg_temp.seed(pg_temp.pid(i), 10, 'BG20_HERO_202'))::int from generate_series(1, 5) as i), 50,
  '50 games of one hero from 5 opted-in accounts, 4 of them made today');

select private.refresh_community_heroes();
select is((select untrusted_games from private.community_state), 40,
  'the rebuild counts the games it left out for young accounts');
select is((select players from private.community_hero_rows
           where period = '30d' and hero = 'BG20_HERO_202'), 1,
  'only the old account is a player; the others do not count');
set local role anon;
select set_config('request.jwt.claims', '{"role": "anon"}', true);
select is_empty($$ select * from public.community_heroes() $$,
  '4 new accounts plus 1 real player show no community row');
reset role;

-- The same accounts a week on.
update public.profiles set created_at = now() - interval '8 days'
where user_id in (select pg_temp.pid(i) from generate_series(2, 5) as i);
select private.refresh_community_heroes();
set local role anon;
select set_config('request.jwt.claims', '{"role": "anon"}', true);
select results_eq($$ select hero, games, players from public.community_heroes() where period = '30d' $$,
  $$ values ('BG20_HERO_202'::text, 50, 5) $$,
  'once 7 days old they count (5 players, 50 games)');
reset role;

-- Boundary: a little under 7 days is still too young; exactly 7 days counts.
update public.profiles set created_at = now() - interval '6 days 23 hours' where user_id = pg_temp.pid(5);
select private.refresh_community_heroes();
select is((select players from private.community_hero_rows where period = '30d' and hero = 'BG20_HERO_202'), 4,
  'an account a little under 7 days old does not count');
update public.profiles set created_at = now() - interval '7 days' where user_id = pg_temp.pid(5);
select private.refresh_community_heroes();
select is((select players from private.community_hero_rows where period = '30d' and hero = 'BG20_HERO_202'), 5,
  'at exactly 7 days it counts');

-- A flood of new accounts' games moves no number.
select pg_temp.seed(pg_temp.pid(6), 30, 'BG20_HERO_202');
select private.refresh_community_heroes();
select is((select games from private.community_hero_rows where period = '30d' and hero = 'BG20_HERO_202'), 50,
  'games of a young account change no number');

-- Clients cannot move their own account age.
select ok(
  not has_column_privilege('authenticated', 'public.profiles', 'created_at', 'update')
  and not has_column_privilege('anon', 'public.profiles', 'created_at', 'update'),
  'no client can write profiles.created_at, which the trust bar reads');
select ok(private.community_min_account_age() = interval '7 days'
  and private.community_min_players() = 5,
  'the bar is 7 days, and still 5 players');
select ok(
  not has_function_privilege('anon', 'private.community_min_account_age()', 'execute')
  and not has_function_privilege('authenticated', 'private.community_min_account_age()', 'execute'),
  'the helper is closed to clients');

select * from finish();
rollback;
