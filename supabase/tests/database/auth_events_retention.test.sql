-- The security log (auth.audit_log_entries, data inventory S5) is exported
-- with the account and trimmed after 90 days by the daily sweep (T-104i,
-- D-048). Run: npx supabase test db
begin;
create extension if not exists pgtap with schema extensions;
select plan(6);

delete from auth.audit_log_entries;
delete from private.upload_events;
delete from private.upload_ip_failures;

insert into auth.users (id, email) values
  ('00000000-0000-4000-8000-0000000000c1', 'c1@example.test');
-- Shapes as Supabase Auth writes them: actor_id for the user's own actions,
-- traits.user_id when an admin or the system acts on the account.
insert into auth.audit_log_entries (id, payload, created_at, ip_address) values
  (gen_random_uuid(), '{"action": "login", "actor_id": "00000000-0000-4000-8000-0000000000c1",
     "log_type": "account", "traits": {"provider": "email"}}', now() - interval '91 days', '192.0.2.1'),
  (gen_random_uuid(), '{"action": "user_recovery_requested", "actor_id": "00000000-0000-4000-8000-0000000000c1",
     "log_type": "user"}', now() - interval '89 days', '192.0.2.1'),
  (gen_random_uuid(), '{"action": "login", "actor_id": "00000000-0000-4000-8000-0000000000c1",
     "log_type": "account", "traits": {"provider": "email"}}', now(), '192.0.2.1'),
  (gen_random_uuid(), '{"action": "user_invited", "actor_id": "00000000-0000-0000-0000-000000000000",
     "traits": {"user_id": "00000000-0000-4000-8000-0000000000c1"}}', now() - interval '120 days', ''),
  (gen_random_uuid(), '{"action": "token_revoked"}', now() - interval '100 days', '');

-- ---------------------------------------------------------------- export
set local role authenticated;
select set_config('request.jwt.claims',
  '{"sub": "00000000-0000-4000-8000-0000000000c1", "role": "authenticated"}', true);
create temp table export_c1 on commit drop as select public.export_my_data() as doc;
reset role;

select is((select jsonb_array_length(doc -> 'auth_events') from export_c1), 4,
  'the export holds every security log entry of the account, by actor or by target');
select is((select doc -> 'auth_events' -> 0 ->> 'ip_address' from export_c1), '',
  'entries are in time order, with their IP address');

-- ---------------------------------------------------------------- sweep
select is((public.sweep_housekeeping() ->> 'auth_events')::int, 3,
  'the sweep removes entries older than 90 days, of any account or none');
select is(
  (select array_agg(payload ->> 'action' order by created_at) from auth.audit_log_entries),
  array['user_recovery_requested', 'login'],
  'entries from the last 90 days stay');
select is((public.sweep_housekeeping() ->> 'auth_events')::int, 0,
  'a second sweep finds nothing more to remove');

set local role authenticated;
select set_config('request.jwt.claims',
  '{"sub": "00000000-0000-4000-8000-0000000000c1", "role": "authenticated"}', true);
select is(jsonb_array_length(public.export_my_data() -> 'auth_events'), 2,
  'the export after the sweep holds the remaining entries');
reset role;

select * from finish();
rollback;
