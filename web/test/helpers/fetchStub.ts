// A stubbed fetch for a real Supabase client (as in test/unit/api.test.ts):
// each request goes to the first route whose path matches.
import type { SupportedStorage } from "@supabase/supabase-js";

export type Route = (url: URL, init: RequestInit) => Response | Promise<Response>;

export function memoryStorage(): SupportedStorage {
  const items = new Map<string, string>();
  return {
    getItem: (k) => items.get(k) ?? null,
    setItem: (k, v) => void items.set(k, v),
    removeItem: (k) => void items.delete(k),
  };
}

/** Replaces globalThis.fetch; returns the "METHOD path?query" of each request, decoded. */
export function stubFetch(routes: Record<string, Route>): string[] {
  const seen: string[] = [];
  globalThis.fetch = (async (input: RequestInfo | URL, init: RequestInit = {}) => {
    const url = new URL(input instanceof Request ? input.url : String(input));
    seen.push(`${init.method ?? "GET"} ${url.pathname}${decodeURIComponent(url.search)}`);
    const route = Object.entries(routes).find(([path]) => url.pathname === path);
    if (!route) return new Response("{}", { status: 404 });
    return route[1](url, init);
  }) as typeof fetch;
  return seen;
}

export function json(body: unknown, status = 200, headers: Record<string, string> = {}): Response {
  return new Response(JSON.stringify(body), { status, headers: { "Content-Type": "application/json", ...headers } });
}

/** PostgREST rows, with the exact count in Content-Range when asked for. */
export function rows(list: unknown[], total = list.length): Route {
  return (_url, init) => {
    const prefer = new Headers(init.headers).get("Prefer") ?? "";
    const range = list.length === 0 ? `*/${total}` : `0-${list.length - 1}/${total}`;
    return json(list, 200, prefer.includes("count=exact") ? { "Content-Range": range } : {});
  };
}
