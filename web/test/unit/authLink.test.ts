// Email links that land on /account/ (D-048): every shape Supabase Auth can
// send is read, nothing malformed is passed on, and each says something.
import { test } from "node:test";
import assert from "node:assert/strict";
import {
  confirmedMessage,
  hasLinkParams,
  implicitLinkMessage,
  LINK_TYPES,
  linkErrorMessage,
  parseAuthLink,
} from "../../src/lib/authLink.ts";

const CODE = "0b5c4a1e-1111-4222-8333-944455556666";
const JWT = "eyJhbGciOiJFUzI1NiJ9.eyJzdWIiOiJ4In0.c2lnbmF0dXJl";

test("no parameters: nothing to do", () => {
  assert.deepEqual(parseAuthLink("", ""), { kind: "none" });
  assert.equal(hasLinkParams("", ""), false);
  assert.equal(hasLinkParams("?", "#"), false);
  assert.equal(hasLinkParams(`?code=${CODE}`, ""), true);
  assert.equal(hasLinkParams("", "#access_token=x"), true);
});

test("a PKCE link brings a code, with or without a flow id", () => {
  assert.deepEqual(parseAuthLink(`?code=${CODE}`, ""), { kind: "code", code: CODE, flowId: null });
  assert.deepEqual(parseAuthLink(`?code=${CODE}&sb_flow_id=${CODE}`, ""), {
    kind: "code",
    code: CODE,
    flowId: CODE,
  });
  assert.deepEqual(parseAuthLink(`?code=${CODE}&sb_flow_id=<x>`, ""), { kind: "code", code: CODE, flowId: null });
});

test("a code that is not shaped like one is an invalid link", () => {
  assert.deepEqual(parseAuthLink("?code=<script>", ""), { kind: "error", code: null });
  assert.deepEqual(parseAuthLink("?code=", ""), { kind: "error", code: null });
});

test("errors are read from the query or the hash and win over anything else", () => {
  const pkce = "?error=access_denied&error_code=otp_expired&error_description=Email+link+is+invalid+or+has+expired";
  assert.deepEqual(parseAuthLink(pkce, pkce.replace("?", "#")), { kind: "error", code: "otp_expired" });
  assert.deepEqual(parseAuthLink("", "#error=access_denied&error_code=otp_expired"), {
    kind: "error",
    code: "otp_expired",
  });
  assert.deepEqual(parseAuthLink(`?code=${CODE}&error=server_error`, ""), { kind: "error", code: "server_error" });
  assert.deepEqual(parseAuthLink("", "#error_description=Something"), { kind: "error", code: null });
});

test("an error code that is not plain lowercase letters, digits and _ is dropped", () => {
  assert.deepEqual(parseAuthLink("?error_code=<b>hi</b>", ""), { kind: "error", code: null });
  assert.deepEqual(parseAuthLink(`?error_code=${"a".repeat(65)}`, ""), { kind: "error", code: null });
});

test("a token_hash link needs a known type", () => {
  assert.deepEqual(parseAuthLink("?token_hash=pkce_0123456789abcdef&type=recovery", ""), {
    kind: "token-hash",
    tokenHash: "pkce_0123456789abcdef",
    type: "recovery",
  });
  assert.deepEqual(parseAuthLink("?token_hash=pkce_0123456789abcdef&type=sms", ""), { kind: "error", code: null });
  assert.deepEqual(parseAuthLink("?token_hash=pkce_0123456789abcdef", ""), { kind: "error", code: null });
  assert.deepEqual(parseAuthLink("?token_hash=<x>&type=signup", ""), { kind: "error", code: null });
});

test("a link without PKCE brings a session in the hash, read with its type", () => {
  const hash = `#access_token=${JWT}&expires_at=1&expires_in=3600&refresh_token=r&token_type=bearer&type=invite`;
  assert.deepEqual(parseAuthLink("", hash), { kind: "implicit", type: "invite", accessToken: JWT });
  for (const type of LINK_TYPES) {
    assert.equal((parseAuthLink("", `#access_token=${JWT}&type=${type}`) as { type: string }).type, type);
  }
  assert.deepEqual(parseAuthLink("", "#access_token=not-a-jwt&type=odd"), {
    kind: "implicit",
    type: null,
    accessToken: null,
  });
});

test("the first half of an email change brings a message only", () => {
  assert.deepEqual(parseAuthLink("", "#message=Confirmation+link+accepted"), { kind: "notice" });
});

test("every link without PKCE says what happened and never that you are signed in", () => {
  for (const type of [...LINK_TYPES, null]) {
    const text = implicitLinkMessage(type);
    assert.ok(text.length > 20, String(type));
    assert.doesNotMatch(text, /signed in/i, String(type));
  }
  assert.match(implicitLinkMessage("signup"), /confirmed\. Sign in/);
  assert.match(implicitLinkMessage("invite"), /Forgot password/);
  assert.match(implicitLinkMessage("recovery"), /Forgot password/);
});

test("confirmations say whether a sign-in is still needed", () => {
  assert.equal(confirmedMessage("signup", true), "Your email is confirmed.");
  assert.equal(confirmedMessage(null, false), "Your email is confirmed. Sign in to continue.");
  assert.match(confirmedMessage("email_change", false), /new email address/);
});

test("link errors are explained in words, never as the server's text", () => {
  assert.match(linkErrorMessage("otp_expired"), /expired or was already used/);
  assert.match(linkErrorMessage("flow_state_not_found"), /expired or was already used/);
  assert.match(linkErrorMessage("pkce_code_verifier_not_found"), /another browser/);
  assert.match(linkErrorMessage("over_email_send_rate_limit"), /Too many attempts/);
  assert.match(linkErrorMessage("network"), /Could not reach/);
  assert.match(linkErrorMessage(null), /did not work/);
  assert.match(linkErrorMessage("something_new"), /did not work/);
});
