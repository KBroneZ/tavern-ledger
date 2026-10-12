# Logo brief

**Outcome (2026-10-11, T-522, D-094):** the user picked a "TL" monogram that forms a tankard, made in Gemini (closest to direction 4 with the tankard of direction 2). It is redrawn by hand in [logo/](logo/index.html); the AI images stay in [reference/](reference/) as reference only.

Task T-520 (D-093). The brief for a Tavern Ledger mark that works in the app, on the website and everywhere else. Ready-to-paste image prompts: [logo-prompts.md](logo-prompts.md).

## The product

Tavern Ledger (name: D-007) is a free, open-source Hearthstone Battlegrounds tracker: a Windows desktop app that reads the game's local log, with a small in-game overlay, plus a website to follow your progress. For players who want to see their own games, places and heroes over time. Unofficial fan project, not affiliated with or endorsed by Blizzard Entertainment.

## The feeling

A fantasy tavern's ledger read by lamplight, drawn in a modern way and in a style of our own (D-052, "Lamplight glass"). Warm, calm, a little crafted; not medieval clip art, not a game logo. It should look like a tool you trust with your numbers.

## Palette (from `web/src/styles/global.css`)

| Role | Token | Hex |
|------|-------|-----|
| Background, smoky warm black | `--bg` | `#120d0b` |
| Deepest background | `--bg-deep` | `#0c0807` |
| Solid panel | `--panel-solid` | `#1e1612` |
| Amber lamplight (main accent) | `--amber` | `#ffb24a` |
| Light amber (highlight) | `--amber-hi` | `#ffd08a` |
| Ink on amber | `--amber-ink` | `#2a1600` |
| Warm white text | `--text` | `#f7efe6` |
| Dim text | `--dim` | `#cbbdaf` |

The site's thin "prism" edge (`#ffd7a1`, `#ff9db8`, `#b3b8ff`, `#8ff0e6`) is a UI detail, not a logo colour: at most a hairline in large versions, never in the small mark.

## Typography

- Wordmark: **Bricolage Grotesque** (SIL OFL 1.1, already self-hosted by the site and app), bold or semibold, possibly slightly condensed through its width axis.
- Supporting text: **Figtree** (SIL OFL 1.1).
- The final wordmark is set in the real font by us, not drawn by an image model.

## Where the mark must work

| Place | Size | Notes |
|-------|------|-------|
| Browser favicon | 16 and 32 px (plus an SVG) | Mark only, no text |
| Windows app, taskbar, Start menu | 16, 24, 32, 48, 64, 128, 256 px in one `.ico` | Mark only; must read on light and dark taskbars |
| Installer | 256 px icon, installer banner | Mark + wordmark on the banner |
| Overlay badge | about 16 to 24 px | Over the game, so it must stay quiet and never look like a game element |
| Website masthead | about 28 to 40 px high | Mark + "Tavern Ledger", horizontal |
| Social card | 1200 × 630 | Mark + wordmark on `#120d0b`, plus the fan-project notice |
| GitHub avatar | 500 × 500, shown as small as 20 px, cropped to a circle in places | Mark only, centred with margin |
| Dark backgrounds | — | The main version |
| Light backgrounds | — | A version with dark ink (`#2a1600` or `#120d0b`) |
| Single colour | — | One flat colour, for print, Windows high contrast and the overlay |

## Rules

1. A simple mark that still reads at **16 px**: two to four shapes, thick enough strokes, no fine detail.
2. **No text inside the small mark** (an initial is a shape, not text, and still has to read at 16 px).
3. **No gradients that die at small sizes.** Flat fills; a soft glow is allowed only in large versions.
4. **Nothing from Blizzard or Hearthstone** (brand rules): no logos, no names, no card frames, no mana gems, no coin, no tavern sign or swirl, no hero portraits, no game typefaces, nothing that could pass as official. The brand never says "Hearthstone" or "Battlegrounds" in the mark or wordmark.
5. The fan-project notice ("Unofficial fan project. Not affiliated with or endorsed by Blizzard Entertainment.") goes next to the brand wherever it is shown in public, as it already does on the site and in the app.
6. No claims in brand material that the app does not make (rule 8).

## Concept directions

1. **Ledger and flame.** An open book seen from the front (two pages as a shallow "V") with a single lamp flame rising from the spine. Fits: it is literally the name, and the flame is the amber lamplight of the whole look. Risk at 16 px: the pages turn into a bar; keep the book to two solid shapes and the flame large.
2. **Tankard as a bar chart.** A tankard whose body is three rising bars, the foam or a small flame on top. Fits: tavern plus stats in one shape. Risk at 16 px: it can read as a generic "stats" icon or a beer app; the handle must stay clear, and the bars no more than three.
3. **Lantern with a chart inside.** A simple lantern outline (a rounded rectangle with a ring on top) whose glowing inside is a small rising line or three bars. Fits: lamplight and tracking. Risk at 16 px: a frame with something inside loses the inside first; the inner shape must be bold, or the lantern itself carries the mark.
4. **"TL" keyhole monogram.** A T and an L joined into one shape whose negative space forms a keyhole or a candle. Fits: short, ownable, works as an app tile. Risk at 16 px: monograms blur into a blob; it needs one clear counter shape. Also the most likely to resemble an existing mark.
5. **Quill and candle.** A quill crossing a candle, or a quill whose nib is a flame. Fits: the ledger is written by hand by lamplight. Risk at 16 px: two thin diagonal objects become one line; probably only works as a single fused shape.

Suggested order to try: 1, 3, 2, then 4 and 5.

## After the user picks one

1. **Pick and check.** The user picks one image and does a reverse image search (Google Images or TinEye) for it, to see that it is not too close to an existing trademark or app icon. Anything close → drop it.
2. **Rebuild by hand.** Later, the chosen image is only a reference: the mark is redrawn as a clean, hand-made SVG (few paths, whole-pixel alignment for 16 and 32 px, a separate simplified version for 16 px if needed), and the wordmark is set in Bricolage Grotesque. The AI raster is never shipped as the logo.
3. **Asset set.** From that SVG: `favicon.svg` plus 16/32 px PNG or `.ico` for the site; the app `.ico` (16 to 256 px); the installer images; the overlay badge; the masthead (mark + wordmark); the 1200 × 630 social card; the GitHub avatar; light, dark and single-colour versions. Then replace `web/public/favicon.svg` and `crates/desktop/icons/icon.ico`.
4. **Record it.** The origin of the mark in `PROVENANCE.md` (which tool helped, that the final file was drawn by us).

## Licence point

**OpenAI (ChatGPT), verified 2026-10-11** on the [Europe Terms of Use](https://openai.com/policies/eu-terms-of-use/) (page dated "Updated: January 16, 2026"; the user lives in the EEA): "As between you and OpenAI, and to the extent permitted by applicable law, you (a) retain your ownership rights in Input and (b) own the Output. We hereby assign to you all our right, title, and interest, if any, in and to Output." The same section adds that "Output may not be unique and other users may receive similar output" and that the assignment "does not extend to other users' output". The non-EEA terms ([row-terms-of-use](https://openai.com/policies/row-terms-of-use/), "Effective: January 1, 2026") say the same.

What this means for us: OpenAI does not claim the image, but it does not promise it is unique or that it can be protected, and an AI image alone may not be protected by copyright in some countries. That is one more reason to redraw the final mark by hand (step 2) and to do the reverse image search (step 1).

**Ideogram and Recraft: unverified.** Their terms on ownership, and whether a free plan makes images public, were not read. Read them before using either tool, and prefer ChatGPT if in doubt.
