# Session 035: Uploads refused, server validator behind the parser (T-104h)

Model: opus
Effort: high
Subagents: haiku for searches; sonnet for code and reviews

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\035-upload-validator`** (branch `035-upload-validator`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English. **Tier R2** (production function, user data): TDD + `/code-review` + `/security-review`.

**Another session runs at the same time** (033 overlay hover: `crates/desktop` overlay, `crates/tracker` live state, maybe `crates/bg-parser`). To stay out of its way:
- Work only in your worktree. Never touch the main checkout or other worktrees.
- Your area: `supabase/functions/upload-game/` (validator and tests), `crates/uploader` (retrying refused games), and a contract test that ties the two to `crates/bg-parser`'s report (read `crates/bg-parser/src/report.rs`; change it only if the test needs a hook, and keep that change minimal). Do not touch `crates/desktop/src/overlay*`, `crates/desktop/ui/overlay*`, `crates/tracker/src/live.rs`.
- Shared files (`plan.md`, `README.md`, `DECISIONS.md`, `PROVENANCE.md`): only your task's row and notes. Your decision number is **D-047**. Keep every changed text file LF (`git ls-files --eol`).
- Only this session may start Docker and the local Supabase stack this round.
- Never stop or restart the user's desktop app; the hub restarts it after your merge.
- To catch up with `main`: `git fetch origin && git merge origin/main`. No rebase, never force-push. **Merging to `main` deploys the functions** (D-031): after the merge, check that `upload-game` still answers `401` without a token and `/auth/v1/settings` still says `disable_signup: true`.

Working rules (user's decisions):
- Decide technical matters yourself. Ask the user: new dependencies (a Deno step in CI counts: say what it adds and ask).
- **`gh` is not logged in inside task sessions.** When your branch is ready (tests green, both reviews done, no open CRITICAL or HIGH), push it and send the hub (`Hub prompts/sessions/025`, find it with `ListAgents`) a message with the branch, the PR title, a short PR body (summary, review results, test plan) and anything still waiting on the user. The hub opens the PR, waits for CI and merges it; then you run the post-merge checks and the user's retry (below) and report to the hub.

## Why

On 2026-10-10 the user signed the app in to the hosted project for the first time and turned uploads on: **all 26 games were refused with `invalid_report`** (`%APPDATA%\TavernLedger\uploads.jsonl`, outcome `rejected`, code `invalid_report`). Cause, found by the hub: `supabase/functions/upload-game/validate.ts` allows only the report keys the parser wrote before parser revision 2. Revision 2 (session 019, T-202/T-203) added `report.start_health` and `rounds[].health_after` (maps from player id to health, `crates/bg-parser/src/report.rs`), so every saved game carries keys the validator rejects. The local end-to-end tests did not catch it (their reports are older or synthetic), and CI runs neither the Deno tests nor pgTAP. Sessions 033 and 034 are about to add more to the parser's output, so this must not happen again.

## Task

1. **Validator.** Accept `start_health` and `rounds[].health_after` with strict checks (object, at most 16 entries, keys are player ids that appear in the report, values integers in a sane health range), no names possible in them. Check the whole current report shape against `report.rs` and the real fixtures (`crates/bg-parser/tests/data/real/`) for anything else missing. Deno tests for each new field and each refusal.
2. **Contract test so the two never drift again.** For example: one checked-in list of the report's keys at each level that both a Rust test (serialising a full report from the real fixtures and every synthetic case) and the Deno validator use, so a new parser field fails `cargo test` in CI until the validator is updated. Pick the simplest design that runs in the existing CI; if running Deno in CI is the better fix, ask the user first (new dependency).
3. **Retry refused games.** Today a refused game is not retried until its bytes change. After a server fix the user's 26 games must go up without manual work: for example a refused mark that remembers the server's validator version (returned in the refusal) or the app's version, and is retried once when it differs, or a "Retry refused games" button in the upload panel. Never a retry loop. Tests in `crates/uploader`.
4. **Check** on the local stack with the user's real report shape (the real fixtures through the parser, then `upload-game`): stored, not refused. After the merge, ask the user to press the retry (or restart the app if the retry is automatic) and confirm the 26 games are stored (the app's counts; you can also count the user's own rows read-only in the SQL editor if the user allows).
5. Plan row **T-104h** "Uploads refused: validator behind the parser" and notes, README, D-047, same PR. Tell session 034's prompt owner (the hub) what 034 must do when it adds shop fields.

## Out of scope

Opening sign-ups, the website, the overlay, new report fields.

## Acceptance criteria

- [ ] A report from the current parser (revision 2, real fixtures) is accepted; names are still refused.
- [ ] A contract test in the existing CI fails when the parser adds a report key the validator does not know.
- [ ] Refused games are retried once after the fix, never in a loop; tests.
- [ ] After the merge the user's 26 games are stored.
- [ ] `/code-review` and `/security-review` with no open CRITICAL or HIGH; PR merged through the hub; post-merge checks done.

## Red line

D-004, D-006, D-012, D-022 apply. Never print BattleTags, `GameAccountId` or the user's email. No accounts, terms, payments or publishing without the user.
