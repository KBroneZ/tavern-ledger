-- T-104d: what the upload-game Edge Function needs.
--
-- * games.parser_version: which parser read the game (T-107, D-024).
-- * Tribe keys may hold digits: the parser prints unknown tribes as RACE_<n>.
-- * Quotas per user (600 uploads an hour, 5,000 a day, 10,000 games and
--   50 MiB in total) and a limit per IP for requests whose token is refused
--   (30 in 10 minutes). The IP is hashed by the function before it gets here.
-- * upload_begin() checks revision and quotas before the file is written;
--   upload_commit() writes the summary row after it.
-- * The daily sweep: files with no row, audit entries of deleted accounts
--   (the T-104a leftover) and expired counters.
-- Every function here is for the service role only.

-- ---------------------------------------------------------------------------
-- Schema changes
-- ---------------------------------------------------------------------------

-- <crate version>+r<PARSER_REVISION>, e.g. 0.1.0+r1. Null: unknown version
-- (a record saved before T-107).
alter table public.games add column parser_version text
  check (
    parser_version is null
    or parser_version ~ '^[0-9]{1,4}\.[0-9]{1,4}\.[0-9]{1,4}\+r[0-9]{1,6}$'
  );

create or replace function private.is_tribe_counts(counts jsonb)
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
        when t.tribe !~ '^[A-Z0-9_]{1,32}$' then true
        when jsonb_typeof(t.n) <> 'number' then true
        else (t.n)::numeric <> trunc((t.n)::numeric)
          or (t.n)::numeric < 0
          or (t.n)::numeric > 10000
      end
    )
  end;
$$;

-- One row per accepted upload, kept two days for the rate limits.
create table private.upload_events (
  user_id uuid not null references auth.users (id) on delete cascade,
  at timestamptz not null default now()
);
create index upload_events_user_at on private.upload_events (user_id, at);

-- Refused tokens per hashed IP and minute, kept one hour.
create table private.upload_ip_failures (
  ip_key text not null check (ip_key ~ '^[0-9a-f]{64}$'),
  window_start timestamptz not null,
  failures integer not null default 0,
  primary key (ip_key, window_start)
);
create index upload_ip_failures_window on private.upload_ip_failures (window_start);

revoke all on private.upload_events, private.upload_ip_failures
  from public, anon, authenticated;

-- ---------------------------------------------------------------------------
-- Upload
-- ---------------------------------------------------------------------------

-- Seconds until the oldest upload since `since` leaves its window.
create function private.retry_after(target uuid, since timestamptz, win interval)
returns integer
language sql
stable
set search_path = ''
as $$
  select greatest(1, ceil(extract(epoch from min(e.at) + win - now())))::integer
  from private.upload_events e
  where e.user_id = target and e.at > since;
$$;

-- Called before the file is written. Returns {"result": ...}:
--   create | replace      go on: write the file, then upload_commit()
--   unchanged             same bytes already stored: nothing to write
--   older                 the stored game has a newer revision (409)
--   rate_limited          with retry_after in seconds (429)
--   quota_games | quota_bytes   total quota reached (403)
-- Accepted calls are counted for the rate limits; refused ones are not.
create function public.upload_begin(
  target uuid,
  p_session text,
  p_game_index integer,
  p_saved_at bigint,
  p_sha256 text,
  p_size integer
)
returns jsonb
language plpgsql
security definer
set search_path = ''
as $$
declare
  hour_limit constant integer := 600;
  day_limit constant integer := 5000;
  games_limit constant integer := 10000;
  bytes_limit constant bigint := 50 * 1024 * 1024;
  n_hour bigint;
  n_day bigint;
  n_games bigint;
  n_bytes bigint;
  has_row boolean;
  old_saved_at bigint;
  old_sha text;
  old_size integer;
