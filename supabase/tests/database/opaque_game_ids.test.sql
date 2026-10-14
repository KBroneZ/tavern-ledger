-- Public game ids and file names carry no time (T-586). Run: npx supabase test db
-- Users and games are made up; everything rolls back at the end.
begin;
create extension if not exists pgtap with schema extensions;
select plan(40);

-- A: public, B: private, C: another signed-in user.
insert into auth.users (id, email) values
  ('00000000-0000-4000-8000-00000000000a', 'a@example.test'),
  ('00000000-0000-4000-8000-00000000000b', 'b@example.test'),
  ('00000000-0000-4000-8000-00000000000c', 'c@example.test');
update public.profiles set display_name = 'Alpha', is_public = true
  where user_id = '00000000-0000-4000-8000-00000000000a';

-- A's two games: an old one (the file still has the session in its name) and
-- a new one (opaque name); two more of B.
insert into public.games (user_id, session, game_index, game_type, status, hero_card_id,
  final_place, played_on, saved_at, content_sha256, size_bytes, storage_path)
values
  ('00000000-0000-4000-8000-00000000000a', 'Hearthstone_2026_10_09_18_30_00', 1,
   'GT_BATTLEGROUNDS', 'ok', 'BG36_HERO_000', 1, '2026-10-09', 1791000000, repeat('a', 64), 2500,
   '00000000-0000-4000-8000-00000000000a/Hearthstone_2026_10_09_18_30_00-1.json.gz'),
  ('00000000-0000-4000-8000-00000000000a', 'Hearthstone_2026_10_08_09_00_00', 2,
   'GT_BATTLEGROUNDS', 'ok', 'BG36_HERO_000', 3, '2026-10-08', 1791000000, repeat('b', 64), 2500,
   null),
  ('00000000-0000-4000-8000-00000000000b', 'Hearthstone_2026_10_09_19_30_00', 1,
   'GT_BATTLEGROUNDS', 'ok', 'BG36_HERO_000', 2, '2026-10-09', 1791000000, repeat('c', 64), 2500,
   null);
insert into public.game_stats (user_id, session, game_index, summary_version, rounds,
  combats_won, combats_lost, combats_tied, combats_unknown, heroes_offered)
values
  ('00000000-0000-4000-8000-00000000000a', 'Hearthstone_2026_10_09_18_30_00', 1, 1, 3, 3, 0, 0, 0,
   array['BG36_HERO_000', 'BG36_HERO_001']),
  ('00000000-0000-4000-8000-00000000000a', 'Hearthstone_2026_10_08_09_00_00', 2, 1, 4, 3, 1, 0, 0,
   array['BG36_HERO_000']);

-- ---------------------------------------------------------------- the id
select matches(
  (select storage_path from public.games where game_index = 2 and user_id = '00000000-0000-4000-8000-00000000000a'),
  '^00000000-0000-4000-8000-00000000000a/[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\.json\.gz$',
  'a game with no stored name gets <user>/<public_id>.json.gz'
);
select is(
  (select storage_path from public.games where game_index = 2 and user_id = '00000000-0000-4000-8000-00000000000a'),
  '00000000-0000-4000-8000-00000000000a/'
    || (select public_id from public.games where game_index = 2 and user_id = '00000000-0000-4000-8000-00000000000a')::text
    || '.json.gz',
  'and the name is its own public_id'
);
select is((select count(distinct public_id)::int from public.games), 3, 'every game has its own public_id');
select is(
  (select count(*)::int from public.games where public_id::text like '%2026%' or public_id::text like '%1009%'),
  0, 'no public_id holds a date or time');

-- ---------------------------------------------------------------- what clients can read
select is_empty(
  $$ select r.role, t.tbl, t.col
     from unnest(array['anon', 'authenticated']) as r(role),
       (values ('public.games', 'session'), ('public.games', 'storage_path'),
               ('public.game_stats', 'session')) as t(tbl, col)
     where has_column_privilege(r.role, t.tbl, t.col, 'select') $$,
  'clients cannot read the session or the stored file name from the tables'
);
select ok(
  has_column_privilege('anon', 'public.games', 'public_id', 'select')
  and has_column_privilege('anon', 'public.games', 'played_on', 'select'),
  'public_id and the day are readable'
);

