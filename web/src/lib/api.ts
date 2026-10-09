// Every call the site makes to Supabase. Pages call these; the local
// end-to-end test (test/local/) calls the same functions against the stack.

import { createClient, type SupabaseClient, type SupportedStorage } from "@supabase/supabase-js";
import { type AuthErrorLike, signInErrorMessage, signUpOutcome, type SignUpOutcome } from "./auth.ts";
import type { SiteConfig } from "./config.ts";
import { type DeleteOutcome, deleteOutcome } from "./deleteAccount.ts";
import { buildExport, type ExportFile, ownFileNames } from "./exportData.ts";
import { type GameView, gameView, profileErrorMessage } from "./profile.ts";

const TIMEOUT_MS = 15_000;
// The public page shows the latest games only.
export const PUBLIC_GAMES_LIMIT = 200;

/** fetch with a timeout, so no request can hang a page. */
function fetchWithTimeout(input: RequestInfo | URL, init: RequestInit = {}): Promise<Response> {
  const timeout = AbortSignal.timeout(TIMEOUT_MS);
  const signal = init.signal ? AbortSignal.any([init.signal, timeout]) : timeout;
  return fetch(input, { ...init, signal });
}

/**
 * The browser keeps the session in Supabase's default storage
 * (localStorage); tests pass an in-memory store.
 */
export function makeClient(config: SiteConfig, storage?: SupportedStorage): SupabaseClient {
  return createClient(config.url, config.anonKey, {
    auth: {
      flowType: "pkce",
      persistSession: true,
      autoRefreshToken: true,
      detectSessionInUrl: storage === undefined,
      ...(storage ? { storage } : {}),
    },
    global: { fetch: fetchWithTimeout },
  });
}

export async function signUp(
  client: SupabaseClient,
  email: string,
  password: string,
  redirectTo: string,
): Promise<SignUpOutcome> {
  try {
    const { error } = await client.auth.signUp({
      email: email.trim(),
      password,
      options: { emailRedirectTo: redirectTo },
    });
    return signUpOutcome(error as AuthErrorLike | null);
  } catch {
    return signUpOutcome({ status: 0 });
  }
}

/** null when signed in, else the message to show. */
export async function signIn(client: SupabaseClient, email: string, password: string): Promise<string | null> {
  try {
    const { error } = await client.auth.signInWithPassword({ email: email.trim(), password });
    return error ? signInErrorMessage(error as AuthErrorLike) : null;
  } catch {
    return signInErrorMessage({ status: 0 });
  }
}

export interface OwnProfile {
  displayName: string | null;
  isPublic: boolean;
}

export async function loadOwnProfile(client: SupabaseClient, userId: string): Promise<OwnProfile> {
  const { data, error } = await client
    .from("profiles")
    .select("display_name, is_public")
    .eq("user_id", userId)
    .maybeSingle();
  if (error || !data) throw new Error("could not load the profile");
  return {
    displayName: typeof data.display_name === "string" ? data.display_name : null,
    isPublic: data.is_public === true,
  };
}

/** null when saved, else the message to show. */
export async function saveProfile(
  client: SupabaseClient,
  userId: string,
  profile: OwnProfile,
): Promise<string | null> {
  try {
    const { data, error } = await client
      .from("profiles")
      .update({ display_name: profile.displayName, is_public: profile.isPublic })
      .eq("user_id", userId)
      .select("user_id");
    if (error) return profileErrorMessage(error);
    // Row-level security filters silently: no row back means nothing was saved.
    if (!data || data.length !== 1) return profileErrorMessage({ code: "PGRST301" });
    return null;
  } catch {
    return profileErrorMessage({});
  }
}

/** Everything export_my_data() returns plus the bytes of each game file. */
export async function exportAll(client: SupabaseClient): Promise<Record<string, unknown>> {
  const { data, error } = await client.rpc("export_my_data");
  if (error) throw new Error("the export failed");
  const files: ExportFile[] = [];
  for (const name of ownFileNames(data)) {
    const { data: blob, error: fileError } = await client.storage.from("games").download(name);
    if (fileError || !blob) throw new Error("a game file could not be read");
    files.push({ name, bytes: new Uint8Array(await blob.arrayBuffer()) });
  }
  return buildExport(data, files);
}

export async function deleteAccount(client: SupabaseClient, config: SiteConfig): Promise<DeleteOutcome> {
  const { data } = await client.auth.getSession();
  const token = data.session?.access_token;
  if (!token) return { kind: "signed-out" };
  let res: Response;
  try {
    res = await fetch(`${config.url}/functions/v1/delete-account`, {
      method: "DELETE",
      headers: { Authorization: `Bearer ${token}`, apikey: config.anonKey },
      // Deleting many files takes a few requests on the server side.
      signal: AbortSignal.timeout(60_000),
    });
  } catch {
    return deleteOutcome(0, null);
  }
  let body: unknown = null;
  try {
    body = await res.json();
  } catch {
    body = null;
  }
  return deleteOutcome(res.status, body);
}

export type PublicProfile =
  | { kind: "found"; name: string | null; games: GameView[] }
  | { kind: "not-found" }
  | { kind: "error" };

/**
 * A profile that is private and one that does not exist look the same, so
 * the page cannot be used to find out which accounts exist.
 */
export async function loadPublicProfile(client: SupabaseClient, userId: string): Promise<PublicProfile> {
  try {
    const profile = await client
      .from("profiles")
      .select("display_name")
      .eq("user_id", userId)
      .eq("is_public", true)
      .maybeSingle();
    if (profile.error) return { kind: "error" };
    if (!profile.data) return { kind: "not-found" };
    const games = await client
      .from("games")
      .select("game_type, status, hero_card_id, final_place, played_on")
      .eq("user_id", userId)
      .order("played_on", { ascending: false })
      .order("session", { ascending: false })
      .order("game_index", { ascending: false })
      .limit(PUBLIC_GAMES_LIMIT);
    if (games.error || !Array.isArray(games.data)) return { kind: "error" };
    const name = typeof profile.data.display_name === "string" ? profile.data.display_name : null;
    return { kind: "found", name, games: games.data.map(gameView) };
  } catch {
    return { kind: "error" };
  }
}
