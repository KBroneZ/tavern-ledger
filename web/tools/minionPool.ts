// The minion pages' data (T-578, D-134), taken from HearthstoneJSON's
// cards.json at build time, from the same lists the simulator's data comes
// from (prepare-sim.ts): every minion in the tavern's current pool with its
// tier, Attack and Health, types, card text and the golden card's stats.
// "In the pool" is the card data's own flag (isBattlegroundsPoolMinion), as
// the simulator reads it. The data is © Blizzard Entertainment (D-038): it is
// never committed; the build writes it next to the pages' code.

import { NEW_LINE, type PoolFile, type PoolMinion, type TextRun } from "../src/lib/minions/poolFile.ts";
import { KNOWN_TRIBES } from "../src/lib/minions/tribes.ts";
import { cleanName, combatFacts, FORMAT_CHARS, validId } from "./simCards.ts";

/** Longer than this is not card text (the longest in 2026 is about 200 characters). */
const MAX_TEXT_CHARS = 600;
/** More than this is not the real pool (about 300 minions in 2026). */
const MAX_POOL = 2000;

type Obj = Record<string, unknown>;

const isObj = (v: unknown): v is Obj => typeof v === "object" && v !== null && !Array.isArray(v);
const int = (v: unknown): number | null => (typeof v === "number" && Number.isInteger(v) ? v : null);

/** Marks a kept line break while the text is read (control characters are refused, so never in the input). */
const BREAK = "\u0001";
const PUNCTUATION_ONLY = /^[\s.,]*$/;

/** A line that is only bold keywords ("<b>Taunt</b>", "<b><b>Reborn</b>.</b>"). */
function keywordLine(line: string): boolean {
  let depth = 0;
  let at = 0;
  let bold = "";
  for (const tag of line.matchAll(/<[^>]*>/g)) {
    const between = line.slice(at, tag.index);
    if (depth > 0) bold += between;
    else if (!PUNCTUATION_ONLY.test(between)) return false;
    at = tag.index + tag[0].length;
    if (tag[0] === "<b>") depth += 1;
    else if (tag[0] === "</b>") depth -= 1;
    else return false;
    if (depth < 0) return false;
  }
  const word = bold.trim();
  // A label ("Battlecry:") reads on with its effect, so it is not a line of its own.
  return depth === 0 && PUNCTUATION_ONLY.test(line.slice(at)) && word.length > 0 && !word.endsWith(":");
}

/**
 * A card's text as runs of plain, bold and italic text, or null when there
 * is none or it holds anything else than <b> and <i> (it is always shown as
 * text, never as markup). The layout mark "[x]" and line breaks go, as the
 * page wraps the text itself; but keywords alone on the first lines
 * ("Taunt", then "Deathrattle: ...") keep their own line: a run that is "\n".
 */
