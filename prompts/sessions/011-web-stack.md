# Session 011: Web stack and backend (T-103, P-003)

Model: opus (architecture decision; it shapes T-104 and the GDPR work)
Effort: high for the comparison; low for the rest
Subagents: haiku for searches; sonnet for drafting and review; `ask-chatgpt` in the background for pricing research and a second opinion

Work in `C:\Users\andia\tavern-ledger` and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English. **Tier R0** (research and docs only, no code beyond a throwaway spike in `spikes/` if one is needed). The final choice is the user's call (it means accounts, terms and maybe costs): recommend, then ask.

Working rules (user's decisions, 2026-10-09):
- Decide technical matters yourself. Ask only what is the user's call: creating accounts, accepting terms, paying, new dependencies, data with unclear licences.
- **You merge your own PRs, always**, once CI is green and no review has open CRITICAL or HIGH findings: `gh pr merge <n> --squash --delete-branch`.
- The user often works remotely without access to the PC: never ask them to do something on it.
- `gh` has no login of its own: export `GH_TOKEN` from the credential Git keeps (`git credential fill`) and never print it.
- Anything long-running goes in a separate window (`Start-Process`), not in the conversation.
- **Do not create any account, project or API key** on any service. Free-plan limits come from the providers' own pricing pages, with the date read.

**Red line (D-004, D-006, D-012):** local log files only on the desktop. The server never crawls the leaderboard and never stores other players' rows. Never print BattleTags or `GameAccountId` in chat, docs, tests or tool output.

## Effort and models per phase

| Phase | Effort | Subagents (if needed) |
|-------|--------|-----------------------|
| Read and check state | low | Haiku |
| Requirements from T-104 and F2-F4 | medium | — |
| Options and real free-plan costs | high | Haiku for pricing pages; `ask-chatgpt` for a second source |
| Recommendation and doc | high | Sonnet for a review of the doc |

## At the start

1. Conversation title: **`#011 Web stack`**.
2. `git switch main && git pull`; branch `011-web-stack`.
3. Read `README.md`, `docs/plan/plan.md` (F1 to F4), `docs/decisions/DECISIONS.md` (D-002, D-003, D-005, D-011 to D-015, D-019; P-003, P-005, P-006) and `docs/research/feasibility.md`, `docs/research/desktop-stack.md`, `docs/research/leaderboard-client.md`.
4. Green before touching anything: `cargo test --workspace --exclude desktop` and `python -m unittest discover -s tests` (with `.venv`).
5. T-101 is still open (live test). Check `%APPDATA%\TavernLedger\games.jsonl` for new games, counting per session without names (session 010 left 23 games from 5 sessions). If a new game appeared on its own while the app was open, mark T-101 done in the plan inside this PR. The app was left running from the session 010 build.

## Task

1. **Requirements**, written down first (from the plan and decisions, not invented):
   - accounts and login; game upload from the desktop app; public profile; privacy policy, export and account deletion (rule 6, T-104);
   - expected sizes: a game record is about 20-40 KB of JSON today (check with the real history); replays (F2) and recaps;
   - free by default (D-003), optional paid extras later (P-006); no ads;
   - the desktop app is Tauri 2 + Rust; the web may share UI with it (D-014);
   - EU users (GDPR): where data lives, who processes it, a data processing agreement.
2. **Options**: at least three full stacks, for example:
   - static site (Astro or similar) + Supabase (Postgres, auth, storage);
   - Next.js on Vercel + a managed Postgres (Neon or similar) + object storage (Cloudflare R2);
   - a small Rust or Node API on a cheap VPS or a container host + Postgres.
   For each: free-plan limits and what happens when they run out, first paid step and its price, EU region, DPA, vendor lock-in and how to export everything, auth options (email, Battle.net OAuth if it exists and its terms), operations load for one person, and licences of what would enter the repo.
3. **Upload format**: how the desktop app would send a game (size, auth, idempotency by session + game index, retries). Design only, no code.
4. **Write** `docs/research/web-stack.md` with sources and the date each price was read. Mark anything unverified.
5. **Recommend** one stack and a plan B, and ask the user to choose. Record the choice as a new D-0xx in `DECISIONS.md`, close P-003, and split T-104 into smaller tasks in the plan if that helps.
6. Plan (T-103), README and `DECISIONS.md` in the same PR.

## Out of scope

- Creating accounts, projects, domains or keys on any service.
- Implementing the web, the API or the upload (T-104).
- MMR on the server (D-012 forbids crawling); payments (P-006, F4).

## Acceptance criteria

- [ ] Requirements written before the options.
- [ ] At least three options with real free-plan limits, prices, EU region and DPA, each with a source and the date read.
- [ ] Upload design: size, auth, idempotency, retries.
- [ ] User's choice recorded as a decision and P-003 closed (or, if the user has not answered, the recommendation is in the doc and P-003 stays open with a note).
- [ ] CI green; PR merged by you.
- [ ] Ask the user whether they want the next prompt.
