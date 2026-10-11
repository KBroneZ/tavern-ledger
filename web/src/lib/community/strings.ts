// Every string the community hero stats page (/heroes/, T-507) shows, kept
// apart from the other pages' tables so pages can change at the same time;
// T-508 adds Spanish beside them. Same rules: {name} placeholders, and ct()
// refuses a missing one.

import { makeT } from "../i18n.ts";
import { ES } from "./strings.es.ts";

export const CH_STRINGS = {
  "page.kicker": "Community",
  "page.title": "Hero stats",
  "page.intro":
    "How each hero does in the games of the players who chose to share them. Self-reported numbers from the Tavern Ledger app, no ratings and no player shown.",
  "page.updated": "Updated {time}, with games up to {day}.",
  "page.updated.never": "Not built yet.",
  "page.loading": "Loading the hero stats",
  "page.loaded": "Hero stats loaded",
  "page.error": "Could not load the hero stats",
  "page.error.body": "Check your connection and try again.",
  "page.retry": "Try again",

  "period.label": "Period",
  "period.30d": "30 days",
  "period.all": "All time",
  "period.30d.in": "in the last 30 days",
  "period.all.in": "over all time",

  "sort.label": "Sort by",
  "sort.games": "Games",
  "sort.avg": "Avg place",
  "sort.top": "Top 4 / Top 2",
  "sort.pick": "Pick rate",
  "sorted.games": "Sorted by games",
  "sorted.avg": "Sorted by average place",
  "sorted.top": "Sorted by top 4 or top 2 rate",
  "sorted.pick": "Sorted by pick rate",

  "col.hero": "Hero",
  "col.games": "Games",
  "col.players": "Players",
  "col.pick": "Picked",
  "col.avg": "Avg place",
  "col.top.solo": "Top 4",
  "col.top.duos": "Top 2",
  "col.win": "1st",
  "col.offered": "Offered",
  "col.offeredPlayers": "Players offered",

  "caption": "Heroes, {mode}, {period}, sorted by {sort}",
  "sort.name.games": "games",
  "sort.name.avg": "average place",
  "sort.name.top": "top 4 or top 2 rate",
  "sort.name.pick": "pick rate",

  "count.atLeast": "{n}+",
  "pick.few": "Not enough hero choices yet",
  "rounding.note":
    "50+ means 50 to 59 games, and 5+ means 5 to 9 players: counts are shown in tens and fives, average place to one decimal and rates in whole percent.",
  "simple.more": "Advanced adds the win rate and how often each hero was offered.",
  "pick.note":
    "Picked: of the games whose hero choice the app recorded and that offered the hero, the share where the player took it. A dash means those games do not reach the same minimum yet.",

  "empty.title": "Not enough games yet",
  "empty.body":
    "A hero shows here once at least {min} games with it, {period}, come from at least {players} players who share their games, and no one or two players have most of them.",
  "empty.join": "Share yours from your account page:",
  "empty.join.link": "Include my games in community hero stats",

  "unavailable.title": "Being rebuilt",
  "unavailable.body":
    "The hero stats are rebuilt once a day and the last rebuild is more than a day old, so nothing is shown until the next one works. Try again later.",

  "how.summary": "How this is counted",
  "how.who":
    "Only games from players who turned on \"Include my games in community hero stats\" on their account page. It is off for everyone until they turn it on, and it does not depend on a public profile: a private player can share, and a public one need not.",
  "how.min":
    "A hero shows only with at least {min} games from at least {players} different players, none with more than half of them and no two with more than three quarters. The pick rate needs the same over the games that offered the hero. All time shows a hero only when its games from before the last 30 days pass the same rule on their own, or there are none. Below that the page says so instead of showing a number from a few games.",
  "how.what":
    "Finished games only, from the last 30 days or all time, up to the day before the rebuild. Average place, top 4 rate in Solo (top 2 in Duos, by team) and win rate are over those games; a skin counts as its hero. Heroes go by card id until the card names reach the site.",
  "how.rounding":
    "Counts are shown in tens of games and fives of players, average place to one decimal and rates in whole percent. Finer numbers would mostly be noise at these volumes, and would let someone work out single games by comparing two days.",
  "how.source":
    "Self-reported: every game comes from a player's own app, and the server checks each report before storing it. A game with a value no real game can have is left out, as on the leaderboards.",
  "how.refresh":
    "Rebuilt once a day from the shared games. Only totals are kept: no player, name or game is stored with them or shown here. A player who stops sharing is out at the next rebuild; if a rebuild fails, the page shows nothing after 26 hours rather than old numbers.",
  "how.privacy": "Privacy policy",
  "how.privacy.lead": "What this means for your data:",
} as const;

export type ChKey = keyof typeof CH_STRINGS;

/** The string for a key in the page's language, with its placeholders filled. */
export const ct = makeT<ChKey>({ name: "community", en: CH_STRINGS, es: ES });
