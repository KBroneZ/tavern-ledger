# Redesign "Lamplight glass" (T-311, D-052)

Session 041, 2026-10-10. The look chosen by the user for the website, the desktop app and the overlay, how it was reached, and the exact build plan for the website (session 042). The app and overlay are built in session 041 itself.

## The brief and how we got here

The user asked for a new direction, chosen from mockups, replacing D-039 (app) and D-040 (site). Brief, in their words and the hub's: modern, with the feel of a fantasy tavern, in our own style; easy to read and easy to use for everyone; a full redesign that keeps nothing from the old site; a home page that sets us apart with the game screen pulled apart into layers; no copy of the game's colours, frames or typefaces (absolute rule 5).

Five rounds of mockups (one private artifact, updated in place, owned by the user):

| Round | Directions | User's verdict |
|-------|-----------|----------------|
| 1 | Hearth, Chalkboard, Timber and lime | Kept too much of the current site; wanted everything new |
| 2 | Bill of fare, Eight windows, The table (with an exploded view of the game screen) | Not convinced; wanted the layered view to move with the scroll, "crystal", nice, readable |
| 3 | Crystal (indigo and mana blue, layers sliding with side cards) | Odd; wanted the game's kind of colours and a still scene that changes as you scroll |
| 4 | Hearth (wood, gold, parchment, minion frames) | Too close to the game itself; frames around minions look cheap; wanted the 3D side view of the layers, glassy |
| 5 | **Lamplight glass** | **Chosen** |

The design skills in `.claude/skills/` (T-311, phase 1) were used for every round; the project's rules win over them (see `.claude/skills/README.md`).

## The look in one paragraph

A tavern at night, seen through glass. Smoky warm black behind everything, amber lamplight for what you can press and for wins, frosted glass panels with a thin prism edge (a rainbow hairline, not a frame), and nothing drawn around minions or heroes: a picture spot, a name and plain numbers. Type is crisp and modern: Bricolage Grotesque at a slightly narrow width for headings and big numbers, Figtree for everything you read. The voice is warm and plain ("Pull up a stool. We kept your games."), never clever at the cost of clarity.

## Tokens

Use these names in `web/src/styles/global.css` (the app uses the same values in `crates/desktop/ui/style.css`).

### Colour

| Token | Value | Role |
|-------|-------|------|
| `--bg` | `#120d0b` | Page background (smoky warm black) |
| `--bg-deep` | `#0c0807` | Wells, inputs, code |
| `--panel` | `rgb(255 238 220 / 0.05)` | Glass panel fill |
| `--panel-solid` | `#1e1612` | Dialogs, panels over busy backgrounds |
| `--panel-hi` | `rgb(255 178 74 / 0.08)` | Row hover, selected |
| `--line` | `rgb(255 228 200 / 0.18)` | Borders, rules |
| `--line-soft` | `rgb(255 228 200 / 0.09)` | Table row separators |
| `--text` | `#f7efe6` | Body text |
| `--dim` | `#cbbdaf` | Secondary text (8.7:1 on `--bg`) |
| `--amber` | `#ffb24a` | Primary action fill, win disc, focus |
| `--amber-hi` | `#ffd08a` | Amber text, headings accents, focus ring |
| `--amber-ink` | `#2a1600` | Text on amber |
| `--good` | `#8fe0a8` | Won, live status |
| `--bad` | `#ff9a8a` | Lost, errors, delete |
| `--info` | `#a9c8ff` | Source marks (inferred, entered, leaderboard) |
| `--prism` | `linear-gradient(120deg, #ffd7a1, #ff9db8 30%, #b3b8ff 60%, #8ff0e6 85%, #ffd7a1)` | Only as the 1px edge of glass panels and panes, at 40 to 60% opacity |

All text pairs above pass WCAG AA on `--bg` and on a panel over `--bg` (checked: lowest is `--bad` at 7.7:1). Never put text on the prism gradient. Amber is the only accent; green and red mean won and lost and nothing else, and always come with a word or a shape.

The site is dark only (the brief is a night tavern). Set `color-scheme: dark` on `:root`.

### Type

Two families, self-hosted, `font-display: swap`:

