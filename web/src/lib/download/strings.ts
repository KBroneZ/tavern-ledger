// The download page's words (T-558). Placeholders are {name}.
// Spanish is in strings.es.ts.

import { makeT } from "../i18n.ts";
import { ES } from "./strings.es.ts";

export const DL_STRINGS = {
  "page.kicker": "Windows · beta",
  "page.title": "Download Tavern Ledger",
  "page.lead":
    "A free Windows app that keeps every Battlegrounds game you play. This is an early build for beta testers: expect rough edges.",

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
  "action.github": "All releases on GitHub",

  "beta.title": "An early build, and not signed yet",
  "beta.body":
    "Windows will warn you when you open the installer (step 2). That is expected for now. Please report problems with the app's “Report a problem” button or on GitHub.",

  "steps.title": "Install it in four steps",
  "step1.title": "Download the installer",
  "step1.text": "Press the gold button. Your browser may ask you to confirm that you want to keep a file it does not know; keep it.",
  "step2.title": "Get past the Windows warning",
  "step2.text":
    "Windows may say “Windows protected your PC”. It shows this because the installer has no code-signing certificate yet and few people have downloaded it. Click More info, check that the file name matches, then click Run anyway. Do this only with the file from this page or from the project's GitHub releases.",
  "step3.title": "Install it",
  "step3.text":
    "Follow the installer. It installs for your Windows user only, so it needs no administrator rights, and adds a Start menu shortcut. If Microsoft Edge WebView2 is missing, the installer gets it from Microsoft (internet needed).",
  "step4.title": "Open it and play",
  "step4.text": "Open Tavern Ledger from the Start menu. It keeps your Battlegrounds games as you play them, and it works without an account.",
  "step4.account": "Want your games on the website too? Sign in there, then turn on uploading in the app's Settings. That part is optional.",
  "step4.closed": "Accounts on the website are not open yet; the app does not need one.",

  "check.title": "Check the file",
  "check.body":
    "The installer is not signed, so you can check that it is the file we published. In PowerShell, in the folder where you saved it, run the command below; the result should be the same as this SHA-256:",
  "check.command": "Get-FileHash \"{file}\"",
  "reads.title": "What the app reads",
  "reads.1": "The game's log files on your PC, to record your Battlegrounds games.",
  "reads.2": "For the overlay, where your pointer is over the game window.",
  "reads.3":
    "Only if you turn it on in Settings (it is off until you do): two things the game already shows you, the lobby's tribes and your own rating, read-only from the game's memory. This goes against the letter of Blizzard's rules, and the app explains the risk before you choose.",
  "reads.4": "It never changes the game, its files or its memory, and it never plays for you.",
  "reads.data":
    "Your games stay on your PC in %APPDATA%\\TavernLedger. Nothing is uploaded unless you sign in and turn uploading on.",
  "reads.privacy": "Privacy policy",
  "update.title": "Updating",
  "update.body":
    "There are no automatic updates yet. To update, download the new installer from this page and run it: it replaces the app and keeps your games and settings.",
  "remove.title": "Uninstalling",
  "remove.body":
    "Remove it in Windows Settings under Apps. Your games and settings in %APPDATA%\\TavernLedger are kept, so a reinstall finds them; delete that folder by hand if you want them gone.",

  "source.body": "Open source under AGPL-3.0. You can read the code, or build the same installer yourself, on",
  "source.link": "GitHub",
} as const;

export type DlKey = keyof typeof DL_STRINGS;

/** The string for a key in the page's language, with its placeholders filled. */
export const dl = makeT<DlKey>({ name: "download", en: DL_STRINGS, es: ES });
