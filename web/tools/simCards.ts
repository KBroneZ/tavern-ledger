// The card data the website's simulator needs (T-606, D-090), taken from
// HearthstoneJSON's cards.json at build time: every Battlegrounds minion's
// combat facts (the same checks as crates/cards/src/combat_facts.rs, so the
// odds match the app's), its name in the page's language, whether it is in
// the tavern's pool, and its golden card. Nothing else is kept. The data is
// © Blizzard Entertainment (D-038): it is never committed; the build writes
// it next to the page's code and the site serves it from its own origin.

import type { SimCard, SimCardFile } from "../src/lib/sim/cardFile.ts";

export type { SimCard, SimCardFile };

/** The keywords the engine tracks, as HearthstoneJSON spells them. */
export const COMBAT_KEYWORDS = ["TAUNT", "DIVINE_SHIELD", "REBORN", "VENOMOUS", "POISONOUS", "WINDFURY", "MEGA_WINDFURY", "STEALTH"];
const MAX_STAT = 1000;
const MAX_TIER = 7;
const MAX_TRIBES = 4;
const MAX_NAME_CHARS = 100;
/** More than this is not the real data (about 2,500 minions in 2026). */
export const MAX_CARDS = 10_000;

type Obj = Record<string, unknown>;

const isObj = (v: unknown): v is Obj => typeof v === "object" && v !== null && !Array.isArray(v);
const int = (v: unknown): number | null => (typeof v === "number" && Number.isInteger(v) ? v : null);
export const validId = (id: unknown): id is string => typeof id === "string" && /^[A-Za-z0-9_]{1,64}$/.test(id);
const validName = (s: string) => s.length > 0 && s.length <= 32 && /^[A-Z_]+$/.test(s);

/** Invisible characters that change how text reads (as crates/cards refuses them). */
const FORMAT_CHARS = /[­؜᠎​-‏‪-‮⁠-⁯﻿]/;

/** A name we are willing to show (always as text, never markup), or null. */
export function cleanName(v: unknown): string | null {
  if (typeof v !== "string") return null;
  const name = v.trim();
  const chars = [...name].length;
  // eslint-disable-next-line no-control-regex
  if (chars < 1 || chars > MAX_NAME_CHARS || /[\u0000-\u001F\u007F-\u009F]/.test(name) || FORMAT_CHARS.test(name)) return null;
  return name;
}

/** The types of a card; null when a field is there but malformed (the card is left out). */
function tribes(card: Obj): string[] | null {
  if (card.races !== undefined && card.races !== null) {
    if (!Array.isArray(card.races) || !card.races.every((r) => typeof r === "string")) return null;
    return card.races as string[];
  }
  if (card.race !== undefined && card.race !== null) return typeof card.race === "string" ? [card.race] : null;
  return [];
}

/** The combat facts of one entry, or null when it is not a Battlegrounds minion or a field fails a check. */
export function combatFacts(card: Obj): Omit<SimCard, "n" | "p" | "g"> | null {
  if (card.set !== "BATTLEGROUNDS" || card.type !== "MINION" || !validId(card.id)) return null;
  const attack = int(card.attack);
  const health = int(card.health);
  if (attack === null || attack < 0 || attack > MAX_STAT || health === null || health < 1 || health > MAX_STAT) return null;
  let tier: number | null = null;
  if (card.techLevel !== undefined && card.techLevel !== null) {
    tier = int(card.techLevel);
    if (tier === null || tier < 1 || tier > MAX_TIER) return null;
  }
  const r = tribes(card);
  if (!r || r.length > MAX_TRIBES || !r.every(validName)) return null;
  let k: string[] = [];
  if (card.mechanics !== undefined && card.mechanics !== null) {
    if (!Array.isArray(card.mechanics) || !card.mechanics.every((m) => typeof m === "string")) return null;
    k = (card.mechanics as string[]).filter((m) => COMBAT_KEYWORDS.includes(m));
  }
  return { i: card.id as string, t: tier, a: attack, h: health, r, k };
}

/**
 * The simulator's cards from HearthstoneJSON's English list and, for names,
 * the list in the page's language (the same as English for English). A name
 * missing in that language falls back to English.
 */
export function extractCards(english: unknown, local: unknown, source: string): SimCardFile {
  if (!Array.isArray(english) || !Array.isArray(local)) throw new Error("the card data is not a list");
  const names = new Map<string, string>();
  for (const c of local) {
    if (!isObj(c) || !validId(c.id)) continue;
    const name = cleanName(c.name);
    if (name) names.set(c.id, name);
  }
  const byDbf = new Map<number, string>();
  const kept: SimCard[] = [];
  const premium = new Map<string, number>();
  for (const c of english) {
    if (!isObj(c)) continue;
    const facts = combatFacts(c);
    const name = facts ? (names.get(facts.i) ?? cleanName(c.name)) : null;
    if (!facts || !name) continue;
    const dbf = int(c.dbfId);
    if (dbf !== null && dbf > 0) byDbf.set(dbf, facts.i);
    const golden = int(c.battlegroundsPremiumDbfId);
    if (golden !== null && golden > 0) premium.set(facts.i, golden);
    kept.push({ ...facts, n: name, ...(c.isBattlegroundsPoolMinion === true ? { p: 1 as const } : {}) });
  }
  if (kept.length > MAX_CARDS) throw new Error(`${kept.length} minions: not the real card data`);
  const cards = kept.map((card) => {
    const golden = byDbf.get(premium.get(card.i) ?? -1);
    return golden && golden !== card.i ? { ...card, g: golden } : card;
  });
  cards.sort((a, b) => (a.i < b.i ? -1 : a.i > b.i ? 1 : 0));
  return { version: 1, source, cards };
}