-- ---------------------------------------------------------------- anon
set local role anon;
select set_config('request.jwt.claims', '{"role": "anon"}', true);

select throws_ok($$ select session from public.games $$, '42501', null, 'anon cannot select games.session');
select throws_ok($$ select storage_path from public.games $$, '42501', null, 'anon cannot select games.storage_path');
select throws_ok($$ select session from public.game_stats $$, '42501', null, 'anon cannot select game_stats.session');
select results_eq(
  $$ select game_index, session is null, storage_path ~ '^[0-9a-f-]{36}/[0-9a-f-]{36}\.json\.gz$',
            public_id is not null
     from public.game_summaries order by play_order $$,
  $$ values (2, true, true, true), (1, true, true, true) $$,
  'anon sees the public games in play order, with no session and an opaque file name'
);
select results_eq(
  $$ select public_id::text from public.game_summaries where game_index = 1 $$,
  $$ select public_id::text from public.games where game_index = 1 and user_id = '00000000-0000-4000-8000-00000000000a' $$,
  'the summary names a game by its public_id'
);
select is(
  (select storage_path from public.game_summaries where game_index = 1),
  '00000000-0000-4000-8000-00000000000a/'
    || (select public_id from public.game_summaries where game_index = 1)::text || '.json.gz',
  'even an old game''s public file name is built from the id, never the stored old name'
);
select is_empty($$ select * from public.game_summaries where user_id = '00000000-0000-4000-8000-00000000000b' $$,
  'a private profile''s games stay hidden');
select results_eq(
  $$ select hero, games::int, offered_games::int, picked_games::int from public.user_heroes order by hero $$,
  $$ values ('BG36_HERO_000'::text, 2, 2, 2), ('BG36_HERO_001', 0, 1, 0) $$,
  'hero totals count each game once without the session'
);
select is(
  (select count(*)::int from public.user_records where session is not null),
  0, 'records carry no session for the public');
select ok(
  (select bool_and(public_id is not null) from public.user_records where record = 'rounds'),
  'records name their game by public_id');
select results_eq(
  $$ select game_index from public.user_records where record = 'rounds' $$,
  $$ values (2) $$,
  'the rounds record is the 4-round game');

-- ---------------------------------------------------------------- the owner
reset role;
set local role authenticated;
select set_config('request.jwt.claims',
  '{"sub": "00000000-0000-4000-8000-00000000000a", "role": "authenticated"}', true);
select results_eq(
  $$ select session, storage_path from public.game_summaries where game_index = 1 and user_id = '00000000-0000-4000-8000-00000000000a' $$,
  $$ values ('Hearthstone_2026_10_09_18_30_00'::text,
             '00000000-0000-4000-8000-00000000000a/Hearthstone_2026_10_09_18_30_00-1.json.gz'::text) $$,
  'the owner still sees the session and the real file name of their old game'
);

-- another signed-in user sees the same as anon
select set_config('request.jwt.claims',
  '{"sub": "00000000-0000-4000-8000-00000000000c", "role": "authenticated"}', true);
select is(
  (select count(*)::int from public.game_summaries where session is not null),
  0, 'another signed-in user sees no session');
reset role;

-- ---------------------------------------------------------------- upload_begin
select is(
  (public.upload_begin('00000000-0000-4000-8000-00000000000c', 'Hearthstone_2026_10_10_10_00_00',
    1, 1791000000, repeat('d', 64), 2000) ->> 'result'),
  'create', 'a new game is created');
select matches(
  (public.upload_begin('00000000-0000-4000-8000-00000000000c', 'Hearthstone_2026_10_10_10_00_00',
    1, 1791000000, repeat('d', 64), 2000) ->> 'path'),
  '^00000000-0000-4000-8000-00000000000c/[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\.json\.gz$',
  'upload_begin gives a new game an opaque file name');
select is(
  (public.upload_begin('00000000-0000-4000-8000-00000000000a', 'Hearthstone_2026_10_09_18_30_00',
    1, 1791000001, repeat('e', 64), 2000) ->> 'path'),
  '00000000-0000-4000-8000-00000000000a/'
    || (select public_id from public.games where game_index = 1 and user_id = '00000000-0000-4000-8000-00000000000a')::text
    || '.json.gz',
  'a replaced game is given its opaque name, also when its row still has the old one');

