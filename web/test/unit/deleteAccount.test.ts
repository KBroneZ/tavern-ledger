import { test } from "node:test";
import assert from "node:assert/strict";
import { CONFIRM_PHRASE, deleteOutcome, isConfirmed } from "../../src/lib/deleteAccount.ts";

test("the typed confirmation must match the phrase", () => {
  assert.equal(CONFIRM_PHRASE, "delete my account");
  assert.equal(isConfirmed("delete my account"), true);
  assert.equal(isConfirmed("  delete my account  "), true);
  assert.equal(isConfirmed("Delete My Account"), false);
  assert.equal(isConfirmed("delete"), false);
  assert.equal(isConfirmed(""), false);
});

test("200 with a deleted count is done", () => {
  assert.deepEqual(deleteOutcome(200, { deleted: { account: 1, files: 0 } }), { kind: "deleted" });
});

test("200 without the expected body is not taken as done", () => {
  assert.equal(deleteOutcome(200, null).kind, "error");
  assert.equal(deleteOutcome(200, { deleted: {} }).kind, "error");
});

test("403 reauthenticate asks for the password again", () => {
  assert.deepEqual(deleteOutcome(403, { reason: "reauthenticate" }), { kind: "reauthenticate" });
});

test("401 means signed out; 500 says to try again and names the step", () => {
  assert.equal(deleteOutcome(401, {}).kind, "signed-out");
  const failed = deleteOutcome(500, { step: "delete files" });
  assert.equal(failed.kind, "error");
  assert.match(failed.kind === "error" ? failed.message : "", /try again/i);
  assert.match(failed.kind === "error" ? failed.message : "", /delete files/);
});

test("a step name from the server is only shown if it is a known one", () => {
  const failed = deleteOutcome(500, { step: "<img src=x>" });
  assert.doesNotMatch(failed.kind === "error" ? failed.message : "", /img/);
});

test("other refusals and network failures", () => {
  assert.equal(deleteOutcome(403, { error: "origin not allowed" }).kind, "error");
  assert.match((deleteOutcome(0, null) as { message: string }).message, /reach/i);
});
