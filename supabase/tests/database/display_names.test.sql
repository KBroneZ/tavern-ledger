-- Display name normalisation, character list and unique public names (T-572).
-- Run: npx supabase test db. Users are made up; everything rolls back.
begin;
create extension if not exists pgtap with schema extensions;
select plan(25);

insert into auth.users (id, email) values
  ('00000000-0000-4000-8000-00000000000a', 'a@example.test'),
  ('00000000-0000-4000-8000-00000000000b', 'b@example.test'),
  ('00000000-0000-4000-8000-00000000000c', 'c@example.test'),
  ('00000000-0000-4000-8000-00000000000d', 'd@example.test');

-- Tries a display name (and visibility) as player A or B; the message, or 'saved'.
create function pg_temp.try_name(who text, name text, pub boolean default false) returns text
language plpgsql as $$
begin
  update public.profiles set display_name = name, is_public = pub
  where user_id = ('00000000-0000-4000-8000-00000000000' || who)::uuid;
  return 'saved';
exception
  when check_violation then return sqlerrm;
  when unique_violation then return 'taken';
end;
$$;

-- ---------------------------------------------------------------- normalisation
select is(private.name_skeleton('Ｆúсk'), 'fuck', 'full-width, accent and a Cyrillic с read as plain Latin');
select is(private.name_skeleton('Аdmіn'), 'admin', 'Cyrillic look-alikes fold to Latin');
select is(private.name_skeleton('ΡΑΟ'), 'pao', 'Greek look-alikes fold to Latin');
select is(private.name_skeleton('Łukasz Ørn ß'), 'lukasz orn ss', 'letters without a decomposition fold too');
select is(private.display_name_key('Á_l.i-ce'), 'alice', 'the key ignores case, accents and separators');

-- ---------------------------------------------------------------- the word filter, bypasses
select results_eq(
  $$ select n, pg_temp.try_name('a', n) from unnest(array[
       'fuсk', 'Аdmin', 'fuuck', 'fúck', 'fuuuuck', 'ѕһіt', 'Ａｄｍｉｎ', 'sstaff', 'f ú c k']) as n $$,
  $$ select n, 'display_name_not_allowed' from unnest(array[
       'fuсk', 'Аdmin', 'fuuck', 'fúck', 'fuuuuck', 'ѕһіt', 'Ａｄｍｉｎ', 'sstaff', 'f ú c k']) as n $$,
  'Cyrillic, accented, repeated and full-width spellings of listed words are refused');

-- ---------------------------------------------------------------- the character list
select results_eq(
  $$ select n, pg_temp.try_name('a', n) from unnest(array[
       'Álvaro Ñ', 'José_2', 'Zoë Øster', 'Łukasz', 'Weiß-Ünal', 'Ana.Maria', 'As', 'Class Act']) as n $$,
  $$ select n, 'saved' from unnest(array[
       'Álvaro Ñ', 'José_2', 'Zoë Øster', 'Łukasz', 'Weiß-Ünal', 'Ana.Maria', 'As', 'Class Act']) as n $$,
  'ordinary accented Latin names are kept');
select results_eq(
  $$ select n, pg_temp.try_name('a', n) from unnest(array[
       'Иван', 'Αλέξης', '日本語', 'Name#1234', 'Bob ×', 'Bob😀', 'Cafe' || U&'\0301',
       'a' || chr(8203) || 'b', 'Ａｂｃ']) as n $$,
  $$ select n, 'display_name_chars' from unnest(array[
       'Иван', 'Αλέξης', '日本語', 'Name#1234', 'Bob ×', 'Bob😀', 'Cafe' || U&'\0301',
       'a' || chr(8203) || 'b', 'Ａｂｃ']) as n $$,
  'other alphabets, symbols, a decomposed accent and invisible characters are refused');
select is(pg_temp.try_name('a', null), 'saved', 'clearing the name always works');
select ok(private.display_name_allowed('Zoë') and not private.display_name_allowed('Зоя')
  and private.display_name_allowed(null),
  'the boards'' check includes the character list');

