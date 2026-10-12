# Session 110: Consent to use uploaded games to improve the simulator (T-547)

Model: opus · Effort: high · Subagents: haiku for searches, sonnet for code and reviews

Work in your worktree `C:\Users\andia\tavern-ledger\.claude\worktrees\110-sim-consent` (branch `110-sim-consent`) and follow its `CLAUDE.md` (D-117: no AI attribution lines; status goes to `docs/status.md`; prompts live in `docs/process/sessions/` — copy this prompt there as `110-sim-consent.md` in your PR). Chat in caveman mode; the user writes in Spanish, answer in English. Repo in English, LF files, TDD. **Tier R2** (user data, consent, migration): `/code-review` and `/security-review`.

Hub: `tavern-ledger-c7` (ListAgents/SendMessage, use its `[ref]`). You cannot run `gh`: push the branch and message the hub with branch, PR title, short body, screenshots and the post-merge checks. Another session (109, page backgrounds) works in `web/` styles and page backgrounds: do not touch its files; tell the hub before editing a shared page layout.

## The user's decision (2026-10-12)

"Add it to the privacy policy, and give each account an option to allow its games to be used to improve the simulator, or not."

## Task

1. **Per-account setting, OFF by default** (explicit opt-in consent, GDPR). Stored server-side (migration on `public.profiles` or a small private table) with the time it was given or withdrawn. Only the owner reads and writes their own value (RLS or an RPC); included in the data export; removed with the account; grants for `anon`/`authenticated` minimal and tested.
2. **Account page toggle** on the website, EN and ES (voice guide `docs/brand/voice.md`): what is used (board states and combat replays of uploaded games), how (aggregate accuracy measurement and rule fixes; never published per player; no names), that it can be withdrawn at any time and what happens then (no further use; nothing already aggregated can be traced back).
3. **Privacy policy EN and ES** (`docs/legal/privacy*.md`) and **data inventory**: new purpose, legal basis consent, retention (only while consent stands and while the files exist, D-109's 28 days for free accounts), dated, with a "What changed" line.
4. **Owner-only way to get the opted-in games** for `combat-validate --games` (e.g. a service-role-only SQL function or a documented query that lists opted-in game file paths). Document it in `docs/research/combat-simulator.md` and `docs/research/deploy.md`. Do not run it on real data in this session.
5. Tests: pgTAP (default off, owner-only access, export, delete), web tests for the toggle, build checks. Decision **D-119**, plan row **T-547**, `docs/status.md` line.

## Red line

Rules 6 and 7. Never print BattleTags, `GameAccountId`, user ids, keys or the user's email; no dashboard change.