| Role | Family | Size (rem at 16px) | Weight / width | Line height |
|------|--------|--------------------|----------------|-------------|
| Hero headline | Bricolage Grotesque | `clamp(2.6rem, 6.8vw, 5.25rem)` | 700, `font-stretch: 88%` | 0.98 |
| Section heading | Bricolage Grotesque | `clamp(1.9rem, 4vw, 3rem)` | 700, 88% | 1.02 |
| Card heading | Bricolage Grotesque | `1.45rem` | 700, 88% | 1.15 |
| Big number | Bricolage Grotesque | `clamp(2.1rem, 4vw, 3rem)` | 700, 80%, `tabular-nums` | 1.05 |
| Lead text | Figtree | `clamp(1.125rem, 2vw, 1.3rem)` | 500 | 1.55 |
| Body | Figtree | `1.0625rem` (17px) | 500 | 1.6 |
| Small, hints | Figtree | `0.95rem` | 500 | 1.5 |
| Labels, buttons | Figtree | `1.0625rem` | 700 to 800 | 1 |

Headings get `text-wrap: balance` and `letter-spacing: -0.015em`; paragraphs `text-wrap: pretty`, at most about 60 characters wide. No all-caps labels, no eyebrows above every heading, no em dashes in copy.

### Space, radius, depth

- Side gutter `clamp(16px, 4vw, 44px)`; sections `56px` apart; groups inside a section `16 to 22px`.
- Radius: glass panels `16px`, buttons `12px` (small `10px`), chips and marks fully round, inputs `10px`.
- Depth: panels get `inset 0 1px 0 rgb(255 255 255 / 0.08)` and a soft shadow `0 18px 40px rgb(0 0 0 / 0.35)`. Ambient light is a few blurred amber radial gradients in the page background, never on text.
- Glass edge: a `::before` with `padding: 1px`, `background: var(--prism)`, masked to a ring (`mask: linear-gradient(#000 0 0) content-box exclude, linear-gradient(#000 0 0)` plus the `-webkit-` form). `backdrop-filter: blur(12px)` only where something sits behind the panel; give it a solid fallback under `prefers-reduced-transparency: reduce`.

### Components

- **Primary button**: amber gradient (`--amber-hi` to `--amber`), `--amber-ink` text, 48px tall, `scale(0.97)` on press (130ms). One per view.
- **Quiet button**: `--panel` fill, `1.5px` inset `--line` ring, amber ring on hover.
- **Danger button**: transparent, `2px` inset `--bad` ring, `--bad` text.
- **Place**: the number always shows. Win: lit amber disc. Top half: amber ring. Bottom half: bare number. No place in the data: a dashed circle with a dash (the app) or "?" (the site) and the `unknown` mark. Place rules of the mode unknown (the app): the number on a soft square.
- **Source marks**: unchanged words and border styles (dashed inferred, double entered, solid leaderboard, square card data, dashed italic possible, dotted unknown).
- **Brand mark**: our own lantern pane, a rounded amber rectangle with a white inner line; no Blizzard shapes. Same in the app.

## Fonts and licences

| File (Latin subset, variable) | Size | From | Licence |
|-------------------------------|------|------|---------|
| `bricolage-grotesque-latin-wdth-normal.woff2` (weight 200 to 800, width 75 to 100%) | 78 KB | `@fontsource-variable/bricolage-grotesque` 5.3.0 | SIL OFL 1.1, Copyright 2022 The Bricolage Grotesque Project Authors |
| `figtree-latin-wght-normal.woff2` (weight 300 to 900) | 20 KB | `@fontsource-variable/figtree` 5.3.0 | SIL OFL 1.1, Copyright 2022 The Figtree Project Authors |
| `figtree-latin-ext-wght-normal.woff2` (accented names) | 10 KB | same | same |

Session 041 copies these exact files and the two OFL texts into `crates/desktop/ui/fonts/` (rows in `PROVENANCE.md`). Session 042 copies the same files into `web/public/fonts/` with `OFL-Bricolage-Grotesque.txt` and `OFL-Figtree.txt`, adds one `PROVENANCE.md` row for the site copy, and removes Unbounded and DM Mono from the site. The `@font-face` rules are in `crates/desktop/ui/style.css` (copy them, with `/fonts/` paths). Preload only the Bricolage file on the home page. Total fonts: about 108 KB, under the 120 KB budget in `web-ui.md`.

## Website build plan (session 042)

### Limits that do not change

