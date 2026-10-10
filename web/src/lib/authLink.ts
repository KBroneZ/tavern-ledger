// The links Supabase Auth's emails bring back to /account/ (D-048), read
// from the address before it is cleaned up. Default email templates go
// through /auth/v1/verify, which redirects here with:
//   ?code=…                      a PKCE flow started in this browser (sign-up,
//                                password reset)
//   #access_token=…&type=…       a link made without PKCE (dashboard invite,
//                                magic link, confirmation or recovery)
//   ?error=…&error_code=…        (and the same in the hash) a link that failed
//   #message=…                   the first half of a double-confirmed email change
// and a custom template could send ?token_hash=…&type=… instead.

/** The email link types Supabase Auth sends, as `type` names them. */
export const LINK_TYPES = ["signup", "invite", "magiclink", "recovery", "email_change", "email"] as const;
export type LinkType = (typeof LINK_TYPES)[number];

export type AuthLink =
  | { kind: "none" }
  | { kind: "code"; code: string; flowId: string | null }
  | { kind: "token-hash"; tokenHash: string; type: LinkType }
  | { kind: "implicit"; type: LinkType | null; accessToken: string | null }
  | { kind: "notice" }
  | { kind: "error"; code: string | null };

// Shapes of what this module passes on; anything else is ignored.
const CODE = /^[A-Za-z0-9-]{8,128}$/;
const TOKEN_HASH = /^[A-Za-z0-9_-]{8,256}$/;
const ACCESS_TOKEN = /^[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+$/;
const ERROR_CODE = /^[a-z0-9_]{1,64}$/;

function linkType(raw: string | null): LinkType | null {
  return (LINK_TYPES as readonly string[]).includes(raw ?? "") ? (raw as LinkType) : null;
}

/** Whether the address carries anything that must not stay in it. */
export function hasLinkParams(search: string, hash: string): boolean {
  return search.length > 1 || hash.length > 1;
}

/** Reads an email link from `location.search` and `location.hash`. */
export function parseAuthLink(search: string, hash: string): AuthLink {
  const query = new URLSearchParams(search.replace(/^\?/, ""));
  const fragment = new URLSearchParams(hash.replace(/^#/, ""));
  const get = (name: string) => query.get(name) ?? fragment.get(name);

  // An error wins over anything else in the address.
  if (get("error") !== null || get("error_code") !== null || get("error_description") !== null) {
    const code = get("error_code") ?? get("error");
    return { kind: "error", code: code !== null && ERROR_CODE.test(code) ? code : null };
  }
  const code = query.get("code");
  if (code !== null) {
    if (!CODE.test(code)) return { kind: "error", code: null };
    const flowId = query.get("sb_flow_id");
    return { kind: "code", code, flowId: flowId !== null && CODE.test(flowId) ? flowId : null };
  }
  const tokenHash = query.get("token_hash");
  if (tokenHash !== null) {
    const type = linkType(query.get("type"));
    if (!TOKEN_HASH.test(tokenHash) || type === null) return { kind: "error", code: null };
    return { kind: "token-hash", tokenHash, type };
  }
  if (fragment.has("access_token")) {
    const token = fragment.get("access_token") ?? "";
    return {
      kind: "implicit",
      type: linkType(fragment.get("type")),
      accessToken: ACCESS_TOKEN.test(token) && token.length <= 8192 ? token : null,
    };
  }
  if (fragment.has("message")) return { kind: "notice" };
  return { kind: "none" };
}

const USE_FORGOT = "To choose a password, use “Forgot password?” on the sign-in page.";

/**
 * What to say after a link made without PKCE. Its session is never used:
 * the site only signs in with a password or a flow it started itself.
 */
export function implicitLinkMessage(type: LinkType | null): string {
  switch (type) {
    case "invite":
      return `Your email is confirmed. ${USE_FORGOT}`;
    case "magiclink":
      return "Sign-in links are for the Tavern Ledger app. Here, sign in with your password, " +
        "or use “Forgot password?” on the sign-in page to set one.";
    case "recovery":
      return "This reset link cannot be used here. Ask for a new one with “Forgot password?” on the sign-in page.";
    case "email_change":
      return "Your new email address is confirmed. Sign in to continue.";
    default:
      return "Your email is confirmed. Sign in to continue.";
  }
}

/** What to say after a confirmation that needs no further step. */
export function confirmedMessage(type: LinkType | null, signedIn: boolean): string {
  if (type === "email_change") return "Your new email address is confirmed.";
  return signedIn ? "Your email is confirmed." : "Your email is confirmed. Sign in to continue.";
}

export const NOTICE_MESSAGE =
  "Link accepted. Now open the link we sent to your other email address to finish the change.";

const EXPIRED =
  "This link has expired or was already used (each new email replaces the previous link). " +
  "If you were confirming your email, try signing in: it may already be confirmed. " +
  "Otherwise ask for a new link.";

/** What to say when a link failed, from the error code in the address or the server. */
export function linkErrorMessage(code: string | null): string {
  switch (code) {
    case "otp_expired":
    case "flow_state_expired":
    case "flow_state_not_found":
    case "bad_code_verifier":
      return EXPIRED;
    case "pkce_code_verifier_not_found":
      return "This link was opened in another browser than the one where you asked for it. " +
        "If you were confirming your email, it is confirmed: sign in. " +
        "If you were resetting your password, open the link in the browser where you asked for it, " +
        "or ask for a new one.";
    case "over_request_rate_limit":
    case "over_email_send_rate_limit":
      return "Too many attempts. Wait a few minutes and try again.";
    case "network":
      return "Could not reach the server. Check your connection and reload the page.";
    default:
      return "This link did not work. Sign in, or ask for a new link.";
  }
}
