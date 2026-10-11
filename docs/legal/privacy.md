# Privacy policy

Last updated: 12 October 2026

**What changed on 12 October 2026:** with your profile public, you now appear on a leaderboard from your first finished game in that mode and period, not after 20 (section 4, "Public profiles and leaderboards"); a number from few games is marked "few games".

**What changed on 11 October 2026:** public leaderboards (section 4, "Public profiles and leaderboards"): what they show, who appears on them, how to leave them and how long the numbers are kept; the check of display names against a short word list; the leaderboard numbers in the download (section 7). Also new that day: card and hero pictures loaded from HearthstoneJSON (sections 3 and 5); community hero stats (section 4, "Community hero stats"), which count your games only if you turn them on, with the switch in the download (section 7) and its retention (section 6).

Tavern Ledger is a free, open-source Hearthstone Battlegrounds tracker: a Windows app that reads the game's log files on your PC, and a website where you can keep and, if you want, share your games. This policy says what personal data we handle, why, where it goes and what you can do about it.

Unofficial fan project. Not affiliated with or endorsed by Blizzard Entertainment.

## 1. Who is responsible

The controller is Andrew Rodrigo, Bulgaria, a private person who runs this project in their spare time.

Contact for anything about your data: privacy@tavernledger.net.

## 2. The desktop app keeps your data on your PC

The app reads the game's log files and saves a summary of each Battlegrounds game in `%APPDATA%\TavernLedger\games.jsonl`: the session's start time, mode, game build, your hero and your Duos teammate's hero, the lobby's heroes by player number, places and health, boards (as card ids), card names in your game's language, tribes offered in the tavern, your own shop each turn (the cards offered, bought and sold, rolls, freezes, gold and tavern tier) and the actions the game recorded you making, each with the time since the game started (to work out your actions per minute; together with the game's start time, this shows roughly when each action happened), and which parser version read the game. The logs contain the names (BattleTags) of the players in your lobby; the app does not save them.

**Reading from the game (optional, off by default).** If you turn on "Read the lobby's tribes and your rating from the game", the app also reads, read-only, two values the game shows you while it runs: the five tribes of the lobby you are in and your own Battlegrounds rating (Solo and Duos). It reads nothing else from the game: no other players' names, BattleTags, account ids or ratings. These values stay in the app's memory on your PC: they are not saved with your games, not put in the game record and never uploaded. Turning the switch off forgets them.

Unless you turn on uploading, the app sends nothing over the internet and we never receive this file. To export it, copy it; to delete it, delete the `TavernLedger` folder.

**Uploading to your account (optional, off by default).** You can sign in from the app with a one-time link sent to your email and opened in your browser; the app never asks for your password. It keeps the sign-in (a refresh token) in Windows Credential Manager on your PC, and a list of the games it uploaded next to the history (`uploads.jsonl`). Only when you turn "Upload games" on does it send each finished game's record (the summary above, without player names) to your account (section 4). Signing out ends the session on our server, removes the sign-in from your PC and turns uploading off.

## 3. When you visit the website

When you load a page, our hosting provider (Cloudflare) and, for pages that talk to our server, our backend provider (Supabase) receive your IP address, browser user agent, the address requested and the time, as with any website. They use it to deliver the page and to protect the service, and keep it in their logs (section 6).

Legal basis: our legitimate interest in running a website that works and is protected from attacks (Art. 6(1)(f) GDPR).

**Card pictures.** Pages that show games (a game, your games, a public profile, the games list and the leaderboards) load card and hero pictures straight from HearthstoneJSON's image server (`art.hearthstonejson.com`), an independent service run by HearthSim, not a provider of ours. Your browser sends it your IP address, browser user agent and the address of each picture, which names only the card: our pages send no referrer, so it does not learn which page, account or game you are looking at. HearthSim decides what it logs; see hearthstonejson.com. Legal basis: our legitimate interest in showing games in a readable way (Art. 6(1)(f) GDPR). To avoid it, block that address in your browser: the pages then show a plain placeholder instead.

There are no ads, no analytics and no tracking.

## 4. If you create an account

