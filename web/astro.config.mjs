// Static site (D-020). Nothing is inlined into the HTML, so the
// Content-Security-Policy can allow scripts and styles from 'self' only.
import { readdirSync, rmSync, writeFileSync } from "node:fs";
import { defineConfig } from "astro/config";
import { loadEnv } from "vite";
import { contentSecurityPolicy, readConfig } from "./src/lib/config.ts";

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
          "  Strict-Transport-Security: max-age=31536000; includeSubDomains",
          "",
          // File names under /_astro/ carry a content hash, so they never change.
          "/_astro/*",
          "  Cache-Control: public, max-age=31536000, immutable",
          "",
        ].join("\n");
        writeFileSync(new URL("_headers", dir), headers);
        // Astro still bundles the account pages' scripts, though no page
        // loads them when accounts are closed: ship none (test/build checks
        // that no page has a script tag).
        if (!config) {
          const assets = new URL("_astro/", dir);
          for (const name of readdirSync(assets)) {
            if (name.endsWith(".js")) rmSync(new URL(name, assets));
          }
        }
      },
    },
  };
}

export default defineConfig({
  output: "static",
  // Same port as site_url in supabase/config.toml.
  server: { host: "127.0.0.1", port: 3000 },
  devToolbar: { enabled: false },
  build: { inlineStylesheets: "never" },
  vite: { build: { assetsInlineLimit: 0 } },
  integrations: [securityHeaders()],
});
