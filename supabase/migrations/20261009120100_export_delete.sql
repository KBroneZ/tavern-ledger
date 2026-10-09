-- T-104a: data export and account deletion (GDPR, CLAUDE.md rule 6).
--
-- Export: public.export_my_data() returns, as one JSON document, everything
-- this project holds about the signed-in user: the auth account (without
-- password hash or tokens), sign-in identities, sessions, auth audit entries,
-- the profile, every game row and the list of their files. The website zips
-- it with the files themselves (T-104c), which the user can already read.
--
-- Deletion: the delete-account Edge Function removes, in this order, the
-- user's files (through the Storage API: deleting rows in storage.objects
-- would leave the bytes behind), then calls public.delete_user_data() for the
-- rows, then deletes the auth user. Each step can be repeated if a later one
-- fails.

create function public.export_my_data()
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
    'auth_events', coalesce((
      select jsonb_agg(jsonb_build_object(
        'created_at', a.created_at,
        'ip_address', a.ip_address,
        'payload', a.payload
      ) order by a.created_at)
      from auth.audit_log_entries a where a.payload ->> 'actor_id' = uid::text
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
    ), '[]'::jsonb)
  );
end;
$$;

revoke all on function public.export_my_data() from public, anon;
grant execute on function public.export_my_data() to authenticated;

-- Names of the user's files, for the delete-account Edge Function.
create function public.user_file_names(target uuid)
returns setof text
language sql
stable
security definer
set search_path = ''
as $$
  select o.name from storage.objects o
  where o.bucket_id = 'games' and (storage.foldername(o.name))[1] = target::text
  order by o.name;
$$;

revoke all on function public.user_file_names(uuid) from public, anon, authenticated;
grant execute on function public.user_file_names(uuid) to service_role;

-- Rows of one user, after their files are gone. Refuses while files remain,
-- so a failed file step cannot leave orphan files behind. Returns counts.
create function public.delete_user_data(target uuid)
returns jsonb
language plpgsql
security definer
set search_path = ''
as $$
declare
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

  delete from public.games where user_id = target;
  get diagnostics n_games = row_count;
  delete from public.profiles where user_id = target;
  get diagnostics n_profiles = row_count;
  -- Auth audit entries have no foreign key to the user and would survive the
  -- auth user's deletion.
  delete from auth.audit_log_entries where payload ->> 'actor_id' = target::text;
  get diagnostics n_events = row_count;

  return jsonb_build_object(
    'games', n_games, 'profiles', n_profiles, 'auth_events', n_events
  );
end;
$$;

revoke all on function public.delete_user_data(uuid) from public, anon, authenticated;
grant execute on function public.delete_user_data(uuid) to service_role;
