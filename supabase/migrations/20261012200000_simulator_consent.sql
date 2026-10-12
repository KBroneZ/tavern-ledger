-- T-547 (D-119): consent to use uploaded games to improve the simulator.
--
-- * Consent: public.simulator_consent, one row per account that ever used the
--   switch; no row means off, so every account, old or new, starts off and no
--   update can turn it on. Only the owner sets it, through
--   public.set_simulator_consent() (auth.uid()), and among client roles only
--   the owner reads it. As with public.community_consent (D-085) it is not a
--   profile column: public profiles are readable by everyone. The row keeps
--   when the choice was last given or withdrawn and the privacy policy
--   version then in force (GDPR Art. 7(1)).
-- * Use: public.simulator_files() lists the game files the owner may download
--   to measure and fix the combat simulator (`combat-validate --games`):
--   files still in the bucket of accounts whose switch is on at that moment.
--   Service role only (the owner, from the SQL editor); no client can call it.
--   The owner runs it from the SQL editor.
-- * Export: export_my_data() gains simulator_consent. Deletion: the row goes
--   with the account (delete_user_data() and the foreign key to the profile).

-- ---------------------------------------------------------------------------
-- Consent
-- ---------------------------------------------------------------------------

-- The privacy policy version a consent was given under (its "Last updated"
-- date): raised when the policy's simulator section changes.
create function private.simulator_policy_version()
returns text language sql immutable set search_path = '' as $$ select '2026-10-12' $$;

revoke all on function private.simulator_policy_version() from public, anon, authenticated, service_role;

create table public.simulator_consent (
  user_id uuid primary key references public.profiles (user_id) on delete cascade,
  -- "Let my games be used to improve the simulator". Off unless the owner turns it on.
  improve_simulator boolean not null default false,
  -- When the owner last gave or withdrew it, and under which privacy policy.
  updated_at timestamptz not null default now(),
  policy_version text not null check (policy_version ~ '^[0-9]{4}-[0-9]{2}-[0-9]{2}$')
);

comment on table public.simulator_consent is
  'Opt-in to the use of uploaded games to improve the simulator (T-547). No row means off. Set only by the owner through set_simulator_consent(); among client roles, read only by the owner.';

alter table public.simulator_consent enable row level security;
revoke all on public.simulator_consent from public, anon, authenticated;
grant select on public.simulator_consent to authenticated;

create policy simulator_consent_read_own on public.simulator_consent
  for select to authenticated
  using (user_id = (select auth.uid()));

-- No insert, update or delete policy or grant: writes go through the function
-- below, which can only touch the caller's own row.

-- Sets the caller's choice and returns it. Signed-in callers with a profile only.
create function public.set_simulator_consent(p_improve boolean)
returns boolean
language plpgsql
security definer
set search_path = ''
as $$
declare
  uid uuid := auth.uid();
begin
  if uid is null or not exists (select 1 from public.profiles p where p.user_id = uid) then
    raise exception 'not signed in' using errcode = '42501';
  end if;
  if p_improve is null then
    raise exception 'set_simulator_consent: true or false' using errcode = '22023';
  end if;
  insert into public.simulator_consent (user_id, improve_simulator, updated_at, policy_version)
  values (uid, p_improve, now(), private.simulator_policy_version())
  on conflict (user_id) do update set
    improve_simulator = excluded.improve_simulator,
    updated_at = excluded.updated_at,
    policy_version = excluded.policy_version;
  return p_improve;
end;
$$;

revoke all on function public.set_simulator_consent(boolean) from public, anon;
grant execute on function public.set_simulator_consent(boolean) to authenticated;

-- ---------------------------------------------------------------------------
-- The owner's list
-- ---------------------------------------------------------------------------

-- The game files that may be used right now: accounts whose switch is on,
-- games whose file has not expired and is still in the bucket, newest first.
-- The path holds the account id: the owner renames each file when it is
-- downloaded (deploy.md 10.15), so no id reaches the measurement.
create function public.simulator_files(p_limit integer)
returns table (storage_path text, content_sha256 text, size_bytes integer)
language plpgsql
stable
security definer
set search_path = ''
as $$
begin
  if p_limit is null or p_limit < 1 or p_limit > 10000 then
    raise exception 'simulator_files: limit between 1 and 10000' using errcode = '22023';
  end if;
  return query
    select g.storage_path, g.content_sha256, g.size_bytes
    from public.games g
    join public.simulator_consent c on c.user_id = g.user_id and c.improve_simulator
    where g.file_expired_at is null
      and exists (
        select 1 from storage.objects o
        where o.bucket_id = 'games' and o.name = g.storage_path
      )
    order by g.played_on desc, g.storage_path
    limit p_limit;
