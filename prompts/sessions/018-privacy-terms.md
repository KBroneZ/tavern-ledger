# Session 018: Privacy policy and terms, draft (T-104b)

Model: opus (GDPR text that people will rely on)
Effort: medium
Subagents: haiku for looking up providers' DPAs and sub-processor lists; `ask-chatgpt` (review mode) for a second opinion on the draft

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\018-privacy-terms`** (branch `018-privacy-terms`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English. **Tier R0** (text and one static page), but the content is legal: say plainly that it is not legal advice, and ask the user for every choice that is theirs.

**Other sessions run at the same time** (#016 desktop upload; #017 provenance and report bundle). To stay out of each other's way:
- Work only in your worktree. Never touch the main checkout or other worktrees.
- Your area: `docs/legal/` (new), `web/src/pages/privacy.astro` and a new `web/src/pages/terms.astro`, and the links to them in the site's layout. Nothing in `crates/`, `supabase/` or `tools/`.
- Shared files (`plan.md`, `README.md`, `DECISIONS.md`): only your task's row and notes. Your decision number, if needed, is **D-027**.
- To catch up with `main`: `git fetch origin && git merge origin/main`. No rebase, never force-push.
- Never start Docker or the desktop app.

Working rules (user's decisions, 2026-10-09):
- Decide wording and structure yourself. Ask the user, in **one** `AskUserQuestion` with a recommendation for each: the email provider for sign-up mails (with its DPA and EU region), the controller's name and contact address to publish (the user is a private person: suggest a dedicated project email, never their personal one without asking), whether to use any analytics (recommended: none), and the minimum age (recommended: 16, the GDPR default for consent without parents in many EU countries).
- If the user hasn't answered, write placeholders that are impossible to miss (`[TO BE DECIDED: …]`) and keep the site's "not open to the public yet" notice.
- **You merge your own PRs, always**, once CI is green and the self-review is done: `gh pr merge <n> --squash --delete-branch`.
- The user often works remotely without access to the PC: never ask them to do something on it.
- `gh` has no login of its own: export `GH_TOKEN` from the credential Git keeps (`git credential fill`) and never print it.
- Never invent facts about the product: every statement in the policy must match the code, the schema and the decisions (what is collected, where, for how long, who processes it).

## Effort and models per phase

| Phase | Effort | Subagents (if needed) |
|-------|--------|-----------------------|
| Read and check state | low | Haiku |
| Inventory of data (from the code, not from memory) | medium | Haiku |
| Questions to the user | low | — |
| Draft and second opinion | medium | `ask-chatgpt` review |

## At the start

1. Conversation title: **`#018 Privacy and terms`**.
2. `git fetch origin && git merge origin/main`.
3. Read `CLAUDE.md` (rules 5, 6, 8), `docs/plan/plan.md` (T-104a to T-104d and notes), `docs/decisions/DECISIONS.md` (D-002, D-003, D-012, D-015, D-020, D-021, D-023), `docs/research/web-stack.md` (requirements, DPAs, regions), `supabase/` (schema, retention, export, deletion, backups) and `web/`.

## Task

1. **Data inventory** (`docs/legal/data-inventory.md`): every piece of personal data, from the code: the desktop app (local only: what it reads, what it stores, nothing sent unless upload is on), the site, Supabase (auth record, profile, game rows and files, audit log, backups and their 35 days), the email provider. For each: purpose, legal basis, where it is stored, processor, retention, how it is exported and deleted. Note what #016 adds (upload) as "planned" until it is merged; re-check before finishing.
2. **Privacy policy** (`docs/legal/privacy.md` and the site page): controller and contact, data and purposes, legal bases, processors and sub-processors with regions and DPAs, transfers outside the EU (if any, with the safeguard), retention, rights (access, export, rectification, deletion, objection, complaint to a supervisory authority: the Spanish AEPD if the controller is in Spain), cookies and local storage (what Supabase's client stores), children, changes. Plain English, short.
3. **Terms** (`docs/legal/terms.md` and the page): free fan project, the "Unofficial fan project. Not affiliated with or endorsed by Blizzard Entertainment." notice, no warranty, acceptable use, account deletion, public profile is public, open-source licence of the code (MIT) separate from Blizzard's material.
4. **Second opinion**: `ask-chatgpt` in review mode on the draft against the inventory; fix what holds up, list what was rejected and why.
5. Plan (T-104b: "draft done; waits for the user's answers" or "done"), README and D-027 if needed, in the same PR.

## Out of scope

- Publishing the site, creating the email provider's account, a cookie banner (only needed if non-essential cookies appear; there should be none).

## Acceptance criteria

- [ ] Inventory written from the code, each item with its file or table.
- [ ] Policy and terms match the inventory; no invented facts; placeholders where the user hasn't answered.
- [ ] Second opinion done and resolved.
- [ ] Site builds and its tests pass; CI green; PR merged by you.
- [ ] Plan and README updated in the same PR.