| Data | Why | Legal basis |
|------|-----|-------------|
| Email address, password (stored only as a hash), account id, sign-up, confirmation and last sign-in times | To create your account and sign you in | Contract (Art. 6(1)(b)): needed to provide the account you asked for |
| Account emails: your address, the message (a confirmation, sign-in or password reset link) and its delivery status | To confirm your address and secure your account | Contract |
| Sign-in sessions: times, IP address and browser user agent; refresh and one-time tokens | To keep you signed in | Contract |
| Security log: sign-up, sign-in, sign-out, password reset requests, password and email changes, with time and IP address, kept in our database for 90 days | To detect and stop abuse of accounts | Legitimate interest in keeping accounts secure (Art. 6(1)(f)) |
| Profile: a display name you choose (optional) and whether your profile is public (off by default) | To show your profile and put you on the public leaderboards, if you make it public | Contract: publication only happens when you switch it on, and you can switch it off at any time |
| Community stats choice: whether you turned on "Include my games in community hero stats" (off by default), when you last changed it and the policy version then in force | To count your games in the community hero stats only if you asked | Consent (Art. 6(1)(a)): you can withdraw it at any time by turning the switch off |
| Community hero stats: from the games of accounts that turned the switch on, totals per hero, mode and period, rebuilt daily, with no account id | To show the community hero stats | Consent, as above |
| Leaderboard numbers: for a public profile with a display name, the numbers each board ranks (worked out from your uploaded games) per mode and period, with how many games back each | To show the public leaderboards | Contract, as for the public profile |
| Your games: a summary row per game (date, mode, hero, place, build, tribes offered, parser version) and the game's full record from the app (no player names), if you upload them | To show your history and stats | Contract |
| Upload times: when each of your uploads happened, kept two days | To enforce the hourly and daily upload limits | Legitimate interest in keeping the service available for everyone (Art. 6(1)(f)) |
| Refused requests: a salted hash of the IP address that sent an upload with an invalid sign-in (the address itself is not stored, and the secret salt is replaced every day), and a count, kept one hour at most | To block abusive requests | Legitimate interest in protecting the service |
| Backups of all of the above, without live sign-in sessions | To recover from failures | Legitimate interest in not losing your data |

Your email and a password are required to have an account; without them we cannot create one. Everything else is optional: a display name, uploading games, a public profile and the community hero stats.

We do **not** collect player names, BattleTags, Battle.net accounts, ratings or MMR. We do not take any automated decision about you that has legal or similarly significant effects.

### Public profiles and leaderboards

**Public profile.** If you turn it on, anyone on the internet can see your display name and read every game you uploaded, in full, and can link to or copy them. Turning it off stops access through our site; copies others already made are out of our reach.

**Leaderboards** (`/leaderboards/`). While your profile is public and has a display name, you can also appear on the public leaderboards, which rank players on their uploaded games, per mode (Solo, Duos) and period (the last 7 days, the last 30 days, all time). A board shows your display name with a link to your public profile, your place on the board, the number it ranks (for example your average place, top 4 or top 2 rate, win rate, games played, longest top-half streak, average place with a hero and that hero, actions per minute, gold beyond income, rolls or triples per game, your biggest board or most golden minions) and how many games it is based on. You appear on a board once you have one finished game in that mode and period that has the number (one with the hero for the per-hero board); a number from fewer than 20 games (10 with the hero) is marked "few games". The numbers are self-reported: they come from the games your app uploaded; our server checks each game's record and leaves out games with values no real game can have, but cannot prove a game was played as uploaded. The leaderboards never show your email, BattleTag, rating or other players' names. The page asks search engines not to index it.

**Leaving the leaderboards.** Turn "Public profile" off on your account page, or remove your display name: from the moment you save, our server stops showing you on every board, and the stored copy of your leaderboard numbers is deleted at the same time (or, if a rebuild is running just then, by that rebuild or the next one). A leaderboard page someone already has open keeps what it loaded until they reload it. If you make your profile public again, you come back at the next rebuild; rebuilds are scheduled every 10 minutes.

**How long the numbers are kept.** The boards are rebuilt from the current games every 10 minutes; each rebuild replaces the last one, so no history of past boards is kept. Your numbers go when you make your profile private, remove your display name or delete your account, and from backups as in section 6.

