// What the minion pages show (T-578, D-134), worked out from the pool file
// at build time: which pages exist, which minions each lists, grouped by
// tier, the marks on each minion, and the pages' structured data.

import { href, locale } from "../i18n.ts";
import type { PoolFile, PoolMinion } from "./poolFile.ts";
import { type MinKey, mt } from "./strings.ts";
import { ALL_TRIBES, KNOWN_TRIBES, NO_TRIBE_SLUG, TRIBES } from "./tribes.ts";

export const MAX_TIER = 7;

/** One type's page: its key in the card data, or null for minions with no type. */
export interface TribePage {
  slug: string;
  key: string | null;
}

/** The pool file the build wrote; anything else stops the build. */
export function readPool(raw: unknown): PoolFile {
  const file = raw as PoolFile | null;
  if (!file || file.version !== 1 || typeof file.source !== "string" || !Array.isArray(file.minions)) {
    throw new Error("not a minion pool file: run tools/prepare-sim.ts");
  }
  return file;
}

/** The minions a page lists: of its type or of every type; for no type, those with none. */
export function tribeMinions(pool: PoolFile, key: string | null): PoolMinion[] {
  if (key === null) return pool.minions.filter((m) => m.tribes.length === 0);
  return pool.minions.filter((m) => m.tribes.includes(key) || m.tribes.includes(ALL_TRIBES));
}

/**
 * The pages to build: each type with at least one minion of its own (a type
 * with only every-type minions is not in the pool), then minions with no type.
 */
export function tribePages(pool: PoolFile): TribePage[] {
  const pages: TribePage[] = TRIBES.filter((t) => pool.minions.some((m) => m.tribes.includes(t.key))).map((t) => ({ slug: t.slug, key: t.key }));
  if (pool.minions.some((m) => m.tribes.length === 0)) pages.push({ slug: NO_TRIBE_SLUG, key: null });
  return pages;
}

export interface OtherType {
  key: string;
  name: string;
  /** By tier, then by name. */
  minions: PoolMinion[];
}

/**
 * Types the card data has but the pages do not know yet (a type added
 * mid-season): no page of their own; the index lists their minions.
 */
export function otherTypes(pool: PoolFile): OtherType[] {
  const keys = [...new Set(pool.minions.flatMap((m) => m.tribes))].filter((k) => !KNOWN_TRIBES.has(k)).sort();
  return keys.map((key) => ({
    key,
    name: typeName(key),
    minions: tierGroups(pool.minions.filter((m) => m.tribes.includes(key))).flatMap((g) => g.minions),
  }));
}

/** A type the pages do not know, named from the card data's own key ("BIG_BOSS" → "Big boss"), else "Other". */
export function typeName(key: string): string {
  const words = key.toLowerCase().replaceAll("_", " ").trim();
  return words ? words[0].toUpperCase() + words.slice(1) : mt("other.type");
}

export type TribeForm = "title" | "one" | "plural" | "inline";

/** A type's name in one of its forms, in the page's language. */
export const tribeName = (slug: string, form: TribeForm): string => mt(`tribe.${slug}.${form}` as MinKey);

export const tribePath = (slug: string): string => `/minions/${slug}/`;

/** The address of a minion on its page (card ids are letters, digits and "_"). */
export const minionAnchor = (id: string): string => `m-${id}`;

export interface TierGroup {
  tier: number;
  minions: PoolMinion[];
}

/** Minions by tier, lowest first; by name within a tier, the language's way. */
export function tierGroups(minions: readonly PoolMinion[]): TierGroup[] {
  const collator = new Intl.Collator(locale());
  const groups: TierGroup[] = [];
  for (let tier = 1; tier <= MAX_TIER; tier += 1) {
    const here = minions.filter((m) => m.tier === tier).sort((a, b) => collator.compare(a.name, b.name) || (a.id < b.id ? -1 : 1));
    if (here.length > 0) groups.push({ tier, minions: here });
  }
  return groups;
}

/** How many minions each tier has, tiers 1 to 7. */
export function tierSpread(minions: readonly PoolMinion[]): number[] {
  return Array.from({ length: MAX_TIER }, (_, i) => minions.filter((m) => m.tier === i + 1).length);
}

export interface Mark {
  text: string;
  /** The other type's page, for a type mark. */
  href?: string;
}

/** A minion's marks on a page: its other types (linked), every type, Duos only. */
export function marks(minion: PoolMinion, pageKey: string | null): Mark[] {
  const out: Mark[] = [];
  for (const key of minion.tribes) {
    if (key === pageKey) continue;
    if (key === ALL_TRIBES) {
      out.push({ text: mt("mark.every") });
      continue;
    }
    const tribe = TRIBES.find((t) => t.key === key);
    // A type the pages do not know yet has no page: named, not linked.
    out.push(tribe ? { text: tribeName(tribe.slug, "one"), href: href(tribePath(tribe.slug)) } : { text: typeName(key) });
  }
  if (minion.duosOnly) out.push({ text: mt("mark.duos") });
  return out;
}

/** "3 minions" in the page's language. */
export function countText(count: number): string {
  return mt(new Intl.PluralRules(locale()).select(count) === "one" ? "count.one" : "count.other", { count });
}

// ---------- Structured data (schema.org) ----------

export interface Linked {
  name: string;
  url: string;
}

export function breadcrumbLd(items: readonly Linked[]) {
  return {
    "@type": "BreadcrumbList",
    itemListElement: items.map((item, i) => ({ "@type": "ListItem", position: i + 1, name: item.name, item: item.url })),
  };
}

export function itemListLd(name: string, items: readonly Linked[]) {
  return {
    "@type": "ItemList",
    name,
    numberOfItems: items.length,
    itemListElement: items.map((item, i) => ({ "@type": "ListItem", position: i + 1, name: item.name, url: item.url })),
  };
}

/**
 * The page's structured data as one JSON text for a
 * <script type="application/ld+json"> block. Every "<" is escaped, so a
 * name can never end the block early; the JSON means the same.
 */
export function jsonLd(graph: readonly object[]): string {
  return JSON.stringify({ "@context": "https://schema.org", "@graph": graph }).replaceAll("<", "\\u003c");
}
