-- T-104a: profiles, game summary rows and the private `games` bucket.
--
-- Rules this schema enforces (see docs/research/web-stack.md, D-012, D-017, D-020):
-- * Clients (anon, authenticated) only read. They read their own rows and the
--   rows of profiles the owner made public. The only client write is a user
--   changing their own display name and visibility.
-- * Game rows and files are written by the upload Edge Function with the
--   service role, never by clients.
-- * No player names, BattleTags, ratings or MMR anywhere: there is no column
--   for them, and the display name rejects BattleTag-shaped values.

-- Helpers live in a schema the API does not expose.
create schema if not exists private;
revoke all on schema private from public, anon, authenticated;
grant usage on schema private to service_role;

-- ---------------------------------------------------------------------------
-- Validation helpers (immutable, so CHECK constraints can use them)
-- ---------------------------------------------------------------------------

-- Tribes offered in the tavern: {"BEAST": 10, "NAGA": 7, ...}. Upper-case
-- tribe tokens as the log prints them, counts 0..10000, at most 32 keys.
create function private.is_tribe_counts(counts jsonb)
returns boolean
language sql
immutable
set search_path = ''
as $$
  -- CASE, not AND/OR: Postgres does not promise to evaluate those in order,
  -- and jsonb_each or a numeric cast on the wrong type would raise an error.
  select case
    when jsonb_typeof(counts) <> 'object' then false
    when (select count(*) from jsonb_object_keys(counts)) > 32 then false
    else not exists (
      select 1
      from jsonb_each(counts) as t(tribe, n)
      where case
        when t.tribe !~ '^[A-Z_]{1,32}$' then true
        when jsonb_typeof(t.n) <> 'number' then true
        else (t.n)::numeric <> trunc((t.n)::numeric)
          or (t.n)::numeric < 0
          or (t.n)::numeric > 10000
      end
    )
  end;
$$;

-- ---------------------------------------------------------------------------
-- profiles
-- ---------------------------------------------------------------------------

create table public.profiles (
  user_id uuid primary key references auth.users (id) on delete cascade,
  -- Chosen by the user; null until they pick one. Letters, digits, spaces,
  -- '_', '.' and '-' only: no '#' or look-alikes, so a BattleTag (Name#1234)
  -- cannot be stored as a display name.
  display_name text
    check (
      display_name is null
      or (
        char_length(display_name) between 1 and 32
        and display_name = btrim(display_name)
        and display_name ~ '^[[:alnum:] _.-]+$'
      )
    ),
  is_public boolean not null default false,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

comment on table public.profiles is
  'One row per account. Private by default; is_public exposes the profile and its games to everyone.';

-- ---------------------------------------------------------------------------
-- games: one summary row per uploaded game; the full record is a gzipped
-- JSON file in the `games` bucket at storage_path.
-- ---------------------------------------------------------------------------

create table public.games (
  user_id uuid not null references public.profiles (user_id) on delete cascade,
  -- Log folder name, e.g. Hearthstone_2026_10_09_18_30_00.
  session text not null
    check (session ~ '^Hearthstone_[0-9]{4}(_[0-9]{2}){5}$'),
  game_index integer not null check (game_index between 1 and 1000),
  game_type text not null
    check (game_type in ('GT_BATTLEGROUNDS', 'GT_BATTLEGROUNDS_DUO')),
  status text not null
    check (status in ('ok', 'incomplete', 'unsupported')),
  hero_card_id text check (hero_card_id ~ '^[A-Za-z0-9_]{1,64}$'),
  -- Solo places are 1-8; Duos places are per team, 1-4. Null when unknown.
  final_place smallint
    check (
      final_place is null
      or (game_type = 'GT_BATTLEGROUNDS' and final_place between 1 and 8)
      or (game_type = 'GT_BATTLEGROUNDS_DUO' and final_place between 1 and 4)
    ),
  build integer check (build between 1 and 100000000),
  -- Local start time of the game client, from the session name.
  played_on date not null,
  tribes_offered jsonb not null default '{}'::jsonb
    check (private.is_tribe_counts(tribes_offered)),
  -- Revision from the desktop history (seconds since 1970); an upload with
  -- an older revision than the stored one is refused by the Edge Function.
  saved_at bigint not null check (saved_at between 0 and 32503680000),
  -- SHA-256 of the inflated JSON bytes, lower-case hex.
  content_sha256 text not null check (content_sha256 ~ '^[0-9a-f]{64}$'),
  -- Gzipped size; the Edge Function reads at most 64 KiB.
  size_bytes integer not null check (size_bytes between 1 and 65536),
  storage_path text generated always as (
    user_id::text || '/' || session || '-' || game_index::text || '.json.gz'
  ) stored,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  primary key (user_id, session, game_index)
);

comment on table public.games is
  'Summary of one uploaded game (self-reported by the user''s desktop app). Written only by the upload Edge Function.';

-- ---------------------------------------------------------------------------
-- updated_at and profile creation
-- ---------------------------------------------------------------------------

create function private.touch_updated_at()
returns trigger
language plpgsql
set search_path = ''
as $$
begin
  new.updated_at := now();
  return new;
end;
$$;

create trigger profiles_touch before update on public.profiles
  for each row execute function private.touch_updated_at();
create trigger games_touch before update on public.games
  for each row execute function private.touch_updated_at();

-- Every new auth user gets a private profile with no display name.
create function private.create_profile()
returns trigger
language plpgsql
security definer
set search_path = ''
as $$
begin
  insert into public.profiles (user_id) values (new.id);
  return new;
end;
$$;

revoke all on function private.create_profile() from public;

create trigger on_auth_user_created after insert on auth.users
  for each row execute function private.create_profile();

-- ---------------------------------------------------------------------------
-- Privileges and row-level security
-- ---------------------------------------------------------------------------

alter table public.profiles enable row level security;
alter table public.games enable row level security;

-- Supabase grants everything on new public tables to anon and authenticated.
-- Take it all back and grant only what the policies below need, so a missing
-- or wrong policy cannot open a write path.
revoke all on public.profiles from anon, authenticated;
revoke all on public.games from anon, authenticated;

grant select on public.profiles to anon, authenticated;
grant update (display_name, is_public) on public.profiles to authenticated;
grant select on public.games to anon, authenticated;

create policy profiles_read_own_or_public on public.profiles
  for select to anon, authenticated
  using (is_public or user_id = (select auth.uid()));

create policy profiles_update_own on public.profiles
  for update to authenticated
  using (user_id = (select auth.uid()))
  with check (user_id = (select auth.uid()));

create policy games_read_own_or_public on public.games
  for select to anon, authenticated
  using (
    user_id = (select auth.uid())
    or exists (
      select 1 from public.profiles p
      where p.user_id = games.user_id and p.is_public
    )
  );

-- No insert, update or delete policy on games: clients cannot write.

-- ---------------------------------------------------------------------------
-- Storage: private bucket, one folder per user, read-only for clients
-- ---------------------------------------------------------------------------

insert into storage.buckets (id, name, public, file_size_limit, allowed_mime_types)
values ('games', 'games', false, 65536, array['application/gzip']);

-- Objects live at <user_id>/<session>-<index>.json.gz. Readable by the owner,
-- and by everyone once the owner's profile is public. No insert, update or
-- delete policy: only the service role writes or removes files.
create policy games_bucket_read_own_or_public on storage.objects
  for select to anon, authenticated
  using (
    bucket_id = 'games'
    and (
      (storage.foldername(name))[1] = (select auth.uid())::text
      or exists (
        select 1 from public.profiles p
        where p.user_id::text = (storage.foldername(name))[1] and p.is_public
      )
    )
  );
