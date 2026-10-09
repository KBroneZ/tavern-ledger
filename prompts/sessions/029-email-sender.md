# Session 029: Auth email sender without link tracking (T-104f)

Model: opus
Effort: high
Subagents: haiku for searches; sonnet for reviews

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\029-email-sender`** (branch `029-email-sender`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English. **Tier R2** (production auth email, DNS, a new provider, user data): `/code-review` + `/security-review`; anything that costs money is R3 (the user's OK first).

**Other sessions may run at the same time** (026 card images, 028 website). To stay out of their way:
- Work only in your worktree. Never touch the main checkout or other worktrees.
- Your area: the email sender (Supabase SMTP settings, DNS records for mail on `tavernledger.net`), `docs/research/deploy.md` section 10, `docs/legal/` (data inventory, privacy draft) and `supabase/` only if the Send Email Hook is chosen. Do not touch `crates/` or `web/` (028 restyles the site; it does not change the legal texts).
- Shared files (`plan.md`, `README.md`, `DECISIONS.md`, `PROVENANCE.md`): only your task's row and notes. Your decision number is **D-041**.
- To catch up with `main`: `git fetch origin && git merge origin/main`. No rebase, never force-push. Merging to `main` deploys the site and the Supabase migrations and functions (D-031, D-036): check after each merge.

Working rules (user's decisions):
- Decide technical matters yourself. Ask the user: the provider, creating accounts, accepting terms, paying, granting access between services.
- **You merge your own PRs**, once CI is green and no review has open CRITICAL or HIGH findings: `gh pr merge <n> --squash --delete-branch`.
- Never create accounts, accept terms or type passwords or keys yourself: the user does it; secrets (SMTP keys) go from the provider's dashboard straight into Supabase's, never through chat, logs or files.
- `gh` is logged in through `GH_TOKEN`; never print it.

## Why (D-037)

The end-to-end test on 2026-10-10 showed that Brevo (D-034) rewrites every link through its click tracker, the one-time sign-in link included, and adds a `List-Unsubscribe` header. Brevo has no self-serve switch for that. Opening sign-ups is blocked until the sender is replaced. Details: `docs/research/deploy.md` 10.4b and 10.9 items 4 to 10; DMARC is `p=reject; adkim=s; aspf=s`.

## Effort and models per phase

| Phase | Effort | Model |
|-------|--------|-------|
| Read and check state | low | Haiku |
| Compare senders (primary pages) | high | Opus, Haiku for search |
| Set up with the user | medium | Opus |
| Docs, privacy, review | medium | `security-reviewer` |

## At the start

1. Conversation title: **`#029 Email sender`**.
2. `git fetch origin && git merge origin/main`.
3. Read `docs/research/deploy.md` section 10 (all of it), `docs/decisions/DECISIONS.md` (D-027, D-030, D-031, D-034 to D-037), `docs/legal/data-inventory.md` and `docs/legal/privacy.md`.

## Task

1. **Compare** at least Resend, Mailjet, Amazon SES and Postmark, plus Cloudflare Email Sending (Workers Paid, about $5 a month, beta), on their own pages: can link and open tracking be off (by default, or by an account setting reachable over plain SMTP, since Supabase cannot add headers), does it add `List-Unsubscribe` to transactional mail, free tier and limits, company and data location, DPA and transfers, SMTP host and ports Supabase supports, DKIM alignment for `tavernledger.net`. Also note the Send Email Hook as a fallback (our own function calling a provider's API), with its cost in code and review. Write it into `docs/research/deploy.md` (new section 10.11).
2. **Ask the user** with the comparison and your recommendation. Record the choice as D-041.
3. **Set it up** with the user: account and terms (user), domain records in Cloudflare DNS (you may add them through the browser if the user allows; DNS only, never proxied; keep the single apex SPF record and DMARC as is unless alignment needs a change, and say why), SMTP key straight into Supabase (user), sender `noreply@tavernledger.net`. Turn off every kind of tracking the provider offers and keep its logs as short as allowed.
4. **End-to-end test** with the user's own account (the user creates it in the dashboard; you never do): a magic link arrives, headers show SPF/DKIM/DMARC pass, **the link points straight at `https://vgttflmexobrqhcyxjks.supabase.co`**, no `List-Unsubscribe` if avoidable; sign in on the site, **download the export** (not tried last time), delete the account, check it is gone.
5. **Brevo**: once the new sender works, remove Brevo's DNS records (TXT code, `brevo1`/`brevo2._domainkey`) and ask the user whether to close the Brevo account (the user does it).
6. Data inventory and privacy draft updated for the new provider (and Brevo removed), plan (T-104f row: "Auth email sender without link tracking"), README, D-041, in the same PR. Opening sign-ups stays the user's decision afterwards.

## Acceptance criteria

- [ ] Comparison from primary pages; the user chose; D-041 recorded.
- [ ] Sign-in links arrive untouched (straight to Supabase), DMARC pass, tracking off.
- [ ] End-to-end test passed, export included; account gone afterwards.
- [ ] Brevo records removed; privacy draft and data inventory match the new provider.
- [ ] `/code-review` and `/security-review` with no open CRITICAL or HIGH; CI green; PR merged by you.
