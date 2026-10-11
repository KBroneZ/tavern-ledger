/// <reference path="../../env.d.ts" />
// The game viewer page's script (T-504): reads the address, fetches the
// game's report once, and draws it or says why it cannot. A private game and
// a missing one get the same words. Loaded only where scripts may run.
// First: the page's language, before any module draws text (T-508).
import "../i18n/boot.ts";
import { dateText } from "../data/dashboard.ts";
import { watchPictureFallback } from "../data/pictures.ts";
import { h, mount, type VNode } from "../view/node.ts";
import { stateBlock } from "../view/pieces.ts";
import { loadGame, type Download, type LoadResult } from "./load.ts";
import { renderGame } from "./render.ts";
import { readRecord } from "./report.ts";
import { shapeGame } from "./shape.ts";
import { g, type GameStringKey } from "./strings.ts";
import { type GameRef, parseGameRef, playedOn } from "./url.ts";
import { pageHeading } from "./words.ts";
import { href } from "../i18n.ts";

function byId(id: string): HTMLElement {
  const node = document.getElementById(id);
  if (!node) throw new Error(`#${id} is missing`);
  return node;
}

const body = byId("game-body");
const heading = byId("game-title");
const status = byId("game-status");
const RETRY = "game-retry";

function show(title: string, nodes: VNode[], announcement: string): void {
  heading.textContent = title;
  document.title = `${title} · Tavern Ledger`;
  mount(body, ...nodes);
  body.removeAttribute("aria-busy");
  status.textContent = announcement;
}

function state(kind: "unknown" | "error", title: GameStringKey, text: GameStringKey, ...extra: VNode[]): VNode {
  return stateBlock({ kind, title: g(title), body: g(text) }, ...extra);
}

function showNotFound(): void {
  show(g("state.notFound.title"), [state("unknown", "state.notFound.title", "state.notFound.body")], g("state.notFound.title"));
}

function showUnavailable(signedIn: boolean): void {
  const hint = signedIn ? [] : [h("p", {}, h("a", { href: href("/signin/") }, g("state.unavailable.signIn")))];
  show(g("state.unavailable.title"), [state("unknown", "state.unavailable.title", "state.unavailable.body", ...hint)], g("state.unavailable.title"));
}

function showError(kind: "unreadable" | "error", retry: () => void): void {
  if (kind === "unreadable") {
    show(g("state.unreadable.title"), [state("error", "state.unreadable.title", "state.unreadable.body")], g("state.unreadable.title"));
    return;
  }
  const button = h("button", { type: "button", id: RETRY }, g("state.retry"));
  show(g("state.error.title"), [state("error", "state.error.title", "state.error.body", button)], g("state.error.title"));
  byId(RETRY).addEventListener("click", retry);
}

function showGame(result: Extract<LoadResult, { kind: "ok" }>, ref: GameRef): void {
  const view = shapeGame(result.record);
  const banners: VNode[] = [];
  if (result.record.newer) banners.push(h("p", { class: "state state-unknown gv-banner" }, g("state.newer")));
  const unreadable = result.record.unreadable;
  if (unreadable > 0) {
    const text = unreadable === 1 ? g("state.unreadableValues.one") : g("state.unreadableValues.other", { count: unreadable });
    banners.push(h("p", { class: "state state-unknown gv-banner" }, text));
  }
  const title = pageHeading(view.summary);
  show(title, [...banners, ...renderGame(view, dateText(playedOn(ref.session)))], g("page.loaded", { title }));
}

async function run(ref: GameRef, download: Download, signedIn: () => Promise<boolean>): Promise<void> {
  // The loading block replaces whatever was there (an error and its button), so a retry runs once.
  mount(body, stateBlock({ kind: "loading", title: g("page.loading"), lines: 10 }));
  body.setAttribute("aria-busy", "true");
  status.textContent = g("page.loading");
  const result = await loadGame(download, ref);
  if (result.kind === "ok") return showGame(result, ref);
  if (result.kind === "unavailable") return showUnavailable(await signedIn().catch(() => false));
  showError(result.kind, () => {
    // The button goes away with the block: focus moves to the page's heading first.
    heading.focus();
    run(ref, download, signedIn).catch(() => showError("error", () => location.reload()));
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
