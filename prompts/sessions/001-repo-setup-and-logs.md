# Session 001: Kickoff — name, GitHub repo and logs

Model: sonnet
Effort: medium
Subagents: haiku

Work in `C:\Users\andia\bg-tracker` and follow its `CLAUDE.md`: caveman in the chat; documents in Spanish; public README and product in English. **Tier R0–R1** (docs, configuration and a log-check script; the push and repo creation are external actions that need the user's OK).

**Red line (D-004, D-006):** local log files only. No injection, BepInEx, DLLs, memory reading, modifying game files or automating actions. If a task seems to need any of that, stop and warn.

## Effort and models per phase

| Phase | Effort | Subagents (if needed) |
|-------|--------|-----------------------|
| Read and plan | low | Haiku (searches) |
| Name (availability research) | medium | `ask-chatgpt` in the background to check names, domains and trademarks |
| Implementation (log script, repo) | medium | Sonnet |
| Review | medium | — (self-review + `/code-review` of the script) |
| Docs (plan, decisions, README) | low | — |

**Autonomy:** Claude proposes names and checks availability; **the user picks the name** and **approves creating the GitHub repo and pushing**. Claude decides the structure of the script and the CI.

## At the start

1. Title **`#001 Repo setup and logs`**.
2. Read: `README.md`, `docs/plan/plan.md` (T-001 and T-002), `docs/decisions/DECISIONS.md` (P-001, D-001 to D-006), `docs/research/feasibility.md`.

## Task

1. **Name (P-001):** propose 5–8 names without "Hearthstone", "Blizzard" or game trademarks. Check that no other known tracker uses them, that the name is free on GitHub and whether a `.gg` or `.app` domain is free (buy nothing). Ask the user which one they pick.
2. **Repo (T-001):** with the user's OK, rename the folder if needed, create the **public** repo on GitHub (Actions are free on public repos) and push `main`. From then on, branches and PRs, never a direct push to `main`.
3. **Minimal CI:** workflow that validates Markdown links and scans for secrets (e.g. gitleaks with a pinned version). Record each action and its version in `PROVENANCE.md`.
4. **Logs (T-002, local part):** read-only script that checks whether Hearthstone's `log.config` enables the `Power` log and where logs are written on this machine. Tell the user what they need to enable, but **do not touch game files or their configuration without their OK**. If the user already has BG game logs, inventory them (date, size, mode) without copying them to the repo yet.
5. Update the plan (T-001 done; T-002 in progress), `DECISIONS.md` (P-001 → decision) and the README.

## Out of scope

- Parser and prototype with `hslog` (T-003, next session).
- Choosing the stack (P-002), web or backend.
- Buying a domain, code signing or any spending.

## Acceptance criteria

- [ ] Name chosen by the user and recorded in `DECISIONS.md`.
- [ ] Public GitHub repo with README, MIT LICENSE, `.gitignore` and CI green (link to the run).
- [ ] Log-check script with its real output as evidence; nothing written to game folders.
- [ ] Plan, decisions and README up to date in the same PR.
