-- Secure by default: a new object in `public` reaches no client until a
-- migration grants it.
--
-- Supabase's default privileges give `anon` and `authenticated` every right on
-- a new `public` table, sequence and function, and PostgreSQL gives PUBLIC
-- EXECUTE on every new function. Each migration so far revoked by hand; one
-- that forgot would have opened its object. These defaults apply to objects
-- that `postgres` (the migration role) creates from now on. Existing objects
-- keep the grants they have; supabase/tests/database/lockdown.test.sql lists
-- them.
--
-- `service_role` keeps its defaults (it bypasses row-level security by
-- design). The function default is not schema-scoped because PUBLIC's EXECUTE
-- comes from PostgreSQL's built-in default, which only a global entry can
-- remove. A new function in any schema is therefore callable by its owner only;
-- grant EXECUTE to the roles that need it (a check constraint that calls a
-- helper runs as the writing role).

alter default privileges for role postgres in schema public
  revoke all on tables from anon, authenticated;
alter default privileges for role postgres in schema public
  revoke all on sequences from anon, authenticated;
alter default privileges for role postgres in schema public
  revoke all on functions from anon, authenticated;
alter default privileges for role postgres
  revoke execute on functions from public;
