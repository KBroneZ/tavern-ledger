import { test } from "node:test";
import assert from "node:assert/strict";
import {
  CHECK_EMAIL,
  checkCredentials,
  checkEmail,
  checkNewPassword,
  MIN_PASSWORD_LENGTH,
  newPasswordErrorMessage,
  RESET_SENT,
  resetRequestOutcome,
  signInErrorMessage,
  signUpOutcome,
} from "../../src/lib/auth.ts";

test("the password length matches supabase/config.toml", () => {
  assert.equal(MIN_PASSWORD_LENGTH, 10);
});

test("credentials: email shape and password length are checked before sending", () => {
  assert.equal(checkCredentials("a@example.test", "0123456789"), null);
  assert.match(checkCredentials("", "0123456789") ?? "", /email/i);
  assert.match(checkCredentials("not-an-email", "0123456789") ?? "", /email/i);
  assert.match(checkCredentials("a@example.test", "short") ?? "", /10 characters/);
  // Length counts characters, not UTF-16 units.
  assert.match(checkCredentials("a@example.test", "🔑🔑🔑🔑🔑") ?? "", /10 characters/);
});

test("sign-in errors never say whether the email has an account", () => {
  const wrong = signInErrorMessage({ code: "invalid_credentials", status: 400 });
  // Same text however the failure is reported.
  assert.equal(signInErrorMessage({ code: "user_not_found", status: 400 }), wrong);
  assert.equal(signInErrorMessage({ status: 400, message: "Invalid login credentials" }), wrong);
  assert.doesNotMatch(wrong, /no account|not found|exist|unknown/i);
});

test("sign-in: too many attempts and network trouble are told apart", () => {
  assert.match(signInErrorMessage({ code: "over_request_rate_limit", status: 429 }), /wait/i);
  assert.match(signInErrorMessage({ status: 0, name: "AuthRetryableFetchError" }), /reach/i);
  assert.match(signInErrorMessage({ code: "email_not_confirmed", status: 400 }), /confirm/i);
});

test("sign-in never shows a server message as is", () => {
  const msg = signInErrorMessage({ status: 500, message: "<b>internal</b> db error" });
  assert.doesNotMatch(msg, /internal|<b>/);
});

test("sign-up: a new and an existing email get the same answer", () => {
  assert.deepEqual(signUpOutcome(null), { kind: "check-email", message: CHECK_EMAIL });
  assert.deepEqual(signUpOutcome({ code: "user_already_exists", status: 422 }), {
    kind: "check-email",
    message: CHECK_EMAIL,
  });
  assert.deepEqual(signUpOutcome({ code: "email_exists", status: 422 }), {
    kind: "check-email",
    message: CHECK_EMAIL,
  });
  assert.doesNotMatch(CHECK_EMAIL, /already|exists|new account/i);
});

test("sign-up: weak password, rate limit and other errors", () => {
  const weak = signUpOutcome({ code: "weak_password", status: 422 });
  assert.equal(weak.kind, "error");
  assert.match(weak.message, /password/i);
  assert.match(signUpOutcome({ code: "over_email_send_rate_limit", status: 429 }).message, /wait/i);
  assert.match(signUpOutcome({ code: "signup_disabled", status: 422 }).message, /not open/i);
  const other = signUpOutcome({ status: 500, message: "boom" });
  assert.equal(other.kind, "error");
  assert.doesNotMatch(other.message, /boom/);
});

test("reset request: the same answer whether the email has an account or not", () => {
  assert.deepEqual(resetRequestOutcome(null), { kind: "check-email", message: RESET_SENT });
  for (const e of [
    { code: "user_not_found", status: 400 },
    { code: "email_not_confirmed", status: 400 },
    { code: "signup_disabled", status: 422 },
    { status: 404 },
    // Only an existing account gets this (a second email within a minute).
    { code: "over_email_send_rate_limit", status: 429 },
  ]) {
    assert.deepEqual(resetRequestOutcome(e), { kind: "check-email", message: RESET_SENT }, JSON.stringify(e));
  }
  assert.doesNotMatch(RESET_SENT, /no account|not found|exist/i);
});

test("reset request: rate limits, no connection and server errors are said in words", () => {
  assert.match(resetRequestOutcome({ code: "over_request_rate_limit", status: 429 }).message, /Too many/);
  assert.match(resetRequestOutcome({ status: 429 }).message, /Too many/);
  assert.match(resetRequestOutcome({ name: "AuthRetryableFetchError", status: 0 }).message, /reach the server/);
  assert.match(resetRequestOutcome({ code: "email_address_invalid", status: 400 }).message, /valid email/);
  const boom = resetRequestOutcome({ status: 500, message: "boom" });
  assert.equal(boom.kind, "error");
  assert.doesNotMatch(boom.message, /boom/);
});

test("reset email is checked before sending", () => {
  assert.equal(checkEmail(" a@example.test "), null);
  assert.match(checkEmail("nope") ?? "", /valid email/);
});

test("new password: length and repeat are checked before sending", () => {
  assert.equal(checkNewPassword("0123456789", "0123456789"), null);
  assert.match(checkNewPassword("short", "short") ?? "", /10 characters/);
  assert.match(checkNewPassword("0123456789", "0123456780") ?? "", /not the same/);
});

test("new password: server refusals in words, never the server's text", () => {
  assert.match(newPasswordErrorMessage({ code: "same_password", status: 422 }), /different/);
  assert.match(newPasswordErrorMessage({ code: "weak_password", status: 422 }), /stronger/);
  assert.match(newPasswordErrorMessage({ code: "reauthentication_needed", status: 400 }), /new link/);
  assert.match(newPasswordErrorMessage({ name: "AuthSessionMissingError", status: 400 }), /new link/);
  assert.match(newPasswordErrorMessage({ status: 401 }), /new link/);
  assert.match(newPasswordErrorMessage({ status: 429 }), /Too many/);
  assert.doesNotMatch(newPasswordErrorMessage({ status: 500, message: "boom" }), /boom/);
});
