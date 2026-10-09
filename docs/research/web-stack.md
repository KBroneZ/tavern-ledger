# Web stack and backend (T-103, P-003)

Date: 2026-10-09. Report to decide P-003. **Decided: A, Supabase (D-020).** Prices and limits come from the providers' own pages, read on 2026-10-09 unless a row says otherwise. Anything not confirmed on a primary source is marked **unverified**. No account, project or key was created on any service.

## Requirements

Taken from the plan (F1 to F4) and the decisions, not invented. "Must" is needed for T-104; "later" is F2 to F4 and only shapes the choice.

| # | Requirement | Source |
|---|-------------|--------|
| R1 | Accounts and login. Email (magic link or password) at least; Battle.net login only if its terms allow it (see [Battle.net login](#battlenet-login)). | T-104 |
| R2 | The desktop app uploads finished games, signed in as the user, without the user typing a password into the app each time. | T-104, D-014 |
| R3 | Public profile: a page per user with their stats and games, only if the user makes it public. Private by default. It shows heroes, places and the board cards seen, never other players' names (D-017). | T-104, rule 6 |
| R4 | Privacy policy, data export (one file with everything) and account deletion (all rows and files, backups expire) from the first day there is a server. | rule 6, T-104 |
| R5 | EU users: data stored in the EU, every processor named in the privacy policy, a DPA (data processing agreement) with each one. | rule 6 |
| R6 | Free by default, no ads; the free tiers must cover a hobby project with no income. Paid extras later (Polar or supporters), so the hosting must allow commercial use once that happens, or have a cheap step that does. | D-003, P-006 |
| R7 | The server never crawls the leaderboard and never stores other players' rows. MMR stays in the desktop app. | D-012 |
| R8 | The web can reuse the desktop UI (HTML and TypeScript) for the game viewer and recaps. | D-014, T-201, T-202 |
| R9 | Later: replays (T-201) and recaps (T-202) on the web, record against each opponent inside a game (T-203, by lobby slot and hero: no identity across games, D-017), opt-in community stats (T-401). | F2, F4 |
| R10 | One person runs it, part time, with Claude Code. Few moving parts, backups without effort, everything exportable to leave the provider. | CLAUDE.md |
| R11 | Every dependency with an OSI licence, recorded in `PROVENANCE.md`. | rule 1 |

### Sizes, measured on the real history

The history (`%APPDATA%\TavernLedger\games.jsonl`, 23 games from 5 sessions, read on 2026-10-09) holds one record per game with the full report: hero, lobby, health per round, opponents and the boards seen at each combat. That record is already enough for the F2 replay: the viewer (T-201) draws from it, so a replay adds no extra data.

| Measure | Value |
|---------|-------|
| One game, compact JSON | 8.8 KB min, 20.6 KB median, 28.8 KB max |
| One game, gzip | 1.5 KB min, 2.5 KB median, 3.3 KB max |
| Summary row (hero, place, mode, date, build) | under 200 bytes (estimate) |

Rough volume, as an assumption to size the plans (not a forecast; the sizes come from 23 games of one player and the rate is a guess): an active player plays about 5 games a day, so about 1,800 games and 36 MB of raw JSON a year, or about 5 MB gzipped. 100 active players: about 3.6 GB raw or 0.5 GB gzipped a year. Storing the full report as JSONB in Postgres is the largest cost driver; storing it gzipped in object storage and only a summary row in the database keeps the database tiny (100 players × 1,800 games × 200 bytes ≈ 36 MB a year).

## Battle.net login

Battle.net login is an OAuth client of the Blizzard Developer APIs, so it falls under the [Blizzard Developer API Terms of Use](https://www.blizzard.com/en-us/legal/a2989b50-5f16-43b1-abec-2ae17cc09dd6/blizzard-developer-api-terms-of-use) (last updated 2019-10-01, read 2026-10-09):

- 2(b): "'Premium' versions of Applications offering additional for-pay features are not permitted", and players cannot be charged to access an application that uses the APIs. That clashes with optional paid extras (D-003, P-006).
- 2(s): "You will retain data pulled from Blizzard Developer APIs for no longer than 30 days." The BattleTag returned at login is such data, so it could not stay on the account.
- 2(m): Blizzard must be named as the source of the data, and no Blizzard trademark in the title or URL.

Scopes and the `userinfo` fields (BattleTag as `battle_tag`) only appear in secondary sources (**unverified**): the official guides now redirect to `community.developer.battle.net` and the fetched pages did not list them.

**Conclusion:** no Battle.net login. Email login (magic link or password) plus, optionally, Discord or GitHub OAuth. The account does not need the BattleTag: the leaderboard lookup stays in the desktop app (D-012), and the public profile uses a name the user picks.

## Options

Four stacks, each complete (site, API, database, auth, file storage). The site in every option is static HTML and TypeScript built with Astro (MIT), so the desktop UI can be reused (R8); the options differ in what runs behind it.

### A. Astro static + Supabase (Postgres, auth, storage)

Supabase is open source (Apache-2.0) and self-hostable, so leaving means running the same stack elsewhere.

| Item | Detail | Source |
|------|--------|--------|
| Free plan | 500 MB database per project, 1 GB file storage (50 MB max upload), 5 GB egress + 5 GB cached, 50,000 MAU, 500,000 Edge Function calls, 2 active projects | [pricing](https://supabase.com/pricing) |
| When it runs out | Email notice and a grace period, then restrictions under the Fair Use Policy, usually on the whole organisation: the project is paused, the database turns read-only, or every API request gets HTTP 402; new projects are blocked. Which one applies is not stated | [billing FAQ](https://supabase.com/docs/guides/platform/billing-faq), [usage](https://supabase.com/docs/guides/platform/manage-your-usage/disk-size) |
| Inactivity | "Free projects are paused after 1 week of inactivity." Restoring is manual | [pricing](https://supabase.com/pricing) |
| Backups | None on Free: daily backups start on Pro (7 days); Supabase advises Free projects to export with `supabase db dump` themselves | [backups](https://supabase.com/docs/guides/platform/backups) |
| Commercial use on free | No Free-plan or non-commercial clause in the terms; use is limited to the customer's "internal business purposes" and Supabase may add caps or throttling (5(c)) | [terms](https://supabase.com/terms) |
| First paid step | Pro, from $25/month: 8 GB disk, 100 GB storage, 250 GB egress, 100,000 MAU; spend cap on by default | [pricing](https://supabase.com/pricing) |
| EU region | Frankfurt, Paris, Ireland, Stockholm (London and Zurich are not EU) | [regions](https://supabase.com/docs/guides/platform/regions) |
| DPA | Data Processing Addendum (v1, 2026-08-01), SCCs modules 2 and 3, public sub-processor list; no plan restriction found | [DPA](https://supabase.com/legal/dpa) |
| Export | `pg_dump` of the whole database (auth users included), storage through its S3-compatible API | [DPA](https://supabase.com/legal/dpa) |
| Auth | Email magic link and password, Discord, GitHub and others built in; PKCE flow. Redirect URLs accept globs (`*`, `**`), but whether a pattern can match any loopback port is **unverified** | [redirect URLs](https://supabase.com/docs/guides/auth/redirect-urls) |
| Site hosting | Static files on Cloudflare Pages free plan (500 builds a month, no bandwidth cap stated). Public profiles stay static: the page fetches the public data from Supabase in the browser, so no server rendering is needed | [Pages limits](https://developers.cloudflare.com/pages/platform/limits/) |
| Ops for one person | Low: managed Postgres and auth. Clients only read through row-level security; every write goes through one Edge Function that validates it. Backups are ours until Pro | — |
| Enters the repo | Astro (MIT) and `@supabase/supabase-js` (MIT) on the web; the desktop app calls the HTTP API from Rust | — |

### B. Next.js on Vercel + Neon (Postgres) + Cloudflare R2

| Item | Detail | Source |
|------|--------|--------|
| Vercel free (Hobby) | 100 GB transfer, 1M function calls, 4 CPU-hours, 1 GB Blob; over the cap, the feature stops until the 30 days pass; **"for personal, non-commercial use"** only | [Hobby](https://vercel.com/docs/plans/hobby), [pricing](https://vercel.com/pricing) |
| Vercel first paid | Pro, $20/month per seat with $20 of usage | [pricing](https://vercel.com/pricing) |
| Vercel DPA | Only for Pro and Enterprise | [DPA](https://vercel.com/legal/dpa) |
| Neon free | 1 GB Postgres per project (20 GB in total), 100 CU-hours, 5 GB egress, Auth up to 60,000 MAU; scales to zero after 5 min | [pricing](https://neon.com/pricing) |
| Neon when it runs out | Compute suspended until the next period; over storage, writes blocked; no data deleted | [pricing](https://neon.com/pricing) |
| Neon first paid | Launch, pay as you go: $0.106 per CU-hour, $0.35 per GB-month | [pricing](https://neon.com/pricing) |
| Neon EU region | Frankfurt only (London is not EU) | [regions](https://neon.com/docs/introduction/regions) |
| Neon DPA | Exists (Neon, LLC, part of the terms); the current text could not be confirmed (**unverified**) | [DPA](https://neon.com/dpa) |
| R2 free | 10 GB-month, 1M writes, 10M reads a month, egress free; then $0.015 per GB-month | [R2 pricing](https://developers.cloudflare.com/r2/pricing/) |
| Auth | Better Auth (ISC) inside Next.js; Auth.js (ISC) now points new projects to Better Auth | [Better Auth](https://github.com/better-auth/better-auth), [Auth.js](https://github.com/nextauthjs/next-auth) |
| Export | `pg_dump`, R2 through the S3 API | — |
| Ops for one person | Medium: three vendors, three bills and three DPAs | — |

The Hobby plan bans commercial use and has no DPA, so paid extras (P-006) would force Pro at $20/month from day one of charging, and GDPR (R5) asks for a DPA before any user signs in. That makes B the weakest fit.

### C. All on Cloudflare: Workers + D1 + R2 + Better Auth

| Item | Detail | Source |
|------|--------|--------|
| Workers free | 100,000 requests a day, 10 ms CPU per request; what happens over the limit is **unverified** (requests fail until the daily reset, per secondary sources) | [Workers pricing](https://developers.cloudflare.com/workers/platform/pricing/) |
| D1 free | 500 MB per database, 10 databases, 5 GB per account; 5M rows read and 100,000 rows written a day; over the limit, queries fail until 00:00 UTC; over storage, inserts blocked | [D1 pricing](https://developers.cloudflare.com/d1/platform/pricing/), [D1 limits](https://developers.cloudflare.com/d1/platform/limits/) |
| R2 free | As in B | [R2 pricing](https://developers.cloudflare.com/r2/pricing/) |
| Inactivity | No pause | — |
| First paid step | Workers Paid, $5/month per account: 10M requests, 30M CPU-ms, 25B D1 rows read, 50M rows written; D1 storage over 5 GB at $0.75 per GB-month | [Workers pricing](https://developers.cloudflare.com/workers/platform/pricing/), [D1 pricing](https://developers.cloudflare.com/d1/platform/pricing/) |
| EU region | D1 `jurisdiction: "eu"` and R2 `eu` jurisdiction, both set at creation and fixed afterwards; whether R2's jurisdiction is on the free tier is not stated (**unverified**). Workers themselves run worldwide | [D1 data location](https://developers.cloudflare.com/d1/configuration/data-location/), [R2 data location](https://developers.cloudflare.com/r2/reference/data-location/) |
| DPA | Customer DPA v6.4 (2026-04-03), SCCs modules 2 and 3, sub-processor list; no plan restriction named | [DPA](https://www.cloudflare.com/cloudflare-customer-dpa/) |
| Commercial use on free | No restriction found (**unverified**) | — |
| Export | `wrangler d1 export` to SQL (SQLite dialect), R2 through the S3 API | — |
| Auth | Better Auth (ISC) on D1; we write and review the login, sessions and token refresh ourselves | [Better Auth](https://github.com/better-auth/better-auth) |
| Ops for one person | Low on infrastructure, higher on code: auth is ours, and Workers bindings tie the API to Cloudflare | — |

### D. Rust API (axum) + Postgres on a small Hetzner VPS

| Item | Detail | Source |
|------|--------|--------|
| Price | CX23 (2 vCPU, 4 GB, 40 GB, 20 TB traffic in the EU): €5.49/month without VAT for new orders since 2026-06-15 (was €3.99), plus €0.50 for IPv4 (second source, **unverified**). The product page showed the plans as "currently unavailable" | [price adjustment](https://docs.hetzner.com/general/infrastructure-and-availability/price-adjustment/), [cost-optimized](https://www.hetzner.com/cloud/cost-optimized/) |
| Free plan | None | — |
| EU region | Nuremberg, Falkenstein, Helsinki | [Hetzner cloud](https://www.hetzner.com/cloud/) |
| DPA | Offered to Cloud customers through the account, no special plan (second source, **unverified**) | [data protection](https://docs.hetzner.com/general/company-and-policy/data-protection-at-hetzner/) |
| Export | Everything is ours: `pg_dump`, files on disk | — |
| Auth | Our own (OAuth server, sessions, email): the most security-sensitive code, written and run by us | — |
| Ops for one person | High: OS updates, TLS, firewall, backups off the box, monitoring, incident response | — |

Plus: the API could share the Rust report types with the desktop app. Minus: one person would be on call for a server holding user data.

Fly.io was also checked and dropped: no free plan for new organisations (trial of 2 hours or 7 days), card required, from $2.28/month in Amsterdam ([pricing](https://fly.io/pricing)).

Also checked by the second source (below) and dropped: Render (the free Postgres expires after 30 days), Railway (after the trial, $1 of credit a month), Netlify (choosing a Functions region needs Pro) and Turso (DPA from the Scaler plan up).

## Comparison

| | A. Supabase | B. Vercel + Neon + R2 | C. Cloudflare | D. Hetzner VPS |
|---|---|---|---|---|
| Cost at start | €0 | €0, but no commercial use | €0 | ~€6/month |
| First paid step | $25/month | $20/month (Vercel Pro) + Neon usage | $5/month | same VPS |
| Free-plan risk | Pauses after 1 week idle; no backups | Hard caps; no commercial use | Daily caps; 500 MB per D1 database | — |
| EU data | Frankfurt | Frankfurt (Neon), Vercel region **unverified** | EU jurisdiction | Germany, Finland |
| DPA on the plan we would use | Yes | No on Hobby | Yes | Yes (**unverified**) |
| Processors (with an email provider) | 3 (Supabase, Cloudflare Pages, email) | 4 | 2 | 2 |
| Auth written by us | No | Partly (library) | Partly (library) | Yes |
| Leaving the vendor | Easy: open source, Postgres | Medium | Hardest: D1 and Workers APIs | Easy |
| Ops load | Low | Medium | Low | High |

## Recommendation

**A: Astro static site + Supabase in Frankfurt.** Login, sessions, password reset and token refresh are the riskiest code for a one-person project, and Supabase ships them already reviewed; Postgres with row-level security covers export and deletion with plain SQL; it is open source, so a move to a VPS later keeps the same database and auth. Store each game gzipped in Supabase Storage and a summary row in Postgres with the fields the stats need (mode, hero, place, date, build, tribes offered); the profile and the stats read the rows, and only the viewer opens the file. 1 GB of storage holds about 400,000 games (about 220 player-years at the rate above), and the database stays far below 500 MB.

Conditions, so the free plan does not hurt users:

- **Pause.** A Free project pauses after a week without traffic, which also takes login and public profiles down. Fine while developing (manual restore); before the first public users, either real traffic keeps it awake or it moves to Pro ($25/month). No keep-alive pings to dodge the rule.
- **Backups.** Free has none. Until Pro, a weekly `supabase db dump` and a copy of the storage bucket, run from the developer's own machine and kept out of the repo (never as a CI artifact: the repo is public). Pro brings 7 days of daily backups.
- **Commercial use.** No clause bans it on Free (read 2026-10-09), unlike Vercel Hobby.

If the pause or the $25 step become a problem, move to plan B.

**Plan B: C, all on Cloudflare.** €0 with no pause, $5/month as the first step and fewer processors, at the price of writing auth with Better Auth and tying the API to Workers.

B is discarded (no commercial use and no DPA on Hobby), and D is kept only as the exit path: if Supabase stops fitting, self-host Supabase or plain Postgres on a VPS.

Email for magic links needs an email provider in A and C (another processor, with its own DPA); choosing it is part of T-104. Supabase's built-in email is only for testing (**unverified** limit).

## Second source

ChatGPT (Codex CLI with web search, 2026-10-09) answered the same questions independently. It agrees on the Supabase, Neon, Vercel and Cloudflare limits above, on Vercel's DPA covering only Pro and Enterprise, and on the Blizzard terms (no paid premium features, 30-day TTL). It added the facts now cited above (Supabase restrictions, D1's 500 MB per database, R2's EU jurisdiction, Hetzner's June 2026 price); each was re-read on the provider's page before going in, except where marked **unverified**. Its answer is not stored in the repo.

## Upload design

Design only; T-104 implements it.

**What is sent.** One game per request: the record the desktop app already keeps (session, index, report), gzipped (`Content-Encoding: gzip`), about 2.5 KB. The server reads at most 64 KB of body and inflates it as a stream that stops at 512 KB, so a gzip bomb costs nothing (the largest real game is 28.8 KB). It checks the JSON against a schema (`schema_version`, known `game_type`, places in range, string lengths capped, only hero and card ids where ids are expected, no BattleTag-shaped strings, and no rating, MMR or leaderboard field at all, D-012) and stores it as untrusted data. The report never carries player names (D-017, T-101); the server checks that again. What a profile shows is self-reported by the user's app, and the site says so; every string is escaped when rendered.

**Who writes.** Clients get no write policy on the games table or the storage bucket. The desktop app calls one Edge Function, which validates, then writes the file and the summary row with the service role. The row is written after the file; a daily sweep deletes files with no row.

**Endpoint.** `PUT /v1/games/{session}/{index}`, where `session` is the log folder name (`Hearthstone_YYYY_MM_DD_HH_MM_SS`, validated with a strict pattern) and `index` the game number inside it. The key in the database is `(user_id, session, index)`.

**Idempotency.** `PUT` to the same key replaces the game: sending it twice is harmless, and a game re-imported locally (the last record wins, D-015) replaces the old copy. The server hashes the exact uploaded JSON bytes after inflating (SHA-256; no cross-language canonical form needed). Same hash: `200` without writing. The record's `saved_at` is the revision: an upload older than the stored one gets `409`, so a second PC cannot overwrite a newer copy with an older one. Answers: `201` created, `200` same or replaced, `400` invalid, `401` missing or expired token, `403` not allowed, `409` older revision, `413` too big, `429` too many.

**Auth.** The user signs in in the system browser, not inside the app: OAuth 2.0 authorization code + PKCE with a loopback redirect on `127.0.0.1` only (RFC 8252), a random `state` checked on return, and a one-time code. Whether Supabase accepts any loopback port is **unverified**; if not, the app listens on one of a few fixed ports registered as redirect URLs. A magic link must be opened in the same browser that started the flow; Discord or GitHub login avoids that problem. The app keeps the refresh token in Windows Credential Manager (never in `games.jsonl` or logs) and saves each rotated token before using it, so a crash cannot lock the user out. Requests carry a short-lived access token (`Authorization: Bearer`). "Sign out" in the app revokes the session on the server. Upload is off until the user signs in and turns it on.

**Retries.** The app keeps an "uploaded" mark per game next to the history. After each game, and at start-up, it sends the games not yet marked, oldest first, one at a time. On a network error, timeout (15 s) or `5xx`: exponential backoff with jitter (2 s, 4 s, 8 s… up to 15 min), and the game stays in the queue across restarts. On `429`: wait for `Retry-After`. On `400`, `403`, `409` or `413`: mark the game as rejected and show it in the app, never retry it in a loop. On `401`: refresh the token once, then ask the user to sign in again. The app shows the queue state ("3 games waiting to upload"), so a failure is never silent.

**Limits on the server.** Per user: 600 uploads an hour (a year of games, about 1,800, uploads in three hours at most), 5,000 a day, and a total quota (10,000 games and 50 MB at first), so one account cannot fill the free storage. Per IP: a lower limit for requests without a token.

**Export and deletion.** `GET /v1/me/export` returns a ZIP with everything held about the user: the auth record (email, sign-in providers, dates), profile settings, consents, every summary row and every game file. `DELETE /v1/me` removes, in this order, the storage objects under the user's prefix, the summary rows and profile, and the auth user (service role). The email provider's logs and the backups expire on their own schedule; the privacy policy names both and their retention.
