// Sign-in and sign-up rules and messages. The messages never say whether an
// email has an account, and never show a server message as it is. The words
// are in lib/account/strings.ts, drawn in the page's language (T-508).

import { ACCOUNT_STRINGS, at } from "./account/strings.ts";

/** Same as minimum_password_length in supabase/config.toml. */
export const MIN_PASSWORD_LENGTH = 10;

/** The English text, for the tests; pages show at("auth.checkEmail"). */
export const CHECK_EMAIL = ACCOUNT_STRINGS["auth.checkEmail"];
/** The English text, for the tests; pages show at("auth.resetSent"). */
export const RESET_SENT = ACCOUNT_STRINGS["auth.resetSent"];

const checkEmailMessage = () => at("auth.checkEmail");
const resetSent = () => at("auth.resetSent");
const wrongCredentials = () => at("auth.wrong");
const rateLimited = () => at("auth.rateLimited");
const unreachable = () => at("auth.unreachable");
const generic = () => at("auth.generic");
const captchaRefused = () => at("auth.captcha");
const invalidEmail = () => at("auth.email");
const passwordLength = () => at("auth.passwordLength", { min: MIN_PASSWORD_LENGTH });
const stronger = () => at("auth.stronger", { min: MIN_PASSWORD_LENGTH });

/** The parts of a Supabase AuthError this module reads. */
export interface AuthErrorLike {
  code?: string;
  status?: number;
  name?: string;
  message?: string;
}

const EMAIL = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;

/** A message if the form cannot be sent, or null. */
export function checkCredentials(email: string, password: string): string | null {
  if (!EMAIL.test(email.trim())) return invalidEmail();
  if ([...password].length < MIN_PASSWORD_LENGTH) return passwordLength();
  return null;
}

function isRateLimit(e: AuthErrorLike): boolean {
  return e.status === 429 || (e.code ?? "").startsWith("over_");
}

function isNetwork(e: AuthErrorLike): boolean {
  return e.name === "AuthRetryableFetchError" || e.status === 0;
}

/**
 * Supabase Auth refused the anti-robot token (T-562). It checks the token
 * before it looks at the email, so saying so tells nothing about an account.
 */
function isCaptchaRefused(e: AuthErrorLike): boolean {
  return e.code === "captcha_failed";
}

export function signInErrorMessage(e: AuthErrorLike): string {
  if (isCaptchaRefused(e)) return captchaRefused();
  if (isRateLimit(e)) return rateLimited();
  if (isNetwork(e)) return unreachable();
  // Only reachable with the right password, so it reveals nothing new.
  if (e.code === "email_not_confirmed") return at("auth.confirmFirst");
  if (e.status !== undefined && e.status >= 400 && e.status < 500) return wrongCredentials();
  return generic();
}

export type SignUpOutcome =
  | { kind: "check-email"; message: string }
  | { kind: "error"; message: string };

/** `error` is null when the sign-up call succeeded. */
export function signUpOutcome(e: AuthErrorLike | null): SignUpOutcome {
  // An email that already has an account gets the same answer as a new one.
  if (e === null || e.code === "user_already_exists" || e.code === "email_exists") {
    return { kind: "check-email", message: checkEmailMessage() };
  }
  if (isCaptchaRefused(e)) return { kind: "error", message: captchaRefused() };
  if (isRateLimit(e)) return { kind: "error", message: rateLimited() };
  if (isNetwork(e)) return { kind: "error", message: unreachable() };
  if (e.code === "weak_password") return { kind: "error", message: stronger() };
  if (e.code === "signup_disabled") return { kind: "error", message: at("auth.signupClosed") };
  if (e.code === "email_address_invalid") return { kind: "error", message: invalidEmail() };
  return { kind: "error", message: generic() };
}

/** A message if the reset request cannot be sent, or null. */
export function checkEmail(email: string): string | null {
  return EMAIL.test(email.trim()) ? null : invalidEmail();
}

/**
 * `error` is null when the request succeeded. Any other refusal gets the
 * same answer as a sent email, so the form never says whether an email has
 * an account; only a busy server or no connection is said as such.
 */
export function resetRequestOutcome(e: AuthErrorLike | null): SignUpOutcome {
  if (e === null) return { kind: "check-email", message: resetSent() };
  if (isCaptchaRefused(e)) return { kind: "error", message: captchaRefused() };
  // Supabase refuses a second reset email to the same account within a
  // minute; an unknown email is never refused, so saying so would tell
  // which emails have an account. Limits per address of the sender stay.
  if (e.code === "over_email_send_rate_limit") return { kind: "check-email", message: resetSent() };
  if (isRateLimit(e)) return { kind: "error", message: rateLimited() };
  if (isNetwork(e)) return { kind: "error", message: unreachable() };
  if (e.code === "email_address_invalid") return { kind: "error", message: invalidEmail() };
  if (e.status !== undefined && e.status >= 400 && e.status < 500) {
    return { kind: "check-email", message: resetSent() };
  }
  return { kind: "error", message: generic() };
}

/** A message if the new password cannot be saved, or null. */
export function checkNewPassword(password: string, repeat: string): string | null {
  if ([...password].length < MIN_PASSWORD_LENGTH) return passwordLength();
  if (password !== repeat) return at("auth.notSame");
  return null;
}

/** Which field `checkNewPassword` is complaining about. */
export function newPasswordField(password: string): "password" | "repeat" {
  return [...password].length < MIN_PASSWORD_LENGTH ? "password" : "repeat";
}

export function newPasswordErrorMessage(e: AuthErrorLike): string {
  if (isRateLimit(e)) return rateLimited();
  if (isNetwork(e)) return unreachable();
  switch (e.code) {
    case "same_password":
      return at("auth.samePassword");
    case "weak_password":
      return stronger();
    case "reauthentication_needed":
    case "session_not_found":
    case "session_expired":
      return at("auth.resetExpired");
  }
  if (e.status === 401 || e.status === 403 || e.name === "AuthSessionMissingError") {
    return at("auth.resetExpired");
  }
  return generic();
}
