// The press kit page's words (T-704). Placeholders are {name}.
// Spanish is in strings.es.ts. Facts only: no figure the project does not
// have (marketing rule), and the fan notice is the site's own fixed text.

import { makeT } from "../i18n.ts";
import { ES } from "./strings.es.ts";

export const PRESS_STRINGS = {
  "meta.title": "Press kit",
  "meta.description":
    "Tavern Ledger for press and creators: what the free Hearthstone Battlegrounds tracker is, its facts, the logo and screenshots, and how to get in touch.",
  "page.kicker": "Press and creators",
  "page.title": "Press kit",
  "page.lead": "What you need to write about Tavern Ledger or show it on stream: what it is, the facts, the logo and the screenshots.",

  "short.title": "In three sentences",
  "short.1": "Tavern Ledger is a free, open-source Hearthstone Battlegrounds tracker for Windows, with a website to follow your progress.",
  "short.2":
    "It keeps every game you play, read from the game's own log: a recap of each game, combat replays attack by attack, stats for Solo and Duos, and an optional overlay with combat odds from its own simulator.",
  "short.3": "It is made by one person in their spare time, as an unofficial fan project, and it is in beta.",

  "facts.title": "Facts",
  "fact.version": "Current version",
  "fact.version.value": "{version} (beta), {date}",
  "fact.windows": "Works on",
  "fact.windows.value": "Windows 10 and 11, 64-bit",
  "fact.price": "Price",
  "fact.price.value": "Free",
  "fact.license": "Licence",
  "fact.license.value": "Open source, AGPL-3.0",
  "fact.languages": "Languages",
  "fact.languages.value": "English and Spanish",
  "fact.maker": "Made by",
  "fact.maker.value": "KBroneZ, one person",

  "reads.title": "What it reads",
  "reads.body":
    "It reads the log files the game writes on the player's PC. By default it also reads two values the game already shows the player, the lobby's tribes and their own rating, from the game's memory, read-only; they stay on the PC and the player can turn that off. That goes against the letter of Blizzard's rules and the account at risk is the player's, so please say so if you show it. It never changes the game and never plays for you.",
  "reads.link": "The terms, section 9",

  "logo.title": "Logo",
  "logo.body": "Use the files as they are: no new colours, no other words in the lockup, and never next to Blizzard's logos.",
  "logo.lockup": "Logo with the name (SVG, for dark backgrounds)",
  "logo.mark": "Mark only (SVG)",
  "logo.tile": "App tile (SVG)",
  "logo.icon": "App icon (PNG, 512 × 512)",
  "logo.card": "Link preview card (PNG, 1200 × 630)",
  "logo.alt": "The Tavern Ledger logo: a T and an L that form a tankard, beside the name",

  "shots.title": "Screenshots",
  "shots.body": "From the app and the website, on the developer's own games. Card art © Blizzard Entertainment.",
  "shot.games": "The app's list of games",
  "shot.recap": "A game's recap in the app",
  "shot.replay": "A combat replay in the app",
  "shot.home": "The website's home page",
  "shot.accuracy": "The simulator accuracy page",

  "please.title": "Please",
  "please.body": "Call it an unofficial fan project. Never say it is made, approved or endorsed by Blizzard. Its notice reads:",

  "contact.title": "Contact",
  "contact.email": "By email:",
  "contact.github": "Or open an issue on GitHub",
} as const;

export type PressKey = keyof typeof PRESS_STRINGS;

/** The string for a key in the page's language, with its placeholders filled. */
export const pt = makeT<PressKey>({ name: "press", en: PRESS_STRINGS, es: ES });
