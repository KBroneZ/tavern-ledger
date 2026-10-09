# Privacy policy (draft)

> **Draft, not in force.** The website is not open to the public yet, and none of the providers below has been signed up yet. Items marked `[TO BE DECIDED: …]` or `[TO BE CHECKED: …]` must be settled before the first public user. This draft was written by the project, not by a lawyer, and is not legal advice. Source of every statement: the [data inventory](https://github.com/KBroneZ/tavern-ledger/blob/main/docs/legal/data-inventory.md).

Last updated: [TO BE DECIDED: date it takes effect]

Tavern Ledger is a free, open-source Hearthstone Battlegrounds tracker: a Windows app that reads the game's log files on your PC, and a website where you can keep and, if you want, share your games. This policy says what personal data we handle, why, where it goes and what you can do about it.

Unofficial fan project. Not affiliated with or endorsed by Blizzard Entertainment.

## 1. Who is responsible

The controller is [TO BE DECIDED: name of the person who runs Tavern Ledger, and country], a private person who runs this project in their spare time.

Contact for anything about your data: [TO BE DECIDED: dedicated project email address].

## 2. The desktop app keeps your data on your PC

The app reads the game's log files and saves a summary of each Battlegrounds game in `%APPDATA%\TavernLedger\games.jsonl`: the session's start time, mode, game build, your hero and your Duos teammate's hero, the lobby's heroes by player number, places and health, boards (as card ids), card names in your game's language, tribes offered in the tavern, and which parser version read the game. The logs contain the names (BattleTags) of the players in your lobby; the app does not save them.

The app sends nothing over the internet. We never receive this file. To export it, copy it; to delete it, delete the `TavernLedger` folder.

Uploading games to your website account is a planned feature. It will be off until you sign in and turn it on, and this policy will be updated before it ships.

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
| Your games: a summary row per game (date, mode, hero, place, build, tribes offered) and the game's full record from the app (no player names) | To show your history and stats | Contract |
| Backups of all of the above, without live sign-in sessions | To recover from failures | Legitimate interest in not losing your data |

Your email and a password are required to have an account; without them we cannot create one. Everything else is optional: a display name, uploading games and a public profile.

We do **not** collect player names, BattleTags, Battle.net accounts, ratings or MMR. We do not take any automated decision about you that has legal or similarly significant effects.

**Public profile.** If you turn it on, anyone on the internet can see your display name and read every game you uploaded, in full, and can link to or copy them. Turning it off stops access through our site; copies others already made are out of our reach.

## 5. Who receives your data

**Our providers**, as processors acting on our instructions, each under a data processing agreement (DPA) that we will sign before the site opens:

| Provider | What it does | Where | Transfers outside the EU |
|----------|--------------|-------|--------------------------|
| Supabase (contracting entity Supabase Pte. Ltd., Singapore) | Database, sign-in, file storage, server functions and their logs | Frankfurt, Germany (EU) | Data is stored in the EU, but Supabase and its sub-processors may access it from other countries; covered by the EU Standard Contractual Clauses in Supabase's DPA. [TO BE CHECKED at sign-up: DPA version and sub-processor list, supabase.com/legal/customer-resources/subprocessor-list] |
| Cloudflare, Inc. (USA) | Hosts the website's pages | Worldwide network | EU–US Data Privacy Framework, with Standard Contractual Clauses as a fallback, in the Cloudflare Customer DPA. [TO BE CHECKED at sign-up: DPA version] |
| [TO BE DECIDED: outbound email sender] | Sends account emails (sign-up confirmation, email change and, once it exists, password reset) | [TO BE CHECKED] | [TO BE CHECKED] |

You can ask us for a copy of the transfer safeguards at the contact address above.

**Everyone**, if you make your profile public (section 4).

**Authorities**, only when the law requires it.

We do not sell or rent your data.

## 6. How long we keep it

- **Account, profile, games, sessions and the security log:** until you delete your account (sessions also end when you sign out or they expire).
- **Backups:** made weekly on the controller's own computer, readable only by their user account, and deleted after 35 days. After you delete your account, your data can remain in backups for up to 35 days. If we ever restore a backup, we delete again the accounts deleted since it was made.
- **Provider logs** (page requests, server requests, emails sent): kept by Cloudflare, Supabase and the email sender for the periods their services set, then deleted. They are not tied to your account in a way we can export or delete one by one. [TO BE CHECKED: Supabase log retention on our plan; Cloudflare Pages request logs; the email sender's log retention, to be set to its shortest option.]

## 7. Your rights

You can, at any time:

- **Download your data:** "Download my data" on your account page gives one JSON file with your account details (not your password hash or sign-in tokens), sign-in sessions, security log, profile, game rows and game files. It is machine-readable, so you can take it elsewhere (data portability).
- **Ask for access** to anything else we hold about you, such as what may be in provider logs or backups, by writing to us.
- **Correct** your display name on the account page; for anything else, write to us.
- **Delete** your account: "Delete account" on your account page removes your files, game rows, profile, sessions, security log and the account itself. Backups and provider logs expire as in section 6.
- **Object** to processing based on legitimate interest, or ask us to **restrict** processing.
- **Stop publishing** your profile by turning the switch off.

Write to [TO BE DECIDED: contact email] for anything the site does not do by itself. We answer within one month.

You can also complain to a data protection authority, in particular in the EU country where you live or work or where you think the problem happened. [TO BE DECIDED: if the controller is in Spain, the Agencia Española de Protección de Datos (AEPD), www.aepd.es.]

## 8. Cookies and browser storage

The website sets no cookies of its own and uses no tracking. When you sign in, the Supabase library stores your sign-in session in your browser's local storage (key `sb-<project>-auth-token`: access and refresh tokens and your account's basic details) and, during sign-in, a one-time code verifier (`…-code-verifier`). These are strictly necessary to keep you signed in and are removed when you sign out. [TO BE CHECKED: whether Cloudflare sets any security cookie on the site.]

## 9. Children

You must be 16 or older to create an account. If you are younger, please do not sign up. If we learn that an account belongs to someone under 16, we delete it.

## 10. Security

Your password is stored only as a hash. The site only works over HTTPS. Each user can read only their own data and public profiles; game data can only be written by our server. The code is open source, so anyone can check how this works.

## 11. Changes

We will post changes here with a new date. If a change affects what we collect or why, we will tell signed-up users by email before it applies.
