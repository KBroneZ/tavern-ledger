// Sign-in and sign-up rules and messages. The messages never say whether an
// email has an account, and never show a server message as it is.

/** Same as minimum_password_length in supabase/config.toml. */
export const MIN_PASSWORD_LENGTH = 10;

export const CHECK_EMAIL =
  "Check your inbox: if this email can be used, a confirmation link is on its way. " +
  "Open it in this browser to finish.";

const WRONG_CREDENTIALS = "Wrong email or password.";
const RATE_LIMITED = "Too many attempts. Wait a few minutes and try again.";
const UNREACHABLE = "Could not reach the server. Check your connection and try again.";
const GENERIC = "Something went wrong. Try again in a moment.";

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
  if (!EMAIL.test(email.trim())) return "Enter a valid email address.";
  if ([...password].length < MIN_PASSWORD_LENGTH) {
    return `The password needs at least ${MIN_PASSWORD_LENGTH} characters.`;
  }
  return null;
}

function isRateLimit(e: AuthErrorLike): boolean {
  return e.status === 429 || (e.code ?? "").startsWith("over_");
}

function isNetwork(e: AuthErrorLike): boolean {
  return e.name === "AuthRetryableFetchError" || e.status === 0;
}

export function signInErrorMessage(e: AuthErrorLike): string {
  if (isRateLimit(e)) return RATE_LIMITED;
  if (isNetwork(e)) return UNREACHABLE;
  // Only reachable with the right password, so it reveals nothing new.
  if (e.code === "email_not_confirmed") {
    return "Confirm your email first: open the link we sent you.";
  }
  if (e.status !== undefined && e.status >= 400 && e.status < 500) return WRONG_CREDENTIALS;
  return GENERIC;
}

export type SignUpOutcome =
  | { kind: "check-email"; message: string }
  | { kind: "error"; message: string };

/** `error` is null when the sign-up call succeeded. */
export function signUpOutcome(e: AuthErrorLike | null): SignUpOutcome {
  // An email that already has an account gets the same answer as a new one.
  if (e === null || e.code === "user_already_exists" || e.code === "email_exists") {
    return { kind: "check-email", message: CHECK_EMAIL };
  }
  if (isRateLimit(e)) return { kind: "error", message: RATE_LIMITED };
  if (isNetwork(e)) return { kind: "error", message: UNREACHABLE };
  if (e.code === "weak_password") {
    return {
      kind: "error",
      message: `Choose a stronger password (at least ${MIN_PASSWORD_LENGTH} characters).`,
    };
  }
  if (e.code === "signup_disabled") {
    return { kind: "error", message: "Sign-up is not open yet." };
  }
  if (e.code === "email_address_invalid") {
    return { kind: "error", message: "Enter a valid email address." };
  }
  return { kind: "error", message: GENERIC };
}

export const RESET_SENT =
  "Check your inbox: if this email has an account, a link to choose a new password is on its way. " +
  "Open it in this browser.";

/** A message if the reset request cannot be sent, or null. */
export function checkEmail(email: string): string | null {
  return EMAIL.test(email.trim()) ? null : "Enter a valid email address.";
}

/**
 * `error` is null when the request succeeded. Any other refusal gets the
 * same answer as a sent email, so the form never says whether an email has
 * an account; only a busy server or no connection is said as such.
 */
export function resetRequestOutcome(e: AuthErrorLike | null): SignUpOutcome {
  if (e === null) return { kind: "check-email", message: RESET_SENT };
  if (isRateLimit(e)) return { kind: "error", message: RATE_LIMITED };
  if (isNetwork(e)) return { kind: "error", message: UNREACHABLE };
  if (e.code === "email_address_invalid") return { kind: "error", message: "Enter a valid email address." };
  if (e.status !== undefined && e.status >= 400 && e.status < 500) {
    return { kind: "check-email", message: RESET_SENT };
  }
  return { kind: "error", message: GENERIC };
}

/** A message if the new password cannot be saved, or null. */
export function checkNewPassword(password: string, repeat: string): string | null {
  if ([...password].length < MIN_PASSWORD_LENGTH) {
    return `The password needs at least ${MIN_PASSWORD_LENGTH} characters.`;
  }
  if (password !== repeat) return "The two passwords are not the same.";
  return null;
}

export function newPasswordErrorMessage(e: AuthErrorLike): string {
  if (isRateLimit(e)) return RATE_LIMITED;
  if (isNetwork(e)) return UNREACHABLE;
  switch (e.code) {
    case "same_password":
      return "Choose a password different from your current one.";
    case "weak_password":
      return `Choose a stronger password (at least ${MIN_PASSWORD_LENGTH} characters).`;
    case "reauthentication_needed":
    case "session_not_found":
    case "session_expired":
      return "This reset has expired. Ask for a new link with “Forgot password?” on the sign-in page.";
  }
  if (e.status === 401 || e.status === 403 || e.name === "AuthSessionMissingError") {
    return "This reset has expired. Ask for a new link with “Forgot password?” on the sign-in page.";
  }
  return GENERIC;
}
