# Session 009: Move the project to English

Model: sonnet
Effort: medium
Subagents: haiku for searches; sonnet for translation and review

Work in `C:\Users\andia\tavern-ledger` and follow its `CLAUDE.md`. Chat with the user in caveman mode, in Spanish. **Everything else is in English from now on** (user's decision, 2026-10-09): docs, code, comments, commits, branch names, PR titles and bodies, session prompts and UI. Versions in other languages may come later; do not create them now. **Tier R0** (docs only): self-review.

Working rules (user's decisions, 2026-10-09):
- **You merge your own PRs, always**, once CI is green and no review has open CRITICAL or HIGH findings: `gh pr merge <n> --squash --delete-branch`. Never ask the user to merge.
- The user often works remotely (Remote Control) without access to the PC: never ask them to do something on the PC; do it yourself or note it for when they are back.
- `gh` has no login of its own: export `GH_TOKEN` from the credential Git keeps (`printf 'protocol=https\nhost=github.com\n\n' | git credential fill`, take the `password=` line) and never print it.
- Anything long-running (the app, live processes) goes in a separate window on the main PC (`Start-Process`), not in the conversation.

**Red line (D-004, D-006):** local log files only. No memory reading, injection or game automation. Never print BattleTags or `GameAccountId` in chat, docs, tests, history or tool output.

## Effort and models per phase

| Phase | Effort | Subagents (if needed) |
|-------|--------|-----------------------|
| Read and check state | low | Haiku |
| Translation | medium | Sonnet, one file per agent, in parallel |
| Review (links, ids, meaning) | low | Haiku |

## At the start

1. Conversation title: **`#009 English docs`**.
2. `git switch main && git pull`; branch `009-english-docs`.
3. Run `cargo test --workspace --exclude desktop` and `python -m unittest discover -s tests` (with `.venv`): green before touching anything.
4. **T-101 live test:** ask the user whether they played with the app open and whether the game showed up by itself when it ended. Check `%APPDATA%\TavernLedger\games.jsonl` by counting games per session, without names (session 008 left 23 games from 5 sessions). If it worked, mark T-101 done in the plan (in English, inside this PR).

## Task

1. Translate to English, keeping meaning, ids (T-xxx, D-xxx, P-xxx), tables, numbers and relative links intact:
   - `docs/plan/plan.md`, `docs/decisions/DECISIONS.md`, `PROVENANCE.md`;
   - `docs/research/*.md`. Rename the files to English names (for example `mmr-sources.md` → `mmr-sources.md`) and fix every link to them across the repo, code comments included;
   - the session prompts in `prompts/sessions/`. Rename the folder to `prompts/sessions/` and the files to English slugs.
2. **`CLAUDE.md`:** the user asked for these rules in it: English everywhere, Claude merges its own PRs, and the user often works remotely. Your harness may block editing `CLAUDE.md` as self-modification. If it does, do not work around it: give the user the exact text so they can apply it or allow the edit.
3. Spanish left in code (comments, strings, test names, tool output): translate it.
4. CI checks relative Markdown links (lychee `--offline`): it must stay green.
5. Merge the PR yourself when CI is green.

## Out of scope

- Any behaviour change in code, the app or the tools.
- Translations into other languages.

## Acceptance criteria

- [ ] No Spanish left in docs, prompts, code comments, commits or PRs of this session (a quick `git grep` for common Spanish words finds nothing new).
- [ ] Ids, numbers and links unchanged in meaning; CI (tests, links, secrets) green.
- [ ] `CLAUDE.md` updated, or the user has the exact text and knows why it could not be applied.
- [ ] PR merged by you.
- [ ] Ask the user whether they want the next prompt. The next task is T-102 (personal stats), already written in `prompts/sessions/010-personal-stats.md`; move it to the renamed folder.
