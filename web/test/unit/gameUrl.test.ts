import { test } from "node:test";
import assert from "node:assert/strict";
import { gamePath, isPublicId, parseGameRef, playedOn, storagePath } from "../../src/lib/game/url.ts";

const ID = "0f8fad5b-d9cb-469f-a165-70867728950e";
const GAME = "3f2b8c1e-5d4a-4b7e-9c6d-1a2b3c4d5e6f";
const SESSION = "Hearthstone_2026_10_09_20_15_03";

test("game address: round trip", () => {
  const path = gamePath(ID, GAME);
  assert.equal(path, `/game/?u=${ID}&id=${GAME}`);
  assert.deepEqual(parseGameRef(path.slice("/game/".length)), { userId: ID, publicId: GAME });
});

test("game address: carries nothing but the two ids", () => {
  assert.doesNotMatch(gamePath(ID, GAME), /Hearthstone|20\d\d_\d\d/);
});

test("game address: ids are read in lower case", () => {
  assert.equal(parseGameRef(`?u=${ID.toUpperCase()}&id=${GAME}`)?.userId, ID);
  assert.equal(parseGameRef(`?u=${ID}&id=${GAME.toUpperCase()}`)?.publicId, GAME);
});

test("game address: anything that is not two ids is refused, the old session address included", () => {
  const bad = [
    "",
    `?id=${GAME}`,
    `?u=${ID}`,
    `?u=not-a-uuid&id=${GAME}`,
    `?u=${ID}&id=not-a-uuid`,
    `?u=${ID}&id=../${GAME}`,
    `?u=${ID}&id=${GAME}%2F..`,
    `?u=${ID}&id=${GAME}x`,
    // The address before T-586: the session carried the start time.
    `?u=${ID}&s=${SESSION}&g=1`,
    `?u=${ID}&s=${SESSION}&g=3`,
  ];
  for (const search of bad) assert.equal(parseGameRef(search), null, search);
});

test("game address: only a uuid is a public id", () => {
  assert.equal(isPublicId(GAME), true);
  for (const value of [SESSION, "a/b", "", null, 3, `${GAME}\n`]) assert.equal(isPublicId(value), false, String(value));
});

test("game address: the storage path is the opaque name, with no time in it", () => {
  assert.equal(storagePath({ userId: ID, publicId: GAME }), `${ID}/${GAME}.json.gz`);
});

test("game date: from the session name, null when it is not a date", () => {
  assert.equal(playedOn(SESSION), "2026-10-09");
  assert.equal(playedOn("Hearthstone_2026_02_30_10_00_00"), null);
  assert.equal(playedOn("nope"), null);
});
