/// <reference path="../../env.d.ts" />
// The game viewer page's script (T-504): reads the address, fetches the
// game's report once, and draws it or says why it cannot. A private game and
// a missing one get the same words. Loaded only where scripts may run.
// First: the page's language, before any module draws text (T-508).
import "../i18n/boot.ts";
import { paintMinions } from "../art/runtime.ts";
import { dateText } from "../data/dashboard.ts";
import { watchPictureFallback } from "../data/pictures.ts";
import { h, mount, type VNode } from "../view/node.ts";
import { stateBlock } from "../view/pieces.ts";
import { EXPIRED_COLUMNS, expiredHeading, loadExpired, type ReadSummary, renderExpired } from "./expired.ts";
import { loadGame, type Download, type LoadResult } from "./load.ts";
import { watchTipEscape, watchTipFit } from "./minionTip.ts";
import { loadCardNames, useCardNames } from "./names.ts";
import { renderGame } from "./render.ts";
import { startReplays } from "./replayPlayer.ts";
import { explainCombats } from "./whyRun.ts";
import { readRecord } from "./report.ts";
import { shapeGame } from "./shape.ts";
import { g, isVisitor, setSeenByVisitor, type GameStringKey } from "./strings.ts";
import { type GameRef, parseGameRef } from "./url.ts";
import { heroTip, pageHeading } from "./words.ts";
import { toggleTurn } from "./turnsView.ts";
import { href } from "../i18n.ts";
import { askForShells, browserDevice, isOn, readSaved, saveGame } from "../pwa/saved.ts";

/** How long the page waits to read the stored sign-in before it goes on without it. */
const SESSION_WAIT_MS = 4000;
/** How long a game that is saved on the device waits for the network before it opens from the saved copy. */
const SAVED_WAIT_MS = 6000;

function byId(id: string): HTMLElement {
  const node = document.getElementById(id);
  if (!node) throw new Error(`#${id} is missing`);
  return node;
}

const body = byId("game-body");
const heading = byId("game-title");
const status = byId("game-status");
const RETRY = "game-retry";

function show(title: string, nodes: VNode[], announcement: string, tip?: string): void {
  heading.textContent = title;
  // An unknown hero reads "Unknown hero"; its id is the tooltip.
  if (tip) heading.title = tip;
  else heading.removeAttribute("title");
  document.title = `${title} · Tavern Ledger`;
  mount(body, ...nodes);
  // The game's own drawing for the minions of the boards, once its textures are in (art/runtime.ts).
  paintMinions(body);
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
  const button = h("button", { type: "button", class: "tb tb-small", id: RETRY }, g("state.retry"));
  show(g("state.error.title"), [state("error", "state.error.title", "state.error.body", button)], g("state.error.title"));
  byId(RETRY).addEventListener("click", retry);
}

/** Draws a game and returns the heading it was given (the offline page lists saved games by it). */
function showGame(result: Extract<LoadResult, { kind: "ok" }>, fromDevice = false): string {
  const view = shapeGame(result.record);
  const banners: VNode[] = [];
  if (fromDevice) banners.push(h("p", { class: "state state-unknown gv-banner" }, g("state.fromDevice")));
  if (result.record.newer) banners.push(h("p", { class: "state state-unknown gv-banner" }, g("state.newer")));
  const unreadable = result.record.unreadable;
  if (unreadable > 0) {
    const text = unreadable === 1 ? g("state.unreadableValues.one") : g("state.unreadableValues.other", { count: unreadable });
    banners.push(h("p", { class: "state state-unknown gv-banner" }, text));
  }
  const title = pageHeading(view.summary);
  show(title, [...banners, ...renderGame(view, dateText(result.record.playedOn))], g("page.loaded", { title }), heroTip(view.summary.hero));
  const replays = document.getElementById("gv-replays");
  if (replays) startReplays(replays, view.replays);
  void explainCombats(result.report);
  return title;
}

