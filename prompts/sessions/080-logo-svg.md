# Session 080: Logo concepts drawn as SVG, on one review page

Model: opus
Effort: high (design quality is the whole task)
Subagents: sonnet for drafts and checks

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\080-logo-svg`** (branch `080-logo-svg`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English. **Tier R0** (docs and SVG files only; do not change the app's or the site's icons yet).

**Other sessions at the same time:** 076 (website simulator). Your area: `docs/brand/` only, plus your rows in `plan.md` and `README.md` (task **T-521**; decision **D-094** only if needed). Every changed text file LF. `gh` is not available: push the branch and message the hub (`tavern-ledger-2b`, `ListAgents`, use its `[ref]`) with the branch, PR title and a short PR body.

## Why

Read `docs/brand/logo-brief.md` and `logo-prompts.md` first (session 079, D-093). The user tried the prompts in Gemini; what came back did not convince them. The hub's notes on those grids:

- **Rejected:** a book with a flame cut-out ("generic stock icon, reads as burning books"); a book, flame and glass mixed ("reads as the letters FN"); podiums, crowns, ladders, scales, dice, shields (generic); a lantern with "TL" letters inside, in nine variants ("two things fighting: a container with a label; the hanging ring reads as a keychain or an ornament; western serif letters say saloon, not fantasy tavern; mush at 16 px"). No mascots (the user's call).
- **The one that worked best:** a diamond-shaped hanging sign whose own edges form a **T and an L**: the shape *is* the idea, not a container with letters inside. Also liked: a lantern with an open book as its glowing glass; a quill crossed with a sword ("the pen and the sword": ledger and combat).

## Task

1. **Draw 12 original concepts by hand as SVG** (`docs/brand/concepts/NN-slug.svg`, `viewBox="0 0 64 64"`, a handful of paths, no raster, no text inside the mark, no external fonts). One idea per mark, the shape carrying the meaning; tavern, ledger, light, combat or climbing, in our palette (`#ffb24a`, `#ffd08a`, `#2a1600` on `#120d0b`). Explore widely: geometric monograms where the strokes form T and L (several takes on the diamond idea), negative space, a lantern-and-book fusion, the pen and the sword, an abstract flame-as-bar-chart, a page corner, and others of your own. Hold to the brief's rules (rule 5: nothing that looks like Blizzard's or Hearthstone's logos, the coin, gems, card frames, the tier star; nothing official-looking).
2. **Pixel discipline:** each mark must read at **16 px** — test it by rendering (e.g. a headless browser or `resvg` if available; no new repo dependency, tooling in your scratchpad only) and look at the result yourself; redraw what turns to mush. Single-colour version for each (one fill, works on dark and light).
3. **Review page** (`docs/brand/concepts/index.html`, one self-contained file, inline SVG, no scripts from outside): each concept shown at 256, 64, 32 and 16 px, on dark and on light, in single colour, and the best ones next to a "Tavern Ledger" wordmark (in the site's Bricolage Grotesque if you can embed it as a path or use the self-hosted file by relative path; else a system bold sans, said so). A one-line note per concept: idea, and its weak point. Number them so the user can answer "7 and 11". **Publish the page as an Artifact** (load the `artifact-design` skill first as the tool requires) so the user can open it on their phone, and give the hub the link.
4. Rank your top 3 with one sentence each. Plan row T-521, README line.

## Acceptance criteria

- [ ] 12 hand-made SVG concepts, each checked at 16 px and in single colour.
- [ ] One review page, published as an Artifact, link sent to the hub.
- [ ] Top 3 with reasons; branch handed to the hub.

## Red line

Rule 5: no Blizzard or Hearthstone logos, names or look-alikes; nothing official-looking. Never print BattleTags or the user's email.
