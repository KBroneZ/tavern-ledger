# Tavern Ledger — Hearthstone Battlegrounds tracker

Personal project (name chosen in D-007; repo and local folder `tavern-ledger`): a Windows desktop app that reads the Hearthstone log and a website to follow your progress. Free by default, with optional paid extras. Open source (D-002).

**Team:** the user (the only human: decides, approves, publishes) + Claude Code (engineering) + ChatGPT Plus (research and second opinion with the `ask-chatgpt` skill).

Rules ported from StartAICareer (`C:\Users\andia\StartAICareer\CLAUDE.md`) and adapted. Anything not written here is not inherited.

## Session protocol

1. At the start: read `README.md` (status) and the task in `docs/plan/plan.md`. Check `docs/decisions/DECISIONS.md` if the task touches something pending.
2. One task per conversation. Conversation title: `#NNN <name>` (this repo's own numbering).
3. At the end: task status in the plan, new decisions in `DECISIONS.md` and README up to date, in the same commit or PR as the task.
4. Ask whether the user wants the prompt for the next session; write it only if they ask, in `prompts/sessions/NNN-slug.md`, with effort per phase and the subagent model (Haiku for searches, Sonnet for code, Opus only for architecture, security or irreversible decisions).

## Absolute rules

1. **Clean-room.** Nothing from the user's employer and no third-party code with an incompatible license ever comes in. The code of HDT ("All Rights Reserved"), Firestone (no license), their simulators, HearthMirror and BobsBuddy **is not copied or ported**. The code of Nomi's Kitchen ("MIT NON-AI" license) **is not read or given to any AI**. Only code with a compatible license (MIT, Apache-2.0…) is reused, and it is recorded in `PROVENANCE.md`. In doubt → stop and tell the user.
2. **Personal hardware and accounts only.**
3. **Limits with Blizzard (D-004):** the MVP only reads local log files. No memory reading, injection (BepInEx or similar), game file changes, automated actions, or showing information the player could not see on their screen. Injection and client modification are ruled out without exception (D-006); any other exception needs an explicit user decision in `DECISIONS.md`.
4. **Respect other people's APIs:** Blizzard's public leaderboard is queried with a cache, few requests, a timeout and an identifiable User-Agent. No scraping beyond the public endpoints the website itself uses, and never at an aggressive rate.
5. **Brand and art:** no Blizzard logos or name in the brand. Card images only under Blizzard's Fan Content Policy. Always the notice "Unofficial fan project. Not affiliated with or endorsed by Blizzard Entertainment."
6. **User data (GDPR):** minimal, with a privacy policy, export and account deletion from the first day there is a server. Never sell data.
7. **Secrets** never in code, fixtures, logs or commits. `.env` and `.local/` in `.gitignore`.
8. **Marketing:** never make up features, figures or testimonials. Never say "approved by Blizzard".
9. **External actions** (creating the GitHub repo, publishing, paying, creating accounts, accepting terms, push) → only with the user's explicit confirmation. Never push straight to `main` and never force-push once a remote exists.

## Working rules (user's decisions, 2026-10-09)

- **Claude merges its own PRs, always**, once CI is green and no review has open CRITICAL or HIGH findings (`gh pr merge <n> --squash --delete-branch`). Never ask the user to merge.
- **The user often works remotely** (Remote Control) without access to the PC: never ask them to do something on the PC; do it yourself or note it for when they are back. Anything long-running (the app, live processes) goes in a separate window (`Start-Process`), not in the conversation.
- **English everywhere** in the repo: docs, code, comments, commits, branch names, PR titles and bodies, session prompts and UI. Versions in other languages may come later.

## Engineering principles

- `main` always releasable. Tests before or alongside the code.
- Never declare "done" without evidence (tests, output, diff).
- Empty ≠ unknown ≠ zero. A silent failure is a defect. If a Hearthstone patch breaks the parser, the app says so ("unsupported version") instead of showing false data.
- Fixtures: only the user's own logs or synthetic ones, with no data from other players that is not public.
- Every network integration: timeout, handled errors, response validation, logging.
- External responses and AI output = untrusted input until verified.
- Every dependency has its origin and license in `PROVENANCE.md`.
- Simple > clever. No speculative abstractions. Deterministic code before LLMs.

## Risk and review

| Tier | Examples | Review |
|------|----------|--------|
| R0 | docs, copy | self-review |
| R1 | normal code | TDD + `/code-review` |
| R2 | network, auth, user data, dependencies, installer | `/code-review` + `/security-review` |
| R3 | payments, secrets, production, code signing | R2 + a second AI (`/santa-loop` or ChatGPT) + user's OK |

## Communication

- Chat with the user in `/caveman full` (global rule).
- Everything in the repo is in plain, concise English.
- Long research → `ask-chatgpt` in the background.

## Map

```
CLAUDE.md                rules (this file)
README.md                current status
PROVENANCE.md            origin and license of dependencies and data
docs/plan/plan.md        phases and tasks
docs/decisions/          decisions taken and pending
docs/research/           verified research
prompts/sessions/        session prompts (only if the user asks)
tools/                   read-only scripts (check_logs.py, parse_bg.py)
spikes/                  throwaway tests (overlay-tauri: the T-005 overlay test)
tests/                   tests (unittest; dependencies pinned in requirements.txt)
supabase/                backend: migrations, database tests, Edge Functions (local stack)
package.json             dev tooling only (Supabase CLI, pinned)
requirements.txt         Python dependencies with version and hash
.github/workflows/       CI: tests, links and secrets
.local/                  local only, never in git
```