end;
$$;

revoke all on function public.simulator_files(integer) from public, anon, authenticated;
grant execute on function public.simulator_files(integer) to service_role;

-- ---------------------------------------------------------------------------
-- Export and deletion
-- ---------------------------------------------------------------------------

-- As in 20261012180000_file_retention.sql, plus simulator_consent (the switch
-- as it is now; updated_at and policy_version are null when it was never set).
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
    -- The switch as it is now; updated_at and policy_version are null when
    -- it was never set.
    'community_consent', jsonb_build_object(
      'hero_stats', coalesce((
        select c.hero_stats from public.community_consent c where c.user_id = uid
      ), false),
      'updated_at', (
        select c.updated_at from public.community_consent c where c.user_id = uid
      ),
      'policy_version', (
        select c.policy_version from public.community_consent c where c.user_id = uid
      )
    ),
    'simulator_consent', jsonb_build_object(
      'improve_simulator', coalesce((
        select c.improve_simulator from public.simulator_consent c where c.user_id = uid
      ), false),
      'updated_at', (
        select c.updated_at from public.simulator_consent c where c.user_id = uid
      ),
      'policy_version', (
        select c.policy_version from public.simulator_consent c where c.user_id = uid
      )
    ),
    'games', coalesce((
      select jsonb_agg(to_jsonb(g) order by g.session, g.game_index)
      from public.games g where g.user_id = uid
    ), '[]'::jsonb),
    'game_stats', coalesce((
      select jsonb_agg(to_jsonb(s) order by s.session, s.game_index)
      from public.game_stats s where s.user_id = uid
    ), '[]'::jsonb),
    'leaderboard_rows', coalesce((
      select jsonb_agg(jsonb_build_object(
        'mode', r.mode,
        'period', r.period,
        'board', r.board,
        'hero', r.hero,
        'value', r.value,
        'games', r.games
      ) order by r.mode, r.period, r.board, r.hero)
      from private.leaderboard_rows r where r.user_id = uid
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
    'file_retention_attempts', coalesce((
      select jsonb_agg(jsonb_build_object(
        'session', a.session,
        'game_index', a.game_index,
        'fails', a.fails,
        'last_at', a.last_at
      ) order by a.session, a.game_index)
      from private.file_retention_attempts a where a.user_id = uid
    ), '[]'::jsonb),
    'unlimited_history', (
      select jsonb_build_object('since', u.since)
      from private.unlimited_history u where u.user_id = uid
    ),
    'upload_events', coalesce((
      select jsonb_agg(e.at order by e.at)
      from private.upload_events e where e.user_id = uid
    ), '[]'::jsonb)
  );
end;
$$;

-- As in 20261012180000_file_retention.sql, plus the simulator consent (the
-- foreign key to the profile would also remove it with the profile).
create or replace function public.delete_user_data(target uuid)
returns jsonb
language plpgsql
security definer
set search_path = ''
as $$
declare
  n_stats bigint;
  n_games bigint;
  n_profiles bigint;
  n_events bigint;
begin
  if exists (
    select 1 from storage.objects o
    where o.bucket_id = 'games' and (storage.foldername(o.name))[1] = target::text
  ) then
    raise exception 'files remain for this user; delete them first'
      using errcode = '55000';
  end if;

  delete from private.leaderboard_rows where user_id = target;
  delete from public.community_consent where user_id = target;
  delete from public.simulator_consent where user_id = target;
  delete from private.unlimited_history where user_id = target;
  delete from public.game_stats where user_id = target;
  get diagnostics n_stats = row_count;
  delete from public.games where user_id = target;
  get diagnostics n_games = row_count;
  delete from public.profiles where user_id = target;
  get diagnostics n_profiles = row_count;
  -- Auth audit entries have no foreign key to the user and would survive the
  -- auth user's deletion.
  delete from auth.audit_log_entries
  where payload ->> 'actor_id' = target::text
     or payload -> 'traits' ->> 'user_id' = target::text;
  get diagnostics n_events = row_count;

  return jsonb_build_object(
    'game_stats', n_stats, 'games', n_games, 'profiles', n_profiles, 'auth_events', n_events
  );
end;
$$;

revoke all on function public.delete_user_data(uuid) from public, anon, authenticated;
grant execute on function public.delete_user_data(uuid) to service_role;
