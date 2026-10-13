// The minion pages' words (T-578, D-134). Placeholders are {name}.
// Spanish is in strings.es.ts.

import { makeT } from "../i18n.ts";
import { ES } from "./strings.es.ts";

export const MIN_STRINGS = {
  // Each type in four forms: a page's title, a type mark on a minion,
  // the index's tile, and inside a sentence.
  "tribe.beast.title": "Beast minions",
  "tribe.beast.one": "Beast",
  "tribe.beast.plural": "Beasts",
  "tribe.beast.inline": "Beast",
  "tribe.demon.title": "Demon minions",
  "tribe.demon.one": "Demon",
  "tribe.demon.plural": "Demons",
  "tribe.demon.inline": "Demon",
  "tribe.dragon.title": "Dragon minions",
  "tribe.dragon.one": "Dragon",
  "tribe.dragon.plural": "Dragons",
  "tribe.dragon.inline": "Dragon",
  "tribe.elemental.title": "Elemental minions",
  "tribe.elemental.one": "Elemental",
  "tribe.elemental.plural": "Elementals",
  "tribe.elemental.inline": "Elemental",
  "tribe.mech.title": "Mech minions",
  "tribe.mech.one": "Mech",
  "tribe.mech.plural": "Mechs",
  "tribe.mech.inline": "Mech",
  "tribe.murloc.title": "Murloc minions",
  "tribe.murloc.one": "Murloc",
  "tribe.murloc.plural": "Murlocs",
  "tribe.murloc.inline": "Murloc",
  "tribe.naga.title": "Naga minions",
  "tribe.naga.one": "Naga",
  "tribe.naga.plural": "Naga",
  "tribe.naga.inline": "Naga",
  "tribe.pirate.title": "Pirate minions",
  "tribe.pirate.one": "Pirate",
  "tribe.pirate.plural": "Pirates",
  "tribe.pirate.inline": "Pirate",
  "tribe.quilboar.title": "Quilboar minions",
  "tribe.quilboar.one": "Quilboar",
  "tribe.quilboar.plural": "Quilboar",
  "tribe.quilboar.inline": "Quilboar",
  "tribe.undead.title": "Undead minions",
  "tribe.undead.one": "Undead",
  "tribe.undead.plural": "Undead",
  "tribe.undead.inline": "Undead",
  "tribe.aberration.title": "Aberration minions",
  "tribe.aberration.one": "Aberration",
  "tribe.aberration.plural": "Aberrations",
  "tribe.aberration.inline": "Aberration",
  "tribe.no-tribe.title": "Neutral minions",
  "tribe.no-tribe.one": "No type",
  "tribe.no-tribe.plural": "Neutral",
  "tribe.no-tribe.inline": "neutral",

  "meta.index.title": "Battlegrounds minions by type",
  "meta.index.description":
    "Every minion in the current Battlegrounds tavern pool, sorted by type and tavern tier: Beasts, Demons, Dragons, Murlocs and more, with Attack, Health, golden stats and card text.",
  "meta.tribe.title": "{title} in Battlegrounds by tier",
  "meta.tribe.description":
    "All {count} {inline} minions in the current Battlegrounds tavern pool, by tavern tier, with Attack, Health, golden stats and card text.",
  "meta.none.title": "Neutral minions (no type) in Battlegrounds by tier",
  "meta.none.description":
    "All {count} neutral Battlegrounds minions, the ones with no type, in the current tavern pool, by tavern tier, with Attack, Health, golden stats and card text.",

  "crumb.label": "Breadcrumb",
  "crumb.home": "Home",
  "crumb.minions": "Minions",

  "index.kicker": "Battlegrounds",
  "index.title": "Minions by type",
  "index.intro":
    "Every minion the tavern offers in the current pool, one page per type. A minion of two types is on both pages; a minion of every type is on all of them.",
  "index.tribes": "Types",
  "index.tiers": "Minions per tavern tier",
  "index.tierCell": "Tier {tier}: {count}",

  "tribe.kicker": "Battlegrounds minions",
  "tribe.intro": "Every {inline} minion in the current tavern pool, by tavern tier.",
  "tribe.intro.none": "Every minion with no type in the current tavern pool, by tavern tier.",
  "tribe.jump": "Tavern tiers",
  "tribe.others": "Other types",

  "count.one": "{count} minion",
  "count.other": "{count} minions",
  "source": "Card art and card data © Blizzard Entertainment, from {source}.",

  "tier": "Tier {tier}",
  "stats.label": "{attack} Attack, {health} Health",
  "golden": "Golden",
  "golden.label": "Golden: {attack} Attack, {health} Health",
  "mark.every": "Every type",
  "mark.duos": "Duos only",
  "types.label": "Types",

  "layout.minions": "Minions by type",
  "heroes.minions": "Every minion in the tavern's pool, by type",
} as const;

export type MinKey = keyof typeof MIN_STRINGS;

/** The text of `key` in the page's language. */
export const mt = makeT({ name: "minions", en: MIN_STRINGS, es: ES });
