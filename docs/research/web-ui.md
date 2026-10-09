# Website redesign: references and constraints (T-104e, session 028)

Checked 2026-10-10. Licences read from the npm registry metadata (`npm view <package> license`); repository LICENSE files were not opened because `gh` was not logged in in this session, so the "MIT" lines below are registry claims to re-check before any code is copied.

## What the site must keep (red line)

- Static Astro, CSP `default-src 'none'`, `script-src 'self'` (or `'none'` when accounts are closed), `style-src 'self'`, `font-src 'self'`, `connect-src` only the Supabase project. No inline script or style, no external request, no analytics (D-020, D-023, D-027, D-030).
- So any font must be a file we ship, and any CSS must be a file we ship. Nothing from a CDN.
- No Blizzard logo or name in the brand; the fan-project notice on every page; no invented features, figures or testimonials.

## References looked at

| Reference | What it is | Licence (registry) | Use |
|-----------|-----------|--------------------|-----|
| Pico CSS (`@picocss/pico` 2.1.1) | Classless semantic CSS framework, no JavaScript | MIT | Idea only: semantic-HTML-first styling. Not adopted: our CSS is already small and a framework would add rules we then override. |
| Open Props (`open-props` 1.7.23) | CSS custom-property design tokens | MIT | Idea only: named scales for spacing, easing and shadows. We keep our own tokens in `global.css`. |
| Fontsource (`@fontsource/*`) | npm packages that ship the `.woff2` files of open fonts, with no request to Google | OFL-1.1 (per package) | Possible way to get font files. Needs a new dev dependency; alternative is to copy the `.woff2` files into `web/public/fonts/` with their OFL text. |
| Anthropic `frontend-design` skill | Agent skill that asks for a committed aesthetic direction before code | Licence not verified | Method only (pick a direction first, avoid template looks). No code or text copied. |
| User rule files `web/design-quality.md` | Anti-template checklist (hierarchy, rhythm, depth, real type pairing, designed states) | user's own | Used as the checklist for the three directions. |

## Fonts (all OFL-1.1 per the registry; self-hosted `.woff2`, `font-display: swap`, max two families)

- Fraunces 5.3.0 (variable serif with "wonk" and optical size): ledger/editorial feel.
- Newsreader 5.3.0: quieter text serif.
- Inter 5.3.0, JetBrains Mono 5.3.0, Space Grotesk 5.3.0, IBM Plex Serif 5.3.0.

Cost: each family adds roughly 20 to 60 KB per weight as `.woff2`; subset to Latin and one or two weights. The web performance budget for a microsite is 80 KB JS and 15 KB CSS gzipped; fonts are separate but should stay under about 120 KB in total.

## Decision needed from the user

Direction (see the mockup page) and permission to add the font files plus a `PROVENANCE.md` line for each family. Recorded as D-040 once chosen.