export function cardText(raw: unknown): TextRun[] | null {
  if (typeof raw !== "string" || raw.length > MAX_TEXT_CHARS) return null;
  // eslint-disable-next-line no-control-regex
  if (/[\u0000-\u0009\u000B-\u001F\u007F-\u009F]/.test(raw) || FORMAT_CHARS.test(raw)) return null;
  // A value the game fills in ($ spell damage, @ and # counters, {0}) is unknown here: no text rather than a symbol.
  if (/[@$#{}]/.test(raw)) return null;
  const lines = raw.replace(/^\[x\]/, "").split("\n");
  let lead = 0;
  while (lead < lines.length - 1 && keywordLine(lines[lead])) lead += 1;
  const text = (lines.slice(0, lead).map((l) => l + BREAK).join("") + lines.slice(lead).join(" ")).replace(/\s+/g, " ");
  const runs: TextRun[] = [];
  // Depths, not flags: the card data nests <b> in places ("<b><b>Reborn</b>.</b>").
  let bold = 0;
  let italic = 0;
  let at = 0;
  const pushText = (s: string) => {
    if (!s) return;
    const b = bold > 0;
    const i = italic > 0;
    const last = runs.at(-1);
    if (last && last.s !== NEW_LINE && !!last.b === b && !!last.i === i) {
      runs[runs.length - 1] = { ...last, s: last.s + s };
      return;
    }
    runs.push({ s, ...(b ? { b: 1 as const } : {}), ...(i ? { i: 1 as const } : {}) });
  };
  const push = (s: string) =>
    s.split(BREAK).forEach((part, n) => {
      if (n > 0) runs.push({ s: NEW_LINE });
      pushText(part);
    });
  for (const tag of text.matchAll(/<[^>]*>/g)) {
    push(text.slice(at, tag.index));
    at = tag.index + tag[0].length;
    const mark = /^<(\/?)([bi])>$/.exec(tag[0]);
    if (!mark) return null;
    const step = mark[1] === "" ? 1 : -1;
    if (mark[2] === "b") bold += step;
    else italic += step;
    if (bold < 0 || italic < 0) return null;
  }
  push(text.slice(at));
  if (bold !== 0 || italic !== 0 || /[<>]/.test(runs.map((r) => r.s).join(""))) return null;
  // No space at either end of the text or of a line; a run left empty goes.
  const trimmed = runs.map((r, n) => {
    if (r.s === NEW_LINE) return r;
    let s = r.s;
    if (n === 0 || runs[n - 1].s === NEW_LINE) s = s.trimStart();
    if (n === runs.length - 1 || runs[n + 1].s === NEW_LINE) s = s.trimEnd();
    return { ...r, s };
  });
  // Line breaks only between text: none at either end, never two in a row.
  const kept = trimmed
    .filter((r) => r.s.length > 0)
    .filter((r, n, all) => r.s !== NEW_LINE || (n > 0 && n < all.length - 1 && all[n - 1].s !== NEW_LINE));
  const tidy = kept.at(-1)?.s === NEW_LINE ? kept.slice(0, -1) : kept;
  return tidy.length > 0 ? tidy : null;
}

/** Each card's name and text in one language, by card id. */
function localCards(list: unknown[]): Map<string, { name: string | null; text: unknown }> {
  const map = new Map<string, { name: string | null; text: unknown }>();
  for (const c of list) {
    if (isObj(c) && validId(c.id)) map.set(c.id, { name: cleanName(c.name), text: c.text });
  }
  return map;
}

/**
 * The pool from HearthstoneJSON's English list and, for names and text,
 * the list in the page's language (the same as English for English); a
 * name or text missing in that language falls back to English. A pool
 * minion with no tier or a type the pages do not know stops the build.
 */
export function extractPool(english: unknown, local: unknown, source: string): PoolFile {
  if (!Array.isArray(english) || !Array.isArray(local)) throw new Error("the card data is not a list");
  const lang = localCards(local);
  const statsByDbf = new Map<number, { attack: number; health: number }>();
  const pool: { minion: Omit<PoolMinion, "golden">; goldenDbf: number | null }[] = [];
  for (const c of english) {
    if (!isObj(c)) continue;
    const facts = combatFacts(c);
    if (!facts) continue;
    const dbf = int(c.dbfId);
    if (dbf !== null && dbf > 0) statsByDbf.set(dbf, { attack: facts.a, health: facts.h });
    if (c.isBattlegroundsPoolMinion !== true) continue;
    if (facts.t === null) throw new Error(`pool minion ${facts.i} has no tavern tier`);
    const unknown = facts.r.find((r) => !KNOWN_TRIBES.has(r));
    if (unknown) throw new Error(`unknown minion type ${unknown} (${facts.i}): name it in src/lib/minions/tribes.ts`);
    const here = lang.get(facts.i);
    const name = here?.name ?? cleanName(c.name);
    if (!name) continue;
    const golden = int(c.battlegroundsPremiumDbfId);
    pool.push({
      minion: {
        id: facts.i,
        name,
        tier: facts.t,
        attack: facts.a,
        health: facts.h,
        tribes: facts.r,
        text: cardText(here?.text) ?? cardText(c.text),
        ...(c.isBattlegroundsDuosExclusive === true ? { duosOnly: true as const } : {}),
      },
      goldenDbf: golden !== null && golden > 0 ? golden : null,
    });
  }
  if (pool.length === 0) throw new Error("no minion in the pool: not the real card data");
  if (pool.length > MAX_POOL) throw new Error(`${pool.length} pool minions: not the real card data`);
  const minions = pool.map(({ minion, goldenDbf }) => {
    const golden = goldenDbf === null ? undefined : statsByDbf.get(goldenDbf);
    return golden ? { ...minion, golden } : minion;
  });
  minions.sort((a, b) => (a.id < b.id ? -1 : a.id > b.id ? 1 : 0));
  return { version: 1, source, minions };
}
