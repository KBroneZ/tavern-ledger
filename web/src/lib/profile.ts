// Display name rules and the public profile address. Messages are drawn in
// the page's language (lib/account/strings.ts, T-508).

import { at } from "./account/strings.ts";
import { NAME_CHARS } from "./displayName.ts";
import { href } from "./i18n.ts";

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;
// Same character list as the database (displayName.ts), so a BattleTag
// (Name#1234) or a look-alike alphabet cannot be entered. The database has the
// last word.
const MAX_NAME = 32;
/** The message private.check_display_name() raises for a listed word. */
const NAME_NOT_ALLOWED = "display_name_not_allowed";

export type NameCheck = { ok: true; value: string | null } | { ok: false; message: string };

export function checkDisplayName(raw: string): NameCheck {
  // Composed form: "e" plus a combining accent is saved as "é".
  const value = raw.trim().normalize("NFC");
  if (value === "") return { ok: true, value: null };
  if ([...value].length > MAX_NAME) {
    return { ok: false, message: at("profile.maxLength", { max: MAX_NAME }) };
  }
  if (!NAME_CHARS.test(value)) {
    return { ok: false, message: at("profile.chars") };
  }
  return { ok: true, value };
}

/** The profile id from a page's query string (?id=<uuid>), or null. */
export function parseProfileId(search: string): string | null {
  const id = new URLSearchParams(search).get("id")?.toLowerCase() ?? "";
  return UUID.test(id) ? id : null;
}

export function publicProfilePath(userId: string): string {
  return href(`/profile/?id=${encodeURIComponent(userId)}`);
}

/** A PostgREST error as the profile form shows it. */
export function profileErrorMessage(e: { code?: string; message?: string }): string {
  // The server's word list (D-077) refused the name; the list stays on the server.
  if (e.code === "23514" && e.message === NAME_NOT_ALLOWED) {
    return at("profile.notAllowed");
  }
  // Another public profile has the name (the unique index).
  if (e.code === "23505") return at("profile.taken");
  if (e.code === "23514") return at("profile.charsRange");
  if (e.code === "PGRST301" || e.code === "42501") return at("profile.sessionEnded");
  return at("profile.saveFailed");
}

/** A failed save of an account switch (community stats, simulator), as the account page shows it. */
export function switchErrorMessage(e: { code?: string }): string {
  if (e.code === "PGRST301" || e.code === "42501") return at("profile.sessionEnded");
  return at("profile.saveFailed");
}
