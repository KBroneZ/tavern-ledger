// Saved games (T-637, second part): the choice, what is kept, the limit, and
// that signing out, turning it off and the delete button remove everything.
// A stand-in Cache Storage and local storage take the browser's place.
import { test } from "node:test";
import assert from "node:assert/strict";
import {
  type Device,
  eraseSaved,
  forgetOthers,
  FLAG_KEY,
  forget,
  isOn,
  listSaved,
  readSaved,
  SAVED_LIMIT,
  saveGame,
  type SavedGame,
  turnOff,
  turnOn,
} from "../../src/lib/pwa/saved.ts";
import { SAVED_CACHE, SAVED_INDEX_KEY, savedGameKey } from "../../src/lib/pwa/rules.js";

const USER = "11111111-1111-4111-8111-111111111111";
const id = (n: number) => `00000000-0000-4000-8000-${String(n).padStart(12, "0")}`;

function device(on = true): Device & { stores: Map<string, Map<string, Response>>; flags: Map<string, string> } {
  const stores = new Map<string, Map<string, Response>>();
  const flags = new Map<string, string>();
  if (on) flags.set(FLAG_KEY, USER);
  return {
    stores,
    flags: Object.assign(flags, {
      getItem: (key: string) => flags.get(key) ?? null,
      setItem: (key: string, value: string) => void flags.set(key, value),
      removeItem: (key: string) => void flags.delete(key),
    }) as never,
    caches: {
      has: async (name: string) => stores.has(name),
      delete: async (name: string) => stores.delete(name),
      open: async (name: string) => {
        const store = stores.get(name) ?? new Map<string, Response>();
        stores.set(name, store);
        return {
          match: async (key: string) => store.get(key)?.clone(),
          put: async (key: string, response: Response) => void store.set(key, response),
          delete: async (key: string) => store.delete(key),
        };
      },
    },
  };
}

const game = (n: number, extra: Partial<SavedGame> = {}): SavedGame => ({ userId: USER, publicId: id(n), savedAt: n, played: "2026-10-2" + (n % 10), title: `Game ${n}`, ...extra });
const file = (n: number) => new Blob([`report ${n}`]);

test("saving is off until the player turns it on, and off again after turning it off", async () => {
  const d = device(false);
  assert.equal(isOn(d), false);
  await turnOn(d, USER);
  assert.equal(isOn(d), true);
  assert.equal(isOn(d, USER), true);
  assert.equal(isOn(d, "22222222-2222-4222-8222-222222222222"), false, "saving is on for the player who chose it only");
  assert.equal(d.flags.get(FLAG_KEY), USER);
  await turnOff(d);
  assert.equal(isOn(d), false);
});

test("a browser that refuses to answer means off", () => {
  const d = device();
  d.flags.getItem = () => {
    throw new Error("blocked");
  };
  assert.equal(isOn(d), false);
});

test("a saved game is listed and read back, for its own player only", async () => {
  const d = device();
  await saveGame(d, game(1), file(1));
  assert.deepEqual(await listSaved(d), [game(1)]);
  assert.equal(await (await readSaved(d, USER, id(1)))!.text(), "report 1");
  assert.equal(await readSaved(d, "22222222-2222-4222-8222-222222222222", id(1)), null, "another player's id");
  assert.equal(await readSaved(d, USER, id(2)), null, "a game that was not saved");
  assert.deepEqual([...d.stores.get(SAVED_CACHE)!.keys()].toSorted(), [SAVED_INDEX_KEY, savedGameKey(id(1))].toSorted());
});

test("only the last SAVED_LIMIT games are kept, newest first, and the oldest files are deleted", async () => {
  const d = device();
  for (let n = 1; n <= SAVED_LIMIT + 3; n++) await saveGame(d, game(n), file(n));
  const listed = await listSaved(d);
  assert.equal(listed.length, SAVED_LIMIT);
  assert.deepEqual(listed.map((g) => g.publicId), Array.from({ length: SAVED_LIMIT }, (_, i) => id(SAVED_LIMIT + 3 - i)));
  const keys = [...d.stores.get(SAVED_CACHE)!.keys()];
  for (const n of [1, 2, 3]) assert.ok(!keys.includes(savedGameKey(id(n))), `game ${n} is gone`);
  assert.equal(keys.filter((k) => k.startsWith("/__saved__/game/")).length, SAVED_LIMIT);
});

test("opening a saved game again moves it to the top and keeps one copy", async () => {
  const d = device();
  for (const n of [1, 2, 3]) await saveGame(d, game(n), file(n));
  await saveGame(d, game(1, { savedAt: 9 }), file(1));
  assert.deepEqual((await listSaved(d)).map((g) => g.publicId), [id(1), id(3), id(2)]);
});

test("delete, turning off and signing out remove every saved game", async () => {
  for (const remove of [eraseSaved, turnOff, forget]) {
    const d = device();
    await saveGame(d, game(1), file(1));
    await remove(d);
    assert.equal(d.stores.has(SAVED_CACHE), false);
    assert.deepEqual(await listSaved(d), []);
  }
});

test("the delete button leaves the choice on; turning off and signing out turn it off", async () => {
  const on = async () => {
    const d = device();
    return d;
  };
  const erased = await on();
  await eraseSaved(erased);
  assert.equal(isOn(erased), true);
  for (const remove of [turnOff, forget]) {
    const d = await on();
    await remove(d);
    assert.equal(isOn(d), false);
  }
});

