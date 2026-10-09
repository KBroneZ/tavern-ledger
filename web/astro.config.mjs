// Static site (D-020). Nothing is inlined into the HTML, so the
// Content-Security-Policy can allow scripts and styles from 'self' only.
import { writeFileSync } from "node:fs";
import { defineConfig } from "astro/config";
import { loadEnv } from "vite";
import { contentSecurityPolicy, readConfig } from "./src/lib/config.ts";

/** Writes dist/_headers (Cloudflare Pages) with the same policy as the pages. */
function securityHeaders() {
  return {
    name: "tavern-ledger-security-headers",
    hooks: {
      "astro:build:done": ({ dir }) => {
        const config = readConfig(loadEnv("production", process.cwd(), "PUBLIC_"));
        const headers = [
          "/*",
          `  Content-Security-Policy: ${contentSecurityPolicy(config.url)}; frame-ancestors 'none'`,
          "  X-Content-Type-Options: nosniff",
          "  Referrer-Policy: no-referrer",
          "  Cross-Origin-Opener-Policy: same-origin",
          "  Permissions-Policy: camera=(), microphone=(), geolocation=(), payment=()",
          "  Strict-Transport-Security: max-age=31536000; includeSubDomains",
          "",
        ].join("\n");
        writeFileSync(new URL("_headers", dir), headers);
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
