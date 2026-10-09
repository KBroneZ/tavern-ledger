# Deploy: website on tavernledger.net and contact mail (T-104c, session 021)

Plan for putting the static site in `web/` online at `https://tavernledger.net`, with accounts closed until the hosted Supabase project exists (T-104a), and for the project's contact mail. Facts about Cloudflare, Brevo and Supabase were read on their pages on 2026-10-09; the URLs are at the end. Items marked **unverified** could not be confirmed on a primary page.

## 1. What goes live

The site built **with no Supabase settings** (`web/src/lib/config.ts`: neither `PUBLIC_SUPABASE_URL` nor `PUBLIC_SUPABASE_ANON_KEY` set):

- Home, privacy (draft) and terms (draft), the 404 page, `robots.txt` and the favicon.
- `/signin/`, `/account/` and `/profile/` show "Accounts are not open yet": no form, no input, no script. The nav hides the account links.
- No JavaScript ships at all (the build removes the account pages' bundles; `test/build` checks that no page has a `<script>`, `<form>` or `<input>` and that no `.js` file is in `dist/`).
- The Content-Security-Policy allows no script, no form submission and no connection (`script-src 'none'`, `form-action 'none'`, `connect-src 'none'`); styles and images from the site only. The CSP limits what a page may do; it does not stop a visitor from following a link.
- Every page carries "Unofficial fan project. Not affiliated with or endorsed by Blizzard Entertainment." (checked by `test/build`).
- One setting without the other still stops the build, so a half-set project cannot ship a broken sign-in.

When the hosted project exists, setting the two variables in the Pages project and redeploying opens accounts (same code, CSP then allows only the Supabase origin). That is T-104a's work, not this session's.

## 2. How it is built and deployed

**Choice: Cloudflare Pages with Git integration** (Pages builds from the GitHub repo), not direct upload from CI.

| | Git integration (chosen) | Direct upload from GitHub Actions |
|---|---|---|
| Secrets | None in GitHub | A Cloudflare API token as a GitHub secret (R3, a token the user must create) |
| New dependencies | None | `wrangler` (new dev dependency) |
| Access granted | Cloudflare Pages GitHub app, installed on `KBroneZ/tavern-ledger` only | Token scoped to Pages edit on the account |
| Build | Cloudflare's build image, from the same lockfile | Our CI runner |
| Main risk | A compromised Cloudflare account can read the (public) repo and set commit statuses | A leaked token can deploy anything to the site |

Git integration has no long-lived secret anywhere and adds nothing to the dependency list; the repo is public, so the app's read access exposes nothing new. CI (GitHub Actions) keeps testing every PR as today; Pages only builds `main`.

Pages project settings:

| Setting | Value | Why |
|---|---|---|
| Project name | `tavern-ledger` | `tavern-ledger.pages.dev` |
| Production branch | `main` | `main` is always releasable (CLAUDE.md) |
| Automatic production deployments | on | each merge to `main` goes live |
| Preview deployments | **None** | previews are public by default; nothing needs them yet |
| Root directory | `web` | the site's own lockfile; Pages clones the whole repo, so `docs/legal/*.md` (rendered by `/privacy/` and `/terms/`) are there |
| Build command | `npm test && npm run build && npm run test:dist` | the same checks as CI; a failing check stops the deploy and the previous deployment stays live |
| Output directory | `dist` | Astro's default |
| Node version | `web/.node-version` (24.14.0) | the build image should read it from the root directory; `engines` needs ≥ 22.18. Checked in the first build log, with the dependency install (`npm clean-install`, so the lockfile is enforced) |
| Environment variables | `ASTRO_TELEMETRY_DISABLED=1` only | no Supabase settings: accounts closed. No secret ever goes here (the build refuses a non-public key anyway) |
| Web Analytics | **off** | D-027: no analytics of any kind |

Pages installs dependencies from `web/package-lock.json`; `web/.npmrc` keeps install scripts off there too.

Rollback: Pages keeps every successful deployment; "Rollback to this deployment" restores an earlier production deployment (files and headers only, not DNS or zone settings). Then revert the bad commit on `main` so the next deploy does not bring it back; pause automatic production deployments while fixing if needed. Taking the site offline: remove both custom domains and the `www` record, then delete the Pages project (removing only the custom domains leaves `tavern-ledger.pages.dev` up). Always in that order: deleting the project while the apex record still points at `tavern-ledger.pages.dev` would leave a dangling name someone else could claim.

## 3. Domains, HTTPS and redirects

- `tavernledger.net` (apex): custom domain of the Pages project. Pages creates the DNS record (a proxied CNAME to `tavern-ledger.pages.dev`, flattened at the apex) and the certificate.
- `www.tavernledger.net` → `https://tavernledger.net` with a **301**, following Cloudflare's own guide for Pages: a proxied `A www 192.0.2.1` record (a documentation address that never answers; the redirect happens at Cloudflare's edge before any origin) and a **Bulk Redirect** list entry (`www.tavernledger.net` → `https://tavernledger.net`, 301, preserve query string, subpath matching, preserve path suffix) with a rule that enables the list. The universal certificate covers `www`.
- `tavern-ledger.pages.dev`: left as is. It serves the same files with the same headers, and every page (but the 404) names `https://tavernledger.net/...` as its canonical address, so search engines index the apex. A redirect would make checking a deployment harder.
- Zone settings (SSL/TLS): Always Use HTTPS **on**, minimum TLS version **1.2**, TLS 1.3 on. The encryption mode (Full/Flexible) does not apply to Pages, whose origin is Cloudflare itself.
- **DNSSEC: on.** The domain is on Cloudflare Registrar, which publishes the DS record itself. Checked after enabling with a validating resolver (the `ad` flag).
- CAA records: **not now.** Cloudflare picks among several certificate authorities for the universal certificate; a wrong CAA set breaks renewal. Revisit with HSTS preload.

