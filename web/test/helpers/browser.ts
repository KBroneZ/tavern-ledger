// A real Chromium (Edge or Chrome) driven over the DevTools protocol, and the
// built site served the way Cloudflare Pages serves it (the rules of
// dist/_headers applied). Used by the end-to-end checks of the installable web
// app (test/local/installable.e2e.test.ts). No browser-automation dependency:
// Node's own fetch, WebSocket and http are enough.
import { spawn, type ChildProcess } from "node:child_process";
import { existsSync, mkdtempSync, readFileSync, rmSync, statSync } from "node:fs";
import http from "node:http";
import { tmpdir } from "node:os";
import { extname, join } from "node:path";

const TYPES: Record<string, string> = {
  ".html": "text/html; charset=utf-8", ".js": "text/javascript", ".css": "text/css", ".png": "image/png", ".svg": "image/svg+xml",
  ".webmanifest": "application/manifest+json", ".woff2": "font/woff2", ".xml": "application/xml", ".webp": "image/webp",
  ".avif": "image/avif", ".json": "application/json", ".ico": "image/x-icon", ".wasm": "application/wasm", ".txt": "text/plain",
};

/** The headers `_headers` gives a path: every matching rule in order, `! Name` removes, a repeated name is joined with a comma. */
export function headersFor(rules: string[][], path: string): Record<string, string> {
  const out = new Map<string, string>();
  for (const [pattern, ...lines] of rules) {
    const match = pattern.endsWith("*") ? path.startsWith(pattern.slice(0, -1)) : pattern === path;
    if (!match) continue;
    for (const line of lines.filter((l) => l.startsWith("! "))) out.delete(line.slice(2).toLowerCase());
    for (const line of lines.filter((l) => !l.startsWith("! "))) {
      const at = line.indexOf(":");
      const name = line.slice(0, at).toLowerCase();
      const value = line.slice(at + 1).trim();
      out.set(name, out.has(name) ? `${out.get(name)}, ${value}` : value);
    }
  }
  return Object.fromEntries(out);
}

export interface Site {
  origin: string;
  /** "METHOD /path?query" of every request the server answered. */
  requests: string[];
  /** Goes offline: the server stops answering. */
  stop(): Promise<void>;
  /** Back online, at the same address. */
  start(): Promise<void>;
}

/** Serves `dist` on a free port, with its `_headers` rules. */
export async function serveDist(dist: string): Promise<Site> {
  const rules = readFileSync(join(dist, "_headers"), "utf8").split(/\n(?=\S)/).map((b) => b.trim().split("\n").map((l) => l.trim()));
  const requests: string[] = [];
  const server = http.createServer((req, res) => {
    const url = new URL(req.url ?? "/", "http://site");
    requests.push(`${req.method} ${url.pathname}${url.search}`);
    let file = join(dist, decodeURIComponent(url.pathname));
    if (url.pathname.endsWith("/")) file = join(file, "index.html");
    else if (existsSync(file) && statSync(file).isDirectory()) return void res.writeHead(308, { Location: `${url.pathname}/` }).end();
    let status = 200;
    if (!existsSync(file) || statSync(file).isDirectory()) {
      file = join(dist, url.pathname.startsWith("/es/") ? "es/404.html" : "404.html");
      status = 404;
    }
    res.writeHead(status, { ...headersFor(rules, url.pathname), "content-type": TYPES[extname(file)] ?? "application/octet-stream" }).end(readFileSync(file));
  });
  const listen = (port: number) => new Promise<void>((done) => server.listen(port, "127.0.0.1", done));
  await listen(0);
  const port = (server.address() as { port: number }).port;
  return {
    origin: `http://127.0.0.1:${port}`,
    requests,
    stop: () => new Promise<void>((done) => { server.close(() => done()); server.closeAllConnections(); }),
    start: () => listen(port),
  };
}

export const ANDROID_CHROME =
  "Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Mobile Safari/537.36";
export const IPHONE_SAFARI =
  "Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Mobile/15E148 Safari/604.1";

const sleep = (ms: number) => new Promise<void>((done) => setTimeout(done, ms));

