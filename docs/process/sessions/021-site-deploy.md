# Session 021: Website on tavernledger.net, contact mail (T-104c deploy, part 1)

Model: opus (production, DNS and the user's accounts)
Effort: high for anything that changes Cloudflare; medium for the rest
Subagents: haiku for doc lookups; sonnet for code; `security-reviewer` before the PR

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\021-site-deploy`** (branch `021-site-deploy`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English. **Tier R3** (production and the user's accounts): `/code-review` + `/security-review` + a second AI (`ask-chatgpt` review mode) on the plan before you change anything in Cloudflare, and the user's OK at every step marked **[user]**.

**Another session runs at the same time** (#016 desktop upload). To stay out of its way:
- Work only in your worktree. Never touch the main checkout or other worktrees.
- Your area: `web/` (build config and what the site shows while there is no hosted backend), `docs/legal/` (only the notes about Cloudflare, the domain and mail), a new `docs/research/deploy.md`, and the Cloudflare dashboard. Nothing in `crates/`, `supabase/` or `tools/`.
- Shared files (`plan.md`, `README.md`, `DECISIONS.md`, `PROVENANCE.md`): only your task's rows and notes. Your decision number, if needed, is **D-030**.
- To catch up with `main`: `git fetch origin && git merge origin/main`. No rebase, never force-push.
- Never start Docker or the local Supabase stack, and never stop or restart the desktop app.

Working rules (user's decisions, 2026-10-09):
- Decide technical matters yourself. Ask only what is the user's call: accounts, accepting terms, payments, granting access between services, making something public, new dependencies.
- **You merge your own PRs, always**, once CI is green and no review has open CRITICAL or HIGH findings: `gh pr merge <n> --squash --delete-branch`.
- The user often works remotely without access to the PC: never ask them to do something on the PC. Things they can do from their phone (approving a GitHub authorization, clicking a verification email, answering here) are fine; say exactly what to tap.
- `gh` is already logged in through `GH_TOKEN` in the environment; never print it.
- The Cloudflare dashboard is open to you through Claude in Chrome (the user's account is signed in). Load the `anthropic-skills:chrome-browser` skill first. Never type passwords, card details or API tokens; never create API tokens unless the user asks.

**Red line:** no Blizzard name or logo in the brand; the notice "Unofficial fan project. Not affiliated with or endorsed by Blizzard Entertainment." on every page. No analytics of any kind (D-027): leave Cloudflare Web Analytics and any beacon **off**. No secrets in the repo, the Pages settings shown in chat, or logs.

## State you start from

- Domain **`tavernledger.net`** bought by the user at Cloudflare Registrar on 2026-10-09 (renews at $11.86/yr), using Cloudflare nameservers, in the same account as the dashboard.
- The hosted Supabase project (T-104a) **does not exist yet**. The site's sign-in, account and profile pages need it.
- Mail: the user chose Brevo as the sender in #018 (D-027, pending), and later said they would like to use Cloudflare's mail features. Cloudflare Email Routing (inbound only) and **Email Sending (beta)** both show in the user's dashboard.

## Effort and models per phase

| Phase | Effort | Subagents (if needed) |
|-------|--------|-----------------------|
| Read and check state | low | Haiku |
| Plan (what goes live, how it is built, DNS and mail records) + second AI review | high | `ask-chatgpt` |
| Code (site without backend) | medium | Sonnet |
| Cloudflare changes, one by one | high | — |
| Review | high | `code-reviewer`, `security-reviewer` |

## At the start

1. Conversation title: **`#021 Site deploy`**.
2. `git fetch origin && git merge origin/main`.
3. Read `README.md`, `docs/plan/plan.md` (T-104a to T-104d and their notes), `docs/decisions/DECISIONS.md` (D-020, D-021, D-023, D-027), `docs/research/web-stack.md`, `docs/legal/` and `web/`.
4. Green before touching anything: `web/` tests and build, Python, Rust workspace.

## Task

1. **Site without backend.** When the Supabase URL and anon key are not set at build time, the site builds and shows the home, privacy and terms pages, and in place of sign-in, account and profile a plain "accounts are not open yet" message. No broken forms, no calls to a missing backend. Tests for both builds (with and without the variables).
2. **Plan and second opinion.** Write `docs/research/deploy.md`: how the site is built and deployed (Cloudflare Pages connected to the GitHub repo with root `web/`, or direct upload from CI; pick one and say why), the production branch (`main`), preview deployments (off or protected), security headers (`web/public/_headers`: CSP, HSTS, `X-Content-Type-Options`, `Referrer-Policy`, `Permissions-Policy`, frame-ancestors), `www` → apex redirect, and the DNS and mail records. Get a second opinion with `ask-chatgpt` and fix what holds up.
3. **[user] Go live.** Ask the user, in one message, before you change anything in Cloudflare: (a) OK to publish the site at `tavernledger.net` now, with accounts closed; (b) OK to authorize the Cloudflare Pages GitHub app for `KBroneZ/tavern-ledger` only (if you chose Git integration): they approve it from their phone. Then: create the Pages project, attach `tavernledger.net` and `www`, check HTTPS and headers with `curl -I`, and check that no analytics is on.
4. **[user] Contact mail.** Email Routing for `contact@tavernledger.net` and `privacy@tavernledger.net`, forwarded to the address the user gives you (they confirm the verification email from their phone). Check the MX, SPF and DMARC records it adds; a strict DMARC policy is fine since nothing sends yet.
5. **Sender research (no setup).** Compare Cloudflare Email Sending (beta) and Brevo as the SMTP sender for Supabase's auth emails: works as custom SMTP in Supabase or not, free limits, DPA and where data is processed, beta status. Recommend one in `docs/research/deploy.md` and ask the user to choose; the setup waits for the hosted Supabase project.
6. Update `docs/legal/` notes (Cloudflare and Email Routing now set up, domain, sender still open), plan (T-104c: live with accounts closed; T-104b notes), README and D-030 for the hosting and mail choices, in the same PR.

## Out of scope

- Creating the hosted Supabase project or opening accounts (T-104a, waits for the user), setting up the sender, upload (#016), analytics of any kind, paid Cloudflare plans.

## Acceptance criteria

- [ ] Site builds and works with and without the backend variables (tests); no broken forms with accounts closed.
- [ ] `docs/research/deploy.md` written and reviewed by a second AI before any Cloudflare change.
- [ ] With the user's OK: `https://tavernledger.net` and `www` serve the site over HTTPS with the security headers; no analytics; fan-project notice on every page.
- [ ] With the user's OK: `contact@` and `privacy@` forward to the user's mailbox; records checked.
- [ ] Sender comparison written; the user's choice recorded (or marked pending).
- [ ] `/code-review` and `/security-review` with no open CRITICAL or HIGH; CI green; PR merged by you.
- [ ] Plan, README, `docs/legal/` notes and D-030 updated in the same PR.
