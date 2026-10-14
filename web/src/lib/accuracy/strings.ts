// The simulator accuracy page's words (T-607). Placeholders are {name}.
// Spanish is in strings.es.ts.

import { makeT } from "../i18n.ts";
import { ES } from "./strings.es.ts";

export const ACC_STRINGS = {
  "page.kicker": "Combat simulator",
  "page.title": "How often the odds are right",
  "page.intro":
    "The simulator behind the overlay's odds, checked on real combats: for each combat it gives the chance of a win, a tie and a loss, and the game's log says what really happened.",
  "page.version": "Engine {version} · measured {date} · {trials} trials per combat",

  "mode.solo": "Solo",
  "mode.duos": "Duos",
  "mode.combats": "{count} combats with the game's result",

  "tile.right": "Most likely result was right",
  "tile.meanP": "Average chance given to the real result",
  "tile.meanP.back": "over {count} scored combats",
  "tile.blocked": "No odds: a card is not modelled",
  "tile.brier": "Brier score",
  "tile.brier.back": "0 is best, 2 is worst",
  "tile.refused": "Refused by the board check",

  "right.backing": "{right} of {scored} combats",
  "blocked.some": "No odds in {share} of {mode} combats ({blocked} of {total}): a card is not modelled.",
  "blocked.none": "Odds in every {mode} combat with a result ({total}): no card was missing.",
  "blocked.empty": "No {mode} combats with a result yet.",

  "cal.title": "Predicted and real",
  "cal.desc": "Every chance the simulator gave in {mode} (a win, a tie and a loss per combat), grouped by what it predicted.",
  "cal.more": "Beside each group, how often that result really happened. When the two bars are close, the odds can be trusted.",
  "cal.key.said": "Predicted",
  "cal.key.happened": "Real",
  "cal.none": "no predictions",
  "cal.mark": "Predicted {range}: {predictions} predictions, {said} on average, real {happened}",
  "cal.table": "Show as a table",
  "cal.col.range": "Predicted",
  "cal.col.predictions": "Predictions",
  "cal.col.said": "Average predicted",
  "cal.col.happened": "Real",

  "causes.title": "Why the odds missed",
  "causes.desc": "{total} combats where the most likely result was not the real one, played on every board as it fought.",
  "causes.more": "A card the simulator does not know counts as a plain minion. Largest cause first; Solo and Duos counted together.",
  "causes.col.cause": "Cause",
  "causes.col.combats": "Combats",
  "causes.col.share": "Share",
  "causes.col.solo": "Solo",
  "causes.col.duos": "Duos",
  "causes.none": "No combat has been missed yet.",
  "cause.chance": "Chance",
  "cause.chance.why": "The replay follows the game to the end: the odds were right and the less likely result happened.",
  "cause.missing_inputs": "Not seen",
  "cause.missing_inputs.why": "The replay parts from the game, and a board brings what the log does not show (Blood Gem upgrades, a hand summon).",
  "cause.approximate_card": "Approximate card",
  "cause.approximate_card.why": "The replay parts from the game, and a card or Dark Gift is not modelled or has a known gap.",
  "cause.replay_first_attacker": "Who attacks first",
  "cause.replay_first_attacker.why": "The simulator lets a different side attack first than the game did.",
  "cause.replay_after_teammate": "Attacker after a teammate joins",
  "cause.replay_after_teammate.why": "In Duos, a different side attacks next once a teammate's board comes in.",
  "cause.replay_attack_order": "Attack order",
  "cause.replay_attack_order.why": "Sides or minions take their turns in another order than in the game.",
  "cause.replay_target": "Target choice",
  "cause.replay_target.why": "The game picked a target the simulator's Taunt or Stealth rules do not allow.",
  "cause.replay_minions": "Deaths and summons",
  "cause.replay_minions.why": "After an attack, different minions die, come back or are summoned than in the game.",
  "cause.replay_stats": "Stats and keywords",
  "cause.replay_stats.why": "The same minions end an attack with other Attack, Health or keywords.",
  "cause.replay_end": "End of combat",
  "cause.replay_end.why": "The combat ends, or its result differs, where the simulator's does not.",
  "cause.replay_no_defender": "Defender not in the log",
  "cause.replay_no_defender.why": "The log does not say who was attacked, so the check stops there.",
  "cause.no_replay_not_modelled": "Card not modelled",
  "cause.no_replay_not_modelled.why": "No replay was saved, and a card the simulator does not know was played as a plain minion.",
  "cause.no_replay_approximate": "Card played approximately",
  "cause.no_replay_approximate.why": "No replay was saved, and a card is played with a known gap.",
  "cause.no_replay_exact": "Not known",
  "cause.no_replay_exact.why": "No replay was saved and every card is exact, so the cause cannot be told.",

  "builds.title": "Supported game versions",
  "builds.desc": "The parser and the simulator were checked on games played on these builds.",
  "builds.newest": "Newest build checked: {build}, on {date}.",
  "builds.none": "No game version has been read yet.",
  "builds.col.build": "Build",
  "builds.col.games": "Games",
  "builds.row.one": "{count} game",
  "builds.row.other": "{count} games",

  "cards.title": "Cards that block the odds most",
  "cards.desc":
    "When one of these is on a board, the simulator gives no odds instead of a wrong number. Solo and Duos counted together.",
  "cards.col.card": "Card",
  "cards.col.id": "Card id",
  "cards.col.combats": "Combats blocked",
  "cards.none": "No card blocked a combat.",
  "card.golden": "{name} (golden)",

  "about.title": "What is measured, and its limits",
  "about.body": "Only the combats of the person who builds Tavern Ledger, from their own games, with the boards as they entered each combat (the first attack). Small numbers, one player: a guide, not a promise.",
  "about.more": "Start of combat effects are off, as in the overlay. A combat counts as scored when the simulator could play it; the share of combats with no odds is always shown beside it.",
  "about.refresh": "These numbers are refreshed after each round of new cards.",
} as const;

export type AccKey = keyof typeof ACC_STRINGS;

/** The string for a key in the page's language, with its placeholders filled. */
export const at = makeT<AccKey>({ name: "accuracy", en: ACC_STRINGS, es: ES });
