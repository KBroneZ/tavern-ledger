-- Export and the row part of deletion (T-104a). Run: npx supabase test db
-- The full deletion, files and auth user included, is tested against the
-- running stack in tests/test_supabase_local.py.
begin;
create extension if not exists pgtap with schema extensions;
select plan(12);

insert into auth.users (id, email) values
  ('00000000-0000-4000-8000-00000000000a', 'a@example.test'),
  ('00000000-0000-4000-8000-00000000000b', 'b@example.test');
insert into auth.identities (provider_id, user_id, identity_data, provider)
values ('00000000-0000-4000-8000-00000000000a', '00000000-0000-4000-8000-00000000000a',
        '{"sub": "00000000-0000-4000-8000-00000000000a", "email": "a@example.test"}', 'email');
insert into auth.audit_log_entries (id, payload)
values (gen_random_uuid(), '{"action": "login", "actor_id": "00000000-0000-4000-8000-00000000000a"}'),
       (gen_random_uuid(), '{"action": "login", "actor_id": "00000000-0000-4000-8000-00000000000b"}');

insert into public.games (user_id, session, game_index, game_type, status, hero_card_id,
  final_place, build, played_on, saved_at, content_sha256, size_bytes)
select u, 'Hearthstone_2026_10_09_18_30_00', i, 'GT_BATTLEGROUNDS', 'ok', 'BG20_HERO_202',
  i, 253216, '2026-10-09', 1791000000, repeat('c', 64), 2000
from (values ('00000000-0000-4000-8000-00000000000a'::uuid),
             ('00000000-0000-4000-8000-00000000000b'::uuid)) as v(u),
     generate_series(1, 2) as i;
insert into storage.objects (bucket_id, name) values
  ('games', '00000000-0000-4000-8000-00000000000a/Hearthstone_2026_10_09_18_30_00-1.json.gz'),
  ('games', '00000000-0000-4000-8000-00000000000b/Hearthstone_2026_10_09_18_30_00-1.json.gz');

-- ---------------------------------------------------------------- export as A
set local role authenticated;
select set_config('request.jwt.claims',
  '{"sub": "00000000-0000-4000-8000-00000000000a", "role": "authenticated"}', true);
create temp table export_a on commit drop as select public.export_my_data() as doc;
reset role;

select is((select doc -> 'account' ->> 'email' from export_a), 'a@example.test',
  'export has the account email');
select is((select jsonb_array_length(doc -> 'identities') from export_a), 1,
  'export has the sign-in identities');
select is((select jsonb_array_length(doc -> 'auth_events') from export_a), 1,
  'export has the user''s auth events and no one else''s');
select is((select doc -> 'profile' ->> 'user_id' from export_a),
  '00000000-0000-4000-8000-00000000000a', 'export has the profile');
select is((select jsonb_array_length(doc -> 'games') from export_a), 2,
  'export has every game row of the user and no one else''s');
select is(
  (select doc -> 'games' -> 0 ?& array(
     select column_name::text from information_schema.columns
     where table_schema = 'public' and table_name = 'games') from export_a),
  true, 'exported game rows carry every column'
);
select is((select doc -> 'files' -> 0 ->> 'name' from export_a),
  '00000000-0000-4000-8000-00000000000a/Hearthstone_2026_10_09_18_30_00-1.json.gz',
  'export lists the user''s files');
select ok(
  (select doc::text !~ '0000000b|encrypted_password|b@example' from export_a),
  'export holds nothing of another user and no password hash'
);

-- ---------------------------------------------------------------- delete rows
set local role service_role;
select results_eq(
  $$ select * from public.user_file_names('00000000-0000-4000-8000-00000000000a') $$,
  $$ values ('00000000-0000-4000-8000-00000000000a/Hearthstone_2026_10_09_18_30_00-1.json.gz'::text) $$,
  'user_file_names lists only that user''s files'
);
select throws_ok(
  $$ select public.delete_user_data('00000000-0000-4000-8000-00000000000a') $$,
  '55000', null, 'rows are not deleted while files remain'
);
reset role;

-- Stand in for the Storage API removing A's file.
set local storage.allow_delete_query = 'true';
delete from storage.objects
where name like '00000000-0000-4000-8000-00000000000a/%';

set local role service_role;
select is(
  public.delete_user_data('00000000-0000-4000-8000-00000000000a'),
  '{"games": 2, "profiles": 1, "auth_events": 1}'::jsonb,
  'delete_user_data removes the games, the profile and the auth events'
);
reset role;

select results_eq(
  $$ select (select count(*) from public.games where user_id = '00000000-0000-4000-8000-00000000000a')
          + (select count(*) from public.profiles where user_id = '00000000-0000-4000-8000-00000000000a')
          + (select count(*) from auth.audit_log_entries
             where payload ->> 'actor_id' = '00000000-0000-4000-8000-00000000000a')
          + (select count(*) from public.games where user_id = '00000000-0000-4000-8000-00000000000b') $$,
  $$ values (2::bigint) $$,
  'nothing of A remains in the rows, and B''s games are untouched'
);

select * from finish();
rollback;
