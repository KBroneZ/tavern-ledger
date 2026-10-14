// The desktop app's sign-in link, asked for from the website (T-562). The app
// cannot pass the anti-robot check itself, so it opens /desktop-sign-in/ in
// the browser with, in the address's fragment (never sent to a server):
//   challenge  its PKCE challenge (S256 of a verifier that stays in the app)
//   redirect   its loopback address, http://127.0.0.1:<port>/desktop-callback/<state>
//   email      the email typed in the app (to fill the form; optional)
// The page asks Supabase Auth for the link with the challenge and the
// redirect, as the app used to (crates/uploader). The email's link ends at
// the app's listener with a code only the app can trade, with its verifier.

import { at } from "./account/strings.ts";
import { checkEmail, type AuthErrorLike, type SignUpOutcome } from "./auth.ts";

export interface DesktopRequest {
  challenge: string;
  redirect: string;
  email: string;
}

// base64url of 32 bytes, as the app makes them (crates/uploader/src/pkce.rs).
const CHALLENGE = /^[A-Za-z0-9_-]{43}$/;
const REDIRECT = /^http:\/\/127\.0\.0\.1:(\d{1,5})\/desktop-callback\/[A-Za-z0-9_-]{43}$/;
const MAX_EMAIL = 254;

/** The app's request from the page's fragment, or null if it is not one. */
export function parseDesktopRequest(hash: string): DesktopRequest | null {
  const params = new URLSearchParams(hash.replace(/^#/, ""));
  const challenge = params.get("challenge") ?? "";
  const redirect = params.get("redirect") ?? "";
  const port = REDIRECT.exec(redirect)?.[1];
  if (!CHALLENGE.test(challenge) || port === undefined) return null;
  if (Number(port) < 1 || Number(port) > 65535) return null;
  const email = (params.get("email") ?? "").trim();
  return { challenge, redirect, email: email.length <= MAX_EMAIL && checkEmail(email) === null ? email : "" };
}

// Supabase Auth's answers for an email with no account (sign-ups are off for
// this request): said like a sent link, so the page never tells which emails
// have an account.
const NO_ACCOUNT = new Set(["otp_disabled", "signup_disabled", "user_not_found"]);

/**
 * What to say after asking for the link. `e` is null when it was sent. As on
 * the other forms, the answer is the same whether the email has an account.
 */
export function desktopLinkOutcome(e: AuthErrorLike | null): SignUpOutcome {
  const sent: SignUpOutcome = { kind: "check-email", message: at("desktop.sent") };
  if (e === null) return sent;
  if (e.code === "captcha_failed") return { kind: "error", message: at("auth.captcha") };
  if (e.code !== undefined && NO_ACCOUNT.has(e.code)) return sent;
  // Only an account gets this (a second link within the minute): said like the first.
  if (e.code === "over_email_send_rate_limit") return sent;
  if (e.status === 429 || (e.code ?? "").startsWith("over_")) return { kind: "error", message: at("auth.rateLimited") };
  if (e.status === 0 || e.name === "AuthRetryableFetchError") return { kind: "error", message: at("auth.unreachable") };
  if (e.code === "email_address_invalid") return { kind: "error", message: at("auth.email") };
  // Any other refusal of this email (a blocked account, say) reads like a
  // sent link too, so no account state shows through.
  if (e.status !== undefined && e.status >= 400 && e.status < 500) return sent;
  return { kind: "error", message: at("auth.generic") };
}
