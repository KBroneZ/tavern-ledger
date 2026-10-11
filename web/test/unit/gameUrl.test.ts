import { test } from "node:test";
import assert from "node:assert/strict";
import { gamePath, parseGameRef, playedOn, storagePath } from "../../src/lib/game/url.ts";

const ID = "0f8fad5b-d9cb-469f-a165-70867728950e";
const SESSION = "Hearthstone_2026_10_09_20_15_03";

test("game address: round trip", () => {
  const path = gamePath(ID, SESSION, 3);
  assert.equal(path, `/game/?u=${ID}&s=${SESSION}&g=3`);
  assert.deepEqual(parseGameRef(path.slice("/game/".length)), { userId: ID, session: SESSION, index: 3 });
});

test("game address: the user id is read in lower case", () => {
  assert.equal(parseGameRef(`?u=${ID.toUpperCase()}&s=${SESSION}&g=1`)?.userId, ID);
});

test("game address: anything that is not a game key is refused", () => {
  const bad = [
    "",
    `?s=${SESSION}&g=1`,
    `?u=${ID}&g=1`,
    `?u=${ID}&s=${SESSION}`,
    `?u=not-a-uuid&s=${SESSION}&g=1`,
    `?u=${ID}&s=Hearthstone_2026_10_09&g=1`,
    `?u=${ID}&s=../${SESSION}&g=1`,
    `?u=${ID}&s=${SESSION}%2F..&g=1`,
    `?u=${ID}&s=${SESSION}&g=0`,
    `?u=${ID}&s=${SESSION}&g=01`,
    `?u=${ID}&s=${SESSION}&g=1001`,
    `?u=${ID}&s=${SESSION}&g=-1`,
    `?u=${ID}&s=${SESSION}&g=1.5`,
    `?u=${ID}&s=${SESSION}&g=1e2`,
  ];
  for (const search of bad) assert.equal(parseGameRef(search), null, search);
  assert.equal(parseGameRef(`?u=${ID}&s=${SESSION}&g=1000`)?.index, 1000);
});

test("game address: the storage path is games.storage_path", () => {
  assert.equal(storagePath({ userId: ID, session: SESSION, index: 12 }), `${ID}/${SESSION}-12.json.gz`);
});

test("game date: from the session name, null when it is not a date", () => {
  assert.equal(playedOn(SESSION), "2026-10-09");
  assert.equal(playedOn("Hearthstone_2026_02_30_10_00_00"), null);
  assert.equal(playedOn("nope"), null);
});
