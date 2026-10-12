// Every string of the account pages (sign-in, account, forgot password) and
// their messages (T-508): the forms, the page scripts and the rules in
// lib/auth.ts, lib/authLink.ts, lib/profile.ts and lib/deleteAccount.ts.
// Placeholders are {name}. Spanish is in strings.es.ts.

import { makeT } from "../i18n.ts";
import { ES } from "./strings.es.ts";

export const ACCOUNT_STRINGS = {
  // Sign in (components/SignIn.astro, scripts/signin.ts).
  "signin.title": "Sign in",
  "signin.meta.before": "Email and password. See",
  "signin.meta.link": "how your data is kept",
  "signin.meta.after": ".",
  "signin.have": "I have an account",
  "signin.email": "Email",
  "signin.password": "Password",
  "signin.submit": "Sign in",
  "signin.forgot": "Forgot password?",
  "signin.create": "Create an account",
  "signin.hint.length": "At least {min} characters.",
  "signin.consent.before": "By creating an account you accept the",
  "signin.consent.terms": "terms",
  "signin.consent.middle": "and confirm you are 16 or older. The",
  "signin.consent.privacy": "privacy policy",
  "signin.consent.after": "says what we do with your data.",
  "signin.createSubmit": "Create account",
  "signin.closed": "Sign-up is not open yet.",
  "signin.signingIn": "Signing in…",
  "signin.creating": "Creating the account…",

  // Forgot password (components/ForgotPassword.astro, scripts/forgot.ts).
  "forgot.title": "Forgot password",
  "forgot.meta": "Enter your account's email. We'll send you a link to choose a new password; open it in this browser.",
  "forgot.heading": "Get a reset link",
  "forgot.submit": "Send the link",
  "forgot.back": "Back to sign in",
  "forgot.sending": "Sending…",

  // Account (components/Account.astro, scripts/account.ts).
  "account.title": "Your account",
  "account.loading": "Loading…",
  "account.signedOut.before": "You are not signed in.",
  "account.signedOut.link": "Sign in",
  "account.signedOut.after": "to see your account.",
  "account.setPassword": "Set a new password",
  "account.newPassword": "New password",
  "account.repeat": "Repeat the new password",
  "account.savePassword": "Save the new password",
  "account.deleted": "Your account and all its data are deleted. Thank you for trying Tavern Ledger.",
  "account.signedInAs.before": "Signed in as",
  "account.profile": "Profile",
  "account.displayName": "Display name",
  "account.displayName.hint": "Up to 32 letters, digits, spaces, _ . or -. Not your BattleTag.",
  "account.displayName.more": "Offensive words and names that look official are refused.",
  "account.public": "Public profile",
  "account.public.hint":
    "Off by default. When on, anyone can find your profile and read your display name and every game you uploaded in full, and you can appear on the public leaderboards. Turning it off hides you everywhere as soon as you save.",
  "account.public.more.before":
    "That is, for each game: hero, place, mode, date, tribes offered and the boards you saw. With a display name and a finished game, you also appear on the",
  "account.public.more.link": "public leaderboards",
  "account.public.more.after": ". Never your rating or other players' names.",
  "account.save": "Save",
  "account.publicPage": "Your public page:",
  "account.data": "Your data",
  "account.data.body": "Download everything this site holds about you as one JSON file.",
  "account.data.more": "It includes your account, sign-in history, profile and settings, every game and the game files themselves.",
  "account.download": "Download my data",
  "account.community": "Community stats",
  "account.community.label": "Include my games in community hero stats",
  "account.community.hint.before": "Off by default, and separate from your public profile. When on, your finished games count toward the",
  "account.community.hint.link": "hero stats",
  "account.community.hint.after": ", never with your name; turning it off takes them out at the next daily update.",
  "account.community.more": "Your games are shown only as rounded totals over at least 5 players and never with your name.",
  "account.community.how": "How this works",
  "account.simulator": "Improve the simulator",
  "account.simulator.label": "Let my games be used to improve the simulator",
  "account.simulator.hint":
    "Off by default. When on, the boards and combat replays of your uploaded games (while their files are kept) can be used to measure how often the combat simulator is right and to fix its rules, only as totals over many games and never with your name. You can turn it off at any time.",
  "account.simulator.more":
    "Results are only ever totals over many games: never published per player, never with your name. If you turn it off, from then on your games are not used again, and totals already worked out from them cannot be traced back to you.",
  "account.simulator.how": "How this works",
  "account.delete": "Delete account",
  "account.delete.body":
    "This deletes your account, your profile, every uploaded game and file, and your sign-in history. It cannot be undone. Download your data first if you want to keep it.",
  "account.delete.type.before": "Type",
  "account.delete.type.after": "to confirm",
  "account.delete.phrase": "delete my account",
  "account.reauth": "For your safety, enter your password again",
  "account.reauth.hint": "To delete your account, you need to have signed in within the last 10 minutes.",
  "account.delete.submit": "Delete my account",
  "account.signOut": "Sign out",
  "account.profile.loadFailed": "Could not load your profile. Reload the page to try again.",
  "account.saving": "Saving…",
  "account.saved": "Saved.",
  "account.setting.loadFailed": "Could not load this setting. Reload the page to try again.",
  "account.community.savedOn": "Saved. Your games count from the next daily update.",
  "account.community.savedOff": "Saved. Your games are left out from the next daily update.",
  "account.community.unchanged": "{error} Your setting is unchanged.",
  "account.simulator.savedOn": "Saved. Your games can now be used to improve the simulator.",
  "account.simulator.savedOff": "Saved. Your games are no longer used for the simulator.",
  "account.simulator.unchanged": "{error} Your setting is unchanged.",
  "account.export.preparing": "Preparing your file…",
  "account.export.done": "Downloaded.",
  "account.export.failed": "The export failed. Try again in a moment.",
  "account.delete.typeExactly": "Type the phrase exactly to confirm.",
  "account.delete.checking": "Checking your password…",
  "account.delete.deleting": "Deleting…",
  "account.delete.needPassword": "Enter your password, then press Delete my account again.",
  "account.password.saved": "Your new password is saved.",
  "account.loadFailed": "Could not load your account. Reload the page to try again.",

  // Rules and server answers (lib/auth.ts).
  "auth.checkEmail": "Check your inbox: if this email can be used, a confirmation link is on its way. Open it in this browser to finish.",
  "auth.wrong": "Wrong email or password.",
  "auth.rateLimited": "Too many attempts. Wait a few minutes and try again.",
  "auth.unreachable": "Could not reach the server. Check your connection and try again.",
  "auth.generic": "Something went wrong. Try again in a moment.",
  "auth.email": "Enter a valid email address.",
  "auth.passwordLength": "The password needs at least {min} characters.",
  "auth.confirmFirst": "Confirm your email first: open the link we sent you.",
  "auth.stronger": "Choose a stronger password (at least {min} characters).",
  "auth.signupClosed": "Sign-up is not open yet.",
  "auth.resetSent": "Check your inbox: if this email has an account, a link to choose a new password is on its way. Open it in this browser.",
  "auth.notSame": "The two passwords are not the same.",
  "auth.samePassword": "Choose a password different from your current one.",
  "auth.resetExpired": "This reset has expired. Ask for a new link with “Forgot password?” on the sign-in page.",

  // Links from emails (lib/authLink.ts).
  "link.useForgot": "To choose a password, use “Forgot password?” on the sign-in page.",
  "link.confirmed": "Your email is confirmed.",
  "link.confirmedSignIn": "Your email is confirmed. Sign in to continue.",
  "link.magic": "Sign-in links are for the Tavern Ledger app. Here, sign in with your password, or use “Forgot password?” on the sign-in page to set one.",
  "link.recovery": "This reset link cannot be used here. Ask for a new one with “Forgot password?” on the sign-in page.",
  "link.newEmail": "Your new email address is confirmed.",
  "link.newEmailSignIn": "Your new email address is confirmed. Sign in to continue.",
  "link.notice": "Link accepted. Now open the link we sent to your other email address to finish the change.",
  "link.expired":
    "This link has expired or was already used (each new email replaces the previous link). If you were confirming your email, try signing in: it may already be confirmed. Otherwise ask for a new link.",
  "link.otherBrowser":
    "This link was opened in a different browser from the one where you asked for it. If you were confirming your email, it is confirmed: sign in. If you were resetting your password, open the link in the browser where you asked for it, or ask for a new one.",
  "link.unreachable": "Could not reach the server. Check your connection and reload the page.",
  "link.failed": "This link did not work. Sign in, or ask for a new link.",

  // Display name and saves (lib/profile.ts).
  "profile.maxLength": "Up to {max} characters.",
  "profile.chars": "Use letters, digits, spaces, _ . or - only.",
  "profile.notAllowed": "This name is not allowed. Choose another one: names show on public pages and leaderboards.",
  "profile.charsRange": "Use 1 to 32 letters, digits, spaces, _ . or - only.",
  "profile.sessionEnded": "Your session ended. Sign in again.",
  "profile.saveFailed": "Could not save. Try again in a moment.",

  // Account deletion (lib/deleteAccount.ts).
  "delete.unexpected": "The server gave an unexpected answer. Try again.",
  "delete.unreachable": "Could not reach the server. Nothing was deleted yet; try again.",
  "delete.unfinished": "Deletion did not finish. Try again: it picks up where it stopped.",
  "delete.unfinishedStep": "Deletion did not finish (step: {step}). Try again: it picks up where it stopped.",
  "delete.refused": "The server refused the request. Reload the page and try again.",
} as const;

export type AccountKey = keyof typeof ACCOUNT_STRINGS;

/** The string for a key in the page's language, with its placeholders filled. */
export const at = makeT<AccountKey>({ name: "account", en: ACCOUNT_STRINGS, es: ES });
