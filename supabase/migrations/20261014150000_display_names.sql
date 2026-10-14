-- Stricter display names and unique public names (T-572).
--
-- * A name is normalised before the word filter: Unicode NFKC, lower case,
--   marks stripped (NFD), look-alikes from Cyrillic and Greek (and a few Latin
--   letters that do not decompose) folded to Latin, and repeated letters
--   collapsed. "fúck", "fuuck" and a Cyrillic "с" inside "fuсk" now match.
-- * The characters a new name may use are an explicit list: Latin letters
--   with accents (U+00C0-U+017F without × and ÷), A-Z, digits, space and
--   _ - . only. Other alphabets are refused; a name saved earlier is left
--   alone until it is changed, but the boards leave it off.
-- * Two public profiles cannot share a name (same letters once look-alikes,
--   accents, case and separators are ignored). Names that already clash are
--   kept: every public profile but the oldest of its group is marked
--   (display_name_legacy_dup) and left out of the index until its owner picks
--   another name, so this migration never fails on existing data.

-- ---------------------------------------------------------------------------
-- Normalisation
-- ---------------------------------------------------------------------------

-- The comparable form of a name: lower case, NFKC, marks stripped, look-alikes
-- folded to Latin. Separators and repeats are kept (callers decide).
create function private.name_skeleton(p_name text)
returns text
language sql
immutable
set search_path = ''
as $$
  select replace(replace(replace(
    translate(
      regexp_replace(normalize(lower(normalize(p_name, nfkc)), nfd), '[̀-ͯ]', '', 'g'),
      -- Cyrillic, Greek, then Latin letters without a decomposition.
      'авекмнорстухіјѕһԁԛԝѵѡαβεικνορτυχγωμηøđłħıðþ',
      'abekmhopctyxijshdqwvwabeikvoptuxywunodlhidp'),
    'ß', 'ss'), 'æ', 'ae'), 'œ', 'oe');
$$;

-- The key two public names are compared by: the skeleton without separators.
create function private.display_name_key(p_name text)
returns text
language sql
immutable
security definer
set search_path = ''
as $$
  select regexp_replace(private.name_skeleton(p_name), '[ _.-]+', '', 'g');
$$;

-- True when every character is on the list (see the top of this file).
create function private.display_name_chars_ok(p_name text)
returns boolean
language sql
immutable
set search_path = ''
as $$
  select p_name ~ '^[A-Za-z0-9À-ÖØ-öø-ſ _.-]+$';
$$;

revoke all on function private.name_skeleton(text) from public, anon, authenticated, service_role;
-- The unique index below calls it as whoever changes a profile.
revoke all on function private.display_name_key(text) from public, anon, authenticated, service_role;
grant execute on function private.display_name_key(text) to authenticated, service_role;
revoke all on function private.display_name_chars_ok(text) from public, anon, authenticated, service_role;

-- The spellings a name is checked in: its skeleton as it is and with digits
-- read as letters (0 o, 1 i or l, 3 e, 4 a, 5 s, 7 t, 8 b, 9 g).
create or replace function private.name_spellings(p_name text)
returns text[]
language sql
immutable
set search_path = ''
as $$
  select array[n, translate(n, '01345789', 'oieastbg'), translate(n, '01345789', 'oleastbg')]
  from (select private.name_skeleton(p_name) as n) s;
$$;

-- True when no listed word is in the name; true for no name. Each spelling's
-- joined-up form and words are worked out once (also with repeated letters
-- collapsed, for words of 3 letters or more once collapsed), then matched
-- against the list.
create function private.display_name_words_ok(p_name text)
returns boolean
language sql
stable
security definer
set search_path = ''
as $$
  with forms as materialized (
    select v.joined, v.words,
      regexp_replace(v.joined, '(.)\1+', '\1', 'g') as joined_c,
      array(select regexp_replace(w, '(.)\1+', '\1', 'g') from unnest(v.words) as w) as words_c
    from (
      select regexp_replace(n, '[ _.-]+', '', 'g') as joined,
        regexp_split_to_array(n, '[ _.-]+') as words
      from unnest(private.name_spellings(p_name)) as n
    ) v
  ),
  listed as materialized (
    select b.word, b.match, regexp_replace(b.word, '(.)\1+', '\1', 'g') as word_c
    from private.name_blocklist b
  )
  select p_name is null or not exists (
    select 1
    from forms f
    join listed b on case b.match
      when 'part' then strpos(f.joined, b.word) > 0
        or (length(b.word_c) >= 3 and strpos(f.joined_c, b.word_c) > 0)
      else b.word = any (f.words) or b.word = f.joined
        or (length(b.word_c) >= 3 and (b.word_c = any (f.words_c) or b.word_c = f.joined_c))
    end
  );
$$;

-- Words and characters together: what the boards need (a name saved before
-- the character list is left off them).
create or replace function private.display_name_allowed(p_name text)
returns boolean
language sql
stable
security definer
set search_path = ''
as $$
  select p_name is null
    or (private.display_name_chars_ok(p_name) and private.display_name_words_ok(p_name));
$$;

revoke all on function private.name_spellings(text) from public, anon, authenticated, service_role;
revoke all on function private.display_name_words_ok(text) from public, anon, authenticated, service_role;
revoke all on function private.display_name_allowed(text) from public, anon, authenticated, service_role;

-- ---------------------------------------------------------------------------
-- Unique public names
-- ---------------------------------------------------------------------------

alter table public.profiles
  add column display_name_legacy_dup boolean not null default false;

comment on column public.profiles.display_name_legacy_dup is
  'Set once by the migration that made public names unique, on a public profile whose name clashed with an older one; cleared when the name changes. Not readable by clients.';

-- Marks every public profile but the oldest of each group of clashing names.
create function private.flag_legacy_duplicate_names()
returns integer
language sql
security definer
set search_path = ''
as $$
  with flagged as (
    update public.profiles p
    set display_name_legacy_dup = true
    from (
      select user_id,
        row_number() over (
          partition by private.display_name_key(display_name) order by created_at, user_id
        ) as n
      from public.profiles
      where is_public and display_name is not null
    ) d
    where d.user_id = p.user_id and d.n > 1
    returning 1
  )
  select count(*)::integer from flagged;
$$;

revoke all on function private.flag_legacy_duplicate_names() from public, anon, authenticated, service_role;

select private.flag_legacy_duplicate_names();

create unique index profiles_public_display_name_key
  on public.profiles (private.display_name_key(display_name))
  where is_public and display_name is not null and not display_name_legacy_dup;

-- The name check for a new or changed name (only a changed name is checked,
-- so an old name that fails today never blocks making the profile private).
-- The messages are fixed keys the account page recognises; nothing from the
-- word list goes back to the caller.
create or replace function private.check_display_name()
returns trigger
language plpgsql
security definer
set search_path = ''
as $$
begin
  if tg_op = 'UPDATE' and new.display_name is not distinct from old.display_name then
    return new;
  end if;
  new.display_name_legacy_dup := false;
  if not private.display_name_words_ok(new.display_name) then
    raise exception 'display_name_not_allowed'
      using errcode = '23514', hint = 'Choose another display name.';
  end if;
  if not private.display_name_allowed(new.display_name) then
    raise exception 'display_name_chars'
      using errcode = '23514', hint = 'Use letters, digits, spaces, _ . or - only.';
  end if;
  return new;
end;
$$;
