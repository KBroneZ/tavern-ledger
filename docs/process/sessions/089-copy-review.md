# Session 089: Wording review of everything public, English and Spanish (T-533)

Model: opus
Effort: high
Subagents: haiku for searches; sonnet for the review

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\089-copy-review`** (branch `089-copy-review`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English. **Tier R0/R1**: self-review + `/code-review` (strings tests must stay green).

**Other sessions at the same time:** 086 (combat replay: `crates/bg-parser/`, `crates/tracker/`, `crates/desktop/`) and 087 (Dark Gifts: `crates/combat-sim/`, `crates/cards/`, `crates/sim-wasm/`, the wasm in `web/src/lib/sim/`). **Your area:** the website's text: `web/src/lib/*/strings.ts` and `strings.es.ts`, page copy in `web/src/pages/` and `web/src/components/`, the legal pages only for plain-language fixes that do not change meaning (`docs/legal/*`; never change what a policy promises). **Not now:** the home page hero (it is being redesigned, T-530: leave its copy, list your suggestions in the doc) and the desktop app's strings (`crates/desktop/ui/`, 086 is there): review them too, but write the app changes as a list in your doc for the next app session (T-531), do not edit them. Shared files: only your rows and notes; decision number **D-102**. Every changed text file LF. To catch up: `git fetch origin && git merge origin/main`; no rebase, never force-push.

- The user may be busy: do not wait on them. Push the branch and message the hub (`tavern-ledger-2b`, `ListAgents`, use its `[ref]`) with the branch, PR title, a short PR body and the name proposals below.

## Why

The user (2026-10-12), on the stats page titled "Tus estadísticas curiosas" with "Volver a tu panel · Estadísticas curiosas" above it: "'Curious stats' sounds weird; review the phrasing of everything public so there are no sentences that sound stupid like these." Players read this, in English and in Spain's Spanish. The Spanish must sound like a Spanish player wrote it, not like a translation; the English like a native player wrote it. Use the game's own esES terms (D-086): Campos de Batalla, Dúos, taberna, esbirro, abalorio, Don siniestro.

## Task

1. **Rename "Curious stats" / "Estadísticas curiosas"** everywhere on the site (title, menu, breadcrumbs, links, page address only if a redirect from the old one is kept). Propose three names in each language with a recommendation (for example "Records" / "Récords", "Fun facts" / "Datos de tus partidas", "Highlights" / "Lo más destacado"), pick the recommended one in the branch, and list the others in the message so the user can switch.
2. **Read every public string and page**, in both languages: what sounds awkward, stiff, literal, robotic, cutesy, too long, repeated, or unclear; titles that repeat the breadcrumb; buttons that do not say what they do; empty states and errors. Fix them. Keep facts and numbers exact (never invent features or figures, rule 8), keep the fan notice exact.
3. **`docs/brand/voice.md`:** a short voice guide (tone, words we use and avoid, examples before and after, the esES glossary) for later sessions.
4. **App strings:** the same review of `crates/desktop/ui/` (English and Spanish), as a table of current → proposed in `docs/brand/voice.md` or a sibling file, for T-531 to apply. The app's "Curious stats" tab gets the same new name.
5. Plan row T-533, D-102 (the new name and the voice guide), README.

## Acceptance criteria

- [ ] New name everywhere on the site; old address redirects if it changed.
- [ ] Every public page read and fixed in EN and ES; the strings tests (missing keys, placeholders, untranslated) green.
- [ ] Voice guide; app strings table for T-531.
- [ ] Web build and tests from a clean clone of the pushed branch (paste counts); `/code-review` with no open CRITICAL or HIGH; branch handed to the hub.

## Red line

Rule 5 (no Blizzard names in the brand; "Hearthstone" only to say what the app is for), rule 8 (no made-up features, figures or testimonials). Never print BattleTags, `GameAccountId`, keys or the user's email.
