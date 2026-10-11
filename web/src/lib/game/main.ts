/// <reference path="../../env.d.ts" />
// The game viewer page's script (T-504): reads the address, fetches the
// game's report once, and draws it or says why it cannot. A private game and
// a missing one get the same words. Loaded only where scripts may run.
import { watchPictureFallback } from "../data/pictures.ts";
import { el } from "./dom.ts";
import { loadGame, type Download, type LoadResult } from "./load.ts";
import { renderGame } from "./render.ts";
import { readRecord } from "./report.ts";
import { shapeGame } from "./shape.ts";
import { g, type GameStringKey } from "./strings.ts";
import { type GameRef, parseGameRef, playedOn } from "./url.ts";
import { pageHeading } from "./words.ts";

function byId<T extends HTMLElement>(id: string): T {
  const node = document.getElementById(id);
  if (!node) throw new Error(`#${id} is missing`);
  return node as T;
}

const body = byId("game-body");
const heading = byId("game-title");
const status = byId("game-status");

function announce(text: string): void {
  status.textContent = text;
}

function done(nodes: HTMLElement[]): void {
  body.replaceChildren(...nodes);
  body.removeAttribute("aria-busy");
}

function stateBlock(kind: "unknown" | "error", title: GameStringKey, text: GameStringKey, ...extra: HTMLElement[]): HTMLElement {
  return el("div", { class: `state state-${kind}` }, el("p", { class: "state-title" }, g(title)), el("p", {}, g(text)), ...extra);
}

function showNotFound(): void {
  heading.textContent = g("state.notFound.title");
  document.title = `${g("state.notFound.title")} · Tavern Ledger`;
  done([stateBlock("unknown", "state.notFound.title", "state.notFound.body")]);
  announce(g("state.notFound.title"));
}

function showUnavailable(signedIn: boolean): void {
  heading.textContent = g("state.unavailable.title");
  document.title = `${g("state.unavailable.title")} · Tavern Ledger`;
  const hint = signedIn ? null : el("p", {}, el("a", { href: "/signin/" }, g("state.unavailable.signIn")));
  done([stateBlock("unknown", "state.unavailable.title", "state.unavailable.body", ...(hint ? [hint] : []))]);
  announce(g("state.unavailable.title"));
}

function showError(kind: "unreadable" | "error", retry: () => void): void {
  const button = el("button", { type: "button" }, g("state.retry"));
  button.addEventListener("click", retry);
  const block = kind === "unreadable"
    ? stateBlock("error", "state.unreadable.title", "state.unreadable.body")
    : stateBlock("error", "state.error.title", "state.error.body", button);
  done([block]);
  announce(g(kind === "unreadable" ? "state.unreadable.title" : "state.error.title"));
}

function showGame(result: Extract<LoadResult, { kind: "ok" }>, ref: GameRef): void {
  const view = shapeGame(result.record);
  const title = pageHeading(view.summary);
  heading.textContent = title;
  document.title = `${title} · Tavern Ledger`;
  const banners: HTMLElement[] = [];
  if (result.record.newer) banners.push(el("p", { class: "state state-unknown gv-banner" }, g("state.newer")));
  const unreadable = result.record.unreadable;
  if (unreadable > 0) {
    banners.push(el("p", { class: "state state-unknown gv-banner" }, unreadable === 1 ? g("state.unreadableValues.one") : g("state.unreadableValues.other", { count: unreadable })));
  }
  done([...banners, ...renderGame(view, playedOn(ref.session))]);
  announce(g("page.loaded"));
}

function loading(): void {
  body.setAttribute("aria-busy", "true");
  announce(g("page.loading"));
}

async function run(ref: GameRef, download: Download, signedIn: () => Promise<boolean>): Promise<void> {
  loading();
  const result = await loadGame(download, ref);
  if (result.kind === "ok") return showGame(result, ref);
  if (result.kind === "unavailable") return showUnavailable(await signedIn().catch(() => false));
  showError(result.kind, () => {
    void run(ref, download, signedIn);
  });
}

async function start(): Promise<void> {
  watchPictureFallback(document);
  const ref = parseGameRef(location.search);
  if (ref === null) return showNotFound();
  // Dev only (removed from the built site): ?demo draws a synthetic game.
  if (import.meta.env.DEV && new URLSearchParams(location.search).has("demo")) {
    const { record } = await import("../../../test/unit/gameFixtures.ts");
    const read = readRecord(record(), ref.session, ref.index);
    if (read.ok) return showGame({ kind: "ok", record: read.record }, ref);
  }
  const { supabase } = await import("../../scripts/client.ts");
  const download: Download = (path) => supabase.storage.from("games").download(path);
  const signedIn = async () => (await supabase.auth.getSession()).data.session !== null;
  await run(ref, download, signedIn);
}

start().catch(() => showError("error", () => location.reload()));
