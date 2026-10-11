# Session 085: Home page concepts with a Hearthstone-like tavern feel (T-525)

Model: opus
Effort: high
Subagents: haiku for searches; sonnet for code and the review

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\085-home-concepts`** (branch `085-home-concepts`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English. **Tier R0/R1**: concepts only, self-review + `/code-review` on any script.

**No other sessions at the same time.** Your area: `docs/brand/home/` (new) and its build script if any. **Do not change the live website** (`web/`) in this session: these are concepts for the user to choose from; a later session builds the chosen one. Decision number if needed **D-098**. Every changed text file LF. To catch up: `git fetch origin && git merge origin/main`; no rebase, never force-push.

- The user may be busy: do not wait on them. Push the branch and message the hub (`tavern-ledger-2b`, `ListAgents`, use its `[ref]`) with the branch, PR title, a short PR body and the review artifact link.

## Why

The user wants the home page to feel closer to Hearthstone (2026-10-11). Their references, AI mockups (in `.local/home-ref/`, git-ignored, never commit them because they contain Blizzard's logo):

- **`reference-1.webp`:** a tavern hero. Dark wood wall, warm candle and fire light with sparks, a parchment panel with the text, two ornate buttons ("Enséñamelo", stone or metal with blue and orange gems, glowing; "¿Es seguro?", darker), tankards of beer, scrolls, card backs, metal-framed medallions, a stone and metal frame with gems around it all. It has Hearthstone's blue swirl logo on the medallions and card backs: **that must not appear**.
- **Second reference** (described by the hub, the user liked it): the same, without the frame, the medallions or the card backs. A big wooden barrel wall in the middle, glowing golden sparks on the left and a cool blue magical glow on the right, the headline in big warm-gold serif letters with a dark outline ("Acerca un taburete. Te hemos guardado las partidas."), the parchment panel with nails at the corners, and the two gem buttons below.

The user also wants a **hero mosaic** (real hero portraits) on the home page, and "buttons in Hearthstone style", which is fine if we draw them ourselves.

## Rules for the concepts (rule 5, the hub's reading)

- **Never** Blizzard's or Hearthstone's logos (the swirl), card backs, the game's own UI assets or frames, nor anything cut from game files or screenshots. The user asked about using the Hearthstone logo; the answer was no.
- **Fine:** a generic fantasy tavern (wood, iron, brass, parchment, candles, sparks, tankards), our own buttons in that style (brass or stone plate, gem caps, glow on hover) drawn in CSS or SVG, our TL tankard logo (`docs/brand/logo/`) wherever a medallion or emblem goes, and real hero portraits loaded at view time from `art.hearthstonejson.com` (D-038, D-074, the Fan Content Policy), never committed. Use hero ids from `web/src/lib/heroes/names.data.ts`.
- All text is real HTML text (translatable, readable), the buttons work with keyboard and touch, and every concept holds up on a phone (400 px) and with reduced motion.
- Generated background pictures are allowed only if made by you here (SVG, CSS, canvas); no image AI is available to you. Wood, light and sparks in CSS and SVG are enough for concepts.

## Task

1. Build **five or six home page hero concepts** as standalone HTML pages in `docs/brand/home/`, each full width, Spanish copy like the references (with the English version one toggle away), sharing the site's fonts and the logo:
   - **A. Tavern wall:** the second reference made with our own pieces (wood, parchment with nails, gold headline, gem buttons, sparks left, blue glow right).
   - **B. Wanted posters:** real hero portraits hung on the wooden wall as framed pictures or "wanted" posters lit by candles, the text on parchment in front.
   - **C. Lobby of eight:** eight hero portraits in an arc behind the title, the eight places of a Battlegrounds lobby, each with its place 1st to 8th engraved in brass.
   - **D. Notice board:** the wall as a board with pinned notes showing key numbers (games saved, most played hero with portrait, top-4 share), marked as an example when not signed in.
   - **E. Tabletop:** the table seen from above, tankards, coins, an open ledger with the copy, hero portraits as tokens.
   - **F (optional):** your own idea in the same spirit.
2. Build a set of **our own Hearthstone-style buttons** (primary with gems and glow, secondary dark, small), with hover, focus and pressed states, shown on one strip in the review page.
3. A **review page** `docs/brand/home/index.html` showing every concept (desktop and phone width) and the buttons, with a short note on each: what it borrows from the references, what it leaves out and why. Publish it with the `Artifact` tool (one page; portraits load from `art.hearthstonejson.com`, so check the artifact's CSP allows that image host; if it does not, use our own placeholder silhouettes in the artifact and say so).
4. Plan row T-525 (concepts, the build of the chosen one is next), README line.

## Acceptance criteria

- [ ] Five or more concepts plus the buttons, none with any Blizzard logo, card back or game asset.
- [ ] Each works at 400 px and with reduced motion; text is real text.
- [ ] Review artifact published; link sent to the hub.
- [ ] Nothing in `web/` changed; branch handed to the hub.

## Red line

Rule 5 (brand), absolute rule 1 (clean room), D-004 (as amended by D-063), D-006. Never print BattleTags, `GameAccountId`, keys or the user's email. Never commit the reference images.
