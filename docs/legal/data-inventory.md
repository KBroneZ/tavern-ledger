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

**Planned (#016, T-104d, not merged on 2026-10-09):** sign-in from the app through the system browser; refresh token in Windows Credential Manager (never in plain text); an "uploaded" mark per game; upload **off until the user turns it on**. When on, each finished game's record (the history line above, without names) is sent to the server (section 2). Re-check this section when #016 is merged.

## 2. Server side (Supabase, EU) — not online yet

Supabase project planned in Frankfurt (D-020, T-104a). Processor: Supabase (section 4).

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
| S9 | Platform logs kept by Supabase (API, auth, database and function logs, with IP addresses and request paths) | Run and secure the service | 6(1)(f) | Supabase's logging | Supabase's own retention for the plan (see [processors](#4-processors-and-sub-processors)) | Not in the export (not reachable per user) | Expire on Supabase's schedule |
| S10 | Backups: full database dump (emails, password hashes, rows; **no live sessions or refresh tokens**) and the bucket | Recovery | 6(1)(f) | Developer's own PC, `.local/backups/`, readable by the developer's Windows user only (`tools/backup_supabase.ps1`) | Weekly; each kept **35 days**, then deleted by the script (D-021). A deleted account can stay in backups up to 35 days | Not exported | Expire after 35 days; a restore must re-apply deletions made since the backup |
| S11 | Public profile, when on: display name, every game row and every game file of the user | Show progress to others | User's choice (6(1)(b); off by default) | RLS policies `profiles_read_own_or_public`, `games_read_own_or_public`, `games_bucket_read_own_or_public` | Public while the switch is on | — | Turn the switch off or delete the account. Copies others made cannot be recalled |

What is **not** collected server-side: player names, BattleTags, Battle.net ids, ratings or MMR (no column for them; a pgTAP test checks it), phone numbers (SMS sign-in off), payment data (no paid extras yet), analytics (none, user's decision 2026-10-09).

**Planned (#016, T-104d):** the `upload-game` Edge Function writes S7 and S8; per-user and per-IP quotas (IP used for rate limiting); a `parser_version` column; a daily sweep of files with no row. Re-check when merged.

## 3. Website (static, Cloudflare Pages) — online at tavernledger.net with accounts closed

| Data | Source (code) | Notes |
|------|---------------|-------|
| Supabase session in the browser's `localStorage` (access token, refresh token, user object incl. email) and a PKCE code verifier during sign-in | `web/src/lib/api.ts` (`persistSession: true`, `flowType: "pkce"`, default storage) | Strictly necessary for sign-in; removed on sign-out. No cookies are set by the site's code. |
| Request data at the static host (IP address, user agent, URL, time) | Cloudflare Pages (live since 2026-10-09) | Kept by Cloudflare as processor; see section 4. Cloudflare's own protection may set a security cookie on a challenged visit; its `NEL` header asks browsers to report failed connections to Cloudflare. `[TO BE CHECKED]`: retention of both. |
| No analytics, no ads, no third-party scripts, no fonts from other servers | `web/src/lib/config.ts` CSP: scripts, styles and images from the site only; connections to the Supabase project only | |

## 4. Processors and sub-processors

None of these has an account or a signed DPA yet; each must be in place before the first user (T-104a, T-104b).

| Processor | Role | Region | DPA | Status |
|-----------|------|--------|-----|--------|
| Supabase | Database, auth, file storage, Edge Functions, logs | Frankfurt (AWS eu-central-1), D-020 | Supabase DPA (see the research notes in [web-stack.md](../research/web-stack.md)); sub-processors listed by Supabase | Not created |
| Cloudflare | Static hosting of the site | Global network | Cloudflare Customer DPA | In use since 2026-10-09 (Free plan; account and domain of the user); DPA version to check |
| Outbound email sender (custom SMTP for Supabase Auth) | Sends sign-up confirmation and account emails: recipient email, message content (confirmation link), delivery events | — | — | **Cloudflare Email Sending** (user's choice, D-030; Cloudflare Customer DPA, beta). Set up by session 022 with the hosted project; region, retention and beta terms `[TO BE CHECKED]` then. |
| Cloudflare Email Routing | Forwards mail sent to `contact@` and `privacy@tavernledger.net` to the controller's mailbox: sender address, message | Global network | Cloudflare Customer DPA | Set up 2026-10-09; catch-all off. Whether Cloudflare keeps anything beyond delivery logs: `[TO BE CHECKED]` |

Read on the providers' pages on 2026-10-09: Supabase's DPA names Supabase Pte. Ltd. (Singapore), version 1 of 2026-08-01, EU SCCs (no Data Privacy Framework), data stored and primarily processed in the chosen region but processable wherever Supabase or its sub-processors have facilities; Cloudflare's Customer DPA v6.4 (2026-04-03) relies on the EU–US Data Privacy Framework with SCCs as fallback. Not confirmed (marked `[TO BE CHECKED]` in the policy): Supabase's log retention per plan and its sub-processor locations (the list is a PDF that could not be read), Cloudflare Pages request-log retention, and every Brevo fact (entity, DPA, data location, log retention), because Brevo's legal pages could not be read.

## 5. Open items before the first user

- Controller name and country. (Contact addresses exist since session 021: `contact@` and `privacy@tavernledger.net`.)
- DPAs with Supabase and Cloudflare (the sender is Cloudflare Email Sending, D-030; Brevo is no longer planned).
- A sign-up notice on `/signin/` ("By creating an account you agree to the terms, confirm you are 16 or older, and confirm you have read the privacy policy"; the policy is information, not something to consent to), plus a password reset (T-104c notes).
- The outbound email sender (see section 4) and its DPA.
- Decide whether the security log (S5) should be trimmed after a fixed time instead of living as long as the account (second opinion, MEDIUM; kept for now because it is small and deleted with the account).
- Re-check sections 1 and 2 when #016 (upload) is merged.

## 6. Second opinion (ChatGPT, 2026-10-09)

ChatGPT reviewed the drafts against this inventory. Applied: a separate section for visitors' request data (IP, user agent) with its own legal basis; the download is described by its real contents and kept apart from a formal access request, with backups and provider logs named as exceptions; providers described as "to be signed" instead of in place; one legal basis per purpose; the security log given its own retention; missing categories added (account id, account emails and delivery status, session start time, teammate and lobby heroes, card names, parser version in place of "app version"); portability, required vs optional data, no automated decisions and the complaint venue stated; public visitors named as recipients; terms made more proportionate (responsibility limited to what the user does, graded enforcement with a reason and a way to ask for review, 30 days' notice of changes, scraping limited to bulk copying of public profiles).

Rejected, with the reason:

- "MIT licence, server-only writes and HTTPS are not established." They are, in the code: `LICENSE`; no insert, update or delete grant or policy for clients on `games` or the bucket (`supabase/migrations/20261009120000_profiles_games.sql`); `web/src/lib/config.ts` refuses a non-HTTPS project URL and the build writes `Strict-Transport-Security` to `_headers`. The reviewer only had the three documents.
- "The parser's failure behaviour is not established." Records carry `status` (`ok`, `incomplete`, `unsupported`) and the app counts unreadable games apart (T-102, T-107); the terms now say "designed to", since a parser can still be wrong.
- "Public profile should be on consent." Kept on contract: it is a feature the user switches on and off, with no other effect; the policy says so. Either basis needs the same switch; the controller can revisit it.
- "Trim the security log after a fixed time." Left as an open item (section 5), not a draft change.
