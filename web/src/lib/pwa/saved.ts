// Saved games (T-637, second part): "Keep my last games on this device". Off
// until the player turns it on; the choice lives in this browser only. When it
// is on, opening one of your own games stores, in a cache of its own (not one
// the worker versions or deletes), the game's report file, up to SAVED_LIMIT
// games, and the worker adds a copy of the game page (the page's policy lets it
// connect to the account service only), so the game opens without a connection
// (the worker reads this cache, see rules.js savedGame). Nothing here is sent
// anywhere. Signing out, turning the choice off and the delete button remove it
// all. The pages that use this are the game page, the games list, the sign-out
// listener and, through public/pwa.js, the offline page.
import { KEEP_SHELL_MESSAGE, SAVED_CACHE, SAVED_INDEX_KEY, savedGameKey } from "./rules.js";

/** How many games are kept; the oldest save goes first. */
export const SAVED_LIMIT = 10;

/** The localStorage key of the choice. public/pwa.js does not read it. */
export const FLAG_KEY = "tl-keep-games";

/** What the offline page lists about a saved game. `title` is the game page's own heading, in the language it was saved in. */
export interface SavedGame {
  userId: string;
  publicId: string;
  savedAt: number;
  played: string | null;
  title: string;
}

interface CacheLike {
  match(key: string): Promise<Response | undefined>;
  put(key: string, response: Response): Promise<void>;
  delete(key: string): Promise<boolean>;
}

/** The parts of the browser this needs, so the tests can stand in for them. */
export interface Device {
  caches: { open(name: string): Promise<CacheLike>; delete(name: string): Promise<boolean> };
  flags: Pick<Storage, "getItem" | "setItem" | "removeItem">;
}

/** This browser's cache storage and local storage, or null where either is missing (an insecure page, a locked-down browser). */
export function browserDevice(): Device | null {
  try {
    if (typeof caches === "undefined" || typeof localStorage === "undefined") return null;
    return { caches, flags: localStorage };
  } catch {
    return null;
  }
}

/** Whether the player turned saving on. A browser that refuses to answer means off. */
export function isOn(device: Device): boolean {
  try {
    return device.flags.getItem(FLAG_KEY) === "1";
  } catch {
    return false;
  }
}

/** Removes every saved game and the shells. The choice stays as it is. */
export async function eraseSaved(device: Device): Promise<void> {
  await device.caches.delete(SAVED_CACHE);
}

export async function turnOn(device: Device): Promise<void> {
  device.flags.setItem(FLAG_KEY, "1");
}

/** Off: the choice and every saved game go. */
export async function turnOff(device: Device): Promise<void> {
  device.flags.removeItem(FLAG_KEY);
  await eraseSaved(device);
}

/** Sign-out: the same as turning off, so the next person to sign in on this browser starts with nothing saved and saving off. */
export const forget = turnOff;

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;

function readEntry(value: unknown): SavedGame | null {
  if (typeof value !== "object" || value === null) return null;
  const v = value as Record<string, unknown>;
  const { userId, publicId, savedAt, played, title } = v;
  if (typeof userId !== "string" || !UUID.test(userId) || typeof publicId !== "string" || !UUID.test(publicId)) return null;
  if (typeof savedAt !== "number" || !Number.isFinite(savedAt)) return null;
  if (played !== null && typeof played !== "string") return null;
  if (typeof title !== "string") return null;
  return { userId, publicId, savedAt, played, title: title.slice(0, 200) };
}

async function readIndex(cache: CacheLike): Promise<SavedGame[]> {
  const answer = await cache.match(SAVED_INDEX_KEY);
  if (!answer) return [];
  try {
    const value: unknown = await answer.json();
    return Array.isArray(value) ? value.map(readEntry).filter((e): e is SavedGame => e !== null) : [];
  } catch {
    return [];
  }
}

/** The saved games, newest first (opening the cache makes an empty one if there was none). */
export async function listSaved(device: Device): Promise<SavedGame[]> {
  return readIndex(await device.caches.open(SAVED_CACHE));
}

/** The stored report file of one of your saved games, or null. */
export async function readSaved(device: Device, userId: string, publicId: string): Promise<Blob | null> {
  const cache = await device.caches.open(SAVED_CACHE);
  if (!(await readIndex(cache)).some((e) => e.userId === userId && e.publicId === publicId)) return null;
  const answer = await cache.match(savedGameKey(publicId));
  return answer ? answer.blob() : null;
}

/**
 * Saves one game: its report file and its place at the top of the list.
 * Past SAVED_LIMIT the oldest saves are deleted. Saving the same game
 * again only moves it to the top.
 */
export async function saveGame(device: Device, game: SavedGame, file: Blob): Promise<void> {
  const cache = await device.caches.open(SAVED_CACHE);
  await cache.put(savedGameKey(game.publicId), new Response(file, { headers: { "Content-Type": "application/octet-stream" } }));
  const all = [game, ...(await readIndex(cache)).filter((e) => e.publicId !== game.publicId)];
  const kept = all.slice(0, SAVED_LIMIT);
  for (const dropped of all.slice(SAVED_LIMIT)) await cache.delete(savedGameKey(dropped.publicId));
  await cache.put(SAVED_INDEX_KEY, new Response(JSON.stringify(kept), { headers: { "Content-Type": "application/json" } }));
}

/**
 * Asks the worker to fetch the game page in both languages and keep it with the
 * saved games, so a saved game has a page to open in offline. The page cannot do
 * it itself: its policy lets it connect to the account service only, not to its
 * own site. Without a worker yet (the very first visit) nothing happens, and the
 * next game opened asks again.
 */
export async function askForShells(): Promise<void> {
  if (!("serviceWorker" in navigator)) return;
  const registration = await navigator.serviceWorker.getRegistration();
  registration?.active?.postMessage({ type: KEEP_SHELL_MESSAGE });
}

