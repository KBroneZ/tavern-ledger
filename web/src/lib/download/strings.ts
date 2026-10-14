// The download page's words (T-558). Placeholders are {name}.
// Spanish is in strings.es.ts.

import { makeT } from "../i18n.ts";
import { ES } from "./strings.es.ts";

export const DL_STRINGS = {
  "page.kicker": "Windows · beta",
  "page.title": "Download Tavern Ledger",
  "page.lead":
    "A free Windows app that keeps every Battlegrounds game you play. Early beta, so expect rough edges.",

  "get.title": "The installer",
  "fact.version": "Version",
  "fact.version.value": "{version} (beta)",
  "fact.windows": "Works on",
  "fact.windows.value": "Windows 10 and 11, 64-bit",
  "fact.size": "Size",
  "fact.size.value": "{size} MB",
  "fact.date": "Published",
  "action.download": "Download",
  "action.download.file": "File: {file}",
  "action.memory": "By default the app reads two values from the game's memory, on your PC only; you can turn that off. It goes against the letter of Blizzard's rules and the account at risk is yours.",
  "action.memory.anchor": "9-the-game",
  "action.memory.link": "Read the terms (section 9)",
  "action.github": "All releases on GitHub",

  "beta.line":
    "Early build, not signed yet: Windows will warn you (step 2). Problems? Use the app's “Report a problem” button.",

  "steps.title": "Install it in four steps",
  "step1.title": "Download the installer",
  "step1.line": "Press Download and keep the file.",
  "step1.more":
    "Your browser may ask you to confirm that you want to keep a file it does not know; keep it.",
  "step2.title": "Get past the Windows warning",
  "step2.line": "Windows will warn you: click “More info”, then “Run anyway”.",
  "step2.more":
    "Windows may say “Windows protected your PC”. It shows this because the installer has no code-signing certificate yet and few people have downloaded it. Check that the file name matches, and do this only with the file from this page or from the project's GitHub releases.",
  "step3.title": "Install it",
  "step3.line": "Follow the installer; it needs no administrator rights.",
  "step3.more":
    "It installs for your Windows user only and adds a Start menu shortcut. If Microsoft Edge WebView2 is missing, the installer gets it from Microsoft (internet needed).",
  "step4.title": "Open it and play",
  "step4.line": "Open Tavern Ledger from the Start menu and play.",
  "step4.more":
    "It keeps your Battlegrounds games as you play them, and it works without an account.",
  "step4.account": "Want your games on the website too? Create an account on the website, then open Settings in the app, find “Upload to the website”, ask for a sign-in link and turn uploading on. That part is optional.",
  "step4.closed": "Accounts on the website are not open yet; the app does not need one.",

  "extras.title": "Good to know",
  "check.title": "Check the file",
  "check.short": "The installer is not signed, so you can check that it is the file we published.",
  "check.body": "In PowerShell, in the folder where you saved it, run the command below. The result should be the same as this SHA-256 (PowerShell writes it in capital letters; the case does not matter):",
  "check.command": "Get-FileHash \"{file}\"",
  "check.sums": "This release also lists the SHA-256 of each of its files, signed with the project's minisign key:",
  "check.sums.list": "the list (SHA256SUMS)",
  "check.sums.signature": "its signature",
  "reads.title": "What the app reads",
  "reads.short": "It reads the game's log files on your PC, and it never changes the game.",
  "reads.1": "The game's log files on your PC, to record your Battlegrounds games.",
  "reads.2": "For the overlay, where your pointer is over the game window.",
  "reads.3":
    "On by default, and you can turn it off in Settings: two things the game already shows you, the lobby's tribes and your own rating, read-only from the game's memory. They stay on your PC and are never uploaded. This goes against the letter of Blizzard's rules, and the account at risk is yours.",
  "reads.4": "It never changes the game, its files or its memory, and it never plays for you.",
  "reads.data":
    "Your games stay on your PC in %APPDATA%\\TavernLedger. Nothing is uploaded unless you sign in and turn uploading on.",
  "reads.privacy": "Privacy policy",
  "update.title": "Updating",
  "update.short":
    "From version 0.2.1 the app updates itself: it checks for a new version when it starts and every few hours, and asks you to restart when one is ready.",
  "update.body":
    "If you have version 0.1.0, install the new version from this page over it once: it replaces the app and keeps your games and settings. After that, updates arrive by themselves, signed by the project, and are installed only when you press “Restart now”. You can turn the checks off in the app's Settings, under Updates.",
  "remove.title": "Uninstalling",
  "remove.short": "Remove it in Windows Settings, under Apps. Your games are kept.",
  "remove.body":
    "Your games and settings in %APPDATA%\\TavernLedger stay, so a reinstall finds them. Delete that folder by hand if you want them gone.",

  "source.body": "Open source under AGPL-3.0. You can read the code, or build the same installer yourself, on",
  "source.link": "GitHub",
} as const;

export type DlKey = keyof typeof DL_STRINGS;

/** The string for a key in the page's language, with its placeholders filled. */
export const dl = makeT<DlKey>({ name: "download", en: DL_STRINGS, es: ES });
