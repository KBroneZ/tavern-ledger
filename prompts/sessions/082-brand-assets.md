# Session 082: Brand assets from the logo: social card, tray icon, avatar, installer images (T-523)

Model: opus
Effort: medium
Subagents: haiku for searches; sonnet for code and the review

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\082-brand-assets`** (branch `082-brand-assets`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English. **Tier R1**: TDD where there is code + `/code-review`.

**Other sessions at the same time:** 083 (simulator cards, `crates/combat-sim/`, `crates/cards/`) and 084 (sweep counts, `supabase/`). Your area: `docs/brand/`, `web/` (head tags and public images only), `crates/desktop/icons/` and the tray icon setup in `crates/desktop/src/` (only the lines that pick the tray image), `tools/build_icons.py`. Shared files: only your rows and notes; decision number if needed **D-095** (084 may take D-096). Every changed text file LF (`git ls-files --eol`, index side). To catch up: `git fetch origin && git merge origin/main`; no rebase, never force-push.

- Do not build or run the user's desktop app (the hub does). Never show a test build over the user's game: check `Hearthstone.exe` is not running before any visual test, prefer headless renders.
- The user may be busy: do not wait on them. Push the branch and message the hub (`tavern-ledger-2b`, `ListAgents`, use its `[ref]`) with the branch, PR title and a short PR body.

## Why

The TL tankard logo is in (D-094, #143): `docs/brand/logo/` (mark, mono, foam, small, app tile, lockup), the website favicon and header mark, the app's `icon.ico`. The follow-ups: the picture shown when someone shares a link, a sharp tray icon, a GitHub avatar and the installer images. Read D-093, D-094 and `docs/brand/logo/index.html` first. Rule 5: no Blizzard logos, names or look-alikes.

## Task

1. **Social card** 1200×630 PNG (the foam mark or the lockup on the brand's dark ground, the line "Hearthstone Battlegrounds tracker", and the fan-project notice in small type if it fits cleanly). Built from the SVGs by a committed script. Add `og:image`, `og:image:width`/`height`/`alt`, `twitter:card=summary_large_image` and `twitter:image` to the site layout, absolute URLs on `https://tavernledger.net`, on English and Spanish pages (alt text in each language). Check the site builds and the tags are in the HTML; say if the CSP needs any change (same-origin images should not).
2. **Tray icon:** a dedicated small-size image that reads at 16 and 20 px on light and dark taskbars (`mark-small.svg` or a cleaner variant); wire it as the tray image. Show renders at 16, 20, 24 and 32 px side by side on both backgrounds.
3. **GitHub avatar:** a 500×500 PNG of the app tile in `docs/brand/logo/`. Do not upload it; the user changes the avatar themselves (say where the file is).
4. **Installer images** for T-105 (Tauri's NSIS bundler): header 150×57 and sidebar 164×314 BMP as Tauri expects, plus their build script. Point the bundler config at them only if that is harmless; T-105 stays pending.
5. Add all of it to the review page `docs/brand/logo/index.html`; publish it again as an artifact if you can (`Artifact` tool) and put the link in the message to the hub.
6. Plan row T-523, PROVENANCE if a new tool or font is used, README.

## Acceptance criteria

- [ ] Social card PNG and meta tags on EN and ES pages; built site checked.
- [ ] Tray icon wired; renders at four sizes on light and dark.
- [ ] Avatar PNG and installer BMPs committed with their build script.
- [ ] Web build and `cargo test --locked --workspace --exclude desktop` from a clean clone of the pushed branch (paste counts); `cargo check -p desktop`; `/code-review` with no open CRITICAL or HIGH; branch handed to the hub.

## Red line

Rule 5 (brand), D-004 (as amended by D-063), D-006. Never print BattleTags, `GameAccountId`, keys or the user's email. No account or dashboard changes.
