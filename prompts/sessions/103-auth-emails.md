# Session 103: Auth emails with the logo, in English and Spanish (T-542)

Model: opus
Effort: high
Subagents: haiku for searches; sonnet for code and the review

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\103-auth-emails`** (branch `103-auth-emails`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English (the Spanish email copy is product copy). **Tier R2** (auth flow, user email): TDD where there is logic, `/code-review` and `/security-review`.

**Other sessions at the same time:** none planned in your area. Your area: `supabase/templates/` (new), `supabase/config.toml` (`[auth.email.template.*]` for the local stack), the sign-up path only if it must store the language (`web/src/lib/account/`), `docs/research/deploy.md` (section 10.3 or a new 10.14 for the templates), `docs/brand/` if a mail-safe logo file is needed, `web/public/` for the hosted logo image. Decision **D-114** (amends D-089 point 5, "Supabase auth emails stay in English"). Every changed text file LF. To catch up: `git fetch origin && git merge origin/main`; no rebase, never force-push.

- The user may be busy: do not wait on them. Push the branch and message the hub (`tavern-ledger-c7`, `ListAgents`, use its `[ref]`) with the branch, PR title, a short PR body, screenshots and the exact dashboard steps.
- **You never change the hosted dashboard.** The hub pastes the templates into Supabase (Authentication → Email Templates) with the user's OK, then sends a test email to the user.

## Why (the user, 2026-10-12)

"Can we edit how the verification emails arrive so they look better, with our logos and so on?" Today the hosted emails (sign-in link, sign-up confirmation, password reset, email change) are bare default HTML in English with no logo (checked by the hub in the user's inbox). Sender: Lettermint, `noreply@tavernledger.net`, tracking off (D-041, README "Auth emails").

## Task

1. **Templates** for every auth email the project sends (confirm sign-up, magic link / sign-in, recovery, email change; invite and reauthentication only if enabled), in the D-105 tavern look adapted to email: the TL logo as an image hosted on `https://tavernledger.net/` (PNG, explicit size, alt text; never a data URI or remote tracking pixel), dark tavern colours with a light-safe fallback, one clear button plus the plain link below it, short copy in the voice guide (`docs/brand/voice.md`), the line "Unofficial fan project. Not affiliated with or endorsed by Blizzard Entertainment." and what to do if you did not ask for it. Table layout and inline CSS that render in Gmail (web, Android), Outlook (desktop and web) and Apple Mail; dark mode readable; no external CSS, fonts or scripts.
2. **English and Spanish:** pick the language from what Supabase's template data allows (Go template conditionals on the user's metadata, e.g. a `lang` value written at sign-up from the site's language). If the site does not store it yet, add it at sign-up and in the desktop sign-in path where an email is triggered, as metadata only (no new personal data; update the data inventory if it counts). Fallback English. Subjects in both languages if Supabase allows per-language subjects; otherwise a neutral bilingual subject — say which.
3. **Never break the link:** keep Supabase's own variables exactly (`{{ .ConfirmationURL }}` or the token-hash form the site uses since D-048; check `web/src/lib/account/` and D-048 before choosing), no link rewriting, no extra query parameters.
4. **Local proof:** wire the templates in `supabase/config.toml`, run the local stack, trigger each email and capture the result from the local mail catcher (Inbucket/Mailpit) as screenshots, EN and ES, light and dark. Tests that each template contains the exact Supabase variable and the disclaimer, and has no `http://` or remote script.
5. **Dashboard steps for the hub:** per template, the subject and the file to paste, in order, and how to roll back (Supabase's default text kept in the doc). Then plan row T-542, README, D-114.

## Acceptance criteria

- [ ] All sent auth emails templated, EN and ES, logo, disclaimer, link intact; screenshots from the local mail catcher.
- [ ] Language stored without new personal data; English fallback; tests green from a clean clone (paste counts).
- [ ] `/code-review` and `/security-review` with no open CRITICAL or HIGH; branch, screenshots and dashboard steps handed to the hub.

## Red line

Rules 5, 6 and 7. Never print BattleTags, keys or the user's email; no dashboard, DNS or sender change by the session.
