# Session 073: The website in Spanish (T-508)

Model: opus
Effort: medium
Subagents: haiku for searches; sonnet for code, the strings and the reviews (`code-reviewer`, `a11y-architect`)

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\073-web-spanish`** (branch `073-web-spanish`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English except the Spanish strings. **Tier R1**: TDD + `/code-review`.

**Other sessions:** the hub starts you when no other session works in `web/`, because you touch every page's strings. If one starts later, the hub tells you which files are theirs. App sessions (`crates/`) may run at the same time. Your area: `web/` (strings, layouts, a language switch, the pages' text), plus docs. No change to `supabase/`, the CSP, dependencies (an i18n library needs the user's OK; prefer none).
- Shared files (`plan.md`, `README.md`, `DECISIONS.md`): only your rows and notes; decision number if needed **D-086**. Every changed text file LF (`git ls-files --eol`, index side; the checkout uses `core.autocrlf=true`).
- To catch up: `git fetch origin && git merge origin/main`. No rebase, never force-push. Stop any dev server you start.

Working rules: decide technical and wording matters yourself. The user is a native Spanish speaker from Spain: use Spain's Spanish, informal "tú", the game's own Spanish names for game terms (esES: "Taberna", "Campos de Batalla", hero and card names from HearthstoneJSON esES as in the app, D-066). The user is busy: do not wait on them; questions go in your hand-over. `gh` is not available: push the branch and message the hub (`tavern-ledger-2b`, `ListAgents`, use its `[ref]`) with the branch, PR title and a short PR body. **Merging deploys the site:** afterwards check every page answers 200 in both languages and the CSP is unchanged; tell the hub.

## Why

Round 5 (read **`docs/plan/web-master-plan.md`**): "English first, Spanish next (the app's i18n approach)". The app did it in session 056 (D-064 and the app's `ui/strings/`); read how. The website now has: home, sign-in, account, forgot password, `/me/`, profile, `/games/`, `/game/`, `/player-heroes/`, `/curious/`, `/leaderboards/`, `/heroes/` (if T-507 merged), privacy, terms, 404, and the strings in `web/src/lib/*/strings.ts`.

## Task

1. **How languages work on a static site:** choose and record (D-086): for example `/es/…` pages built from the same components with a `lang` prop, `hreflang` links, a switch in the header that keeps the current page and query, the choice remembered per viewer, and the browser's language only as the first default. No layout shift; the CSP unchanged (no inline script).
2. **Every string** in English and Spanish: pages, the data pages' pieces and client renderers (`web/src/lib/view/`), states, folds, "how this is counted" texts, charts' text alternatives, form messages, errors, emails' links if any. Numbers and dates formatted per language (`Intl`), place ordinals, plural rules.
3. **Legal pages:** a Spanish version of the privacy policy and terms, marked "translation; the English version prevails" (or the reverse if you find a reason; say which) and dated the same as the English.
4. **Tests:** a test that every key exists in both languages (no missing or empty string), unit tests for the formatting, build tests for both languages in every build mode, `lang` attributes and `hreflang` right, axe clean in Spanish on the main pages.
5. Plan row **T-508** done with notes; README.

## Acceptance criteria

- [ ] Every page in English and Spanish with a switch; numbers and dates per language; legal pages translated and marked.
- [ ] Key-parity test; build tests green in both languages (paste counts); axe clean.
- [ ] `/code-review` with no open CRITICAL or HIGH; branch handed to the hub; post-merge checks reported.

## Red line

No CSP change, no new dependency without the user's OK, no real player data in fixtures; never print keys, tokens, the user's email or BattleTags.
