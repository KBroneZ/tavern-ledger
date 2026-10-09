# Privacy policy (draft)

> **Draft, not in force.** The website is not open to the public yet: sign-ups are closed. Supabase and Cloudflare are in use; the email sender is being set up. Items marked `[TO BE DECIDED: …]` or `[TO BE CHECKED: …]` must be settled before the first public user. This draft was written by the project, not by a lawyer, and is not legal advice. Source of every statement: the [data inventory](https://github.com/KBroneZ/tavern-ledger/blob/main/docs/legal/data-inventory.md).

Last updated: [TO BE DECIDED: date it takes effect]

Tavern Ledger is a free, open-source Hearthstone Battlegrounds tracker: a Windows app that reads the game's log files on your PC, and a website where you can keep and, if you want, share your games. This policy says what personal data we handle, why, where it goes and what you can do about it.

Unofficial fan project. Not affiliated with or endorsed by Blizzard Entertainment.

## 1. Who is responsible

The controller is Andrew Rodrigo, Bulgaria, a private person who runs this project in their spare time.

Contact for anything about your data: privacy@tavernledger.net.

## 2. The desktop app keeps your data on your PC

The app reads the game's log files and saves a summary of each Battlegrounds game in `%APPDATA%\TavernLedger\games.jsonl`: the session's start time, mode, game build, your hero and your Duos teammate's hero, the lobby's heroes by player number, places and health, boards (as card ids), card names in your game's language, tribes offered in the tavern, and which parser version read the game. The logs contain the names (BattleTags) of the players in your lobby; the app does not save them.

Unless you turn on uploading, the app sends nothing over the internet and we never receive this file. To export it, copy it; to delete it, delete the `TavernLedger` folder.

**Uploading to your account (optional, off by default).** You can sign in from the app with a one-time link sent to your email and opened in your browser; the app never asks for your password. It keeps the sign-in (a refresh token) in Windows Credential Manager on your PC, and a list of the games it uploaded next to the history (`uploads.jsonl`). Only when you turn "Upload games" on does it send each finished game's record (the summary above, without player names) to your account (section 4). Signing out ends the session on our server, removes the sign-in from your PC and turns uploading off.

## 3. When you visit the website

When you load a page, our hosting provider (Cloudflare) and, for pages that talk to our server, our backend provider (Supabase) receive your IP address, browser user agent, the address requested and the time, as with any website. They use it to deliver the page and to protect the service, and keep it in their logs (section 6).

Legal basis: our legitimate interest in running a website that works and is protected from attacks (Art. 6(1)(f) GDPR).

There are no ads, no analytics and no tracking.

## 4. If you create an account

| Data | Why | Legal basis |
|------|-----|-------------|
| Email address, password (stored only as a hash), account id, sign-up, confirmation and last sign-in times | To create your account and sign you in | Contract (Art. 6(1)(b)): needed to provide the account you asked for |
| Account emails: your address, the message (a confirmation link) and its delivery status | To confirm your address and secure your account | Contract |
| Sign-in sessions: times, IP address and browser user agent; refresh and one-time tokens | To keep you signed in | Contract |
| Security log: sign-up, sign-in, sign-out, password and email changes, with time and IP address | To detect and stop abuse of accounts | Legitimate interest in keeping accounts secure (Art. 6(1)(f)) |
| Profile: a display name you choose (optional) and whether your profile is public (off by default) | To show your profile, if you make it public | Contract: publication only happens when you switch it on, and you can switch it off at any time |
| Your games: a summary row per game (date, mode, hero, place, build, tribes offered, parser version) and the game's full record from the app (no player names), if you upload them | To show your history and stats | Contract |
| Upload times: when each of your uploads happened, kept two days | To enforce the hourly and daily upload limits | Legitimate interest in keeping the service available for everyone (Art. 6(1)(f)) |
| Refused requests: a hash of the IP address that sent an upload with an invalid sign-in, and a count, kept one hour | To block abusive requests | Legitimate interest in protecting the service |
| Backups of all of the above, without live sign-in sessions | To recover from failures | Legitimate interest in not losing your data |

Your email and a password are required to have an account; without them we cannot create one. Everything else is optional: a display name, uploading games and a public profile.

We do **not** collect player names, BattleTags, Battle.net accounts, ratings or MMR. We do not take any automated decision about you that has legal or similarly significant effects.

**Public profile.** If you turn it on, anyone on the internet can see your display name and read every game you uploaded, in full, and can link to or copy them. Turning it off stops access through our site; copies others already made are out of our reach.

## 5. Who receives your data

**Our providers**, as processors acting on our instructions, each under a data processing agreement (DPA) that is part of its terms:

| Provider | What it does | Where | Transfers outside the EU |
|----------|--------------|-------|--------------------------|
| Supabase (contracting entity Supabase Pte. Ltd., Singapore) | Database, sign-in, file storage, server functions and their logs | Frankfurt, Germany (EU) | Data is stored in the EU, but Supabase and its sub-processors may access it from other countries; covered by the EU Standard Contractual Clauses in Supabase's DPA, which is part of its terms of service; Supabase also provides a transfer impact assessment. Sub-processors: list of 1 June 2026 at supabase.com/legal/subprocessors. |
| Cloudflare, Inc. (USA) | Hosts the website's pages; forwards mail sent to our contact addresses | Worldwide network | EU–US Data Privacy Framework, with Standard Contractual Clauses as a fallback, in the Cloudflare Customer DPA. [TO BE CHECKED: DPA version] |
| Brevo [TO BE CHECKED: legal entity and country] | Sends account emails from noreply@tavernledger.net (sign-up confirmation, sign-in links, email change and, once it exists, password reset) | [TO BE CHECKED: where Brevo processes and stores messages and logs] | [TO BE CHECKED: Brevo's DPA and transfer basis] |

You can ask us for a copy of the transfer safeguards at the contact address above.

**Everyone**, if you make your profile public (section 4).

**Authorities**, only when the law requires it.

We do not sell or rent your data.

## 6. How long we keep it

- **Account, profile, games, sessions and the security log:** until you delete your account (sessions also end when you sign out or they expire).
- **Backups:** made weekly on the controller's own computer, readable only by their user account, and deleted after 35 days. After you delete your account, your data can remain in backups for up to 35 days. If we ever restore a backup, we delete again the accounts deleted since it was made.
- **Upload times:** two days. **Refused-request counters:** one hour.
- **Provider logs** (page requests, server requests, emails sent): kept by Cloudflare, Supabase and the email sender for the periods their services set, then deleted. Supabase keeps its API and database logs for one day on our plan. They are not tied to your account in a way we can export or delete one by one. [TO BE CHECKED: Cloudflare Pages request logs; Email Sending's log retention, to be set to its shortest option.]

## 7. Your rights

You can, at any time:

- **Download your data:** "Download my data" on your account page gives one JSON file with your account details (not your password hash or sign-in tokens), sign-in sessions, security log, profile, game rows and game files. It is machine-readable, so you can take it elsewhere (data portability).
- **Ask for access** to anything else we hold about you, such as what may be in provider logs or backups, by writing to us.
- **Correct** your display name on the account page; for anything else, write to us.
- **Delete** your account: "Delete account" on your account page removes your files, game rows, profile, sessions, security log and the account itself. Backups and provider logs expire as in section 6.
- **Object** to processing based on legitimate interest, or ask us to **restrict** processing.
- **Stop publishing** your profile by turning the switch off.

Write to privacy@tavernledger.net for anything the site does not do by itself. We answer within one month.

You can also complain to a data protection authority, in particular in the EU country where you live or work or where you think the problem happened. The controller's authority is the Bulgarian Commission for Personal Data Protection (Комисия за защита на личните данни), www.cpdp.bg.

## 8. Cookies and browser storage

The website sets no cookies of its own and uses no tracking. When you sign in, the Supabase library stores your sign-in session in your browser's local storage (key `sb-<project>-auth-token`: access and refresh tokens and your account's basic details) and, during sign-in, a one-time code verifier (`…-code-verifier`). These are strictly necessary to keep you signed in and are removed when you sign out. Our hosting provider, Cloudflare, sets a cookie only if its protection against attacks shows you a challenge: `cf_clearance`, which records that you passed it so you are not asked again for a while. It is strictly necessary and not used to track you. Cloudflare's bot cookie `__cf_bm` is not used on this site (its bot features are off), and Cloudflare's Network Error Logging (browser reports of failed connections) is turned off. [TO BE CHECKED: the challenge passage time set on our zone, which is how long `cf_clearance` lasts.]

## 9. Children

You must be 16 or older to create an account. If you are younger, please do not sign up. If we learn that an account belongs to someone under 16, we delete it.

## 10. Security

Your password is stored only as a hash. The site only works over HTTPS. Each user can read only their own data and public profiles; game data can only be written by our server. The code is open source, so anyone can check how this works.

## 11. Changes

We will post changes here with a new date. If a change affects what we collect or why, we will tell signed-up users by email before it applies.
