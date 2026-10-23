// Runs the generated /sw.js in a bare VM with a stand-in Cache Storage and
// network (and a clock that runs the worker's five seconds in twenty milliseconds), so the tests can send it install, activate and fetch events and
// look at what it answered and what it kept.
import { runInNewContext } from "node:vm";
import { NEVER_CACHED, serviceWorkerSource, type WorkerBuild } from "../../src/lib/pwa/build.ts";

export const ORIGIN = "https://tavernledger.test";
export const SUPABASE = "https://abcd.supabase.co";

export const BUILD: WorkerBuild = {
  version: "v1",
  precache: ["/offline/", "/es/offline/", "/_astro/Base.abc.css"],
  never: NEVER_CACHED,
  known: ["/", "/privacy/", "/es/privacy/", "/terms/", "/download/", "/es/download/", "/offline/", "/es/offline/", "/leaderboards/", "/simulator/", "/lang.js", "/fonts/figtree-latin-wght-normal.woff2", "/art/x.webp", "/_astro/Base.abc.css", "/_astro/game.BxMRQOT6.css", ...NEVER_CACHED],
};

type Handler = (event: FakeEvent) => void;

export interface FakeEvent {
  request?: Request;
  waitUntil(promise: Promise<unknown>): void;
  respondWith(promise: Promise<Response> | Response): void;
}

/** A response as the browser gives one for our own address (the Response constructor makes type "default"). */
export function page(body: string, headers: Record<string, string> = { "Content-Type": "text/html" }, status = 200): Response {
  const response = new Response(body, { status, headers });
  Object.defineProperty(response, "type", { value: "basic" });
  return response;
}

/** A Request with the `mode` a browser sets (the constructor does not allow "navigate"). */
export function request(url: string, mode: "navigate" | "same-origin" = "same-origin", method = "GET"): Request {
  const req = new Request(url, { method });
  Object.defineProperty(req, "mode", { value: mode });
  return req;
}

export class Worker {
  readonly stores = new Map<string, Map<string, Response>>();
  readonly fetched: string[] = [];
  private readonly handlers = new Map<string, Handler>();
  /** What the stand-in network answers; a thrown error is "offline". */
  network: (req: Request) => Response | Promise<Response> = () => {
    throw new TypeError("offline");
  };
  skipped = false;
  claimed = false;

  constructor(build: WorkerBuild = BUILD) {
    const self = {
      location: { origin: ORIGIN },
      addEventListener: (type: string, handler: Handler) => this.handlers.set(type, handler),
      skipWaiting: async () => void (this.skipped = true),
      clients: { claim: async () => void (this.claimed = true) },
    };
    const caches = {
      keys: async () => [...this.stores.keys()],
      delete: async (name: string) => this.stores.delete(name),
      open: async (name: string) => this.cache(name),
    };
    const fetch = async (input: Request | string) => {
      const req = typeof input === "string" ? new Request(input) : input;
      this.fetched.push(`${req.method} ${req.url}`);
      return this.network(req);
    };
    runInNewContext(serviceWorkerSource(build), { self, caches, fetch, Request, Response, URL, Error, TypeError, Promise, setTimeout: (fn: () => void) => setTimeout(fn, 20), clearTimeout });
  }

  private cache(name: string) {
    const store = this.stores.get(name) ?? new Map<string, Response>();
    this.stores.set(name, store);
    return {
      put: async (req: Request | string, res: Response) => void store.set(typeof req === "string" ? req : req.url, res),
      match: async (req: Request | string) => store.get(typeof req === "string" ? new URL(req, ORIGIN).href : req.url)?.clone(),
      addAll: async (reqs: (Request | string)[]) => {
        const got: [string, Response][] = [];
        for (const req of reqs) {
          const url = typeof req === "string" ? new URL(req, ORIGIN).href : req.url;
          const res = await this.network(new Request(url));
          this.fetched.push(`GET ${url}`);
          if (!res.ok) throw new TypeError(`bad status for ${url}`);
          got.push([url, res]);
        }
        for (const [url, res] of got) store.set(url, res);
      },
    };
  }

  /** Every address in every cache. */
  kept(): string[] {
    return [...this.stores.values()].flatMap((store) => [...store.keys()]);
  }

  /** Sends an install or activate event and waits for what it asked to wait for. */
  async lifecycle(type: "install" | "activate"): Promise<void> {
    const waits: Promise<unknown>[] = [];
    this.handlers.get(type)!({ waitUntil: (p) => void waits.push(p), respondWith: () => {} });
    await Promise.all(waits);
  }

  /** Sends a fetch event. `answer` is the worker's response, or null if it left the request to the browser. */
  async fetch(req: Request): Promise<Response | null> {
    const waits: Promise<unknown>[] = [];
    let answer: Promise<Response> | Response | null = null;
    this.handlers.get("fetch")!({
      request: req,
      waitUntil: (p) => void waits.push(p),
      respondWith: (r) => void (answer = r),
    });
    const response = answer === null ? null : await answer;
    await Promise.all(waits);
    return response;
  }
}
