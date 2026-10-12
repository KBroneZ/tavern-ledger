# Session 100: Simulator page in the tavern look, simpler and more visual (T-526)

Model: opus
Effort: high
Subagents: haiku for searches; sonnet for code and the review

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\100-simulator-tavern`** (branch `100-simulator-tavern`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English. **Tier R1**: TDD where there is logic + `/code-review`.

**Other sessions at the same time:** 101 (game-file retention: `supabase/`, `docs/legal/`). Your area: the simulator page (`web/src/pages/[...lang]/simulator.astro`, `web/src/components/sim/`, `web/src/lib/sim/` view code, its styles and strings), shared styles and components only to extend them (tell the hub before changing something the home page uses), `web/public/art/` for new images, `tools/build_home_art.py` or a sibling tool for cuts. Decision **D-112**. Every changed text file LF. To catch up: `git fetch origin && git merge origin/main`; no rebase, never force-push.

- The user may be busy: do not wait on them. Push the branch and message the hub (`tavern-ledger-c7`, `ListAgents`, use its `[ref]`) with the branch, PR title, a short PR body and headless screenshots. Visual change: the user decides from screenshots or a Cloudflare preview before the hub merges.
- Need new painted art? Do not draw it in SVG to pass as painting: ask the hub with a precise brief (what, size, ratio, background); the hub generates it in the user's Gemini and gives you the original file.

## Why (the user, 2026-10-12)

The home page in the painted tavern look (T-530, D-105, #163) was a hit. The simulator page (`/simulator/`; the user's screenshot shows a plain dark page, a flat "Load a combat" box whose row of games runs off the box and the page to the right, empty boards as dashed squares, a flat odds box) must get the same care: "follow the concepts, but different". Same painted world and craft, its own identity, not a copy of the home page.

## Task

1. **Look:** a painted background of its own that fits a combat table or arena corner of the tavern (`docs/brand/art/home/barrel-wide.webp` is available, or ask the hub for new art); framed panels in the D-105 family (the unused tall card of `panels-sheet.webp`, or new frames); headings and type as the home page. Different composition and atmosphere from the home page. No Blizzard logos, swirl or card backs (rule 5).
2. **Main flow obvious:** pick a game, pick a fight, see the odds. Boards drawn as **cards** (minion art in a card-like frame with attack and health, Dark Gift marks as D-107); "You" and "Opponent" facing each other as on a table; the odds (win / tie / loss) as the hero element once both sides have minions.
3. **Fix "Load a combat":** the game row never overflows the box or the page (wrap, or scroll inside the box with a designed, non-native scrollbar), at every width, EN and ES, Simple and Advanced.
4. Keep everything the simulator does today (Solo/Duos, tavern tier, editor, Advanced, the accuracy link, the engine and its worker); same results. Keyboard, focus, touch sizes, reduced motion, Windows high contrast.
5. **Footer copy check (rule 8):** the shared footer says the site is not open to the public yet (`layout.openSource`, and `signin.meta.link`), but sign-ups are open since D-089. Propose corrected EN and ES text to the hub in your message; do not change shared copy without the hub's OK.
6. Performance: mobile LCP at most 2.5 s, CLS about 0 (Lighthouse before and after). Headless screenshots at 400, 1024 and 1440 px, EN and ES, empty and with a loaded combat. Plan row T-526, README, D-112.

## Acceptance criteria

- [ ] New look of its own; flow pick game, fight, odds is clear; boards as cards; no overflow anywhere.
- [ ] Same simulator results; accessibility and reduced motion ok; performance measured.
- [ ] Web tests and build checks green from a clean clone (paste counts); `/code-review` with no open CRITICAL or HIGH; branch and screenshots handed to the hub.

## Red line

Rules 1, 5 and 8. Never print BattleTags, `GameAccountId`, player names, keys or the user's email.
