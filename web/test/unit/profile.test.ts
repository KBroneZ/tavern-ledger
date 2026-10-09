import { test } from "node:test";
import assert from "node:assert/strict";
import {
  checkDisplayName,
  gameView,
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

test("game view: Solo and Duos, place, hero and date", () => {
  assert.deepEqual(
    gameView({
      game_type: "GT_BATTLEGROUNDS",
      status: "ok",
      hero_card_id: "BG20_HERO_202",
      final_place: 1,
      played_on: "2026-10-09",
    }),
    { date: "2026-10-09", mode: "Solo", hero: "BG20_HERO_202", place: "1st" },
  );
  assert.deepEqual(
    gameView({
      game_type: "GT_BATTLEGROUNDS_DUO",
      status: "ok",
      hero_card_id: "TB_BaconShop_HERO_02",
      final_place: 3,
      played_on: "2026-10-08",
    }).place,
    "3rd",
  );
});

test("game view: unknown is not zero and unfinished is not a place", () => {
  const base = { game_type: "GT_BATTLEGROUNDS", hero_card_id: null, played_on: "2026-10-09" };
  assert.deepEqual(gameView({ ...base, status: "ok", final_place: null }), {
    date: "2026-10-09",
    mode: "Solo",
    hero: "unknown",
    place: "unknown",
  });
  assert.equal(gameView({ ...base, status: "incomplete", final_place: 4 }).place, "unfinished");
  assert.equal(gameView({ ...base, status: "unsupported", final_place: null }).place, "unreadable");
});

test("game view: unexpected values from the server are not shown as they are", () => {
  const view = gameView({
    game_type: "<script>",
    status: "ok",
    hero_card_id: "<img src=x>",
    final_place: 99,
    played_on: "not a date",
  });
  assert.deepEqual(view, { date: "unknown", mode: "unknown", hero: "unknown", place: "unknown" });
});

test("profile save errors: the database check reads as a name rule", () => {
  assert.match(profileErrorMessage({ code: "23514" }), /letters/i);
  assert.match(profileErrorMessage({ code: "PGRST301" }), /sign in/i);
  assert.doesNotMatch(profileErrorMessage({ code: "XX000", message: "secret detail" }), /secret/);
});