## 4. Security headers

The build writes `dist/_headers` (`web/astro.config.mjs`), because the CSP depends on the build settings (the Supabase origin when accounts open); a static `web/public/_headers` could drift from the `<meta>` CSP in the pages. `test/build` checks that the header policy equals the pages' policy plus `frame-ancestors`.

| Header | Value | Note |
|---|---|---|
| `Content-Security-Policy` | `default-src 'none'; script-src 'none'; style-src 'self'; img-src 'self'; connect-src 'none'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'` | With accounts open: `script-src 'self'`, `form-action 'self'` and `connect-src` the Supabase origin. Also as `<meta>` in every page (all but `frame-ancestors`, which only works as a header). |
| `Strict-Transport-Security` | `max-age=86400; includeSubDomains` | One day while the site is new (security review): a mistake then locks returning visitors out for a day, not a year. Raise to `31536000` once the site has run cleanly for a couple of weeks (T-104c notes). No `preload` yet: preload is hard to undo; add it once the site has run cleanly for a while. `includeSubDomains` commits every future web subdomain to HTTPS: any new one must be proxied through Cloudflare (covered by the universal certificate) or have its own certificate. |
| `X-Content-Type-Options` | `nosniff` | |
| `X-Frame-Options` | `DENY` | For browsers without `frame-ancestors`. |
| `Referrer-Policy` | `no-referrer` | Also as `<meta>`. |
| `Cross-Origin-Opener-Policy` | `same-origin` | |
| `Permissions-Policy` | `camera=(), microphone=(), geolocation=(), payment=()` | |
| `Cache-Control` on `/_astro/*` | `public, max-age=31536000, immutable` | File names there carry a content hash. HTML keeps Pages' default (revalidate). |

Cloudflare features that **change the HTML** must stay off, or the CSP blocks what they inject (and some set cookies the privacy policy does not list):

