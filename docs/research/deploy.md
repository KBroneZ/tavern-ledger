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
| Access granted | Cloudflare Pages GitHub app, installed on `KBroneZ/tavern-ledger` only: read and write to code, checks, deployments, pull requests and administration of that repo | Token scoped to Pages edit on the account |
| Build | Cloudflare's build image, from the same lockfile | Our CI runner |
| Main risk | A compromised Cloudflare account could write to the repo through the app (branch protection and review of `main` still apply) | A leaked token can deploy anything to the site |

Git integration has no long-lived secret anywhere and adds nothing to the dependency list. Its cost is the app's write access to this one repo; the user approved it. CI (GitHub Actions) keeps testing every PR as today; Pages only builds `main`.

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

**Claude's recommendation was Brevo** (plain SMTP on Supabase's documented port, free at this volume, EU company, not a beta). **The user chose Cloudflare Email Sending** (2026-10-09, D-030): one provider and one dashboard for the domain, the site and all mail. Known costs of that choice: Workers Paid ($5/month) and a beta (no SLA; SMTP on port 465, which Supabase's docs do not show, so test it first or use the Send Email Hook instead). **Changed on 2026-10-10 (D-034): the user chose Brevo.** Section 10.4 is kept for a later switch; the Brevo setup is section 10.4b.

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

## 9. Live state (2026-10-09, end of session 021)

Done in this order, each checked:

1. Zone: Always Use HTTPS on, minimum TLS 1.2 (a client-side TLS 1.1 test was inconclusive: the local clients no longer speak 1.1), TLS 1.3 on. Email Address Obfuscation turned off (it was on). Already off: Bot Fight Mode, AI Labyrinth, Real User Monitoring, Speed Brain. Automatic HTTPS Rewrites is on; it changes nothing today (the served pages are byte-for-byte the build).
2. Pages project `tavern-ledger` from `main` with the settings in section 2; first build: Node 24.14.0, 56 unit tests, build, build checks, "Parsed 2 valid header rules". Preview deployments set to **None**.
3. `https://tavern-ledger.pages.dev`: pages identical to `dist`, every header present, no script.
4. Custom domain `tavernledger.net`: Active; the CNAME to `tavern-ledger.pages.dev` was created by Pages.
5. `www`: `A 192.0.2.1` proxied; Bulk Redirect list `tavernledger_www` (preserve query string, subpath matching, preserve path suffix) and rule "tavernledger www to apex": `https://www.tavernledger.net/privacy/?a=1` → 301 `https://tavernledger.net/privacy/?a=1`; `http://www…` also 301.
6. DNSSEC enabled; the DS record was still being added at the registrar at the end of the session. **Check later** that `dig DS tavernledger.net` answers and validating resolvers set the `ad` flag.
7. Email Routing: `contact@` and `privacy@` → the user's mailbox (already verified on the account), catch-all disabled (drop). Records resolve: MX `route1/2/3.mx.cloudflare.net`, `v=spf1 include:_spf.mx.cloudflare.net ~all`, DKIM `cf2024-1._domainkey`, `_dmarc` `v=DMARC1; p=reject; sp=reject; adkim=s; aspf=s`.
8. `https://tavernledger.net`: the six pages are byte-for-byte the build, no injected script, no `Set-Cookie` or `Speculation-Rules`, all security headers, 404 with the CSP, `http` → `https` 301.

Seen in the responses and not ours: `Access-Control-Allow-Origin: *` (Pages default for static files; harmless for public pages with no credentials) and Cloudflare's `Report-To`/`NEL` headers (browsers report failed connections to Cloudflare; noted for the privacy policy, T-104b).

Not done: a test message to `contact@` from an outside address (the user can send one from their phone), CAA, HSTS raised to a year (after a couple of clean weeks).

## 10. Hosted backend (T-104a hosted, session 022)

Plan for the hosted Supabase project and the auth emails. Project `tavern-ledger`, ref `vgttflmexobrqhcyxjks`, org "Tavern Ledger" (Free plan), Central EU (Frankfurt). Created by the hub with the user on 2026-10-09: Data API on, "automatically expose new tables" off (the migrations grant explicitly), automatic RLS on, no migration applied. Facts read on the providers' pages and dashboards on 2026-10-09 (sources at the end); **unverified** marks what no primary page confirmed.

What the dashboards showed before any change: the current JWT signing key is ECC P-256 (ES256), the legacy HS256 secret is a "previous key" that still verifies old tokens; the new publishable (`sb_publishable_…`) and secret (`sb_secret_…`) keys exist and the legacy `anon`/`service_role` keys are still enabled; no GitHub connection; Cloudflare Email Sending is not enabled on the account ("currently only available with the Workers Paid plan").

### 10.1 How migrations and Edge Functions reach the hosted project

**Choice: the Supabase GitHub integration, deploying `main` to production, branching off.** Supabase made it available on all plans in April 2026: on each push to `main` it applies new migrations, deploys the Edge Functions declared in `supabase/config.toml` and the buckets declared there. It ignores auth and API settings and the seed.

| | GitHub integration (chosen) | CLI from GitHub Actions | CLI from this PC |
|---|---|---|---|
| Long-lived secret | none | a personal access token (scoped to the project) and the database password as GitHub secrets | the same two, on this PC |
| New dependency | none | none (the CLI is already pinned) | none |
| Access granted | the Supabase GitHub app on `KBroneZ/tavern-ledger` (the user approves on GitHub) | token with write access to the project | same |
| Who deploys | every merge to `main`, like the site (D-030) | every merge to `main` | whoever runs it; easy to forget or to run from the wrong branch |
| Main risk | a compromised Supabase account could read the repo through the app | a leaked token or password changes production directly | same, plus drift between `main` and production |

No secret anywhere and the same "merge is deploy" rule as Pages decided it. What the integration does not cover is set once by hand and listed in 10.3 to 10.6: auth settings, function secrets, the Vault entry for the sweep, SMTP. A migration runs in one transaction on the integration (no `create index concurrently`); none of ours needs more. A failed deploy shows in the dashboard (Branches / integration logs); after each merge that touches `supabase/`, check it there and with the parity query in 10.8. Rollback of a bad migration is a new migration (nothing is reverted by hand on production). **Fallback** if the integration fails or the user declines it: the CLI from this PC with a project-scoped access token (30-day expiry) kept in Windows Credential Manager by `supabase login`, never in a file or in GitHub.

`config.toml` declares the three functions. `upload-game` and `delete-account` keep `verify_jwt = true` (the default): Supabase's auth-headers page says the check accepts legacy HS256 and new asymmetric (ES256) user tokens, and the publishable and secret keys (**unverified** on this project until one real ES256 user token passes `upload-game` after the deploy), and each function still authenticates the caller itself (`/auth/v1/user`). `sweep` has `verify_jwt = false`: its caller is the database's own job, which sends only the sweep token (no key to keep in Vault), and the function refuses anything but the service key or a valid token before doing any work. If the gateway ever refuses valid ES256 tokens (reported in a few GitHub issues; **unverified** for this project), switch that function to `verify_jwt = false` in a PR: its own checks are the real gate.

### 10.2 Where each secret lives

| Secret | Where | Who can see it |
|---|---|---|
| Database password | set at project creation; not needed by the integration | the user (dashboard can reset it) |
| `service_role` / `sb_secret_…` keys | Supabase only; the functions get `SUPABASE_SERVICE_ROLE_KEY` from the platform | never copied anywhere |
| Sweep token | Vault, created by the migration from 32 random bytes inside the database (`sweep_token`); compared by the `sweep` function through a service-role-only RPC | nobody needs to read it |
| Cloudflare API token for SMTP (Email Sending: Edit, this account only) | Supabase's SMTP password field only (copied from Cloudflare's dashboard to Supabase's by clipboard, never shown in chat, logs or files) | Supabase, Cloudflare |
| Publishable key, project URL | Pages build variables, the desktop default, docs | public by design |

Not secrets: `delete-account` reads `SITE_ORIGINS`, which `config.toml` sets to the live site and the local preview; the GitHub integration copies `[edge_runtime.secrets]` to the hosted project on every deploy (found on the first deploy, D-036), so that value is what production uses; the Vault entry `project_url` (`https://vgttflmexobrqhcyxjks.supabase.co`) for the sweep schedule is set once with SQL.

### 10.3 Auth settings (dashboard, mirroring `config.toml` where it applies)

| Setting | Value | Why |
|---|---|---|
| Site URL | `https://tavernledger.net` | the default redirect |
| Redirect URLs | `https://tavernledger.net/account/` | the only page the site redirects to (sign-up confirmation). The desktop's `http://127.0.0.1:<port>/desktop-callback/<state>` is accepted by the loopback rule on the local auth version; checked on the hosted one first and only if refused, `http://127.0.0.1:*/desktop-callback/*` is added |
| Allow new users to sign up | **off** | sign-ups stay closed until the user says so |
| Confirm email | on | |
| Secure email change, secure password change | on | as `config.toml` |
| Minimum password length | 10, no character classes | as `config.toml` and the site's check |
| Leaked password protection | not available | Pro plan only; revisit with Pro |
| Email OTP / link expiry | 3600 s | as `config.toml` |
| Anonymous sign-ins, phone, OAuth providers, MFA | off | not used |
| Rate limits | emails 30 an hour (Supabase's default once custom SMTP is on), sign-ups and sign-ins 30 per 5 minutes per IP, token verifications 30 per 5 minutes per IP, token refreshes 150 per 5 minutes per IP | the defaults; low enough for a project that is not public |
| Email templates | Supabase's defaults (the magic link and confirmation go through `/auth/v1/verify`, which the desktop and the site's PKCE flow need) | branding later |
| GraphQL (`pg_graphql`) | off if the extension is on | as the local schema; nothing uses it |

### 10.4 Auth emails: custom SMTP with Cloudflare Email Sending (D-030, replaced by 10.4b)

- **Needs the user's OK to pay:** Email Sending requires Workers Paid (3,000 emails a month included, then $0.35 per 1,000; the plan's own price is shown in the dashboard at purchase). It is a beta.
- Onboard `tavernledger.net` in Email Sending. Its records live on the `cf-bounce` subdomain (MX and SPF) plus a DKIM key on its own selector; none goes on the apex, so the apex SPF record of Email Routing stays as it is. If the dashboard asks for anything at the apex, merge it into the single `v=spf1` record.
- API token: "Email Sending: Edit", this account only, no other permission, no expiry shorter than the plan for rotation (yearly, noted in the plan).
- Supabase SMTP: host `smtp.mx.cloudflare.net`, port **465** (implicit TLS; Cloudflare refuses STARTTLS on 587), user `api_token`, sender `noreply@tavernledger.net`, name "Tavern Ledger". Supabase's docs only show port 587: if it cannot connect on 465, stop and ask the user (Send Email Hook with our own code, or Brevo).
- DMARC (`_dmarc`, taken over from 021): keep `p=reject; sp=reject; adkim=s` and relax `aspf` to `r`. Sent mail carries `From: noreply@tavernledger.net`, DKIM `d=tavernledger.net` (aligned under strict rules) and an envelope on `cf-bounce.tavernledger.net` (aligned for SPF only under relaxed rules). Forwarded mail is not affected (Email Routing keeps the sender's own `From:`).
- Check: a magic link to the user's own address, then the received headers (`Authentication-Results`: SPF, DKIM and DMARC `pass`).

### 10.4b Auth emails: custom SMTP with Brevo (D-034)

- First read on Brevo's own pages (the help pages refused our fetcher on 2026-10-09: try the browser): the free plan's daily limit and branding, the legal entity, where messages and logs are stored and for how long, and the DPA. Put the answers in `docs/legal/data-inventory.md` and the privacy draft; anything still unclear stays `[TO BE CHECKED]`.
- **[user]** Create the Brevo account (free plan) and accept its terms; the user does this, never Claude.
- Authenticate `tavernledger.net` in Brevo: add exactly the records its dashboard shows (DKIM and any verification record). If it asks for an SPF include at the apex, merge it into the single `v=spf1` record next to Email Routing's.
- SMTP key: one key for Supabase only, named for it, copied from Brevo's dashboard straight into Supabase's SMTP password field (never shown in chat, logs or files).
- Supabase SMTP: host `smtp-relay.brevo.com`, port 587 (STARTTLS), user the SMTP login Brevo shows, sender `noreply@tavernledger.net`, name "Tavern Ledger".
- DMARC (`_dmarc`): keep `p=reject`. Check that Brevo signs with DKIM `d=tavernledger.net` after authentication (aligned) before relying on it; relax `aspf` to `r` only if Brevo's envelope sits on a subdomain of ours.
- Check: a magic link to the user's own address, then the received headers (`Authentication-Results`: DKIM and DMARC `pass`).

### 10.5 Sweep schedule

New migration `20261010090000_sweep_schedule.sql`: `pg_cron` and `pg_net`, a Vault secret `sweep_token` made inside the database from 32 random bytes, `public.sweep_token_valid(text)` (service role only, compares SHA-256 digests, false when either side is missing), and `private.run_sweep()` scheduled daily at 03:17 UTC (nobody but the database owner may call it). It reads `project_url` and `sweep_token` from Vault and calls `POST <project_url>/functions/v1/sweep` with the token in `X-Sweep-Token`, 30 s timeout. Without `project_url` (the local stack, or before it is set) it sends nothing and logs a warning. The `sweep` function accepts either the service key (manual runs, as today) or a well-formed token that the database confirms; a token it cannot check is a 500, never a run.

Known limit (second opinion and security review): while a request waits in `pg_net`'s queue (seconds), its headers, and so the token, are readable by database roles that can read `net.http_request_queue`. Supabase's own admin role grants `anon` and `authenticated` the `net` schema, its functions and the queue when the extension is created, and a migration cannot take those grants back (they are not ours; checked on the local stack). They are unreachable in practice: both roles cannot log in to the database, and the Data API exposes only `public`, where no function of ours calls `net`. Re-checked after deploy (10.8 step 3). The token only lets someone run the sweep early, which is safe at any time. If it ever leaks, delete the Vault entry and re-run the `do` block of the migration to make a new one.

Failures are not silent: every sweep that finishes leaves a row in `private.sweep_runs` (time and counts, kept 90 days; only the owner can read it). `cron.job_run_details` says the request was queued and `net._http_response` holds the function's answer for six hours. Check after the first run, then weekly until there is alerting: `select max(ran_at) from private.sweep_runs` must be under 25 hours old. That check is the only one that catches a missing `project_url` (the job then logs a warning but pg_cron still records it as succeeded) or a sweep that fails before housekeeping (no row is written).

### 10.6 Function secrets and checks

- No function secret to set by hand: the integration sets `SITE_ORIGINS` from `config.toml` (D-036). `SUPABASE_URL` and `SUPABASE_SERVICE_ROLE_KEY` come from the platform; they are the legacy service key while legacy keys stay enabled. Before the legacy keys are ever disabled, the functions must read the new secret key instead (follow-up, noted in the plan).
- Client IP on the hosted gateway: read one refused request's headers in the function logs to see whether `CF-Connecting-IP` or `X-Forwarded-For` carries the client (D-025 left it **unverified**).

### 10.7 Site and desktop

- Pages, **production** environment only: `PUBLIC_SUPABASE_URL=https://vgttflmexobrqhcyxjks.supabase.co`, `PUBLIC_SUPABASE_ANON_KEY=<sb_publishable_…>`. New build setting `PUBLIC_SIGNUPS_OPEN` (unset = closed): while closed, `/signin/` shows the sign-in form and "Sign-up is not open yet" instead of the sign-up form. Redeploy, then check the CSP allows only the project's origin and that sign-in, account, export and deletion work.
- Desktop: when the data folder has no `server.json`, the app uses the hosted project's URL and publishable key built in (`crates/uploader`); `server.json` still overrides it for the local stack. Upload stays off until the user signs in and turns it on.

### 10.8 Order of the changes (each checked before the next)

1. **[user]** Approve the Supabase GitHub app on `KBroneZ/tavern-ledger` only; connect it with "Deploy to production" on `main`, branching off, Supabase directory `supabase`.
2. Auth settings (10.3) with sign-ups off, before anything is deployed; check `disable_signup: true` on the public `/auth/v1/settings`.
3. Merge this session's PR: the integration applies the four migrations and deploys the three functions. Check: migrations listed as applied; parity query (tables, columns, constraints, indexes, policies, grants, functions and their security, triggers, buckets) gives the same fingerprint on the local stack and the hosted project; RLS on every table in `public`; `anon` and `authenticated` hold only the grants in the migrations; the bucket is private with its policies; no default privileges that would hand a future table to `anon` or `authenticated`; `pg_graphql` off or exposing nothing; with the publishable key alone, every REST, RPC and Storage write is refused and a sign-up and an email link for an unknown address are refused; functions answer `401` without a token and the right CORS on `delete-account`'s preflight; `net` is still not in the Data API's exposed schemas. After **every** merge that touches `supabase/`: `/auth/v1/settings` still says `disable_signup: true`, and the site URL and redirect list are unchanged (the integration's docs say it ignores auth settings by default: "All other configurations, including API, Auth, and seed files, are ignored by default"; checked, not assumed).
4. Vault `project_url`, then run the sweep once by hand (`select private.run_sweep()`) and read the response.
5. **[user]** Brevo account (D-034, replacing Workers Paid and Email Sending); then domain authentication, SMTP key, Supabase SMTP, DMARC (section 10.4b).
6. Pages variables and redeploy.
7. **[user]** End-to-end check (the user, from the phone, about five minutes): create their own account in Supabase (Authentication → Users → Add user, their address, auto-confirm), sign in on `https://tavernledger.net/signin/`, open the account page, download the export, delete the account. Meanwhile Claude sends one desktop-style magic link to that address to check SMTP and DMARC, and checks on the server that nothing of the account remains after deletion. (Claude does not create accounts or type passwords on a production site.)
8. Desktop default, then sign-ups stay closed until the user says so (**[user]**).