-- ---------------------------------------------------------------- upload_commit
select is(
  public.upload_commit('00000000-0000-4000-8000-00000000000c', 'Hearthstone_2026_10_10_10_00_00', 1,
    'GT_BATTLEGROUNDS', 'ok', 'BG36_HERO_000', 1, 253216, '2026-10-10', '{}'::jsonb,
    1791000000, repeat('d', 64), 2000, '0.1.0+r1', null, null,
    '99999999-9999-4999-8999-999999999999'),
  'created', 'a new game is written with the id upload_begin gave');
select is(
  (select storage_path from public.games where user_id = '00000000-0000-4000-8000-00000000000c'),
  '00000000-0000-4000-8000-00000000000c/99999999-9999-4999-8999-999999999999.json.gz',
  'and its file name is built from that id');
select is(
  public.upload_commit('00000000-0000-4000-8000-00000000000c', 'Hearthstone_2026_10_10_10_00_00', 1,
    'GT_BATTLEGROUNDS', 'ok', 'BG36_HERO_000', 2, 253216, '2026-10-10', '{}'::jsonb,
    1791000009, repeat('f', 64), 2000, '0.1.0+r1', null, null, null),
  'replaced', 'a replacement is written');
select is(
  (select storage_path from public.games where user_id = '00000000-0000-4000-8000-00000000000c'),
  '00000000-0000-4000-8000-00000000000c/99999999-9999-4999-8999-999999999999.json.gz',
  'and keeps its file name and id');
-- An older Edge Function sends no id: the file went to the session's name.
select is(
  public.upload_commit('00000000-0000-4000-8000-00000000000c', 'Hearthstone_2026_10_11_10_00_00', 1,
    'GT_BATTLEGROUNDS', 'ok', 'BG36_HERO_000', 1, 253216, '2026-10-11', '{}'::jsonb,
    1791000000, repeat('a', 64), 2000, '0.1.0+r1'),
  'created', 'an upload from the older function still works');
select is(
  (select storage_path from public.games where session = 'Hearthstone_2026_10_11_10_00_00'),
  '00000000-0000-4000-8000-00000000000c/Hearthstone_2026_10_11_10_00_00-1.json.gz',
  'and its file keeps the session name until the sweep renames it');

-- ---------------------------------------------------------------- renaming the old files
select results_eq(
  $$ select old_path from public.game_files_to_move(100) order by old_path $$,
  $$ values ('00000000-0000-4000-8000-00000000000a/Hearthstone_2026_10_09_18_30_00-1.json.gz'::text),
            ('00000000-0000-4000-8000-00000000000c/Hearthstone_2026_10_11_10_00_00-1.json.gz') $$,
  'the sweep is told only of the files that still have an old name'
);
select is(
  (select new_path from public.game_files_to_move(100)
   where old_path like '%Hearthstone_2026_10_09_18_30_00-1.json.gz'),
  '00000000-0000-4000-8000-00000000000a/'
    || (select public_id from public.games where game_index = 1 and user_id = '00000000-0000-4000-8000-00000000000a')::text
    || '.json.gz',
  'with the name built from the game''s public_id');
select is((select count(*)::int from public.game_files_to_move(1)), 1, 'at most the asked number');

-- The file is under the new name (Storage moved it) before the row says so;
-- it is not an orphan meanwhile, but a stray file is.
insert into storage.objects (bucket_id, name)
select 'games', g.user_id::text || '/' || g.public_id::text || '.json.gz'
from public.games g where g.game_index = 1 and g.user_id = '00000000-0000-4000-8000-00000000000a';
insert into storage.objects (bucket_id, name)
values ('games', '00000000-0000-4000-8000-00000000000a/stray.json.gz');
select results_eq(
  $$ select * from public.sweep_orphan_files(interval '-1 second') $$,
  $$ values ('00000000-0000-4000-8000-00000000000a/stray.json.gz'::text) $$,
  'a file under a game''s new name is not swept before its row is updated'
);

select is(
  public.game_file_moved(
    '00000000-0000-4000-8000-00000000000a/Hearthstone_2026_10_09_18_30_00-1.json.gz',
    '00000000-0000-4000-8000-00000000000a/'
      || (select public_id from public.games where game_index = 1 and user_id = '00000000-0000-4000-8000-00000000000a')::text
      || '.json.gz'),
  true, 'the row is updated once the file is under the new name');