- Web Analytics (Pages project and zone): off (D-027).
- Email Address Obfuscation (Scrape Shield): **off**. It rewrites addresses in the HTML and injects a script; the privacy page will show `privacy@tavernledger.net`.
- Bot Fight Mode: **off**. It turns on JavaScript detections, which inject a script under `/cdn-cgi/challenge-platform/` that cannot be turned off separately, and set a cookie.
- Rocket Loader: off.
- Speed Brain: **off**. On by default on Free zones; it adds a `Speculation-Rules` header so browsers prefetch pages the visitor has not asked for.

Cookies: Cloudflare's own security features (Browser Integrity Check, challenges, the DDoS protection) can set strictly necessary security cookies such as `__cf_bm` or `cf_clearance` on a visitor who is challenged; the site itself sets none. The privacy policy should say so (left for T-104b with the other `[TO BE CHECKED]` items).

Check after go-live: the HTML served at `https://tavernledger.net/` is byte-for-byte the built `dist/index.html` (no injected script), no `Set-Cookie` or `Speculation-Rules` on a normal request, every header above is present on a page, an asset and the 404 (`curl -I`).

## 5. DNS and mail records

| Name | Type | Content | Set by |
|---|---|---|---|
| `tavernledger.net` | CNAME (flattened, proxied) | `tavern-ledger.pages.dev` | Pages custom domain |
| `www` | A (proxied) | `192.0.2.1` | by hand (redirect only) |
| `tavernledger.net` | MX ×3 | `*.mx.cloudflare.net` (three hosts, different priorities) | Email Routing |
| `tavernledger.net` | TXT | `v=spf1 include:_spf.mx.cloudflare.net ~all` | Email Routing |
| `cf2024-1._domainkey` | TXT | Email Routing's DKIM key | Email Routing |
| `_dmarc` | TXT | `v=DMARC1; p=reject; sp=reject; adkim=s; aspf=s` | by hand |

DMARC: nothing sends mail as `@tavernledger.net` yet, so the strictest policy costs nothing and stops others from spoofing the domain. Forwarding is not affected: Email Routing keeps the original `From:` (the sender's domain, judged by the sender's DMARC) and rewrites only the envelope sender to Cloudflare's own forwarding domain (SRS). No `rua` (aggregate reports) for now: it would fill the user's mailbox with reports about a domain that sends nothing. When the outbound sender exists (section 7), its SPF include and DKIM key are added and DMARC stays at `reject` once its mail passes.

## 6. Contact mail (Cloudflare Email Routing)

