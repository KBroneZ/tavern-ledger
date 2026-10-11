import { test } from "node:test";
import assert from "node:assert/strict";
import {
  checkDisplayName,
  parseProfileId,
  profileErrorMessage,
  publicProfilePath,
} from "../../src/lib/profile.ts";

const ID = "0f8fad5b-d9cb-469f-a165-70867728950e";

test("display name: trimmed, 1-32 letters, digits, spaces, _ . -", () => {
  assert.deepEqual(checkDisplayName("  Tavern Keeper_1.0-a "), { ok: true, value: "Tavern Keeper_1.0-a" });
  assert.deepEqual(checkDisplayName("Álvaro Ñ"), { ok: true, value: "Álvaro Ñ" });
  assert.deepEqual(checkDisplayName("x".repeat(32)), { ok: true, value: "x".repeat(32) });
  assert.equal(checkDisplayName("x".repeat(33)).ok, false);
});

test("display name: empty clears it", () => {
  assert.deepEqual(checkDisplayName("   "), { ok: true, value: null });
});

test("display name: no BattleTag shapes or other symbols", () => {
  for (const name of ["Name#1234", "Name＃1234", "a<b", "a/b", "line\nbreak", "tab\there", "a@b"]) {
    const result = checkDisplayName(name);
    assert.equal(result.ok, false, name);
  }
});

test("profile id: only a UUID from ?id=, lower-cased", () => {
  assert.equal(parseProfileId(`?id=${ID}`), ID);
  assert.equal(parseProfileId(`?id=${ID.toUpperCase()}`), ID);
  assert.equal(parseProfileId(""), null);
  assert.equal(parseProfileId("?id="), null);
  assert.equal(parseProfileId("?id=1 or 1=1"), null);
  assert.equal(parseProfileId(`?id=${ID}x`), null);
  assert.equal(parseProfileId(`?id=${ID}&id=other`), ID);
});

test("public profile path", () => {
  assert.equal(publicProfilePath(ID), `/profile/?id=${ID}`);
});

test("profile save errors: the database check reads as a name rule", () => {
  assert.match(profileErrorMessage({ code: "23514" }), /letters/i);
  assert.match(profileErrorMessage({ code: "PGRST301" }), /sign in/i);
  assert.doesNotMatch(profileErrorMessage({ code: "XX000", message: "secret detail" }), /secret/);
});
