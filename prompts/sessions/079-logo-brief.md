# Session 079: Logo brief and image-generation prompts

Model: opus
Effort: medium
Subagents: haiku for searches; sonnet for drafts

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\079-logo-brief`** (branch `079-logo-brief`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English (the image prompts themselves in English). Everything in the repo is in English. **Tier R0** (docs only; no change to the app, the site or their icons in this session).

**Other sessions at the same time:** 076, 077, 078 (website). Your area: `docs/brand/` (new) plus your rows in `plan.md`, `README.md` and, if needed, `DECISIONS.md` (decision number **D-093**). Every changed text file LF. `gh` is not available: push the branch and message the hub (`tavern-ledger-2b`, `ListAgents`, use its `[ref]`) with the branch, PR title and a short PR body.

## Why

The user (2026-10-11): "Prepare a prompt, for ChatGPT or another site, to start creating a logo that works for the app, the website and everything." Today the site has a placeholder `web/public/favicon.svg` (an amber lamp square) and the app `crates/desktop/icons/icon.ico`.

## Task

1. **Read first:** D-007 (name), D-052 and D-056 (Lamplight glass: smoky warm black, amber lamplight, frosted glass with a thin prism edge, Bricolage Grotesque and Figtree), `docs/research/redesign.md` if present, the current favicon and app icon, CLAUDE.md rules 5 and 8.
2. **Brand brief** (`docs/brand/logo-brief.md`, one page): what the product is, who it is for, the feeling (a fantasy tavern's ledger by lamplight, modern, our own style), the palette as hex values from the design tokens, typography, and **every place the mark must work**, with sizes: favicon 16 and 32 px, Windows app and taskbar icon (16 to 256 px, `.ico`), installer, the overlay's small badge, the site's masthead (mark + wordmark, horizontal), a square social image (1200×630 card), GitHub avatar, dark and light backgrounds, single colour. Rules: a simple mark readable at 16 px; no Blizzard or Hearthstone logos, names, card frames, gems, the coin, the tavern sign or anything that could pass as official (rule 5, "Unofficial fan project…"); no text inside the small mark; no gradients that die at small sizes.
3. **Three to five concept directions**, each a short paragraph (e.g. an open ledger with a lamp flame, a quill crossing a candle, a lit tankard as a bar chart, a monogram "TL" as a keyhole), with why it fits and its risk at 16 px.
4. **Ready-to-paste prompts** (`docs/brand/logo-prompts.md`), one block per direction, for **ChatGPT image generation** (the user's Plus account; they paste it themselves, the `ask-chatgpt` skill does not do images) and adapted variants for one or two other tools (e.g. Ideogram or Recraft, which can export vector). Each prompt: flat vector style, few shapes, centred on a plain background, a mark-only version and a mark + "Tavern Ledger" wordmark version, the palette's hex values, and the "do not" list. Plus a short **follow-up prompt** for iterating ("simplify to three shapes", "make a single-colour version", "show it at 16 px").
5. **What happens after the user picks one** (in the brief): rebuild the chosen mark as a clean hand-made SVG in a later session (an AI raster is only a reference, not the final file), then the asset set (favicons, `.ico`, social card, masthead), and a check that the chosen image is not too close to an existing trademark (a reverse image search by the user). Note the licence point: what ChatGPT's terms say about owning generated images, verified on OpenAI's page, with the link and the date read; say plainly if unverified.
6. Optional second opinion: send the brief (not code) to `ask-chatgpt` in `review` mode and apply what is useful; say what you changed.
7. Plan row (new task id from **T-510** onward, after the ones 076 to 078 take: use **T-520**), README line.

## Acceptance criteria

- [ ] `docs/brand/logo-brief.md` and `docs/brand/logo-prompts.md`, short and in plain English.
- [ ] Prompts ready to paste, with the palette, sizes and the "do not" list; follow-up prompts.
- [ ] Licence point verified or marked unverified.
- [ ] Branch handed to the hub; in your last message to the user, paste the first prompt so they can try it right away.

## Red line

Rule 5: no Blizzard logos or name in the brand, nothing that looks official. Rule 8: no made-up claims. Never print BattleTags or the user's email.
