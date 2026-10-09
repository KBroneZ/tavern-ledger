# Data inventory (T-104b)

Every piece of personal data Tavern Ledger handles, written from the code on 2026-10-09 (`main` at `ac22751`). The privacy policy ([privacy.md](privacy.md)) is built from this file; when the code changes, this file changes first.

This is an engineering inventory, not legal advice. Legal bases are proposals for the controller to confirm.

Status words: **live** = in the code on `main`; **planned** = designed but not merged; **not online** = nothing is hosted yet (no hosted Supabase project, no deployed site, no email provider account).

## 1. Desktop app and command-line tools (local only)

Nothing here leaves the user's PC. The desktop app makes no network request: its CSP only allows `ipc:` (`crates/desktop/tauri.conf.json`) and it has no HTTP dependency (`crates/desktop/Cargo.toml`). The controller never receives any of it.

| Data | Source (code) | Stored where | Notes |
|------|---------------|--------------|-------|
| Hearthstone log files (`Power.log`, `Hearthstone.log`, `log.config`, `client.config`) | `crates/tracker/src/discover.rs`, `tail.rs`, `setup.rs`; `tools/check_logs.py` | Read in place, never copied or written | The logs contain BattleTags (`PlayerName=`) of every player in the lobby. The parser reads them only to tell players apart and does not keep them (`crates/bg-parser/src/power.rs`, `store.rs` header). |
| Game history: per game, session folder name (local start time), game index, mode, build, status, own hero and teammate hero, lobby player numbers with their heroes, places and health, boards per combat (card ids, attack, health), tribes offered, card names in the game's language, parser version, save time | `crates/tracker/src/store.rs`, `crates/bg-parser/src/report.rs` | `%APPDATA%\TavernLedger\games.jsonl` and `games.lock` (D-015) | No player names, BattleTags, account ids, ratings or MMR. Kept until the user deletes the file. Export = copy the file; deletion = delete the folder. |
| "Start with Windows" setting | `crates/desktop/src/main.rs` (`tauri-plugin-autostart`) | `HKCU\...\Run` entry, only after the user turns it on | Not personal data in itself. |
| Desktop app's web view data | Tauri 2 / WebView2 | WebView2's own folder under `%LOCALAPPDATA%` for the app id `dev.tavernledger.app` | The app's pages load no remote content; nothing personal is put there by our code. |
| Leaderboard lookup (prototype script, not in the app) | `tools/leaderboard.py` | Cache in `.local/leaderboard-cache.json` | Sends requests to Blizzard's public leaderboard (`hearthstone.blizzard.com`) from the user's own machine and keeps only the user's own row (D-012). Blizzard receives the user's IP as with any visit to its site. Not run by the controller. |

