# Session 092: The home page in the tavern look, with a preview (T-530)

Model: opus
Effort: high
Subagents: haiku for searches; sonnet for code and the review

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\092-home-tavern`** (branch `092-home-tavern`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English. **Tier R1**: TDD where there is logic + `/code-review`.

**Other sessions at the same time:** the app session 093 (`crates/desktop/`) and the Dark Gifts parser session 094 (`crates/bg-parser/`, `crates/tracker/`, the web game viewer). Your area: the home page (`web/src/pages/[...lang]/index.astro`, `web/src/styles/home.css`, `scene.css`, new components and images for it), the shared button component and its styles, and `web/public/` images. Decision **D-105**. Every changed text file LF. To catch up: `git fetch origin && git merge origin/main`; no rebase, never force-push.

- The user may be busy: do not wait on them. Push the branch, **then give the hub the Cloudflare Pages preview address of your branch** (`gh` is not available to you: the hub opens a draft PR, which makes Cloudflare build a preview; or find the branch preview address pattern in `docs/deploy.md`). The user decides from that preview; do not ask for a merge before they have seen it.

## Why and what the user chose (2026-10-12, through the hub)

T-525's hand-drawn concepts were rejected ("nothing like the references"). The user then chose painted art made in Gemini, now in **`docs/brand/art/home/`**:

- **`tavern-tall.webp`** (1344×3168): one continuous painting from top to bottom: the big barrel door with golden sparks (left) and a blue magical glow (right), the warm tavern with fireplace and bar in the middle, stone stairs down to a cellar with blue light at the bottom.
- **`buttons-sheet.webp`**: four buttons on a dark tavern backdrop: gold stone with blue and orange gems, glowing (hover) and plain (rest); dark wood with bronze frame and rivets; small stone with a thin gold frame.
- **`barrel-wide.webp`**: the barrel door scene, wide (for other pages later).

No Blizzard logo, swirl, card back or game asset anywhere (rule 5; the blue magic at the top right is magic, not the logo; if it reads as the swirl at any size, soften it with a gradient over it).

## Task

1. **Background that scrolls with the page:** the tall painting stays fixed behind the whole page and moves as you scroll, so you feel you are going down into the tavern; never a picture at the top and an empty page below. **During the pinned five-step scene (D-052, the "slider"; keep it exactly as it works today)** the background slows by 90 %, just enough to feel the descent; outside it, normal speed, from the barrel door at the top to the cellar at the bottom of the page. A darker centre for reading is done in CSS, not in the art. A light layer of rising sparks (gold left, blue right) in front, cheap (one canvas or CSS), paused off screen. Phones: the centre of the painting, scaled to cover. `prefers-reduced-motion`: the background stays still and the sparks are off. Serve responsive sizes (AVIF/WebP, a small one for phones), with explicit dimensions; LCP stays under 2.5 s on a mid phone (measure).
2. **Buttons by use** (cut cleanly from the sheet, transparent, @1x and @2x, or rebuilt in SVG/CSS if the cut is not clean at every size; text is always real HTML on top):
   - **Gold stone with gems:** only for the few key actions (for now: Download on the home page). Rest = plain; hover and focus = the glowing one, cross-faded.
   - **Wood and bronze:** common actions ("Is it safe?", "Show me"), with a short knock/press animation when used.
   - **Small stone:** small actions, with worn, more detailed edges and its own small animation.
   - Keyboard focus visible, touch sizes ok, reduced motion respected. One component used by the home page; other pages adopt it later.
3. **Keep the home page's content and the five-step scene**, restyled to sit on the painting (parchment panel for the lead, gold headline as in the reference). Copy: use whatever main has (session 089 is reviewing wording; merge main before you hand over).
4. Headless screenshots at 400, 1024 and 1440 px, EN and ES, at the top, inside the pinned scene and at the bottom, in the message to the hub; Lighthouse numbers for the home page before and after.
5. D-105 (the look, the scroll rule, the button roles), plan row T-530, PROVENANCE for the Gemini art (made by the user's Gemini account for this project), README.

## Acceptance criteria

- [ ] Background scrolls behind the whole page; 90 % slower during the pinned scene; still with reduced motion.
- [ ] The three buttons by role, clean at every size, accessible.
- [ ] The pinned five-step scene unchanged in behaviour.
- [ ] Web build and tests from a clean clone of the pushed branch (paste counts); performance measured; `/code-review` with no open CRITICAL or HIGH; branch and preview handed to the hub.

## Red line

Rule 5 (brand), rule 8 (no made-up figures: any number on the page is real or marked as an example). Never print BattleTags, `GameAccountId`, keys or the user's email.