/**
 * When the player turned on "Keep my last games on this device" and this is
 * their own game, saves its file, and has the worker keep the game page, for
 * offline reading (lib/pwa/saved.ts). A failure (no room, a private window) only means no copy.
 */
async function keepCopy(ref: GameRef, file: Blob | null, title: string, played: string | null, currentPlayer: () => Promise<string | null>): Promise<void> {
  const device = browserDevice();
  if (!device || !file || !isOn(device)) return;
  // Asked again now, not when the page opened: the player may have signed out, or another account in, while the game loaded.
  if (isVisitor(await currentPlayer(), ref.userId) || !isOn(device, ref.userId)) return;
  await saveGame(device, { userId: ref.userId, publicId: ref.publicId, savedAt: Date.now(), played, title }, file);
  await askForShells();
}

/** The copy of this game saved on the device, read as if it had just come from the server; null when there is none or it cannot be read. */
async function savedCopy(ref: GameRef): Promise<Extract<LoadResult, { kind: "ok" }> | null> {
  try {
    const device = browserDevice();
    const file = device ? await readSaved(device, ref.userId, ref.publicId) : null;
    if (!file) return null;
    const result = await loadGame(async () => ({ data: file, error: null }), ref);
    return result.kind === "ok" ? result : null;
  } catch {
    return null;
  }
}

/** Opens the replay of a turn from its row: picks it in the player and moves there. */
function watch(turn: string): void {
  const section = document.getElementById("gv-replays");
  if (!section) return;
  const chip = section.querySelector<HTMLElement>(`.rp-chip[data-round="${CSS.escape(turn)}"]`);
  chip?.click();
  section.scrollIntoView({ block: "start" });
  chip?.focus({ preventScroll: true });
}

/** One listener for the turn table: a row's toggle opens its detail; "Watch this combat" opens its replay. */
body.addEventListener("click", (event) => {
  const target = event.target as HTMLElement;
  const toggle = target.closest<HTMLElement>("[data-toggle]");
  if (toggle) return toggleTurn(toggle);
  const replay = target.closest<HTMLElement>("[data-replay]");
  if (replay?.dataset.replay) watch(replay.dataset.replay);
});

/** A game whose full file has expired (T-544): its summary and why the details are gone. */
function showExpired(row: Parameters<typeof renderExpired>[0]): void {
  const title = expiredHeading(row);
  show(title, renderExpired(row), g("page.loaded", { title }));
}

interface Sources {
  download: Download;
  readSummary: ReadSummary;
  signedIn: () => Promise<boolean>;
  /** The signed-in player's id as of now, or null (none, or no way to tell without a connection). */
  currentPlayer: () => Promise<string | null>;
}

