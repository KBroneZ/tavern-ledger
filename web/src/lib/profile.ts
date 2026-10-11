// Display name rules and the public profile address.

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;
// Same rule as the profiles.display_name check (letters, digits, space, _ . -),
// so a BattleTag (Name#1234) cannot be entered. The database has the last word.
const NAME = /^[\p{L}\p{N} _.-]+$/u;
const MAX_NAME = 32;
/** The message private.check_display_name() raises for a listed word. */
const NAME_NOT_ALLOWED = "display_name_not_allowed";

export type NameCheck = { ok: true; value: string | null } | { ok: false; message: string };

export function checkDisplayName(raw: string): NameCheck {
  const value = raw.trim();
  if (value === "") return { ok: true, value: null };
  if ([...value].length > MAX_NAME) {
    return { ok: false, message: `Up to ${MAX_NAME} characters.` };
  }
  if (!NAME.test(value)) {
    return { ok: false, message: "Use letters, digits, spaces, _ . or - only." };
  }
  return { ok: true, value };
}

/** The profile id from a page's query string (?id=<uuid>), or null. */
export function parseProfileId(search: string): string | null {
  const id = new URLSearchParams(search).get("id")?.toLowerCase() ?? "";
  return UUID.test(id) ? id : null;
}

export function publicProfilePath(userId: string): string {
  return `/profile/?id=${encodeURIComponent(userId)}`;
}

/** A PostgREST error as the profile form shows it. */
export function profileErrorMessage(e: { code?: string; message?: string }): string {
  // The server's word list (D-077) refused the name; the list stays on the server.
  if (e.code === "23514" && e.message === NAME_NOT_ALLOWED) {
    return "This name is not allowed. Choose another one: names show on public pages and leaderboards.";
  }
  if (e.code === "23514") return "Use 1-32 letters, digits, spaces, _ . or - only.";
  if (e.code === "PGRST301" || e.code === "42501") return "Your session ended. Sign in again.";
  return "Could not save. Try again in a moment.";
}

/** A failed save of the community stats switch, as the account page shows it. */
export function communityErrorMessage(e: { code?: string }): string {
  if (e.code === "PGRST301" || e.code === "42501") return "Your session ended. Sign in again.";
  return "Could not save. Try again in a moment.";
}