**Display names.** When you set a display name, our server checks it against a short list of offensive words and names that would look official (such as "admin" or "Blizzard") and refuses it if it matches; the account page then asks you for another one. A name set before a word was added to the list is hidden from the leaderboards until you change it. The check only compares words; it decides nothing else about you.

### Community hero stats

**What it is.** The hero stats page (`/heroes/`) shows, per mode (Solo, Duos) and period (the last 30 days, all time), how each hero does: roughly how many games and players are behind it, how often it is picked when offered, its average place, top 4 (Solo) or top 2 (Duos) rate and win rate.

**Whose games.** Only the games of accounts that turned on "Include my games in community hero stats" on their account page. The switch is off for every account, including accounts made before this page existed, and no update turns it on. It is separate from your public profile: a public profile is not counted unless you turn the switch on, and a private profile can be. We keep whether the switch is on, when you last changed it and which version of this policy was in force then.

**What is used.** From each of your finished uploaded games that our server has summarized, up to the day before each rebuild: the mode, the day it was played, your hero, your final place and, if the app recorded your hero choice, the heroes you were offered. Games with values no real game can have are left out, as on the leaderboards. To count distinct players and apply the rules below, the rebuild uses your account id internally; it is not stored with the totals.

**What is shown.** Only totals per hero, and only for a hero with at least 50 games from at least 5 players, where no one player has more than half of the games and no two players more than three quarters. The pick rate follows the same rule over the games that offered the hero. All time shows a hero only if its games from before the last 30 days pass the same rule on their own (or there are none), so the two periods cannot be subtracted to single out a few games. Counts are shown in tens of games and fives of players ("50+", "5+"), average place to one decimal and rates in whole percent. No display name, account, BattleTag, rating or single game is shown, and the stored totals keep no account id. The page asks search engines not to index it.

**What remains possible.** The totals are rebuilt once a day from whole days of games, so each change mixes everyone's games of that day. Still, with few players, someone who saves the page every day and already knows something about you (for example that you have just started or stopped sharing, or the games on your public profile) could make a rough guess about your results or about whether you share; the rounding keeps it to a guess, not an exact result. Anyone can save what the page shows; we keep no history of past totals ourselves.

**Stopping.** Turn the switch off on your account page. Your games are left out from the next daily rebuild. If a rebuild ever fails, the page shows no numbers once the last one is more than 26 hours old, so your games never stay in what is shown for longer than that. Deleting your account works the same way. A page someone already has open keeps what it loaded until they reload it. Withdrawing your consent does not change what was shown before.

## 5. Who receives your data

**Our providers**, as processors acting on our instructions, each under a data processing agreement (DPA) that is part of its terms:

| Provider | What it does | Where | Transfers outside the EU |
|----------|--------------|-------|--------------------------|
| Supabase (contracting entity Supabase Pte. Ltd., Singapore) | Database, sign-in, file storage, server functions and their logs | Frankfurt, Germany (EU) | Data is stored in the EU, but Supabase and its sub-processors may access it from other countries; covered by the EU Standard Contractual Clauses in Supabase's DPA, which is part of its terms of service; Supabase also provides a transfer impact assessment. Sub-processors: list of 1 June 2026 at supabase.com/legal/subprocessors. |
| Cloudflare, Inc. (USA) | Hosts the website's pages; forwards mail sent to our contact addresses | Worldwide network | EU–US Data Privacy Framework, with Standard Contractual Clauses as a fallback, in the Cloudflare Customer DPA (version 6.4 of 3 April 2026) |
| Lettermint B.V., the Netherlands | Sends account emails from noreply@tavernledger.net (sign-up confirmation, sign-in links, email change and password reset). Open and click tracking is switched off on our account, so the links reach you unchanged | European Union | None for email data: Lettermint's data processing agreement (part of its terms) keeps it in the EU and allows no transfer outside the EEA without our approval. Two of its sub-processors, Slack (staff messaging) and Stripe (billing), are outside the EU and do not receive the emails |

You can ask us for a copy of the transfer safeguards at the contact address above.

**HearthSim**, the independent service behind HearthstoneJSON, receives the requests for card pictures described in section 3.

**Everyone**, if you make your profile public: your public profile and, with a display name and enough games, the leaderboards (section 4). If you turn on community hero stats: totals per hero that include your games without naming you (section 4).