async function run(ref: GameRef, sources: Sources): Promise<void> {
  const { download, readSummary, signedIn, currentPlayer } = sources;
  // The loading block replaces whatever was there (an error and its button), so a retry runs once.
  mount(body, stateBlock({ kind: "loading", title: g("page.loading"), lines: 10 }));
  body.setAttribute("aria-busy", "true");
  status.textContent = g("page.loading");
  // The file as the server sent it, for the copy kept on the device (lib/pwa/saved.ts).
  const got: { file: Blob | null } = { file: null };
  const grab: Download = async (path) => {
    const answer = await download(path);
    if (answer.data) got.file = answer.data;
    return answer;
  };
  const shown = (result: Extract<LoadResult, { kind: "ok" }>): void => {
    const title = showGame(result);
    keepCopy(ref, got.file, title, result.record.playedOn, currentPlayer).catch((error) => console.warn("Tavern Ledger: this game was not saved on the device.", error));
  };
  // The card names load beside the game (a script chunk from this site); without them, ids show.
  // A game the player saved on this device does not wait for a stalled connection (or a sign-in that cannot be renewed): after
  // SAVED_WAIT_MS it opens from the copy. Any other game waits for the network, however slow.
  const hasCopy = (await savedCopy(ref)) !== null;
  const bounded = <T>(work: Promise<T>, onTimeout: T): Promise<T> =>
    hasCopy ? Promise.race([work, new Promise<T>((done) => setTimeout(() => done(onTimeout), SAVED_WAIT_MS))]) : work;
  const [result, names] = await Promise.all([bounded<LoadResult>(loadGame(grab, ref), { kind: "error" }), loadCardNames()]);
  useCardNames(names);
  if (result.kind === "ok") return shown(result);
  const retry = () => {
    // The button goes away with the block: focus moves to the page's heading first.
    heading.focus();
    run(ref, sources).catch(() => showError("error", () => location.reload()));
  };
  if (result.kind === "unavailable") {
    // Refused by Storage: private, missing, or a file the retention deleted.
    const expired = await bounded(loadExpired(readSummary, ref), { kind: "error" as const });
    if (expired.kind === "expired") return showExpired(expired.row);
    if (expired.kind === "error") return showSavedOrError(ref, "error", retry);
    if (expired.path !== null) {
      // The owner's own game whose file the sweep has not renamed yet: the row knows where it is.
      const own = await bounded<LoadResult>(loadGame(grab, ref, expired.path), { kind: "error" });
      if (own.kind === "ok") return shown(own);
      if (own.kind !== "unavailable") return showSavedOrError(ref, own.kind, retry);
    }
    return showUnavailable(await signedIn().catch(() => false));
  }
  await showSavedOrError(ref, result.kind, retry);
}

/** The error for a game that could not be read; for a network error, the copy saved on this device first, if the player kept one (it is theirs). */
async function showSavedOrError(ref: GameRef, kind: "unreadable" | "error", retry: () => void): Promise<void> {
  const copy = kind === "error" ? await savedCopy(ref) : null;
  if (!copy) return showError(kind, retry);
  setSeenByVisitor(false);
  showGame(copy, true);
}

async function start(): Promise<void> {
  watchPictureFallback(document);
  watchTipEscape(document);
  watchTipFit(document);
  const ref = parseGameRef(location.search);
  if (ref === null) return showNotFound();
  // Dev only (removed from the built site): ?demo draws a synthetic game.
  if (import.meta.env.DEV && new URLSearchParams(location.search).has("demo")) {
    const fixtures = await import("../../../test/unit/gameFixtures.ts");
    const demo = new URLSearchParams(location.search).get("demo");
    // ?demo=full: a full table in the first combat; ?demo=duos: a Duos game with its second boards.
    const made = fixtures.record(demo === "duos" ? fixtures.duosReplayReport() : demo === "full" ? fixtures.fullReport() : undefined);
    const read = readRecord(made);
    useCardNames(await loadCardNames());
    if (read.ok) return showGame({ kind: "ok", record: read.record, report: made.report });
  }
  const { supabase } = await import("../../scripts/client.ts");
  const download: Download = (path) => supabase.storage.from("games").download(path);
  const readSummary: ReadSummary = async (r) =>
    await supabase
      .from("game_summaries")
      .select(EXPIRED_COLUMNS)
      .eq("user_id", r.userId)
      .eq("public_id", r.publicId)
      .limit(1);
  // Without a connection the stored sign-in cannot always be read or renewed (an expired one is renewed over the network, with retries):
  // the page does not wait longer than SESSION_WAIT_MS, goes on as a visitor, and a saved copy still opens.
  const readSession = () =>
    Promise.race([
      supabase.auth.getSession().then(
        (answer) => answer.data.session,
        () => null,
      ),
      new Promise<null>((done) => setTimeout(() => done(null), SESSION_WAIT_MS)),
    ]);
  const session = await readSession();
  // Only the game's own player is told "you"; anyone else, signed in or not, reads "the player" (D-102).
  setSeenByVisitor(isVisitor(session?.user.id, ref.userId));
  const signedIn = async () => session !== null;
  const currentPlayer = async () => (await readSession())?.user.id ?? null;
  await run(ref, { download, readSummary, signedIn, currentPlayer });
}

start().catch(() => showError("error", () => location.reload()));