### 10.9 Live state (session 022)

1. 2026-10-09, before any deploy: "Allow new users to sign up" was **on** (Supabase's default for a new project) and was turned off first; the public `/auth/v1/settings` answers `disable_signup: true`. Confirm email was already on. Email provider: secure email change on (already), secure password change on, current password required to change it, minimum length 10 (was 6), email link expiry 3600 s. Site URL `https://tavernledger.net` (was `http://localhost:3000`); redirect URLs: `https://tavernledger.net/account/` only.
2. Cloudflare zone: Network Error Logging turned off (Network settings).
3. PR #40 merged to `main` with the integration **not** connected, so nothing reached the hosted project. Pending user decisions (plan, T-104a notes): the email sender (Workers Paid for Email Sending, or Brevo), the GitHub connection, the end-to-end test, the Vault `project_url` entry after the first deploy, opening sign-ups.
4. 2026-10-10, Brevo (D-034, section 10.4b): Brevo is Sendinblue SAS, Paris (RCS 498 019 298); its help pages say data is stored in the EU (OVH in France and Germany, Google Cloud in Belgium); the DPA is Appendix 3 of its Terms of Service (version of 2025-10-01), with Standard Contractual Clauses and the EU-US Data Privacy Framework for sub-processors outside the EU; the Free plan sends 300 emails a day and always adds a "Sent with Brevo" mark (read on Brevo's pages on 2026-10-10). The user created the account (free plan; the login is the user's personal address, never shown on the site). Domain authenticated with **manual** records (not Brevo's automatic setup, which would have taken access to the Cloudflare zone): TXT `brevo-code:…` at the apex, CNAME `brevo1._domainkey` and `brevo2._domainkey` to `b1`/`b2.tavernledger-net.dkim.brevo.com` (DNS only). No SPF change was needed; DMARC stays `p=reject; sp=reject; adkim=s; aspf=s` (Brevo accepted it; its suggested `p=none` was not applied). Sender `Tavern Ledger <noreply@tavernledger.net>` verified (DKIM `tavernledger.net`). Transactional settings: tracking set to **anonymous** (opens and clicks cannot be switched off there; the end-to-end test checks whether sign-in links are rewritten), logs kept **1 month**, previews **never stored**. Branded tracking subdomain not set up. One SMTP key, `supabase-auth`, made by the user and pasted by the user into Supabase only.
5. 2026-10-10, Supabase custom SMTP on: host `smtp-relay.brevo.com`, port 587, the Brevo SMTP login, sender `noreply@tavernledger.net`, name "Tavern Ledger", minimum interval 60 s.
6. 2026-10-10, Supabase GitHub integration connected: app installed on `KBroneZ/tavern-ledger` only, working directory `.`, deploy to production on `main`, branching off (Pro only). No migration was applied by connecting; the first deploy is the next merge to `main`.
7. 2026-10-10, first deploy (merge of PR #44): the four migrations applied and the three functions deployed. Public checks: `disable_signup: true`; sign-up and an email link for an unknown address refused (`signup_disabled`); with the publishable key, writes to `games` refused (`permission denied`), reads of `profiles` and `games` return nothing (granted `select`, filtered by RLS), the storage bucket lists nothing; GraphQL off; schema `net` not exposed; the three functions answer `401` without a token. **Found:** the integration copied `[edge_runtime.secrets]` from `config.toml` to the function secrets, so production's `SITE_ORIGINS` was the local `http://127.0.0.1:3000` and `delete-account` refused the live site (403 on its preflight). Fixed by D-036 (PR #45): after its deploy the preflight from `https://tavernledger.net` answers `204` with that origin and a foreign origin gets `403`; the dashboard still shows the site URL `https://tavernledger.net`, the one redirect URL and the Brevo SMTP settings (the integration left auth alone).
8. 2026-10-10, sweep schedule: the user added the Vault entry `project_url` and ran `select private.run_sweep()` in the SQL editor; `private.sweep_runs` got a row (`{"auth_events":0,"ip_failures":0,"upload_events":0}`), so the database reached `sweep` with the Vault token. From now on `pg_cron` runs it daily at 03:17 UTC.
9. 2026-10-10, Pages production variables `PUBLIC_SUPABASE_URL` and `PUBLIC_SUPABASE_ANON_KEY` (the publishable key) added next to `ASTRO_TELEMETRY_DISABLED`; `PUBLIC_SIGNUPS_OPEN` stays unset, so the site shows sign-in but no sign-up form. They take effect with the deploy of the PR that records this item.

### 10.10 Second opinion on this plan (ChatGPT, 2026-10-09)

The reviewer could not read the repository (its sandbox refused file reads), so it reviewed the approach from the plan's questions and Supabase's docs. Applied: the token can sit in `pg_net`'s queue table for seconds and is readable there by roles with a direct connection (documented as a known limit; the token can only start a sweep); a queued request is not a finished sweep, so finished sweeps now leave a row in `private.sweep_runs`; closed sign-ups are checked on the server (`/auth/v1/settings`), and the post-deploy checks include writes refused with the publishable key alone, default privileges and `pg_graphql`; DMARC alignment is confirmed from the received headers, not assumed. Not applied: "rehearse the first deploy from the remote state" (the hosted database has no migration yet, which is the same start as `supabase db reset`, run locally before the merge); "rotate the sweep token" on a schedule (it only allows an early sweep; rotation steps are written above).

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
- Supabase GitHub integration: https://supabase.com/docs/guides/deployment/branching/github-integration ; on all plans: https://supabase.com/changelog/44713-developer-update-april-2026
- Edge Functions auth headers and `verify_jwt`: https://supabase.com/docs/guides/functions/auth-headers ; function secrets: https://supabase.com/docs/guides/functions/secrets
- Scheduling Edge Functions (pg_cron, pg_net, Vault): https://supabase.com/docs/guides/functions/schedule-functions ; pg_net headers in the queue: https://supabase.com/docs/guides/troubleshooting/database-roles-can-read-request-headers-queued-by-pg_net-ad6357
- Password security (leaked password protection, Pro only): https://supabase.com/docs/guides/auth/password-security ; redirect URLs: https://supabase.com/docs/guides/auth/redirect-urls
- Cloudflare Email Sending SMTP (port 465 only): https://developers.cloudflare.com/email-service/api/send-emails/smtp/
- Supabase custom SMTP: https://supabase.com/docs/guides/auth/auth-smtp ; rate limits: https://supabase.com/docs/guides/auth/rate-limits ; Send Email Hook: https://supabase.com/docs/guides/auth/auth-hooks/send-email-hook
