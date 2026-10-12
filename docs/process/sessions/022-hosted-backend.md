# Session 022: Hosted Supabase backend, open accounts (T-104a hosted, T-104c accounts)

Model: opus (production database, auth, user data, secrets)
Effort: high
Subagents: haiku for doc lookups; sonnet for code; `security-reviewer` and `database-reviewer` before the PR

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\022-hosted-backend`** (branch `022-hosted-backend`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English. **Tier R3** (production, secrets, user data): `/code-review` + `/security-review` + a second AI (`ask-chatgpt` review mode) on the plan before you change the hosted project, and the user's OK at every step marked **[user]**.

**Starts after #016 (desktop upload) has merged**: it changed `supabase/` (upload Edge Function, quotas, migrations). Read what it left in the plan and `DECISIONS.md` (D-025). If another session runs at the same time, the hub tells you in the launch message.
- Work only in your worktree. Never touch the main checkout or other worktrees.
- Your area: `supabase/` (config for the hosted project, deploy scripts, docs), `web/` (only the build variables and what changes when accounts open), `docs/legal/` (Supabase notes), `docs/research/deploy.md`, the Supabase and Cloudflare dashboards. Nothing in `crates/` beyond a config default if the desktop needs the hosted URL.
- Shared files (`plan.md`, `README.md`, `DECISIONS.md`, `PROVENANCE.md`): only your task's rows and notes. Your decision number, if needed, is **D-031**.
- To catch up with `main`: `git fetch origin && git merge origin/main`. No rebase, never force-push.
- Never start Docker or the local Supabase stack unless no other session uses it; never stop or restart the desktop app.

Working rules (user's decisions, 2026-10-09):
- Decide technical matters yourself. Ask only what is the user's call: accounts, accepting terms, payments, granting access between services, making something public, new dependencies, opening sign-ups to the public.
- **You merge your own PRs, always**, once CI is green and no review has open CRITICAL or HIGH findings: `gh pr merge <n> --squash --delete-branch`.
- The user often works remotely without access to the PC: never ask them to do something on the PC. Things they can do from their phone are fine; say exactly what to tap.
- `gh` is already logged in through `GH_TOKEN` in the environment; never print it.
- The Supabase and Cloudflare dashboards are open to you through Claude in Chrome (the user is signed in to both). Load the `anthropic-skills:chrome-browser` skill first. Never type passwords, card details or tokens; never print keys other than the public anon/publishable key; the service-role key and the database password never leave the dashboard or a secret store.

**Red line (D-004, D-012, D-021, D-027):** clients only read; rows and files are written only by the Edge Function with the service role. No BattleTags or `GameAccountId` anywhere. No analytics. Sign-ups stay closed until the user says so.

## State you start from

- Hosted project created by the hub with the user on 2026-10-09: org **Tavern Ledger** (Free plan), project **`tavern-ledger`**, ref `vgttflmexobrqhcyxjks`, region **Central EU (Frankfurt) eu-central-1**. Data API on, "automatically expose new tables" **off** (the migrations grant explicitly), automatic RLS **on**. No migrations applied, no GitHub connection.
- **Supabase DPA:** part of Supabase's Terms of Service for every organization; no separate signature needed (Organization settings → Legal Documents, checked 2026-10-09). A Transfer Impact Assessment is downloadable there. Record this in `docs/legal/` and close that pending item.
- The site is (or is about to be) live on `tavernledger.net` from #021, with accounts closed because the backend variables are not set.
- Mail: inbound via Cloudflare Email Routing (#021). The outbound sender for auth emails is the user's choice from #021's comparison (Cloudflare Email Sending or Brevo); if it is still open, ask.

## Effort and models per phase

| Phase | Effort | Subagents (if needed) |
|-------|--------|-----------------------|
| Read and check state | low | Haiku |
| Plan (how migrations and functions reach the hosted project, secrets, auth settings) + second AI review | high | `ask-chatgpt` |
| Apply to the hosted project, one step at a time | high | — |
| Review | high | `security-reviewer`, `database-reviewer` |

## At the start

1. Conversation title: **`#022 Hosted backend`**.
2. `git fetch origin && git merge origin/main`.
3. Read `README.md`, `docs/plan/plan.md` (T-104a to T-104d), `docs/decisions/DECISIONS.md` (D-020, D-021, D-023, D-025, D-027, D-030), `docs/research/web-stack.md`, `docs/research/deploy.md`, `docs/legal/`, `supabase/` and `web/`.
4. Green before touching anything: pgTAP (local stack, if free), `web/` tests and build, Python, Rust workspace.

## Task

1. **Plan.** In `docs/research/deploy.md`, a "Hosted backend" section: how migrations and Edge Functions reach the hosted project (Supabase GitHub integration on `main`, or CLI from CI with a token in GitHub secrets, or CLI from this PC; pick one and say why), where each secret lives, auth settings (site URL `https://tavernledger.net`, redirect URLs, email confirmation on, password rules, rate limits, leaked-password check if the plan allows), custom SMTP, sign-ups closed until opened. Second opinion with `ask-chatgpt`; fix what holds up.
2. **[user] Apply.** One message to the user listing what needs them (a GitHub authorization, an access token they create and paste into a secret store, the sender account), then apply step by step and check each: migrations applied and matching the local schema, RLS on every table, grants as in the migrations, Edge Functions deployed, storage buckets and policies.
3. **Auth emails.** Custom SMTP with the chosen sender, on `tavernledger.net`; SPF, DKIM and DMARC records in Cloudflare that stay valid alongside Email Routing's. Test with a sign-up to the user's own address only.
4. **Site.** Set `PUBLIC_SUPABASE_URL` and the public key in the Pages build (production only), redeploy, check sign-in, account, export and deletion against the hosted project with a test account of yours that you delete afterwards.
5. **[user] Open accounts.** Ask the user before allowing public sign-ups. Until then, leave sign-ups closed on the hosted project and the site's message in place.
6. Update `docs/legal/` (Supabase DPA and TIA, sender, sub-processors), plan (T-104a hosted, T-104c, T-104b), README and D-031, in the same PR.

## Out of scope

- Paid plans, analytics, desktop upload changes (only a config default for the hosted URL if needed), community stats.

## Acceptance criteria

- [ ] Deploy plan written and reviewed by a second AI before any change to the hosted project.
- [ ] Hosted schema matches the migrations; RLS and grants checked; functions deployed; no secret in the repo, logs or chat.
- [ ] Auth emails sent from `tavernledger.net` with SPF, DKIM and DMARC passing.
- [ ] Site works against the hosted project (sign-in, account, export, deletion) with a test account that is deleted afterwards.
- [ ] Sign-ups open only with the user's OK (or still closed, recorded).
- [ ] `/code-review` and `/security-review` with no open CRITICAL or HIGH; CI green; PR merged by you.
- [ ] Plan, README, `docs/legal/` and D-031 updated in the same PR.
