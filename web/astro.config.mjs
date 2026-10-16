// Static site (D-020). Nothing is inlined into the HTML, so the
// Content-Security-Policy can allow scripts and styles from 'self' only.
import { existsSync, readdirSync, renameSync, rmSync, writeFileSync } from "node:fs";
import { defineConfig } from "astro/config";
import { loadEnv } from "vite";
import { contentSecurityPolicy, NO_STORE_PAGES, readConfig, TURNSTILE_PAGES, WASM_WORKER } from "./src/lib/config.ts";

/**
 * The simulator's worker alone gets the same policy plus 'wasm-unsafe-eval'
 * (T-606, D-090), nothing else: it compiles the engine, the page does not.
 * Cloudflare Pages applies every matching rule in file order and joins
 * repeated headers with a comma, which would make two policies that both
 * apply; so the rule first drops the site-wide one ("! Content-Security-Policy")
 * and then sets its own (Pages runs a rule's removals before its values). A
 * worker gets its policy from its own script's response, hence this rule.
 */
function simulatorHeaders(config, dir) {
  if (!config) return [];
  const workers = readdirSync(new URL("_astro/", dir)).filter((name) => WASM_WORKER.test(name));
  if (workers.length !== 1) throw new Error(`expected one simulator worker in _astro/, found ${workers.length}`);
  const policy = `${contentSecurityPolicy(config.url, { wasm: true })}; frame-ancestors 'none'`;
  return [
    `/_astro/${workers[0]}`,
    "  ! Content-Security-Policy",
    `  Content-Security-Policy: ${policy}`,
    "",
  ];
}

/**
 * The pages with the anti-robot check (T-562) get the same policy plus
 * Cloudflare Turnstile's origin for its script and frame, the way the
 * simulator's worker gets its own: drop the site-wide policy, set this one.
 * The page's own meta policy says the same (layouts/Base.astro). Accounts
 * closed: no form, no rule.
 */
function turnstileHeaders(config) {
  if (!config) return [];
  const policy = `${contentSecurityPolicy(config.url, { turnstile: true })}; frame-ancestors 'none'`;
  return TURNSTILE_PAGES.flatMap((path) => [path, "  ! Content-Security-Policy", `  Content-Security-Policy: ${policy}`, ""]);
}

/** Writes dist/_headers (Cloudflare Pages) with the same policy as the pages. */
function securityHeaders() {
  return {
    name: "tavern-ledger-security-headers",
    hooks: {
      "astro:build:done": ({ dir }) => {
        // null: accounts closed, so the policy allows no connection at all.
        const config = readConfig(loadEnv("production", process.cwd(), "PUBLIC_"));
        const headers = [
          "/*",
          `  Content-Security-Policy: ${contentSecurityPolicy(config?.url ?? null)}; frame-ancestors 'none'`,
          "  X-Content-Type-Options: nosniff",
          "  X-Frame-Options: DENY",
          "  Referrer-Policy: no-referrer",
          "  Cross-Origin-Opener-Policy: same-origin",
          "  Permissions-Policy: camera=(), microphone=(), geolocation=(), payment=()",
          // One day while the site is new; raised to a year once it has run
          // cleanly (design notes).
          "  Strict-Transport-Security: max-age=86400; includeSubDomains",
          "",
          // File names under /_astro/ carry a content hash, so they never change.
          "/_astro/*",
          "  Cache-Control: public, max-age=31536000, immutable",
          "",
          // Pages about one account are never kept (Back after signing out).
          ...NO_STORE_PAGES.flatMap((path) => [path, "  Cache-Control: no-store", ""]),
          ...simulatorHeaders(config, dir),
          ...turnstileHeaders(config),
        ].join("\n");
        writeFileSync(new URL("_headers", dir), headers);
        // Cloudflare Pages serves the nearest 404.html: the Spanish one must
        // be es/404.html, not es/404/index.html (T-508).
        const es404 = new URL("es/404/index.html", dir);
        if (existsSync(es404)) {
          renameSync(es404, new URL("es/404.html", dir));
          rmSync(new URL("es/404/", dir), { recursive: true });
        }
        // Astro still bundles the account pages' scripts, though no page
        // loads them when accounts are closed: ship none (test/build checks
        // that no page has a script tag).
        if (!config) {
          const assets = new URL("_astro/", dir);
          for (const name of readdirSync(assets)) {
            if (name.endsWith(".js")) rmSync(new URL(name, assets));
          }
          // The language script (public/lang.js) too: no page loads it here.
          rmSync(new URL("lang.js", dir), { force: true });
        }
      },
    },
  };
}

export default defineConfig({
  // The public address (canonical links); the same files also answer on
  // tavern-ledger.pages.dev and on the local preview.
  site: "https://tavernledger.net",
  output: "static",
  // Same port as site_url in supabase/config.toml.
  server: { host: "127.0.0.1", port: 3000 },
  devToolbar: { enabled: false },
  // One page at a time: each page sets the language its strings are drawn
  // in (lib/i18n.ts, T-508), so pages must not render side by side.
  build: { inlineStylesheets: "never", concurrency: 1 },
  // The simulator's worker (lib/sim/simWorker.ts) is an ES module worker.
  // The minions' drawing is the app's own module, one source for both (T-372):
  // the dev server may serve it from outside web/.
  vite: {
    build: { assetsInlineLimit: 0 },
    worker: { format: "es" },
    server: { fs: { allow: [".", "../crates/desktop/ui/minion-art"] } },
  },
  integrations: [securityHeaders()],
});