export class Browser {
  private id = 0;
  private sessionId: string | undefined;
  private readonly pending = new Map<number, [(value: any) => void, (error: Error) => void]>();
  private readonly process: ChildProcess;
  private readonly socket: WebSocket;
  private readonly profile: string;
  private constructor(process: ChildProcess, socket: WebSocket, profile: string) {
    this.process = process;
    this.socket = socket;
    this.profile = profile;
    socket.onmessage = (message) => {
      const data = JSON.parse(String(message.data));
      const waiting = data.id === undefined ? undefined : this.pending.get(data.id);
      if (!waiting) return;
      this.pending.delete(data.id);
      if (data.error) waiting[1](new Error(JSON.stringify(data.error)));
      else waiting[0](data.result);
    };
  }

  /** Starts a headless browser at `executable` with a throw-away profile. */
  static async launch(executable: string): Promise<Browser> {
    const profile = mkdtempSync(join(tmpdir(), "tl-browser-"));
    const process = spawn(executable, ["--headless=new", "--disable-gpu", "--no-first-run", "--remote-debugging-port=0", `--user-data-dir=${profile}`, "about:blank"], { stdio: "ignore" });
    let endpoint = "";
    for (let tries = 0; tries < 80 && !endpoint; tries++) {
      const file = join(profile, "DevToolsActivePort");
      if (existsSync(file)) {
        const [port, path] = readFileSync(file, "utf8").split(/\r?\n/);
        if (port && path) endpoint = `ws://127.0.0.1:${port}${path}`;
      }
      if (!endpoint) await sleep(250);
    }
    if (!endpoint) {
      process.kill();
      throw new Error("the browser did not open its debugging port");
    }
    const socket = new WebSocket(endpoint);
    await new Promise((done) => (socket.onopen = done));
    const browser = new Browser(process, socket, profile);
    const { targetId } = await browser.send("Target.createTarget", { url: "about:blank" });
    const { sessionId } = await browser.send("Target.attachToTarget", { targetId, flatten: true });
    browser.sessionId = sessionId;
    for (const domain of ["Page", "Runtime", "Network"]) await browser.page(`${domain}.enable`);
    return browser;
  }

  private send(method: string, params: object = {}, sessionId?: string): Promise<any> {
    return new Promise((done, fail) => {
      const id = ++this.id;
      this.pending.set(id, [done, fail]);
      this.socket.send(JSON.stringify({ id, method, params, sessionId }));
    });
  }

  /** A DevTools call on the page. */
  page(method: string, params: object = {}): Promise<any> {
    return this.send(method, params, this.sessionId);
  }

  async emulate(userAgent: string, width: number, height: number): Promise<void> {
    await this.page("Network.setUserAgentOverride", { userAgent });
    await this.page("Emulation.setDeviceMetricsOverride", { width, height, deviceScaleFactor: 2, mobile: true });
    await this.page("Emulation.setTouchEmulationEnabled", { enabled: true });
  }

  /** Runs an expression in the page and returns its value (promises are awaited). */
  async evaluate<T>(expression: string): Promise<T> {
    const result = await this.page("Runtime.evaluate", { expression, awaitPromise: true, returnByValue: true });
    if (result.exceptionDetails) throw new Error(String(result.exceptionDetails.exception?.description ?? result.exceptionDetails.text));
    return result.result.value as T;
  }

  /** Opens an address and gives the page a moment (and the worker, on a first visit) to settle. */
  async go(url: string, settleMs = 1200): Promise<void> {
    await this.page("Page.navigate", { url });
    await sleep(settleMs);
  }

  async screenshot(): Promise<Buffer> {
    const { data } = await this.page("Page.captureScreenshot", { format: "png" });
    return Buffer.from(data, "base64");
  }

  async close(): Promise<void> {
    try {
      await this.page("Browser.close");
    } catch {
      // The browser may close before it answers.
    }
    this.socket.close();
    this.process.kill();
    await sleep(500);
    rmSync(this.profile, { recursive: true, force: true, maxRetries: 5, retryDelay: 300 });
  }
}
