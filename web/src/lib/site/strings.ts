// The site's own words outside the data pages (T-508): the layout (menu,
// footer, language switch), each page's title and description, the home
// page, the 404 page and the accounts-closed notice. Placeholders are {name}.
// Spanish is in strings.es.ts.

import { makeT } from "../i18n.ts";
import { ES } from "./strings.es.ts";

export const SITE_STRINGS = {
  // Layout (layouts/Base.astro).
  "layout.skip": "Skip to content",
  "layout.nav": "Main",
  "layout.menu": "Menu",
  "layout.menu.close": "Close",
  "layout.home": "Home",
  "layout.download": "Download",
  "layout.privacy": "Privacy",
  "layout.terms": "Terms",
  "layout.signing": "Code signing policy",
  "layout.accuracy": "Simulator accuracy",
  "layout.leaderboards": "Leaderboards",
  "layout.heroes": "Hero stats",
  "layout.games": "Your games",
  "layout.simulator": "Simulator",
  "layout.signin": "Sign in",
  "layout.account": "Account",
  "layout.notice": "Unofficial fan project. Not affiliated with or endorsed by Blizzard Entertainment.",
  "layout.openSource": "Open source (AGPL-3.0). Accounts are open; the desktop app is in beta for Windows.",
  // The same line in the accounts-closed build.
  "layout.openSource.closed": "Open source (AGPL-3.0). Accounts are not open yet; the desktop app is in beta for Windows.",
  // The switch names the other language in that language: on an English page "Español".
  "layout.switch": "Español",
  "layout.switch.label": "Leer esta página en español",
  // Alt text of the link preview image (public/social-card.png, T-523).
  "layout.socialAlt": "Tavern Ledger logo, a tankard drawn from the letters TL, over the line Hearthstone Battlegrounds tracker",

  // Page titles and descriptions.
  "meta.index.title": "Your Battlegrounds games, kept for you",
  "meta.index.description":
    "Free, open-source Battlegrounds tracker for Windows that reads the game's log files, plus a website to follow your progress.",
  "meta.download.title": "Download for Windows",
  "meta.download.description":
    "Download the Tavern Ledger beta for Windows 10 and 11: a free app that keeps your Hearthstone Battlegrounds games. Four small steps to install it.",
  "meta.privacy.title": "Privacy",
  "meta.privacy.description": "Tavern Ledger privacy policy: what personal data we handle, why, and your rights.",
  "meta.signing.title": "Code signing policy",
  "meta.signing.description": "How Tavern Ledger's Windows installer is signed: what is signed, who approves it and the free signing provided by SignPath.io.",
  "meta.terms.title": "Terms",
  "meta.terms.description": "Tavern Ledger terms of use for the website and its accounts.",
  "meta.me.title": "Your games",
  "meta.me.description": "Your Tavern Ledger dashboard: your games, places, heroes and records.",
  "meta.profile.title": "Player profile",
  "meta.profile.description": "A Tavern Ledger player's public games and stats.",
  "meta.games.title": "Games",
  "meta.games.description": "A Tavern Ledger player's games, with filters.",
  "meta.curious.title": "Records",
  "meta.curious.description": "A Tavern Ledger player's records and totals, each with the games behind it.",
  "meta.playerHeroes.title": "Heroes",
  "meta.playerHeroes.description": "A Tavern Ledger player's heroes: games, places and picks per hero.",
  "meta.heroes.title": "Hero stats",
  "meta.heroes.description":
    "Community Battlegrounds hero stats from Tavern Ledger players who chose to share their games: games, pick rate, average place and top-half rate. Self-reported, no players shown.",
  "meta.leaderboards.title": "Leaderboards",
  "meta.leaderboards.description":
    "Public Tavern Ledger leaderboards: players who made their profile public, ranked on their uploaded Battlegrounds games. Self-reported, no ratings.",
  "meta.accuracy.title": "Simulator accuracy",
  "meta.accuracy.description":
    "How often Tavern Ledger's combat odds are right, measured on real Battlegrounds combats, per mode, with the share of combats that got no odds shown beside every number.",
  "meta.signin.title": "Sign in",
  "meta.signin.description": "Sign in or create a Tavern Ledger account.",
  "meta.account.title": "Your account",
  "meta.account.description": "Your Tavern Ledger account.",
  "meta.forgot.title": "Forgot password",
  "meta.forgot.description": "Choose a new password for your Tavern Ledger account.",
  "meta.desktopSignIn.title": "Sign in to the app",
  "meta.desktopSignIn.description": "Get a sign-in link for the Tavern Ledger desktop app.",

  // 404.
  "notFound.title": "Page not found",
  "notFound.description": "This page does not exist.",
  "notFound.lead": "Nothing is written on this page of the ledger.",
  "notFound.back": "Back to the start",

  // Accounts closed (components/AccountsClosed.astro).
  "closed.label": "Accounts",
  "closed.strong": "Accounts are not open yet.",
  "closed.body":
    "You cannot sign up or sign in on this site for now, and there are no public profiles. The desktop app works on its own and keeps your games on your PC.",
  "closed.read.before": "Until then, read about",
  "closed.read.project": "the project",
  "closed.read.middle": ", the",
  "closed.read.privacy": "privacy policy",
  "closed.read.and": "and the",
  "closed.read.terms": "terms",

  // Home (pages/[...lang]/index.astro).
  "home.step1.title": "Your screen, while you play",
  "home.step1.text": "Seen from the side, it comes apart into layers of glass.",
  "home.step2.title": "The board",
  "home.step2.text": "Your minions. Drawn by the game, as always.",
  "home.step3.title": "The tavern",
  "home.step3.text": "Where you buy. Also drawn by the game.",
  "home.step4.title": "The leaderboard",
  "home.step4.text": "The eight heroes down the side, drawn by the game. Point at one.",
  "home.step5.title": "Tavern Ledger, in front",
  "home.step5.text":
    "One pane of ours: a few notes at the side and the last board you met of the hero you point at. Only what you already saw, and your clicks pass through to the game.",
  "home.scene.label":
    "The game screen seen from the side, split into four panes of glass: the board with your minions, the tavern where you buy and the leaderboard of eight heroes, all drawn by the game, and Tavern Ledger's pane in front, lit, with a few notes at the side and a card for the hero pointed at.",
  "home.hero.title": "Pull up a stool.",
  "home.hero.lit": "We kept your games.",
  "home.lead":
    "A free Windows app that reads Hearthstone's own log on your PC and keeps every Battlegrounds game: your hero, your place and the boards you faced.",
  "home.cta.download": "Download",
  "home.cta.download.note": "Beta for Windows",
  "home.cta.show": "Show me",
  "home.cta.safe": "Is it safe?",
  "home.scene.title": "What Tavern Ledger adds to your screen",
  "home.step": "{n} of {total}",
  "home.pane.board": "Board",
  "home.pane.tavern": "Tavern",
  "home.pane.leaderboard": "Leaderboard",
  "home.keeps.title": "What it keeps for you",
  "home.keeps.body": "Everything comes from the game's own log. When the log does not say, the app says “unknown” instead of guessing.",
  "home.keeps.example": "games kept, in this made-up example",
  "home.keeps.games.title": "Every game, every place",
  "home.keeps.games.body": "Your hero and where you finished, Solo and Duos apart.",
  "home.keeps.boards.title": "The boards you met",
  "home.keeps.boards.body": "Each opponent's board at the start of every combat, as the log records it.",
  "home.keeps.recap.title": "A recap after each game",
  "home.keeps.recap.body": "Your health over the rounds and your record against each opponent.",
  "home.safe.title": "What it does to your game",
  "home.safe.body":
    "It reads the log files the game already writes on your PC and, for the overlay, where your pointer is over the game window. By default it also reads two things the game already shows you, the lobby's tribes and your own rating, from the game's memory, read-only, on your PC only. You can turn that off in Settings. It goes against the letter of Blizzard's rules and the account at risk is yours, as the terms say.",
  "home.safe.1":
    "It never changes the game or adds code to it. The only files it can write are Hearthstone's two logging settings files, and only when you click Fix, with a backup.",
  "home.safe.2": "It never plays or clicks for you.",
  "home.safe.3": "It never shows what you could not see on your own screen.",
  "home.yours.title": "Your games stay yours",
  "home.yours.private.title": "Private until you say",
  "home.yours.private.body": "Your profile is hidden until you make it public.",
  "home.yours.take.title": "Take it all with you",
  "home.yours.take.body": "Download everything the site keeps about you in one file.",
  "home.yours.gone.title": "Gone when you ask",
  "home.yours.gone.body": "Delete your account and every game with it.",
  "home.status.label": "Project status",
  "home.top": "Back up to the door",
  "home.status.title": "The doors are not open yet.",
  "home.status.body": "The desktop app is an early beta for Windows and the website is being built.",
  "home.status.testing": "Accounts are open.",
  "home.status.closed": "Accounts are not open yet.",
  // What's new (lib/site/news.ts): one short dated line each, newest first.
  "news.title": "What's new",
  "news.accuracy": "How often the overlay's combat odds are right, measured on real combats.",
  "news.accuracy.link": "Simulator accuracy",
  "news.heroes": "Community hero stats. Your games only count if you turn on the switch on your account page; it is off until you do.",
  "news.heroes.link": "Hero stats",
  "news.simulator": "A combat simulator on the website: load a board from one of your games, change it and run it.",
  "news.simulator.link": "Simulator",
} as const;

export type SiteKey = keyof typeof SITE_STRINGS;

/** The string for a key in the page's language, with its placeholders filled. */
export const st = makeT<SiteKey>({ name: "site", en: SITE_STRINGS, es: ES });
