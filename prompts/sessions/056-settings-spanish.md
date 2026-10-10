# Session 056: Settings page, the app in Spanish, overlay picture crop (T-324, T-325)

Model: opus
Effort: medium (high for the translation layer design)
Subagents: haiku for searches; sonnet for code and reviews

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\056-settings-spanish`** (branch `056-settings-spanish`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English except the Spanish strings themselves. **Tier R1** (UI only): TDD + `/code-review`. A new dependency makes it R2: avoid one (a small own string table is enough) or ask the hub first.

**Other sessions at the same time:** 055 (memory reader: a new crate, `crates/tracker/` wiring, a small config flag and Tauri commands in `crates/desktop/src/`, and text changes in `web/`, `docs/legal/`, `README.md`, `CLAUDE.md`). To stay out of its way:
- Work only in your worktree. Never touch the main checkout or other worktrees.
- Your area: `crates/desktop/ui/` (main window and overlay files), the settings parts of `crates/desktop/src/main.rs` and a new settings module if you need one. Do not touch `crates/tracker/`, `crates/bg-parser/`, the new reader crate, `supabase/`, `web/`, `docs/legal/`.
- **The memory switch (D-063):** 055 adds a config flag (off by default) and Tauri commands. Build the Settings page so that switch has a place, but **wire it only after 055 merges** (the hub tells you; then `git merge origin/main` and add the switch with 055's risk paragraph, off by default, and its status). If 055 is not merged when you are done otherwise, hand over without it; it becomes a small follow-up.
- Shared files (`plan.md`, `README.md`, `DECISIONS.md`): only your tasks' rows and notes. Decision number if you need one: **D-065**. Every changed text file LF (`git ls-files --eol`, index side).
- Never stop or restart the user's desktop app; test with your own build and a scratch copy of the history. After a build, `git checkout -- crates/desktop/gen/schemas`. CI does not build `crates/desktop` nor run its Node UI tests: run them yourself and paste the counts.
- To catch up with `main`: `git fetch origin && git merge origin/main`. No rebase, never force-push.

Working rules (user's decisions):
- Decide technical and design matters yourself (Lamplight glass, D-052; the compact style of 053, D-062).
- **Ask your questions through the hub** (`tavern-ledger-47`, find it with `ListAgents` and use its `[ref]`) as well as in your tab.
- **`gh` is not available in task sessions.** Push the branch and message the hub with the branch, PR title, a short PR body and anything waiting on the user.

## Why: the user's notes (2026-10-10 and 2026-10-11)

- "A settings part to change some things in the app, like adding other languages, starting with Spanish."
- The overlay's pictures have the same white bars 053 fixed in the main window (053's note: `object-view-box: inset(1% 16% 2%)` for minions, `inset(0 8%)` for heroes, cover on a dark background).

## Task

1. **Settings page** (T-324): one place, reached from the tab bar (a gear), for the options that exist today (overlay on or off, theme, uploads on or off, pictures on or off if it exists, language) and room for the memory switch. Changes apply at once and persist where the app already keeps its settings. Keyboard and screen reader friendly (045's rules).
2. **Languages** (T-325): a small string table (English as the source, Spanish complete), every UI string in the main window and the overlay through it, numbers and dates formatted for the language, default from the Windows language with the Settings choice winning. A test fails if a key is missing in Spanish. Card and hero names stay as the card data gives them (card names in the game's language is a separate later item). Spanish should read naturally (Spain), short labels.
3. **Overlay pictures:** the same crop as 053 in the hover card and panels; a check like 053's.
4. Node UI tests and markup checks; plan rows T-324, T-325 with notes; README; same PR.

## Out of scope

The memory reader itself (055), the website in Spanish, card names in the game's language.

## Acceptance criteria

- [ ] Settings page with the current options, persisted, accessible (tested).
- [ ] The whole app and overlay in Spanish when chosen; no missing key (tested).
- [ ] No white bars in overlay pictures.
- [ ] `cargo test --workspace`, the desktop Node UI tests and a release build of `desktop` pass locally (paste counts).
- [ ] `/code-review` with no open CRITICAL or HIGH; branch handed to the hub.

## Red line

D-004 (as amended by D-063), D-006, D-012, D-022 apply. Never print BattleTags, `GameAccountId`, player names or the user's email.
