# Design skills (third-party)

Instructions for Claude, copied from the user's kit at the commits listed in [`PROVENANCE.md`](../../PROVENANCE.md). Every file was read before it entered the repo (T-311, 2026-10-10). None tells an agent to run commands on its own, send data, change settings or ignore its rules. One script was left out: `web-quality-audit/scripts/analyze.sh`.

**The project's rules win over any skill** ([`CLAUDE.md`](../../CLAUDE.md), D-004, D-038). Where a skill says otherwise, follow these:

- No third-party scripts, CDNs or remote images on the site or in the app. Skills that suggest `picsum.photos`, `cdn.simpleicons.org`, Google Fonts links or a CDN script do not apply here. Fonts are open-licence and self-hosted, and every new font is a new dependency (the user's OK and a `PROVENANCE.md` row first).
- No new dependencies without the user's OK: no React, Tailwind, Motion, GSAP, shadcn or design-system packages because a skill names them as defaults. The app and site stay plain HTML, CSS and JS (D-039, D-040, D-052).
- No analytics or real-user monitoring (`performance/references/RUM.md` does not apply).
- No image generation of game art. Card images only under Blizzard's Fan Content Policy (D-038); no Blizzard logos, names, typefaces or trade dress.
- Never invented features, numbers, testimonials or logos (absolute rule 8). `copywriting` and `seo` serve the site's own text only, and any made-up number in a mockup is labelled as made up.
- The fan-project notice stays on every surface.

Skills: `frontend-design`, `design-taste-frontend`, `redesign-existing-projects`, `better-typography`, `better-layout`, `better-colors`, `animate`, `animation-vocabulary`, `web-quality-audit`, `accessibility`, `performance`, `seo`, `copywriting`. Some reference sibling skills that are not installed (`better-ui`, `better-accessibility`, `core-web-vitals`, `review-animations`…); those links lead nowhere.
