// Every call the site makes to Supabase. Pages call these; the local
// end-to-end test (test/local/) calls the same functions against the stack.

import { createClient, type EmailOtpType, type SupabaseClient, type SupportedStorage } from "@supabase/supabase-js";
import {
  type AuthErrorLike,
  newPasswordErrorMessage,
  resetRequestOutcome,
  signInErrorMessage,
  signUpOutcome,
  type SignUpOutcome,
} from "./auth.ts";
import {
  type AuthLink,
  confirmedMessage,
  implicitLinkMessage,
  linkErrorMessage,
  NOTICE_MESSAGE,
} from "./authLink.ts";
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
      // Email links are read by finishAuthLink(), never by the client on
      // its own: it would leave a session from a link the site did not
      // start, or tokens in the address when it refuses one.
      detectSessionInUrl: false,
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

/** Sends a password reset link; the answer never says whether the email has an account. */
export async function requestPasswordReset(
  client: SupabaseClient,
  email: string,
  redirectTo: string,
): Promise<SignUpOutcome> {
  try {
    const { error } = await client.auth.resetPasswordForEmail(email.trim(), { redirectTo });
    return resetRequestOutcome(error as AuthErrorLike | null);
  } catch {
    return resetRequestOutcome({ status: 0 });
  }
}

/** null when saved, else the message to show. Needs the session of a reset link. */
export async function setNewPassword(client: SupabaseClient, password: string): Promise<string | null> {
  try {
    const { error } = await client.auth.updateUser({ password });
    return error ? newPasswordErrorMessage(error as AuthErrorLike) : null;
  } catch {
    return newPasswordErrorMessage({ status: 0 });
  }
}

export interface LinkOutcome {
  /** A reset link worked here: show the "set a new password" form. */
  recovery: boolean;
  /** What to tell the user, or null when the link needs no word. */
  message: string | null;
  tone: "ok" | "error" | "info";
}

const NOTHING: LinkOutcome = { recovery: false, message: null, tone: "info" };

function errorCodeOf(e: unknown): string | null {
  const err = e as AuthErrorLike | null;
  if (!err) return null;
  if (err.name === "AuthRetryableFetchError" || err.status === 0) return "network";
  return typeof err.code === "string" ? err.code : null;
}

/**
 * Ends the session a link made without PKCE carries, so it is not left
 * open on the server. Best effort: the site never uses that session.
 */
async function revokeLinkSession(config: SiteConfig, accessToken: string): Promise<void> {
  try {
    await fetchWithTimeout(`${config.url}/auth/v1/logout?scope=local`, {
      method: "POST",
      headers: { Authorization: `Bearer ${accessToken}`, apikey: config.anonKey },
    });
  } catch {
    // The session expires on its own.
  }
}

/**
 * Finishes an email link read by parseAuthLink() (D-048). Only flows this
 * browser started (PKCE) leave it signed in; a recovery link, by either
 * flow, leaves the session that lets the user choose a new password.
 */
export async function finishAuthLink(
  client: SupabaseClient,
  config: SiteConfig,
  link: AuthLink,
): Promise<LinkOutcome> {
  switch (link.kind) {
    case "none":
      return NOTHING;
    case "notice":
      return { recovery: false, message: NOTICE_MESSAGE, tone: "ok" };
    case "error":
      return { recovery: false, message: linkErrorMessage(link.code), tone: "error" };
    case "implicit":
      if (link.accessToken) await revokeLinkSession(config, link.accessToken);
      return { recovery: false, message: implicitLinkMessage(link.type), tone: "ok" };
    case "code": {
      try {
        const { data, error } = await client.auth.exchangeCodeForSession(
          link.code,
          link.flowId ? { flowId: link.flowId } : undefined,
        );
        if (error) {
          // The email is confirmed by the server before it redirects here,
          // so a sign-up link opened in another browser still did its job.
          return { recovery: false, message: linkErrorMessage(errorCodeOf(error)), tone: "error" };
        }
        // auth-js returns the flow's kind, kept with the code verifier by
        // resetPasswordForEmail(), though its type does not declare it.
        const redirectType = (data as { redirectType?: unknown }).redirectType;
        if (redirectType === "recovery") return { recovery: true, message: null, tone: "info" };
        return { recovery: false, message: confirmedMessage(null, true), tone: "ok" };
      } catch (e) {
        return { recovery: false, message: linkErrorMessage(errorCodeOf(e)), tone: "error" };
      }
    }
    case "token-hash": {
      try {
        const { error } = await client.auth.verifyOtp({
          token_hash: link.tokenHash,
          type: link.type as EmailOtpType,
        });
        if (error) return { recovery: false, message: linkErrorMessage(errorCodeOf(error)), tone: "error" };
        if (link.type === "recovery") return { recovery: true, message: null, tone: "info" };
        // A link the site did not start: confirm, but do not stay signed in with it.
        await client.auth.signOut({ scope: "local" });
        return { recovery: false, message: confirmedMessage(link.type, false), tone: "ok" };
      } catch (e) {
        return { recovery: false, message: linkErrorMessage(errorCodeOf(e)), tone: "error" };
      }
    }
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