- Addresses: `contact@tavernledger.net` and `privacy@tavernledger.net`, both forwarded to the destination the user gives (verified by Cloudflare's email to that address; the user taps the link on their phone).
- Catch-all: **off** (mail to any other address is rejected), so the domain does not collect spam for made-up names.
- Inbound only: Email Routing cannot send. Replies from the user's own mailbox come from their personal address unless an outbound sender is set up later; the privacy policy only promises a contact address.
- Forwarding can still fail for a sender whose own DMARC is strict and whose mail fails it (Cloudflare checks incoming SPF/DKIM and rejects by the sender's policy); tested after setup with a message from an outside address.
- Data: messages pass through Cloudflare in transit under the Cloudflare Customer DPA. Whether Cloudflare keeps a copy or logs beyond delivery status is **unverified** (the postmaster page does not say); `docs/legal/data-inventory.md` keeps it as `[TO BE CHECKED]`.

## 7. Outbound sender for Supabase's auth emails (comparison; nothing set up)

Supabase Auth needs a sender for sign-up confirmation, email change and password reset. Its built-in sender is for testing only (2 emails an hour). Two ways in: **custom SMTP** (host, port, user, password; Supabase then allows 30 emails an hour by default, adjustable) or the **Send Email Hook** (Supabase calls our HTTP endpoint and we send the mail ourselves; available on the Free plan). Custom SMTP needs no code of ours.

| | Cloudflare Email Sending | Brevo |
|---|---|---|
| Status | Public beta since 2026-04-16; SMTP submission added in beta on 2026-06-08 | Generally available |
| Works as Supabase custom SMTP | Yes in principle: `smtp.mx.cloudflare.net`, port **465** (implicit TLS), user `api_token`, password an API token with "Email Sending: Edit". Supabase's docs show port 587; 465 support is **unverified** | Yes: `smtp-relay.brevo.com`, port 587 (STARTTLS), password is an SMTP key |
| Cost | **Needs Workers Paid ($5/month)**; 3,000 emails a month included, then $0.35 per 1,000. Not available on Workers Free | Free plan: 300 emails a day (**unverified** on a primary page: Brevo's help pages refused our fetcher) |
| Branding | None stated | Free plan adds a "Sent with Brevo" mark to emails (**unverified**, search summary) |
| Domain setup | Same dashboard; adds SPF, DKIM (`cf-bounce` selector), MX on a bounce subdomain and DMARC | DKIM and DMARC records added by hand in Cloudflare DNS |
| Company, data location | Cloudflare, Inc. (USA); Customer DPA v6.4 (EU–US Data Privacy Framework, SCCs as fallback); processing may be outside the EEA; Email Service not named in the DPA annex we read | Sendinblue SAS (Brevo), France; data hosted in the EU (OVH France and Germany, Google Cloud Belgium) (**unverified**, search summary); DPA part of the terms |
| Beta caveat | Cloudflare's terms exclude beta services from the SLA and limit liability for their data | — |
| New credential to keep | A Cloudflare API token (stored only in Supabase's SMTP settings) | A Brevo SMTP key (stored only in Supabase's SMTP settings) |

**Claude's recommendation was Brevo** (plain SMTP on Supabase's documented port, free at this volume, EU company, not a beta). **The user chose Cloudflare Email Sending** (2026-10-09, D-030): one provider and one dashboard for the domain, the site and all mail. Known costs of that choice: Workers Paid ($5/month) and a beta (no SLA; SMTP on port 465, which Supabase's docs do not show, so test it first or use the Send Email Hook instead).

**Setup belongs to session 022** (with the hosted Supabase project, after #016 merges), not to this session. 022 will:

1. Onboard `tavernledger.net` in Email Sending and add its records (MX and SPF on the `cf-bounce` subdomain, DKIM, anything else the dashboard asks for).
2. Keep **one** SPF record at the apex: if sending needs anything at the apex, merge it into the existing `v=spf1 include:_spf.mx.cloudflare.net ~all`; never a second `v=spf1` record (two SPF records make SPF fail for both).
3. Take over `_dmarc`, keeping it passing for both forwarded and sent mail, and keep `p=reject` once sent mail passes SPF or DKIM in alignment.
4. Check the privacy policy's sender row and the data inventory with the provider's actual terms.

## 7a. DNS records owned by session 021 (do not break)

| Name | Type | Purpose | Notes for 022 |
|---|---|---|---|
| `tavernledger.net` | CNAME (flattened, proxied) to `tavern-ledger.pages.dev` | the website | managed by the Pages custom domain; do not edit by hand |
| `www` | A `192.0.2.1` (proxied) | `www` → apex redirect (Bulk Redirect) | the address is a placeholder; the redirect needs the record proxied |
| `tavernledger.net` | MX ×3 (`*.mx.cloudflare.net`) | Email Routing for `contact@` and `privacy@` | sending uses its own MX on `cf-bounce`, not the apex |
| `tavernledger.net` | TXT `v=spf1 include:_spf.mx.cloudflare.net ~all` | Email Routing | merge into it, never add a second SPF record |
| `cf2024-1._domainkey` | TXT | Email Routing DKIM | sending uses its own selector |
| `_dmarc` | TXT `v=DMARC1; p=reject; sp=reject; adkim=s; aspf=s` | anti-spoofing while nothing sends | **022 takes it over** when sending starts |

## 8. Order of the Cloudflare changes (each checked before the next)

1. Zone: Web Analytics off, Email Address Obfuscation off, Bot Fight Mode off, Rocket Loader off, Speed Brain off; Always Use HTTPS on, minimum TLS 1.2.
2. Pages: connect GitHub (app on `KBroneZ/tavern-ledger` only; the user approves on GitHub if asked), create `tavern-ledger` with the settings in section 2, previews **None**, Web Analytics off. First build from `main`.
3. Check `https://tavern-ledger.pages.dev` (pages, headers, no script).
4. Custom domain `tavernledger.net`; wait for the certificate; check headers and HTTPS.
5. `www`: DNS record, Bulk Redirect list and rule; check the 301.
6. DNSSEC on.
7. Email Routing: destination address, `contact@` and `privacy@`, catch-all off; check MX, SPF, DKIM; add DMARC.
8. Final checks (section 4) and `dig`/`Resolve-DnsName` of every record in section 5.

## Second opinion (ChatGPT, 2026-10-09)

Code and security reviews (Claude subagents) found no CRITICAL or HIGH; applied: HSTS starts at one day, no canonical link on `noindex` pages, the open-mode build test checks that every script a page loads exists, the privacy draft names Cloudflare's possible security cookies, the takedown order above.

Reviewed this plan and the site's code before any Cloudflare change. Applied: closed-mode CSP tightened to `script-src 'none'` and `form-action 'none'`; Speed Brain added to the features turned off; security cookies from Cloudflare's own protection noted; rollback and takedown steps completed (`pages.dev` stays up unless the project is deleted); canonical links to the apex; a test that no closed-build file holds a project URL or key; header checks for COOP and asset caching; wording on DNSSEC, forks and the `includeSubDomains` commitment corrected; an outside-sender forwarding test added. Not applied, with reasons: "do not publish while the privacy policy is a draft" (the user decided to publish with accounts closed; the site collects nothing itself, and the drafts say what is still open); "guard against accidental account opening" (opening means setting two build variables in the Pages project, a deliberate T-104a step owned by session 022, and the build refuses a half or non-public setting); "verify Brevo's branding" (moot: the user chose Cloudflare Email Sending).

## Sources (read 2026-10-09)

- Pages branch build controls: https://developers.cloudflare.com/pages/configuration/branch-build-controls/
- Pages preview deployments: https://developers.cloudflare.com/pages/configuration/preview-deployments/
- Pages www redirect: https://developers.cloudflare.com/pages/how-to/www-redirect/
- Pages rollbacks: https://developers.cloudflare.com/pages/configuration/rollbacks/
- Speed Brain: https://developers.cloudflare.com/speed/optimization/content/speed-brain/
- Cloudflare cookies: https://developers.cloudflare.com/fundamentals/reference/policies-compliances/cloudflare-cookies/
- JavaScript detections (Bot Fight Mode): https://developers.cloudflare.com/cloudflare-challenges/challenge-types/javascript-detections/
- Email Routing postmaster: https://developers.cloudflare.com/email-routing/postmaster/
- Email Sending pricing: https://developers.cloudflare.com/email-service/platform/pricing/
- Email Sending setup and SMTP: https://developers.cloudflare.com/email-service/get-started/send-emails/ and https://developers.cloudflare.com/changelog/post/2026-06-08-smtp-submission/
- Email Sending public beta: https://developers.cloudflare.com/changelog/post/2026-04-16-email-sending-public-beta/
- Cloudflare Customer DPA: https://www.cloudflare.com/cloudflare-customer-dpa/
- Brevo SMTP: https://developers.brevo.com/docs/smtp-integration
- Brevo free plan, branding, data location, DPA (help pages, not readable by our fetcher; search summaries only): https://help.brevo.com/hc/en-us/articles/208580669 , https://help.brevo.com/hc/en-us/articles/360001005510 , https://help.brevo.com/hc/en-us/articles/15403782599570
- Supabase custom SMTP: https://supabase.com/docs/guides/auth/auth-smtp ; rate limits: https://supabase.com/docs/guides/auth/rate-limits ; Send Email Hook: https://supabase.com/docs/guides/auth/auth-hooks/send-email-hook
