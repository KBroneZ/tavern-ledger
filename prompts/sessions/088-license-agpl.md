# Session 088: Relicense from MIT to AGPL-3.0 (T-529)

Model: opus
Effort: medium
Subagents: haiku for searches; sonnet for the review

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\088-license-agpl`** (branch `088-license-agpl`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English. **Tier R2** (legal text, dependency licenses): `/code-review` + `/security-review` of the license checks.

**Other sessions at the same time:** 086 (combat replay: `crates/bg-parser/`, `crates/tracker/`, `crates/desktop/`) and 087 (Dark Gifts: `crates/combat-sim/`, `crates/cards/`, `crates/sim-wasm/`). Your area: `LICENSE`, the `license` fields (`Cargo.toml` workspace, `package.json`, `web/package.json`, `crates/desktop/tauri.conf.json` if it names one), the license policy files (`deny.toml` or whatever CI uses), `README.md`, `PROVENANCE.md`, `CLAUDE.md` (the one line in the header), `docs/legal/terms.md` and `terms.es.md` (only where they name the license), and the website's footer or about text if it names MIT. **Do not touch the crates' code.** Decision **D-101**. Every changed text file LF. To catch up: `git fetch origin && git merge origin/main`; no rebase, never force-push.

- The user may be busy: do not wait on them. Push the branch and message the hub (`tavern-ledger-2b`, `ListAgents`, use its `[ref]`) with the branch, PR title and a short PR body.

## Why

The user's decision (2026-10-12, through the hub): the project stays public and open source, but changes from **MIT to AGPL-3.0-only** ("AGPL-3.0-or-later" only if you find a strong reason; say it). The user is the only author and copyright holder, so they can relicense. Reasons: a public launch on Reddit and Discord soon (an open tracker is trusted, and free code signing through SignPath Foundation needs an OSI license), while someone who copies the app has to publish their copy under the same license; paid extras live on the server (D-002, D-003). **No "contributions not accepted" note**: the user is open to good contributions. This changes D-002's license, not "premium lives on the server".

## Task

1. `LICENSE`: the full, unmodified AGPL-3.0 text from the FSF (gnu.org), with a short header naming the copyright holder (the GitHub handle `KBroneZ`, as the MIT file does, never the user's email or real name). Every `license` field to `AGPL-3.0-only` (SPDX).
2. Check every dependency's license is compatible with AGPL-3.0 (Rust crates, npm packages, data and fonts in `PROVENANCE.md`); list any that is not, and stop and tell the hub if one is. Update the license allow list CI uses so CI stays green, and make sure our own crates' new license passes it.
3. Docs: README (license section, a sentence that versions published before the change stay available under MIT), `PROVENANCE.md`, the `CLAUDE.md` header line, the terms pages (EN and ES) where they name the license, the website if it names MIT anywhere. Historic docs (old decisions, research, session prompts) stay as written.
4. D-101 (amends D-002's license; why; the earlier MIT versions; contributions welcome under AGPL), plan row T-529, README.
5. **Not in this session:** the SignPath application (the hub drafts it), any change to the repo's GitHub settings.

## Acceptance criteria

- [ ] AGPL-3.0 text unmodified; every license field `AGPL-3.0-only`; no MIT left except in history and third-party notices.
- [ ] Dependency compatibility checked and listed; CI license check green.
- [ ] Workspace tests and the web build from a clean clone of the pushed branch (paste counts); `/code-review` with no open CRITICAL or HIGH; branch handed to the hub.

## Red line

Never print the user's email or real name, BattleTags, `GameAccountId` or keys. No account or GitHub settings changes.