test("an index entry that is not a game is ignored, and an unreadable index is an empty list", async () => {
  const d = device();
  const cache = await d.caches.open(SAVED_CACHE);
  const bad = [game(1), { userId: "x", publicId: id(2), savedAt: 1, played: null, title: "" }, { ...game(3), savedAt: "now" }, { ...game(4), title: 5 }, null, "text", { ...game(5), played: null }];
  await cache.put(SAVED_INDEX_KEY, new Response(JSON.stringify(bad)));
  assert.deepEqual((await listSaved(d)).map((g) => g.publicId), [id(1), id(5)]);
  await cache.put(SAVED_INDEX_KEY, new Response("not json"));
  assert.deepEqual(await listSaved(d), []);
  await cache.put(SAVED_INDEX_KEY, new Response(JSON.stringify({ not: "a list" })));
  assert.deepEqual(await listSaved(d), []);
});

test("a game's file is only read if the index lists it", async () => {
  const d = device();
  const cache = await d.caches.open(SAVED_CACHE);
  await cache.put(savedGameKey(id(1)), new Response("orphan"));
  assert.equal(await readSaved(d, USER, id(1)), null);
});

test("reading saved games never makes the cache for a player who saved nothing", async () => {
  const d = device();
  assert.deepEqual(await listSaved(d), []);
  assert.equal(await readSaved(d, USER, id(1)), null);
  assert.equal(d.stores.has(SAVED_CACHE), false);
});

test("a session for another player removes the saved games and turns saving off; the same player's session keeps them", async () => {
  const d = device();
  await saveGame(d, game(1), file(1));
  await forgetOthers(d, USER);
  assert.equal((await listSaved(d)).length, 1);
  assert.equal(isOn(d), true);
  await forgetOthers(d, "22222222-2222-4222-8222-222222222222");
  assert.deepEqual(await listSaved(d), []);
  assert.equal(isOn(d), false);
  assert.equal(d.stores.has(SAVED_CACHE), false);
});

test("turning off still turns the choice off when the browser refuses to delete the games", async () => {
  const d = device();
  d.caches.delete = async () => {
    throw new Error("blocked");
  };
  await assert.rejects(turnOff(d));
  assert.equal(isOn(d), false);
});

test("saves take turns when the browser has locks, and the list is written before a file is deleted", async () => {
  const d = device();
  let busy = false;
  let tail: Promise<unknown> = Promise.resolve();
  d.lock = (step) => {
    const run = tail.then(async () => {
      assert.equal(busy, false, "no two saves inside the lock at once");
      busy = true;
      try {
        return await step();
      } finally {
        busy = false;
      }
    });
    tail = run.catch(() => {});
    return run as never;
  };
  await Promise.all([1, 2].map((n) => saveGame(d, game(n), file(n))));
  assert.deepEqual((await listSaved(d)).map((g) => g.publicId).toSorted(), [id(1), id(2)]);
  // Past the limit: the index put comes before the delete of the oldest file.
  const cache = await d.caches.open(SAVED_CACHE);
  const order: string[] = [];
  const put = cache.put.bind(cache);
  const del = cache.delete.bind(cache);
  const spy = { ...d, caches: { ...d.caches, open: async () => ({ ...cache, match: cache.match, put: async (k: string, r: Response) => (order.push("put " + k), put(k, r)), delete: async (k: string) => (order.push("delete " + k), del(k)) }) } };
  for (let n = 3; n <= SAVED_LIMIT + 1; n++) await saveGame(d, game(n), file(n));
  order.length = 0;
  await saveGame(spy, game(SAVED_LIMIT + 2), file(SAVED_LIMIT + 2));
  assert.ok(order.indexOf("put " + SAVED_INDEX_KEY) < order.findIndex((e) => e.startsWith("delete ")), order.join(", "));
});

test("a choice made by another player is turned off when a different player's session starts, even with nothing saved", async () => {
  const d = device();
  await forgetOthers(d, USER);
  assert.equal(isOn(d), true, "the same player keeps it");
  await forgetOthers(d, "22222222-2222-4222-8222-222222222222");
  assert.equal(isOn(d), false);
  // No choice at all: nothing to turn off, and no cache is made.
  await forgetOthers(d, USER);
  assert.equal(d.stores.has(SAVED_CACHE), false);
});

test("nothing is saved when saving is off for this player at the moment of the save, and a refused list leaves no file behind", async () => {
  const off = device(false);
  await saveGame(off, game(1), file(1));
  assert.equal(off.stores.has(SAVED_CACHE), false);
  const other = device();
  await saveGame(other, game(1, { userId: "22222222-2222-4222-8222-222222222222" }), file(1));
  assert.equal(other.stores.has(SAVED_CACHE), false, "the choice is another player's");
  const d = device();
  const cache = await d.caches.open(SAVED_CACHE);
  const put = cache.put.bind(cache);
  const full = { ...d, caches: { ...d.caches, open: async () => ({ ...cache, put: async (k: string, r: Response) => (k === SAVED_INDEX_KEY ? Promise.reject(new Error("quota")) : put(k, r)) }) } };
  await assert.rejects(saveGame(full, game(1), file(1)), /quota/);
  assert.deepEqual([...d.stores.get(SAVED_CACHE)!.keys()], []);
});

test("a refused list update keeps the file of a game that was already listed", async () => {
  const d = device();
  await saveGame(d, game(1), file(1));
  const cache = await d.caches.open(SAVED_CACHE);
  const put = cache.put.bind(cache);
  const full = { ...d, caches: { ...d.caches, open: async () => ({ ...cache, put: async (k: string, r: Response) => (k === SAVED_INDEX_KEY ? Promise.reject(new Error("quota")) : put(k, r)) }) } };
  await assert.rejects(saveGame(full, game(1, { savedAt: 9 }), file(1)), /quota/);
  assert.equal(await (await readSaved(d, USER, id(1)))!.text(), "report 1");
});

