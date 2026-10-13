// The minion types the minion pages know (T-578, D-134): the key the card
// data uses and the page's address. A type the card data adds that is not
// here stops the build (tools/minionPool.ts), so it is named before it shows.

export interface Tribe {
  /** As HearthstoneJSON names it. */
  key: string;
  /** The page's address under /minions/. */
  slug: string;
}

/** Every type with a page, in the order the index lists them. */
export const TRIBES: readonly Tribe[] = [
  { key: "BEAST", slug: "beast" },
  { key: "DEMON", slug: "demon" },
  { key: "DRAGON", slug: "dragon" },
  { key: "ELEMENTAL", slug: "elemental" },
  { key: "MECHANICAL", slug: "mech" },
  { key: "MURLOC", slug: "murloc" },
  { key: "NAGA", slug: "naga" },
  { key: "PIRATE", slug: "pirate" },
  { key: "QUILBOAR", slug: "quilboar" },
  { key: "UNDEAD", slug: "undead" },
  { key: "ABERRATION", slug: "aberration" },
];

/** A minion of every type (an amalgam): it shows on every type's page. */
export const ALL_TRIBES = "ALL";

/** The page for minions with no type. */
export const NO_TRIBE_SLUG = "no-tribe";

export const KNOWN_TRIBES: ReadonlySet<string> = new Set([...TRIBES.map((t) => t.key), ALL_TRIBES]);
