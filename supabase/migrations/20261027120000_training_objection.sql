-- T-651 (D-174): the switch to object to training the expected-place model.
--
-- * Choice: public.training_objection, one row per account that ever used the
--   switch; no row means on. The legal basis is legitimate interest, so every
--   account, old or new, starts on, and turning it off is the right to object
--   (GDPR Art. 21): it holds for every later run, with no reason needed. Only
--   the owner sets it, through public.set_training_choice() (auth.uid()), and
--   among client roles only the owner reads it. As with the simulator switch
--   it is not a profile column: public profiles are readable by everyone. The
--   row keeps when the choice was last changed and the privacy policy version
--   then in force.
-- * Use: public.training_files() lists the game files the owner may download
--   to train the expected-place model: files still in the bucket of accounts
--   that have not objected at that moment. Service role only (the owner, from
--   the SQL editor); no client can call it. Only the model's weights and
--   totals are kept from a run.
-- * Export: export_my_data() gains training_choice (on, with no date or
--   version, when never set). Deletion: the row goes with the account
--   (delete_user_data() and the foreign key to the profile).

-- ---------------------------------------------------------------------------
-- Choice
-- ---------------------------------------------------------------------------

-- The privacy policy version a choice was made under (its "Last updated"
-- date): raised when the policy's training section changes.
create function private.training_policy_version()
returns text language sql immutable set search_path = '' as $$ select '2026-10-27' $$;

revoke all on function private.training_policy_version() from public, anon, authenticated, service_role;

create table public.training_objection (
  user_id uuid primary key references public.profiles (user_id) on delete cascade,
  -- "Use my uploaded games to train the expected-place model". False is the objection.
  train_model boolean not null,
  -- When the owner last changed it, and under which privacy policy.
  updated_at timestamptz not null default now(),
  policy_version text not null check (policy_version ~ '^[0-9]{4}-[0-9]{2}-[0-9]{2}$')
);

comment on table public.training_objection is
  'Objection to the use of uploaded games to train the expected-place model (T-651). No row means on (legitimate interest); train_model = false is the objection. Set only by the owner through set_training_choice(); among client roles, read only by the owner.';

alter table public.training_objection enable row level security;
revoke all on public.training_objection from public, anon, authenticated;
grant select on public.training_objection to authenticated;

create policy training_objection_read_own on public.training_objection
  for select to authenticated
  using (user_id = (select auth.uid()));

-- No insert, update or delete policy or grant: writes go through the function
-- below, which can only touch the caller's own row.

-- Sets the caller's choice and returns it. Signed-in callers with a profile only.
create function public.set_training_choice(p_train boolean)
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
  if p_train is null then
    raise exception 'set_training_choice: true or false' using errcode = '22023';
  end if;
  insert into public.training_objection (user_id, train_model, updated_at, policy_version)
  values (uid, p_train, now(), private.training_policy_version())
  on conflict (user_id) do update set
    train_model = excluded.train_model,
    updated_at = excluded.updated_at,
    policy_version = excluded.policy_version;
  return p_train;
end;
$$;

revoke all on function public.set_training_choice(boolean) from public, anon;
grant execute on function public.set_training_choice(boolean) to authenticated;

-- ---------------------------------------------------------------------------
-- The owner's list
-- ---------------------------------------------------------------------------

-- The game files that may be used right now: accounts that have not objected,
-- games whose file has not expired and is still in the bucket, newest first.
-- The path holds the account id: the owner renames each file when it is
-- downloaded, so no id reaches the training.
create function public.training_files(p_limit integer)
returns table (storage_path text, content_sha256 text, size_bytes integer)
language plpgsql
stable
security definer
set search_path = ''
as $$
begin
  if p_limit is null or p_limit < 1 or p_limit > 10000 then
    raise exception 'training_files: limit between 1 and 10000' using errcode = '22023';
  end if;
  return query
    select g.storage_path, g.content_sha256, g.size_bytes
    from public.games g
    where g.file_expired_at is null
      and not exists (
        select 1 from public.training_objection t
        where t.user_id = g.user_id and not t.train_model
      )
      and exists (
        select 1 from storage.objects o
        where o.bucket_id = 'games' and o.name = g.storage_path
      )
    order by g.played_on desc, g.storage_path
    limit p_limit;
end;
$$;

revoke all on function public.training_files(integer) from public, anon, authenticated;
grant execute on function public.training_files(integer) to service_role;

-- ---------------------------------------------------------------------------
-- Export and deletion
-- ---------------------------------------------------------------------------

-- As in 20261015120000_community_trust_scale.sql, plus training_choice (the
-- switch as it is now: on, with updated_at and policy_version null, when it
-- was never set).
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
    -- The account's own events as they are. An event someone else caused (an
    -- admin acting on the account) loses who and from where: the actor's id,
    -- name and sign-in method, and the IP address (the actor's, not the
    -- owner's).
    'auth_events', coalesce((
      select jsonb_agg(jsonb_build_object(
        'created_at', a.created_at,
        'ip_address', case when a.payload ->> 'actor_id' = uid::text then a.ip_address end,
        'payload', case when a.payload ->> 'actor_id' = uid::text then a.payload::jsonb
          else a.payload::jsonb - 'actor_id' - 'actor_username' - 'actor_name' - 'actor_via_sso' end
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
    'training_choice', jsonb_build_object(
      'train_model', coalesce((
        select t.train_model from public.training_objection t where t.user_id = uid
      ), true),
      'updated_at', (
        select t.updated_at from public.training_objection t where t.user_id = uid
      ),
      'policy_version', (
        select t.policy_version from public.training_objection t where t.user_id = uid
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

-- As in 20261012200000_simulator_consent.sql, plus the training choice (the
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
  delete from public.training_objection where user_id = target;
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
