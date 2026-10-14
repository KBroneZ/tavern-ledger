import { test } from "node:test";
import assert from "node:assert/strict";
import { desktopLinkOutcome, parseDesktopRequest } from "../../src/lib/desktopLink.ts";
import { ACCOUNT_STRINGS } from "../../src/lib/account/strings.ts";

const CHALLENGE = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";
const STATE = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
const REDIRECT = `http://127.0.0.1:50000/desktop-callback/${STATE}`;

const hash = (params: Record<string, string>) => `#${new URLSearchParams(params)}`;

test("reads the app's challenge, loopback address and email from the fragment", () => {
  assert.deepEqual(parseDesktopRequest(hash({ challenge: CHALLENGE, redirect: REDIRECT, email: " player@example.test " })), {
    challenge: CHALLENGE,
    redirect: REDIRECT,
    email: "player@example.test",
  });
  assert.deepEqual(parseDesktopRequest(hash({ challenge: CHALLENGE, redirect: REDIRECT })), {
    challenge: CHALLENGE,
    redirect: REDIRECT,
    email: "",
  });
});

test("a page not opened by the app, or with anything else, is not a request", () => {
  assert.equal(parseDesktopRequest(""), null);
  assert.equal(parseDesktopRequest("#"), null);
  assert.equal(parseDesktopRequest(hash({ redirect: REDIRECT })), null, "no challenge");
  assert.equal(parseDesktopRequest(hash({ challenge: CHALLENGE })), null, "no redirect");
  for (const redirect of [
    `https://evil.example/desktop-callback/${STATE}`,
    `http://127.0.0.1.evil.example:50000/desktop-callback/${STATE}`,
    `http://localhost:50000/desktop-callback/${STATE}`,
    `http://127.0.0.1:50000/elsewhere/${STATE}`,
    `http://127.0.0.1:50000/desktop-callback/${STATE}?x=1`,
    `http://127.0.0.1:50000/desktop-callback/short`,
    `http://127.0.0.1:0/desktop-callback/${STATE}`,
    `http://127.0.0.1:70000/desktop-callback/${STATE}`,
    `http://user@127.0.0.1:50000/desktop-callback/${STATE}`,
  ]) {
    assert.equal(parseDesktopRequest(hash({ challenge: CHALLENGE, redirect })), null, redirect);
  }
  for (const challenge of ["short", `${CHALLENGE}x`, CHALLENGE.replace("-", "+"), CHALLENGE.replace("E", "=")]) {
    assert.equal(parseDesktopRequest(hash({ challenge, redirect: REDIRECT })), null, challenge);
  }
});

test("an email that does not look like one is left out, not refused", () => {
  for (const email of ["not an email", `${"a".repeat(250)}@example.test`, "a@b"]) {
    assert.equal(parseDesktopRequest(hash({ challenge: CHALLENGE, redirect: REDIRECT, email }))?.email, "", email.slice(0, 20));
  }
});

test("an email with no account gets the same answer as one with an account", () => {
  const sent = { kind: "check-email", message: ACCOUNT_STRINGS["desktop.sent"] };
  assert.deepEqual(desktopLinkOutcome(null), sent);
  for (const e of [
    { code: "otp_disabled", status: 422 },
    { code: "signup_disabled", status: 422 },
    { code: "user_not_found", status: 400 },
    // Only an account gets this one (a second link within the minute).
    { code: "over_email_send_rate_limit", status: 429 },
    // Nor does any other refusal of one account tell it apart.
    { code: "user_banned", status: 403 },
    { status: 400 },
  ]) {
    assert.deepEqual(desktopLinkOutcome(e), sent, JSON.stringify(e));
  }
  assert.doesNotMatch(ACCOUNT_STRINGS["desktop.sent"], /no account|not found|exist/i);
});

test("a refused check, too many tries, no connection and other errors are said in words", () => {
  assert.deepEqual(desktopLinkOutcome({ code: "captcha_failed", status: 400 }), {
    kind: "error",
    message: ACCOUNT_STRINGS["auth.captcha"],
  });
  assert.match(desktopLinkOutcome({ code: "over_request_rate_limit", status: 429 }).message, /Too many/);
  assert.match(desktopLinkOutcome({ status: 429 }).message, /Too many/);
  assert.match(desktopLinkOutcome({ status: 0 }).message, /reach the server/);
  assert.match(desktopLinkOutcome({ code: "email_address_invalid", status: 400 }).message, /valid email/);
  const boom = desktopLinkOutcome({ status: 500, message: "<b>boom</b>" });
  assert.equal(boom.kind, "error");
  assert.doesNotMatch(boom.message, /boom/);
});
