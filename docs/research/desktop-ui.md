# Desktop app and overlay redesign: references (T-305, session 027)

Checked 2026-10-10. `gh` was not logged in in this session, so no repository LICENSE file was opened. Nothing below was copied; every row is an idea or a file we already hold.

## What the app must keep (red line)

- The overlay stays a separate window that only draws; it shows only what the player could see (D-004, D-006, D-032, D-033).
- A source label on every value, no meaning by colour alone, the contrast test on every overlay theme, the fan-project notice, card images only as T-304 set them up.
- Content-Security-Policy of the app: `default-src 'self'`, no external request. So fonts must be files inside the app.

## References looked at

| Reference | Licence | Use |
|-----------|---------|-----|
| Website look "Combat Round" (D-040, this repo) and its research in [`web-ui.md`](web-ui.md) | ours | The direction chosen. Same tokens, same fonts, so the site and the app read as one product. |
| Unbounded and DM Mono (`.woff2` from `@fontsource`, already in `web/public/fonts/`) | SIL OFL-1.1 (registry metadata, per `web-ui.md`) | Copied into `crates/desktop/ui/fonts/` with the OFL texts. Recorded in `PROVENANCE.md`. |
| Pico CSS, Open Props | MIT (registry) | Ideas only (semantic HTML first, named design tokens). Not adopted: the app has about 400 lines of CSS and no build step. |
| Anthropic `frontend-design` skill and the user's `web/design-quality.md` rules | licence not verified / the user's own | Method only: pick a committed direction first, three real directions with a cost each, avoid the default-template look. |
| Hearthstone Deck Tracker, Firestone, Nomi's Kitchen | not compatible (CLAUDE.md rule 1) | Not looked at, not read. |

## The three directions shown to the user

- **A. Combat Round**: dark teal, Unbounded and DM Mono. Cost: four font files copied (OFL). Chosen.
- **B. Folio**: one calm paper sheet, system serif. No new asset.
- **C. Cockpit**: dense sidebar with master/detail and a recap side pane, system fonts. No new asset, and a bigger change to `app.js`.

## Notes for the build

- Class names and markup did not change, so `app.js`, `overlay.js` and the tests did not need to.
- Place is shown as a number first; the fill (gold, teal) and the outline (bottom half) only repeat it.
- Source marks keep their border style (dashed, double, solid, dotted) next to the colour.
- The overlay's default theme becomes `round`; the older three stay so saved layouts and choices keep working.
