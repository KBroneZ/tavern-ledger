// Every string the leaderboards page shows (T-506), kept apart from the data
// pages' table (lib/data/strings.ts) so the two pages can change at the same
// time; T-508 adds Spanish beside both. Same rules: {name} placeholders, and
// lt() refuses a missing one.

import { makeT } from "../i18n.ts";
import { ES } from "./strings.es.ts";

export const LB_STRINGS = {
  "page.kicker": "Public",
  "page.title": "Leaderboards",
  "page.intro":
    "Players who made their profile public, ranked on the games they uploaded from the app. Self-reported numbers, no ratings.",
  "page.updated": "Updated {time}.",
  "page.updated.never": "Not built yet.",
  "page.loading": "Loading the boards",
  "page.loaded": "Boards loaded",
  "page.error": "Could not load the boards",
  "page.error.body": "Check your connection and try again.",
  "page.retry": "Try again",

  "period.label": "Period",
  "period.7d": "7 days",
  "period.30d": "30 days",
  "period.all": "All time",
  "period.7d.long": "the last 7 days",
  "period.30d.long": "the last 30 days",
  "period.all.long": "all time",

  "section.simple": "Main boards",
  "section.advanced": "Economy and fun",

  "col.standing": "#",
  "col.player": "Player",
  "col.hero": "Hero",
  "col.games": "Games",

  "board.avg_place": "Average place",
  "board.avg_place.col": "Avg place",
  "board.avg_place.how":
    "Average final place over the player's finished games. Lower is better. In Duos the place is the team's (1st to 4th).",
  "board.top_half_rate": "Top 4 rate",
  "board.top_half_rate.duos": "Top 2 rate",
  "board.top_half_rate.col": "Top 4",
  "board.top_half_rate.col.duos": "Top 2",
  "board.top_half_rate.how": "Share of finished games that ended in the top 4 (Solo) or the top 2 (Duos).",
  "board.win_rate": "Win rate",
  "board.win_rate.col": "1st",
  "board.win_rate.how": "Share of finished games that ended in 1st place.",
  "board.games": "Games played",
  "board.games.col": "Games",
  "board.games.how": "Finished games uploaded for this mode and period.",
  "board.top_half_streak": "Longest top 4 streak",
  "board.top_half_streak.duos": "Longest top 2 streak",
  "board.top_half_streak.col": "In a row",
  "board.top_half_streak.col.duos": "In a row",
  "board.top_half_streak.how":
    "Most top 4 (Solo) or top 2 (Duos) finishes in a row, in the order the games were played, counting only games in the period.",
  "board.hero_avg_place": "Best average place per hero",
  "board.hero_avg_place.col": "Avg place",
  "board.hero_avg_place.how":
    "Average place with one hero (a skin counts as its hero), over the player's finished games with it in the period. A player can appear once per hero.",
  "board.apm": "Actions per minute",
  "board.apm.col": "APM",
  "board.apm.how":
    "Actions the game's log records the player's client sending (buys, sells, rolls, freezes, cards and hero powers played, minions moved, picks) per minute, averaged over the games that recorded them. No keyboard or mouse input is read.",
  "board.gold_beyond_income": "Gold beyond income",
  "board.gold_beyond_income.col": "Per game",
  "board.gold_beyond_income.how":
    "Gold beyond each turn's own income (extra gold and gold from selling), per game, over the games that recorded it.",
  "board.rolls": "Rolls",
  "board.rolls.col": "Per game",
  "board.rolls.how": "Tavern rolls per game, free ones included, over the games that recorded them.",
  "board.triples": "Triples",
  "board.triples.col": "Per game",
  "board.triples.how": "Triples per game, as the game's own leaderboard counts them at the end of the game.",
  "board.biggest_board": "Biggest board",
  "board.biggest_board.col": "Attack + health",
  "board.biggest_board.how":
    "The biggest attack plus health total of the player's own board entering a combat, in any one game of the period.",
  "board.most_golden": "Most golden minions",
  "board.most_golden.col": "Golden",
  "board.most_golden.how": "The most golden minions on the player's own board entering a combat, in any one game of the period.",

  "board.how": "How this is counted",
  "board.how.count":
    "Counts finished games from {period} that recorded this number. A player is listed from their first one; a value from fewer than {min} games is marked \"few games\", as it can change a lot with the next game.",
  "board.how.count.hero":
    "Counts finished games from {period}. A player is listed from their first game with a hero; a value from fewer than {hero} games with the hero is marked \"few games\", as it can change a lot with the next game.",
  "board.how.source":
    "Self-reported: every game comes from the player's own app, and the server checks each report before storing it. A game with a value no real game can have is left out of every board.",
  "board.how.ties": "Equal values share a place; then more games come first.",
  "board.empty": "No public players yet",
  "board.empty.body": "No player with a public profile has a finished game for this board in {period}.",
  "few": "few games",
  "few.tip": "Rests on fewer than {min} games: it can change a lot with the next one.",
  "few.tip.hero": "Rests on fewer than {min} games with this hero: it can change a lot with the next one.",
  "board.caption": "{board}, {mode}, {period}",

  "about.summary": "About these boards",
  "about.who":
    "Only players who turned on \"Public profile\" on their account page are listed, by the display name they chose, with a link to their public profile. Turning the profile private removes them from every board as soon as they save.",
  "about.min": "A board lists every public player with a finished game in the mode and period. A value from fewer than {min} games ({hero} with the hero on the hero board) is marked \"few games\": a 100% win rate from one game is one win. Each board shows its top 50.",
  "about.refresh": "The boards are rebuilt from the uploaded games every 10 minutes. A new public profile or new games show at the next rebuild.",
  "about.data":
    "Nothing here comes from Blizzard: no ratings, no BattleTags and no other players' names. Numbers are self-reported by each player's app; see the privacy policy for what becomes public.",
  "about.privacy": "Privacy policy",

  "hero.unknown": "Unknown hero",
} as const;

export type LbKey = keyof typeof LB_STRINGS;

/** The string for a key in the page's language, with its placeholders filled. */
export const lt = makeT<LbKey>({ name: "leaderboards", en: LB_STRINGS, es: ES });