select is(
  public.game_file_moved(
    '00000000-0000-4000-8000-00000000000a/Hearthstone_2026_10_09_18_30_00-1.json.gz',
    '00000000-0000-4000-8000-00000000000a/'
      || (select public_id from public.games where game_index = 1 and user_id = '00000000-0000-4000-8000-00000000000a')::text
      || '.json.gz'),
  false, 'asking again changes nothing');
select is(
  public.game_file_moved(
    '00000000-0000-4000-8000-00000000000c/Hearthstone_2026_10_11_10_00_00-1.json.gz',
    '00000000-0000-4000-8000-00000000000c/not-its-id.json.gz'),
  false, 'a name that is not the game''s own is refused');

-- ---------------------------------------------------------------- the row follows the file
-- C's game from the older function still has the old name; its replacement is
-- written under the opaque one and the row moves to it.
select is(
  public.upload_commit('00000000-0000-4000-8000-00000000000c', 'Hearthstone_2026_10_11_10_00_00', 1,
    'GT_BATTLEGROUNDS', 'ok', 'BG36_HERO_000', 1, 253216, '2026-10-11', '{}'::jsonb,
    1791000009, repeat('b', 64), 2000, '0.1.0+r1', null, null,
    (select public_id from public.games where session = 'Hearthstone_2026_10_11_10_00_00')),
  'replaced', 'a replacement of an old-named game is written');
select is(
  (select storage_path from public.games where session = 'Hearthstone_2026_10_11_10_00_00'),
  '00000000-0000-4000-8000-00000000000c/'
    || (select public_id from public.games where session = 'Hearthstone_2026_10_11_10_00_00')::text
    || '.json.gz',
  'and the row names the opaque file now');

-- Two first uploads of the same game at once: the row names the file of the last copy.
select public.upload_commit('00000000-0000-4000-8000-00000000000c', 'Hearthstone_2026_10_12_10_00_00', 1,
  'GT_BATTLEGROUNDS', 'ok', 'BG36_HERO_000', 1, 253216, '2026-10-12', '{}'::jsonb,
  1791000000, repeat('1', 64), 2000, '0.1.0+r1', null, null, '11111111-1111-4111-8111-111111111111');
select public.upload_commit('00000000-0000-4000-8000-00000000000c', 'Hearthstone_2026_10_12_10_00_00', 1,
  'GT_BATTLEGROUNDS', 'ok', 'BG36_HERO_000', 1, 253216, '2026-10-12', '{}'::jsonb,
  1791000000, repeat('2', 64), 2000, '0.1.0+r1', null, null, '22222222-2222-4222-8222-222222222222');
select is(
  (select storage_path from public.games where session = 'Hearthstone_2026_10_12_10_00_00'),
  '00000000-0000-4000-8000-00000000000c/22222222-2222-4222-8222-222222222222.json.gz',
  'the row names the file written with the last copy''s hash');

-- A game whose file was deleted has nothing to rename.
insert into public.games (user_id, session, game_index, game_type, status, played_on, saved_at,
  content_sha256, size_bytes, storage_path, file_expired_at)
values ('00000000-0000-4000-8000-00000000000b', 'Hearthstone_2026_09_01_10_00_00', 1,
  'GT_BATTLEGROUNDS', 'ok', '2026-09-01', 1791000000, repeat('e', 64), 2000,
  '00000000-0000-4000-8000-00000000000b/Hearthstone_2026_09_01_10_00_00-1.json.gz', now());
select is(
  (select count(*)::int from public.game_files_to_move(500) where old_path like '%Hearthstone_2026_09_01%'),
  0, 'a game with no file is not listed for renaming');

-- ---------------------------------------------------------------- closed to clients
select ok(
  not has_function_privilege('anon', 'public.game_files_to_move(integer)', 'execute')
  and not has_function_privilege('authenticated', 'public.game_files_to_move(integer)', 'execute')
  and not has_function_privilege('anon', 'public.game_file_moved(text, text)', 'execute')
  and not has_function_privilege('authenticated', 'public.game_file_moved(text, text)', 'execute')
  and has_function_privilege('service_role', 'public.game_files_to_move(integer)', 'execute')
  and has_function_privilege('service_role', 'public.game_file_moved(text, text)', 'execute'),
  'only the service role can rename files'
);

select * from finish();
rollback;