| Upload sign-in and queue (#016, T-104d, D-025) | `crates/uploader` (`auth.rs`, `queue.rs`, `config.rs`), `crates/desktop/src/upload.rs` | Refresh token in Windows Credential Manager (one credential per data folder; never in a file); `uploads.jsonl` (game, account id, SHA-256 of the JSON sent, result) and `upload.json` (on/off) next to the history | Sign-in by a one-time email link opened in the system browser (PKCE, loopback listener on `127.0.0.1`); the app never sees a password. Upload is **off until the user turns it on**; when on, each finished game's record (the history line above, without names) goes to the server (section 2). Sign-out revokes the session, removes the credential and turns upload off. Server: the hosted project built in (`config.rs`), or `server.json` for the local stack. |

## 2. Server side (Supabase, EU) — hosted project, sign-ups closed

Supabase project `tavern-ledger` in Frankfurt (D-020, T-104a, created 2026-10-09, Free plan; deploy in [deploy.md](../research/deploy.md) section 10). Sign-ups are closed until the owner opens them. Processor: Supabase (section 4).

| # | Data | Purpose | Proposed legal basis (GDPR art. 6) | Stored where (code) | Retention | Export | Deletion |
|---|------|---------|-----------------------------------|---------------------|-----------|--------|----------|
| S1 | Account: email, password hash (bcrypt, by Supabase Auth), user id, created/updated/confirmed/last sign-in times, app and user metadata | Sign-in, account security | 6(1)(b) contract | `auth.users` (Supabase Auth); settings in `supabase/config.toml` (password ≥ 10, email confirmation, secure password change) | While the account exists | `export_my_data()` → `account` (no password hash) | `delete-account` Edge Function deletes the auth user |
| S2 | Sign-in identities (provider = email, identity data = email and user id) | Sign-in | 6(1)(b) | `auth.identities` | While the account exists | `identities` | With the auth user (cascade) |
| S3 | Sessions: created/updated times, expiry, **user agent, IP address**; refresh tokens; one-time tokens (confirmation links) | Keep the user signed in; security | 6(1)(b) and 6(1)(f) security | `auth.sessions`, `auth.refresh_tokens`, `auth.one_time_tokens` | Until sign-out, expiry or account deletion | `sessions` (user agent and IP; no tokens) | With the auth user (cascade) |
| S4 | Second factors (metadata only) | Not in use: TOTP and phone factors are off (`[auth.mfa]` in `supabase/config.toml`), so this table stays empty | — | `auth.mfa_factors` | — | `mfa_factors` (empty) | With the auth user |
| S5 | Auth audit log: event, time, **IP address**, payload (actor id, email in some events) | Security, abuse detection | 6(1)(f) | `auth.audit_log_entries` | While the account exists; removed by account deletion | `auth_events` | `delete_user_data()` and `delete_user_auth_events()` (`supabase/migrations/20261009120100_export_delete.sql`) |
| S6 | Profile: display name (optional, 1–32 letters/digits/space/`_.-`, no `#`), public switch (off by default), created/updated | Public profile, if the user turns it on | 6(1)(b); publication only on the user's own action | `public.profiles` (`supabase/migrations/20261009120000_profiles_games.sql`) | While the account exists | `profile` | `delete_user_data()` |
| S7 | Game summary rows: session name (local start date and time), game index, mode, status, hero card id, final place, build, date played, tribes offered, revision, SHA-256, size, storage path | Stats and history on the site | 6(1)(b) | `public.games` | While the account exists | `games` | `delete_user_data()` |
| S8 | Game files: the gzipped game record (section 1, no names) | Game viewer (F2), export | 6(1)(b) | Storage bucket `games`, `<user_id>/<session>-<index>.json.gz`, private | While the account exists | File list in `files`; the site adds every file's bytes (`web/src/lib/exportData.ts`) | `delete-account` removes files through the Storage API first |
| S9 | Platform logs kept by Supabase (API, auth, database and function logs, with IP addresses and request paths) | Run and secure the service | 6(1)(f) | Supabase's logging | One day for API and database logs on the Free plan (Supabase pricing page, 2026-10-09) | Not in the export (not reachable per user) | Expire on Supabase's schedule |
| S10 | Backups: full database dump (emails, password hashes, rows; **no live sessions or refresh tokens**) and the bucket | Recovery | 6(1)(f) | Developer's own PC, `.local/backups/`, readable by the developer's Windows user only (`tools/backup_supabase.ps1`); the Free plan has no Supabase backups | Weekly; each kept **35 days**, then deleted by the script (D-021). A deleted account can stay in backups up to 35 days | Not exported | Expire after 35 days; a restore must re-apply deletions made since the backup |
| S11 | Public profile, when on: display name, every game row and every game file of the user | Show progress to others | User's choice (6(1)(b); off by default) | RLS policies `profiles_read_own_or_public`, `games_read_own_or_public`, `games_bucket_read_own_or_public` | Public while the switch is on | — | Turn the switch off or delete the account. Copies others made cannot be recalled |

What is **not** collected server-side: player names, BattleTags, Battle.net ids, ratings or MMR (no column for them; a pgTAP test checks it), phone numbers (SMS sign-in off), payment data (no paid extras yet), analytics (none, user's decision 2026-10-09).

| S12 | Upload times: user id and time of each upload | Hourly and daily upload limits | 6(1)(f) availability | `private.upload_events` (`supabase/migrations/20261009150000_upload.sql`) | **Two days**, removed by the daily sweep | `upload_events` in `export_my_data()` | Daily sweep; with the account (cascade) |
| S13 | Refused-request counters: SHA-256 of the client IP (not salted, so reversible by trying every address: treated as personal data) and a count, for uploads with an invalid token | Block abusive requests | 6(1)(f) security | `private.upload_ip_failures` | **One hour**, removed by the daily sweep and on every write | Not in the export (not tied to an account) | Expire after one hour |
| S14 | Sweep runs: time and counts of each daily sweep | Notice a sweep that stopped running | — (no personal data) | `private.sweep_runs` (`supabase/migrations/20261010090000_sweep_schedule.sql`) | 90 days | — | — |

Writes: only the `upload-game` Edge Function writes S7 and S8, with the service role, after checking the record field by field (D-025). The daily `sweep` (pg_cron at 03:17 UTC) removes files with no row, audit entries of deleted accounts and expired S12/S13 rows.

## 3. Website (static, Cloudflare Pages) — online at tavernledger.net with accounts closed

| Data | Source (code) | Notes |
|------|---------------|-------|
| Supabase session in the browser's `localStorage` (access token, refresh token, user object incl. email) and a PKCE code verifier during sign-in | `web/src/lib/api.ts` (`persistSession: true`, `flowType: "pkce"`, default storage) | Strictly necessary for sign-in; removed on sign-out. No cookies are set by the site's code. |
| Request data at the static host (IP address, user agent, URL, time) | Cloudflare Pages (live since 2026-10-09) | Kept by Cloudflare as processor; see section 4. Cloudflare's own protection sets `cf_clearance` only on a visit it challenges (bot features, and so `__cf_bm`, are off). Network Error Logging turned off for the zone on 2026-10-09 (session 022). `[TO BE CHECKED]`: Pages request-log retention and the zone's challenge passage time. |
| No analytics, no ads, no third-party scripts, no fonts from other servers | `web/src/lib/config.ts` CSP: scripts, styles and images from the site only; connections to the Supabase project only | |

## 4. Processors and sub-processors

Supabase's DPA is part of its terms of service for every organization (no separate signature; Organization settings → Legal Documents, checked 2026-10-09), where a Transfer Impact Assessment can be downloaded. Cloudflare's Customer DPA is part of its terms.

| Processor | Role | Region | DPA | Status |
|-----------|------|--------|-----|--------|
| Supabase | Database, auth, file storage, Edge Functions, logs | Frankfurt (AWS eu-central-1), D-020 | Supabase DPA, part of the terms; TIA available. Sub-processors (list of 2026-06-01): AWS, Google, Cloudflare, Fly.io, Vercel and Upstash for hosting; Sentry and Braintrust for monitoring; OpenAI for language features; others for support and communication with the project's own dashboard users | In use since 2026-10-09 (project `tavern-ledger`, Free plan, sign-ups closed) |
| Cloudflare | Static hosting of the site | Global network | Cloudflare Customer DPA | In use since 2026-10-09 (Free plan; account and domain of the user); DPA version to check |
| Outbound email sender (custom SMTP for Supabase Auth) | Sends sign-up confirmation and account emails: recipient email, message content (confirmation link), delivery events | — | — | **Brevo**, free plan (user's choice, D-034, replacing Cloudflare Email Sending from D-030). Legal entity, region, retention, DPA and the free plan's branding: `[TO BE CHECKED]` on Brevo's own pages before setup. |
| Cloudflare Email Routing | Forwards mail sent to `contact@` and `privacy@tavernledger.net` to the controller's mailbox: sender address, message | Global network | Cloudflare Customer DPA | Set up 2026-10-09; catch-all off. Whether Cloudflare keeps anything beyond delivery logs: `[TO BE CHECKED]` |

Read on the providers' pages on 2026-10-09: Supabase's DPA names Supabase Pte. Ltd. (Singapore), version 1 of 2026-08-01, EU SCCs (no Data Privacy Framework), data stored and primarily processed in the chosen region but processable wherever Supabase or its sub-processors have facilities; Cloudflare's Customer DPA v6.4 (2026-04-03) relies on the EU–US Data Privacy Framework with SCCs as fallback. Not confirmed (marked `[TO BE CHECKED]` in the policy): the sub-processors' locations (the 2026-06-01 list names them and their purpose only), Cloudflare Pages request-log retention, and where Email Sending processes and how long it logs messages. Read in session 022: Supabase Free keeps API and database logs one day; Cloudflare's NEL pipeline keeps no client IP, and NEL can be turned off per zone (done).

## 5. Open items before the first user

- Done in session 021: controller Andrew Rodrigo, Bulgaria (user's answer); contact addresses `contact@` and `privacy@tavernledger.net`; authority the Bulgarian CPDP; governing law Bulgaria. Bulgaria sets the age of digital consent at 14, so the project's minimum age of 16 (D-027) stays stricter than the law.
- Cloudflare DPA version.
- A sign-up notice on `/signin/` ("By creating an account you agree to the terms, confirm you are 16 or older, and confirm you have read the privacy policy"; the policy is information, not something to consent to), plus a password reset (T-104c notes).
- The outbound email sender (see section 4) and its DPA.
- Decide whether the security log (S5) should be trimmed after a fixed time instead of living as long as the account (second opinion, MEDIUM; kept for now because it is small and deleted with the account).
- The effective date ("Last updated"): set the day accounts open, with the user's OK.

## 6. Second opinion (ChatGPT, 2026-10-09)

ChatGPT reviewed the drafts against this inventory. Applied: a separate section for visitors' request data (IP, user agent) with its own legal basis; the download is described by its real contents and kept apart from a formal access request, with backups and provider logs named as exceptions; providers described as "to be signed" instead of in place; one legal basis per purpose; the security log given its own retention; missing categories added (account id, account emails and delivery status, session start time, teammate and lobby heroes, card names, parser version in place of "app version"); portability, required vs optional data, no automated decisions and the complaint venue stated; public visitors named as recipients; terms made more proportionate (responsibility limited to what the user does, graded enforcement with a reason and a way to ask for review, 30 days' notice of changes, scraping limited to bulk copying of public profiles).

Rejected, with the reason:

- "MIT licence, server-only writes and HTTPS are not established." They are, in the code: `LICENSE`; no insert, update or delete grant or policy for clients on `games` or the bucket (`supabase/migrations/20261009120000_profiles_games.sql`); `web/src/lib/config.ts` refuses a non-HTTPS project URL and the build writes `Strict-Transport-Security` to `_headers`. The reviewer only had the three documents.
- "The parser's failure behaviour is not established." Records carry `status` (`ok`, `incomplete`, `unsupported`) and the app counts unreadable games apart (T-102, T-107); the terms now say "designed to", since a parser can still be wrong.
- "Public profile should be on consent." Kept on contract: it is a feature the user switches on and off, with no other effect; the policy says so. Either basis needs the same switch; the controller can revisit it.
- "Trim the security log after a fixed time." Left as an open item (section 5), not a draft change.
