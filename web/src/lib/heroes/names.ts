// Hero names for the website (T-510, D-092). The server keeps heroes by card
// id; the site shows each one by its base hero's name in the page's language
// (English on /…, the game's Spanish on /es/…, D-086), from a static table
// built from HearthstoneJSON card data by tools/hero_names.py. Skins go by
// their base hero's name, as in the app. Nothing is fetched at run time.
import { lang } from "../i18n.ts";
import { HERO_NAMES, HERO_SKINS } from "./names.data.ts";

const SKIN_SUFFIX = /_SKIN_[A-Za-z0-9]+$/;

/** The base hero of a hero card id (a skin's base, or the id itself); null when the table does not know it. */
export function baseHero(id: string | null | undefined): string | null {
  if (!id) return null;
  if (Object.hasOwn(HERO_NAMES, id)) return id;
  const listed = Object.hasOwn(HERO_SKINS, id) ? HERO_SKINS[id] : id.replace(SKIN_SUFFIX, "");
  return Object.hasOwn(HERO_NAMES, listed) ? listed : null;
}

/** The hero's name in the page's language; null for no id or one the table does not know. */
export function heroName(id: string | null | undefined): string | null {
  const base = baseHero(id);
  if (base === null) return null;
  const [en, es] = HERO_NAMES[base];
  return lang() === "es" ? es : en;
}

/**
 * What a page shows for a hero: its name, or a fallback that never is the
 * bare card id. `unknownId` is set when an id came but the table does not
 * know it, so Advanced can show it.
 */
export interface HeroLabel {
  name: string;
  unknownId: string | null;
}

export function heroLabel(id: string | null | undefined, notRecorded: string, unknown: string): HeroLabel {
  if (!id) return { name: notRecorded, unknownId: null };
  const name = heroName(id);
  return name === null ? { name: unknown, unknownId: id } : { name, unknownId: null };
}

/** Compares two hero ids by their names in the page's language, then by id. */
export function byHeroName(a: string | null, b: string | null): number {
  const an = heroName(a) ?? a ?? "";
  const bn = heroName(b) ?? b ?? "";
  return an.localeCompare(bn, lang()) || (a ?? "").localeCompare(b ?? "");
}

/**
 * The games list's hero filter (T-510): one option per hero, named and
 * sorted by name. A hero in the address with no game still shows, so the
 * filter in force is visible; an id the table does not know reads
 * "Unknown hero (id)", so two of them stay apart.
 */
export function heroOptions(ids: readonly string[], selected: string | null, unknown: string): { value: string; label: string }[] {
  const list = selected && !ids.includes(selected) ? [selected, ...ids] : [...ids];
  return [...new Set(list)].toSorted(byHeroName).map((id) => ({ value: id, label: heroName(id) ?? `${unknown} (${id})` }));
}
