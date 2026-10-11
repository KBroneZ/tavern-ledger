# Session 081: The TL tankard logo as SVG, and the icons from it

Model: opus
Effort: high (faithful drawing is the whole task)
Subagents: sonnet for checks

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\081-logo-final`** (branch `081-logo-final`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English. **Tier R1**: self-review + `/code-review` for the site and app changes.

**No other sessions running.** Your area: `docs/brand/`, `web/public/` (icons), `web/src/layouts/Base.astro` (icon links only), `crates/desktop/icons/` and the icon list in `crates/desktop/tauri.conf.json` if it needs one; plus your rows in `plan.md`, `README.md`, `DECISIONS.md` (**D-094**, task **T-522**). Every changed text file LF. `gh` is not available: push the branch and message the hub (`tavern-ledger-2b`, `ListAgents`, use its `[ref]`) with the branch, PR title and a short PR body. Do not build or run the user's desktop app (the hub rebuilds it); never show a test build's overlay.

## The user's choice (2026-10-11)

After sessions 079 and 080, the user picked a mark they made in Gemini and said it beats every concept so far: **a "TL" monogram that forms a tankard**. The T is the rim and the left of the body, the L overlaps it and its foot is the base, a handle on the right; chunky slab-serif letters like old wood type, two amber tones, on smoky black. References (raster, AI-made, reference only): `docs/brand/reference/gemini-original.png` (the first one), `gemini-isolated.png` (the same alone, large: **the shape to follow**), `gemini-refinements.png` (a 3x3 grid of small changes, numbered left to right, top to bottom).

The hub's proposal, accepted by the user:
- **Main mark = refinement 2** (the isolated design with slightly thicker strokes).
- **Large uses = refinement 3** (the same with a small head of foam on top of the T): site masthead at large size, social image.
- **App icon = refinement 6** (the mark in dark on a rounded amber tile).

## Task

1. **Redraw by hand as SVG, faithful to the reference shapes** (the user rejected geometric reinterpretations: keep the slab serifs, the T/L overlap with its thin dark gap, the slight taper of the body, the handle's shape, the two tones: T in `#ffb24a`, L and handle in `#ffd08a`; dark details `#120d0b`). Trace the isolated reference closely (place it under your drawing in a headless browser and compare side by side at the same size; iterate until the silhouette matches), then thicken strokes as in refinement 2. Clean geometry: few nodes, straight lines straight, symmetric serifs. Files in `docs/brand/logo/`: `mark.svg` (two-tone, for dark), `mark-mono.svg` (`currentColor`), `mark-foam.svg`, `app-tile.svg` (rounded amber square, mark in `#120d0b`), `lockup.svg` (mark + "Tavern Ledger" in Bricolage Grotesque converted to paths, so it needs no font).
2. **Check small sizes** by rendering at 16, 24, 32, 48 px; if the gap between T and L or the serifs turn to mush at 16-32 px, make a simplified **small-size variant** (`mark-small.svg`: fewer serifs, wider gaps) and use it for the favicon. Look at the renders yourself.
3. **Put it in use:**
   - Site: replace `web/public/favicon.svg` (with a dark-scheme-safe version), add `apple-touch-icon.png` (180) and a 32 px PNG fallback if the site's tests allow; the masthead's lamp square becomes the mark if that is a small, contained change (else leave the masthead and say so). No CSP change.
   - App: regenerate `crates/desktop/icons/icon.ico` (16 to 256) from `app-tile.svg`, plus the PNG sizes Tauri lists, with a script in `tools/` that rebuilds them (no new dependency in the repo; tooling in your scratchpad is fine if the script documents it, or use an existing tool already on the PC).
4. **Review page** (`docs/brand/logo/index.html`, self-contained): the reference beside your drawing at the same size, every file at 256/64/32/16 on dark and light, the lockup, the app tile as a taskbar would show it. **Publish it as an Artifact** (load the `artifact-design` skill first) and send the hub the link.
5. D-094 (the logo, its files, its sizes, what is AI-made reference and what is hand-drawn), T-522, README; `docs/brand/logo-brief.md` gets a line saying which concept won. Rule 5 check: nothing Blizzard-like (it is a tankard and letters; fine).

## Acceptance criteria

- [ ] SVGs that match the reference's silhouette (side-by-side shown), clean, two-tone and mono.
- [ ] Readable at 16 and 32 px (render shown), with a small variant if needed.
- [ ] Favicon and app icon replaced; site build tests and `cargo test --locked --workspace --exclude desktop` green from a clean clone (paste counts).
- [ ] Review page published, link sent; branch handed to the hub.

## Red line

Rule 5: no Blizzard or Hearthstone logos, names or look-alikes. Never print BattleTags or the user's email.
