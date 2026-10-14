-- The data export keeps the account's own security log events and drops who
-- and where from for events someone else caused. Run: npx supabase test db
begin;
create extension if not exists pgtap with schema extensions;
select plan(7);

delete from auth.audit_log_entries;
insert into auth.users (id, email) values
  ('00000000-0000-4000-8000-0000000000e1', 'e1@example.test');
insert into auth.audit_log_entries (id, payload, created_at, ip_address) values
  (gen_random_uuid(), '{"action": "login", "actor_id": "00000000-0000-4000-8000-0000000000e1",
     "actor_username": "e1@example.test", "log_type": "account", "traits": {"provider": "email"}}',
     now() - interval '3 days', '192.0.2.1'),
  (gen_random_uuid(), '{"action": "user_modified", "actor_id": "00000000-0000-4000-8000-0000000000ad",
     "actor_username": "admin@example.test", "actor_name": "Admin", "actor_via_sso": false,
     "log_type": "team", "traits": {"user_id": "00000000-0000-4000-8000-0000000000e1"}}',
     now() - interval '2 days', '203.0.113.9'),
  (gen_random_uuid(), '{"action": "user_invited", "traits": {"user_id": "00000000-0000-4000-8000-0000000000e1"}}',
     now() - interval '1 day', '');

set local role authenticated;
select set_config('request.jwt.claims',
  '{"sub": "00000000-0000-4000-8000-0000000000e1", "role": "authenticated"}', true);
create temp table export_e1 on commit drop as select public.export_my_data() as doc;
reset role;

select is((select jsonb_array_length(doc -> 'auth_events') from export_e1), 3,
  'every event stays in the export');
select is((select doc -> 'auth_events' -> 0 -> 'payload' ->> 'actor_username' from export_e1), 'e1@example.test',
  'the account''s own event keeps its actor name');
select is((select doc -> 'auth_events' -> 0 ->> 'ip_address' from export_e1), '192.0.2.1',
  'and its IP address');
select ok(
  not (select doc -> 'auth_events' -> 1 -> 'payload' ?| array['actor_id', 'actor_username', 'actor_name', 'actor_via_sso']
       from export_e1),
  'an admin''s action on the account loses the admin''s id, name and sign-in method');
select is((select doc -> 'auth_events' -> 1 ->> 'ip_address' from export_e1), null,
  'and the admin''s IP address');
select is((select doc -> 'auth_events' -> 1 -> 'payload' ->> 'action' from export_e1), 'user_modified',
  'while what happened stays');
select ok((select doc::text !~ 'admin@example\.test|203\.113\.9' from export_e1),
  'nothing about the admin is anywhere in the export');

select * from finish();
rollback;