- Static Astro. CSP from `contentSecurityPolicy()` in `web/src/lib/config.ts`: `default-src 'none'`, `script-src 'self'` (or `'none'` while accounts are closed), **`style-src 'self'`**, `img-src 'self'`, `font-src 'self'`, `connect-src` the Supabase project only. Do not change it.
- `style-src 'self'` blocks inline `style="..."` attributes and `<style>` blocks. Every per-element value (a pane's depth, a delay) must come from a class in `global.css`, not from a `style` attribute.
- **No script for the look.** The pinned scene and every effect are CSS. No GSAP, Motion or any library; no new npm dependency.
- No analytics, no third-party request, no CDN, no external font. The fan-project notice on every page. Never invent features, numbers or testimonials; any example number on the page is labelled as an example.
- No Blizzard logos, art, screenshots, typefaces or trade dress. The game screen in the scene is drawn with our own simple shapes (circles, rounded rectangles, lines).

### Pages

Keep every route, slug, form field name and the legal text as they are (`index`, `signin`, `account`, `profile`, `privacy`, `terms`, `404`); only the look and the home page's content order change.

1. **Home** (`index.astro`), in this order:
   1. Nav: lantern mark and "Tavern Ledger" on the left; Home, Privacy, Sign in (or Account) on the right; the current page underlined in amber.
   2. Hero, centred, no image: headline "Pull up a stool. We kept your games." (the second sentence in `--amber-hi`), lead "A free Windows app that reads Hearthstone's own log on your PC and keeps every Battlegrounds game: your hero, your place and the boards you faced.", two buttons: "Show me" (primary, to `#scene`) and "Is it safe?" (quiet, to `#safe`). Three blurred amber background lights, decorative, `aria-hidden`.
   3. **The pinned scene** (below).
   4. "What it keeps for you": one lead line ("Everything comes from the game's own log. When the log does not say, the app says "unknown" instead of guessing.") and three glass cards in a `1.2fr 1fr 1fr` grid: every game and place, the boards you met, a recap after each game. No made-up number unless labelled.
   5. "Safe for your account" (`id="safe"`), one glass panel: the three "never" lines (memory, injection or file changes; playing or clicking for you; showing what you could not see), each with a round red-tinted cross.
   6. "Your games stay yours": three columns split by hairlines: private until you say, take it all with you, gone when you ask.
   7. Status: "The doors are not open yet." with the current status line (keep the logic that says whether accounts are for testing or closed).
   8. Footer: the fan-project notice, Privacy, Terms.
2. **Sign in**, **Account**, **Public profile**: one column, max `1000px`, glass panels with big fields (48px tall, labels above, hints below, errors in `--bad` with a word, never colour alone). Account: Profile (display name, public switch, Save), Your data (Download), Delete account (danger panel with a `--bad` ring, typed confirmation as today).
3. **Privacy, Terms, 404**: the reading layout: one column of at most 68 characters, section headings in Bricolage, links in `--amber-hi` underlined.

### The pinned scene (home page)

What it shows: the game screen, seen from the side, coming apart into four panes of glass while the scene holds still, then the page carries on.

Structure:

```html
<section class="pin" id="scene" aria-labelledby="scene-title">
  <h2 id="scene-title" class="visually-hidden">What Tavern Ledger adds to your screen</h2>
  <div class="pin-stage">
    <div class="captions">
      <div class="pips" aria-hidden="true"><i></i><i></i><i></i><i></i><i></i></div>
      <div class="cap c1">…</div> … <div class="cap c5">…</div>
    </div>
    <div class="stage3d" role="img" aria-label="…">
      <div class="scene3">
        <div class="pane p1"><span class="label">Board</span>…</div>
        <div class="pane p2"><span class="label">Tavern</span>…</div>
        <div class="pane p3"><span class="label">Leaderboard</span>…</div>
        <div class="pane p4 ours"><span class="label">Tavern Ledger</span>…</div>
      </div>
    </div>
  </div>
</section>
```

- `.pin` is `460vh` tall (`420vh` under 860px wide) and declares `view-timeline: --pin block`. `.pin-stage` is `position: sticky; top: <nav height>; height: calc(100vh - <nav height>)`, a two-column grid (captions `0.75fr`, scene `1.75fr`; one column on phones, captions on top).
- `.stage3d` has `perspective: 1900px; perspective-origin: 50% 35%`. `.scene3` (16:10, `transform-style: preserve-3d`) rests at `rotateY(-42deg) rotateX(9deg)`.
- Each `.pane` is a glass rectangle (`border-radius: 20px`, fill `rgb(255 238 220 / 0.045)`, prism edge at 60%) placed with `transform: translateZ(var(--z))`: board `-210px`, tavern `-70px`, leaderboard `70px`, ours `210px`. `--z` is set in the pane's class, never inline.
- Pane contents, all our own shapes: board, seven soft amber lights in a row near the bottom; tavern, four outlined rounded rectangles near the top; leaderboard, eight small circles down the left with the third one white and glowing; ours, a dark card with an amber ring at the top centre, three small panels on the right and a thin amber line from the glowing circle to the card. Each pane has a label chip above its top left corner; ours is amber.
- Five captions, one visible at a time, in a fixed spot: "Your screen, while you play" / "The board" / "The tavern" / "The leaderboard" / "Tavern Ledger, in front". Copy as in the mockup (session 041), short, each saying who draws the layer; the last says the panes are only what the player could already see and that clicks pass through. The pips above show which step is on.

Motion (inside `@supports (animation-timeline: view())` and `@media (prefers-reduced-motion: no-preference)` only), every animation on `animation-timeline: --pin; animation-range: contain 0% contain 100%; animation-fill-mode: both; animation-timing-function: linear`:

| Progress | Scene | Captions |
|----------|-------|----------|
| 0 to 26% | `.scene3` turns from `rotateY(-16deg) rotateX(4deg)` to its resting angle while the panes move from `translateZ(0)` (one screen) to their depths (from 6% to 26%) | Step 1 |
| 27 to 42% | Board pane lit (opacity 1, amber glow `0 0 50px rgb(255 178 74 / 0.45)`), others at opacity 0.35 | Step 2 |
| 46 to 58% | Tavern pane lit | Step 3 |
| 63 to 75% | Leaderboard pane lit | Step 4 |
| 80 to 100% | Our pane lit with a stronger glow (`0 0 80px rgb(255 178 74 / 0.6)`), the game's panes stay at 0.35 | Step 5 |

Captions cross-fade in place (opacity and a `10px` vertical shift, `visibility: hidden` when out) with 3 to 4% gaps between steps; they never slide past each other. Animate only `transform`, `opacity` and `box-shadow`. The mockup's keyframes (`turn`, `z1` to `z4`, `hl1` to `hl4`, `c1` to `c5`, `pp2` to `pp5`) are the reference: the round 5 mockup is saved as [`redesign-mockup.html`](redesign-mockup.html) (open it in a browser; it uses a little script only to fill its example tables, which the site does not need). The live version is the user's private artifact `https://claude.ai/artifact/BrB48Wmcni6ihz2573Qye4`.

Fallback (Firefox without scroll timelines, any browser with reduced motion): no animation at all; the scene is one still picture with the panes already apart and our pane lit, and the captions are a plain list (the `.cap` elements stay in normal flow). This is the default CSS; the animated version only switches on inside the two queries above. Check both states.

### Motion elsewhere

- Buttons: `scale(0.97)` on press, 130ms, `cubic-bezier(0.23, 1, 0.32, 1)`. Hover changes colour or ring only, gated by `@media (hover: hover) and (pointer: fine)` if it moves anything.
- No entrance animations on sections, no scroll reveals besides the pinned scene, no loops. The blurred lights do not move.
- `prefers-reduced-motion: reduce` turns every animation and transition off.

### Accessibility and quality checks for 042

- Lighthouse accessibility 100 and a keyboard pass on every page; visible focus ring (`3px` `--amber-hi`).
- The scene's `role="img"` has an `aria-label` that says the same as the five captions together; captions stay in the DOM in reading order.
- Text at 200% zoom and 320px wide without horizontal scroll; the pinned scene's captions must not overlap the panes on a phone.
- Performance: no script added for the look; fonts about 108 KB; LCP is the hero headline. Run the `performance` and `accessibility` skills on the built site, both states of the scene (animated and still).

## Desktop app and overlay (session 041)

Built in session 041 with the same tokens: `crates/desktop/ui/style.css`, `upload.css`, `shop.css` and `overlay.css`. The overlay's new default theme is **Glass** (`glass` in `overlay_layout.rs`); **High contrast** stays; `round`, `tavern` and `parchment` are retired and read as `glass` when a saved `overlay.json` names them. The contrast test checks Glass over black, white and mid grey at the lowest panel opacity.