**Authorities**, only when the law requires it.

We do not sell or rent your data.

## 6. How long we keep it

- **Account, profile, games and sessions:** until you delete your account (sessions also end when you sign out or they expire).
- **Leaderboard numbers:** rebuilt every 10 minutes while your profile is public; deleted when you make it private or delete your account.
- **Community stats choice:** until you delete your account. **Community hero stats:** rebuilt once a day; your games leave them at the first rebuild after you turn the switch off or delete your account, and the page shows nothing once the last rebuild is over 26 hours old.
- **Security log:** 90 days, then deleted every day by our clean-up job; all of it goes earlier if you delete your account.
- **Backups:** made weekly on the controller's own computer, readable only by their user account, and deleted after 35 days. After you delete your account, your data can remain in backups for up to 35 days. If we ever restore a backup, we delete again the accounts deleted since it was made.
- **Upload times:** two days. **Refused-request counters:** one hour.
- **Provider logs** (page requests, server requests, emails sent): kept by Cloudflare, Supabase and the email sender for the periods their services set, then deleted. They are not tied to your account in a way we can export or delete one by one. Supabase keeps its API, database and sign-in logs for one day on our plan. Lettermint keeps each email it sent, with its delivery record, for 28 days and then deletes it. Cloudflare does not give us the website's request logs on our plan and does not publish a fixed period for them; under its privacy policy it keeps them only as long as it needs to run and protect its network. For mail you send to our contact addresses, Cloudflare keeps a delivery record (sender, recipient, subject, time and result) for 31 days; the message itself is passed on to our mailbox.

## 7. Your rights

You can, at any time:

- **Download your data:** "Download my data" on your account page gives one JSON file with your account details (not your password hash or sign-in tokens), sign-in sessions, security log, profile, your community stats choice, game rows, your leaderboard numbers and game files. It is machine-readable, so you can take it elsewhere (data portability).
- **Ask for access** to anything else we hold about you, such as what may be in provider logs or backups, by writing to us.
- **Correct** your display name on the account page; for anything else, write to us.
- **Delete** your account: "Delete account" on your account page removes your files, game rows, leaderboard numbers, community stats choice, profile, sessions, security log and the account itself. Backups and provider logs expire as in section 6.
- **Object** to processing based on legitimate interest, or ask us to **restrict** processing.
- **Stop publishing** your profile and leave the leaderboards by turning the switch off.
- **Withdraw your consent** to the community hero stats by turning "Include my games in community hero stats" off; your games leave the totals at the next daily rebuild.

Write to privacy@tavernledger.net for anything the site does not do by itself. We answer within one month.

You can also complain to a data protection authority, in particular in the EU country where you live or work or where you think the problem happened. The controller's authority is the Bulgarian Commission for Personal Data Protection (Комисия за защита на личните данни), www.cpdp.bg.

## 8. Cookies and browser storage

The website sets no cookies of its own and uses no tracking. When you sign in, the Supabase library stores your sign-in session in your browser's local storage (key `sb-<project>-auth-token`: access and refresh tokens and your account's basic details) and, while you sign up or reset your password, a one-time code verifier (`…-code-verifier`) that proves the email link came back to the browser that asked for it. These are strictly necessary: the session is removed when you sign out, and a code verifier when its link is used or a newer one replaces it. Our hosting provider, Cloudflare, sets a cookie only if its protection against attacks shows you a challenge: `cf_clearance`, which records that you passed it so you are not asked again for a while. It lasts 30 minutes (our zone's setting), is strictly necessary and is not used to track you. Cloudflare's bot cookie `__cf_bm` is not used on this site (its bot features are off), and Cloudflare's Network Error Logging (browser reports of failed connections) is turned off.

## 9. Children

You must be 16 or older to create an account. If you are younger, please do not sign up. If we learn that an account belongs to someone under 16, we delete it.

## 10. Security

Your password is stored only as a hash. The site only works over HTTPS. Each user can read only their own data and public profiles; game data can only be written by our server. The code is open source, so anyone can check how this works.

## 11. Changes

We will post changes here with a new date. If a change affects what we collect or why, we will tell signed-up users by email before it applies.