-- A name saved before the list (a Cyrillic one) never blocks other changes.
alter table public.profiles disable trigger profiles_display_name_allowed;
update public.profiles set display_name = 'Иван' where user_id = '00000000-0000-4000-8000-00000000000b';
alter table public.profiles enable trigger profiles_display_name_allowed;
select lives_ok(
  $$ update public.profiles set is_public = true where user_id = '00000000-0000-4000-8000-00000000000b' $$,
  'an old name that fails the list does not stop the owner making the profile public');
update public.profiles set is_public = false, display_name = null
  where user_id = '00000000-0000-4000-8000-00000000000b';

-- ---------------------------------------------------------------- unique public names
select is(pg_temp.try_name('a', 'Alice', true), 'saved', 'a public name is saved');
select results_eq(
  $$ select n, pg_temp.try_name('b', n, true) from unnest(array[
       'Alice', 'alice', 'ALICE', 'Álice', 'A l i c e', 'a_l.i-c e']) as n $$,
  $$ select n, 'taken' from unnest(array[
       'Alice', 'alice', 'ALICE', 'Álice', 'A l i c e', 'a_l.i-c e']) as n $$,
  'a second public profile cannot take the name, however it is spelled');
select is(pg_temp.try_name('b', 'Alice2', true), 'saved', 'a different name is fine');
select is(pg_temp.try_name('b', 'Alice', false), 'saved', 'a private profile may hold the same name');
select throws_ok(
  $$ update public.profiles set is_public = true where user_id = '00000000-0000-4000-8000-00000000000b' $$,
  '23505', null, 'but cannot become public with it');
update public.profiles set display_name = null where user_id = '00000000-0000-4000-8000-00000000000b';
select is(pg_temp.try_name('a', 'Alice', true), 'saved', 'the owner can save their own name again');

-- ---------------------------------------------------------------- names that clashed before
-- Two public profiles with the same name, made as the old rule allowed.
drop index public.profiles_public_display_name_key;
update public.profiles set created_at = '2026-01-01', display_name = 'Dupe', is_public = true
  where user_id = '00000000-0000-4000-8000-00000000000c';
update public.profiles set created_at = '2026-02-01', display_name = 'dupe', is_public = true
  where user_id = '00000000-0000-4000-8000-00000000000d';

select is(private.flag_legacy_duplicate_names(), 1, 'one profile of the clashing pair is marked');
select results_eq(
  $$ select user_id::text, display_name_legacy_dup from public.profiles
     where display_name ilike 'dupe' order by created_at $$,
  $$ values ('00000000-0000-4000-8000-00000000000c', false),
            ('00000000-0000-4000-8000-00000000000d', true) $$,
  'the oldest keeps the name unmarked, the later one is marked');
select lives_ok(
  $$ create unique index profiles_public_display_name_key
       on public.profiles (private.display_name_key(display_name))
       where is_public and display_name is not null and not display_name_legacy_dup $$,
  'the index builds although two names clash');
select is(
  (select count(*)::int from public.profiles where display_name ilike 'dupe' and is_public), 2,
  'both profiles stay public with their name');
select throws_ok(
  $$ update public.profiles set display_name = 'DUPE', is_public = true
     where user_id = '00000000-0000-4000-8000-00000000000b' $$,
  '23505', null, 'a new public profile cannot take a name that is in use');

-- Renaming the marked profile clears the mark, and the new name is covered.
update public.profiles set display_name = 'Dupe Two' where user_id = '00000000-0000-4000-8000-00000000000d';
select is(
  (select display_name_legacy_dup from public.profiles where user_id = '00000000-0000-4000-8000-00000000000d'),
  false, 'a new name clears the mark');
select throws_ok(
  $$ update public.profiles set display_name = 'Dupe'
     where user_id = '00000000-0000-4000-8000-00000000000d' $$,
  '23505', null, 'and the old name is then covered like any other');

-- ---------------------------------------------------------------- privileges
select ok(
  not has_function_privilege('anon', 'private.display_name_key(text)', 'execute')
  and not has_function_privilege('anon', 'private.display_name_allowed(text)', 'execute')
  and not has_function_privilege('authenticated', 'private.display_name_words_ok(text)', 'execute')
  and not has_function_privilege('authenticated', 'private.flag_legacy_duplicate_names()', 'execute')
  and not has_column_privilege('authenticated', 'public.profiles', 'display_name_legacy_dup', 'select,update'),
  'the helpers and the mark are not reachable from the API');

select * from finish();
rollback;
