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

test("display name: only Latin letters with accents; look-alike alphabets and marks are refused", () => {
  assert.deepEqual(checkDisplayName("Zoë Øster Łukasz Weiß"), { ok: true, value: "Zoë Øster Łukasz Weiß" });
  // A letter plus a combining accent is saved composed.
  assert.deepEqual(checkDisplayName("Café"), { ok: true, value: "Café" });
  for (const name of ["fuсk", "Аdmin", "Αlex", "日本語", "Ａｂｃ", "Bob×", "a​b", "Bob😀"]) {
    assert.equal(checkDisplayName(name).ok, false, name);
  }
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

test("profile save errors: a name another public profile uses says so", () => {
  assert.match(profileErrorMessage({ code: "23505" }), /already uses this name/i);
  assert.match(profileErrorMessage({ code: "23514", message: "display_name_chars" }), /letters/i);
});

test("profile save errors: a name the word list refuses says so, without the list", () => {
  const text = profileErrorMessage({ code: "23514", message: "display_name_not_allowed" });
  assert.match(text, /not allowed/i);
  assert.match(text, /leaderboards/i);
  assert.doesNotMatch(profileErrorMessage({ code: "23514", message: "new row violates check constraint" }), /not allowed/i);
});