begin
  -- One check at a time per user, so two uploads at once cannot both pass
  -- the same last free slot. (The quota can still be passed by the uploads
  -- in flight between this call and their commit; the app sends one at a time.)
  perform pg_advisory_xact_lock(hashtextextended('upload:' || target::text, 0));

  select count(*) filter (where e.at > now() - interval '1 hour'), count(*)
  into n_hour, n_day
  from private.upload_events e
  where e.user_id = target and e.at > now() - interval '1 day';
  if n_hour >= hour_limit then
    return jsonb_build_object('result', 'rate_limited', 'retry_after',
      private.retry_after(target, now() - interval '1 hour', interval '1 hour'));
  end if;
  if n_day >= day_limit then
    return jsonb_build_object('result', 'rate_limited', 'retry_after',
      private.retry_after(target, now() - interval '1 day', interval '1 day'));
  end if;

  select true, g.saved_at, g.content_sha256, g.size_bytes
  into has_row, old_saved_at, old_sha, old_size
  from public.games g
  where g.user_id = target and g.session = p_session and g.game_index = p_game_index;
  has_row := coalesce(has_row, false);

  if has_row and old_sha = p_sha256 then
    insert into private.upload_events (user_id) values (target);
    return jsonb_build_object('result', 'unchanged');
  end if;
  if has_row and old_saved_at > p_saved_at then
    return jsonb_build_object('result', 'older');
  end if;

  select count(*), coalesce(sum(g.size_bytes), 0)
  into n_games, n_bytes
  from public.games g
  where g.user_id = target;
  if not has_row and n_games >= games_limit then
    return jsonb_build_object('result', 'quota_games');
  end if;
  if n_bytes - coalesce(old_size, 0) + p_size > bytes_limit then
    return jsonb_build_object('result', 'quota_bytes');
  end if;

  insert into private.upload_events (user_id) values (target);
  return jsonb_build_object('result', case when has_row then 'replace' else 'create' end);
end;
$$;

-- Called after the file is written: creates or replaces the summary row
-- unless a newer revision landed in between. Returns created | replaced | older.
create function public.upload_commit(
  target uuid,
  p_session text,
  p_game_index integer,
  p_game_type text,
  p_status text,
  p_hero_card_id text,
  p_final_place integer,
  p_build integer,
  p_played_on date,
  p_tribes_offered jsonb,
  p_saved_at bigint,
  p_sha256 text,
  p_size integer,
  p_parser_version text
)
returns text
language plpgsql
security definer
set search_path = ''
as $$
declare
  inserted boolean;
begin
  insert into public.games as g (
    user_id, session, game_index, game_type, status, hero_card_id, final_place,
    build, played_on, tribes_offered, saved_at, content_sha256, size_bytes, parser_version
  ) values (
    target, p_session, p_game_index, p_game_type, p_status, p_hero_card_id, p_final_place,
    p_build, p_played_on, p_tribes_offered, p_saved_at, p_sha256, p_size, p_parser_version
  )
  on conflict (user_id, session, game_index) do update set
    game_type = excluded.game_type,
    status = excluded.status,
    hero_card_id = excluded.hero_card_id,
    final_place = excluded.final_place,
    build = excluded.build,
    played_on = excluded.played_on,
    tribes_offered = excluded.tribes_offered,
    saved_at = excluded.saved_at,
    content_sha256 = excluded.content_sha256,
    size_bytes = excluded.size_bytes,
    parser_version = excluded.parser_version
  where g.saved_at <= excluded.saved_at
  returning (xmax = 0) into inserted;

  if inserted is null then
    return 'older';
  end if;
  return case when inserted then 'created' else 'replaced' end;
end;
$$;

-- Requests whose token was refused, per hashed IP: blocked at 30 in 10 minutes.
create function public.upload_ip_blocked(p_ip_key text)
returns boolean
language sql
stable
security definer
set search_path = ''
as $$
  select coalesce(sum(f.failures), 0) >= 30
  from private.upload_ip_failures f
  where f.ip_key = p_ip_key and f.window_start > now() - interval '10 minutes';
$$;

