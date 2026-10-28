// The expected-place calibration page's words (T-655). Placeholders are
// {name}. Spanish is in strings.es.ts.

import { makeT } from "../i18n.ts";
import { ES } from "./strings.es.ts";

export const CAL_STRINGS = {
  "page.kicker": "Expected place",
  "page.title": "How close the expected place comes",
  "page.intro":
    "On each turn of a game, the expected place is the place the game was heading for, from what the player could see at that moment. This page checks it on games the model never saw: for the turns where it said between 3 and 4, say, where those games really finished.",
  "page.date": "Checked on {date}",
  "page.accuracy": "The combat odds have their own check:",
  "page.accuracy.link": "Simulator accuracy",

  "empty.label": "No model is published yet",
  "empty.body":
    "The expected place needs a model trained on at least {min} games per mode, and none has been published. Until one is, neither the app nor the site shows an expected place, and this page has no numbers to show: none rather than made-up ones.",
  "empty.next.title": "What this page will show",
  "empty.next.groups":
    "For each mode and game version, the turns grouped by the expected place given (1 to 2, 2 to 3, and so on), with the average place given, the average place those games really finished in, and how many games and combats are behind each group. When the two averages are close, the expected place can be trusted.",
  "empty.next.thin":
    "A group with fewer than {thin} games shows a dash instead of its averages: too few to tell anything, and too few to keep any one game's place out of sight.",
  "empty.next.distance":
    "Beside them, how far the expected place was from the real place on average, next to two guesses that know nothing about the board.",

  "mode.solo": "Solo",
  "mode.duos": "Duos",
  "mode.none": "No {mode} model is published yet.",
  "mode.summary":
    "Trained on {games} games ({rows} combats) and checked in {folds} parts: each game was predicted by a model trained without it.",
  "mode.notReady": "Not shown to players yet: it was trained on fewer than {min} games of this mode.",
  "mode.ready": "Players can see it: it was trained on at least {min} games of this mode.",

  "distance.label": "Average distance from the real place",
  "distance.back": "In places; lower is better. Guessing from the turn alone: {perTurn}. Guessing from how often each place happens: {overall}.",
  "distance.more":
    "Both guesses are worked out from the games each model was trained on, like the model itself, and checked on the same unseen games. The model's settings were chosen on these same parts, so its own figure is a little optimistic.",

  "build.caption": "Build {build}",
  "build.summary": "{games} games, {rows} combats",
  "build.col.range": "Expected place given",
  "build.col.said": "Average given",
  "build.col.real": "Average finish",
  "build.col.games": "Games",
  "build.col.rows": "Combats",
  "build.range": "{from} to {to}",
  "build.thin": "fewer than {thin} games",
  "build.empty": "none",
  "build.note":
    "Each combat's expected place counts once. A group runs from its first number up to the next one, not included; the last group includes {last}. A dash: no combat in that group, or fewer than {thin} games.",
  "builds.other.one":
    "{count} more game, on a game version with fewer than {thin} games or an unknown one, counts in the total but is not shown by version.",
  "builds.other.other":
    "{count} more games, on game versions with fewer than {thin} games or unknown ones, count in the total but are not shown by version.",
  "builds.none": "No game version has {thin} games yet, so no group can be shown.",

  "about.title": "How it is checked, and its limits",
  "about.body":
    "Cross-validation by game: the games are split into parts, and each part is predicted by a model trained on the others, so a game's turns are never both learned from and checked. The numbers say how the training does on games it has not seen; the published model is then trained on all of them. The model's settings were picked by how well they did on these same parts, so the numbers are a little optimistic.",
  "about.private":
    "Only totals are published, never a game, a player or a hero. An average needs at least {thin} games behind it; a group with fewer shows neither its averages nor its exact counts, only that it has fewer than {thin} games. Each game version is counted on its own, with no total across versions beside it.",
} as const;

export type CalKey = keyof typeof CAL_STRINGS;

/** The string for a key in the page's language, with its placeholders filled. */
export const ct = makeT<CalKey>({ name: "calibration", en: CAL_STRINGS, es: ES });