create function public.upload_ip_failed(p_ip_key text)
returns void
language sql
security definer
set search_path = ''
as $$
  -- Expired counters go on every write, so forged IPs cannot pile up rows
  -- between two sweeps.
  delete from private.upload_ip_failures f where f.window_start < now() - interval '1 hour';
  insert into private.upload_ip_failures (ip_key, window_start, failures)
  values (p_ip_key, date_trunc('minute', now()), 1)
  on conflict (ip_key, window_start)
  do update set failures = private.upload_ip_failures.failures + 1;
$$;

-- ---------------------------------------------------------------------------
-- Daily sweep (the sweep Edge Function deletes the files through the Storage
-- API, since deleting storage.objects rows would leave the bytes behind)
-- ---------------------------------------------------------------------------

-- Game files with no summary row, untouched for at least min_age (so an
-- upload between its file and its row is not swept). At most 1000 per call.
create function public.sweep_orphan_files(min_age interval)
returns setof text
language sql
stable
security definer
set search_path = ''
as $$
  select o.name
  from storage.objects o
  where o.bucket_id = 'games'
    and greatest(o.created_at, coalesce(o.updated_at, o.created_at)) < now() - min_age
    and not exists (select 1 from public.games g where g.storage_path = o.name)
  order by o.name
  limit 1000;
$$;

-- Audit entries that name an account that no longer exists (the last step of
-- delete-account can leave one, T-104a), upload times older than two days and
-- IP counters older than one hour. Entries younger than an hour are left
-- alone, in case their account is being created right now.
create function public.sweep_housekeeping()
returns jsonb
language plpgsql
security definer
set search_path = ''
as $$
declare
  n_auth bigint;
  n_events bigint;
  n_ip bigint;
begin
  delete from auth.audit_log_entries a
  where a.created_at < now() - interval '1 hour'
    and (
      (
        a.payload ->> 'actor_id' ~ '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
        and a.payload ->> 'actor_id' <> '00000000-0000-0000-0000-000000000000'
        and not exists (select 1 from auth.users u where u.id::text = a.payload ->> 'actor_id')
      )
      or (
        a.payload -> 'traits' ->> 'user_id' is not null
        and not exists (
          select 1 from auth.users u where u.id::text = a.payload -> 'traits' ->> 'user_id'
        )
      )
    );
  get diagnostics n_auth = row_count;
  delete from private.upload_events e where e.at < now() - interval '2 days';
  get diagnostics n_events = row_count;
  delete from private.upload_ip_failures f where f.window_start < now() - interval '1 hour';
  get diagnostics n_ip = row_count;
  return jsonb_build_object('auth_events', n_auth, 'upload_events', n_events, 'ip_failures', n_ip);
end;
$$;

revoke all on function private.retry_after(uuid, timestamptz, interval) from public;
revoke all on function public.upload_begin(uuid, text, integer, bigint, text, integer)
  from public, anon, authenticated;
revoke all on function public.upload_commit(
  uuid, text, integer, text, text, text, integer, integer, date, jsonb, bigint, text, integer, text
) from public, anon, authenticated;
revoke all on function public.upload_ip_blocked(text) from public, anon, authenticated;
revoke all on function public.upload_ip_failed(text) from public, anon, authenticated;
revoke all on function public.sweep_orphan_files(interval) from public, anon, authenticated;
revoke all on function public.sweep_housekeeping() from public, anon, authenticated;
grant execute on function public.upload_begin(uuid, text, integer, bigint, text, integer)
  to service_role;
grant execute on function public.upload_commit(
  uuid, text, integer, text, text, text, integer, integer, date, jsonb, bigint, text, integer, text
) to service_role;
grant execute on function public.upload_ip_blocked(text) to service_role;
grant execute on function public.upload_ip_failed(text) to service_role;
grant execute on function public.sweep_orphan_files(interval) to service_role;
grant execute on function public.sweep_housekeeping() to service_role;

-- ---------------------------------------------------------------------------
-- Export: the same document as before plus the upload times kept for the
-- rate limits (two days at most).
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
