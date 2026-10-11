/// <reference path="../../env.d.ts" />
// The combat simulator page's script (T-606, D-090). It keeps the boards,
// redraws the editor after each edit, and plays the boards in a Web Worker
// (the engine is crates/sim-wasm built to WebAssembly), so the page never
// freezes. Signed in, it lists the player's own games and loads a round
// from one; signed out, the editor works with nothing read.
// First: the page's language, before any module draws text (T-508).
import "../i18n/boot.ts";
import { DEFAULT_FILTERS } from "../data/filters.ts";
import { share } from "../data/format.ts";
import { watchPictureFallback } from "../data/pictures.ts";
import { t } from "../data/strings.ts";
import { lang } from "../i18n.ts";
import { h, mount, type VNode } from "../view/node.ts";
import { loadGame } from "../game/load.ts";
import type { GameRecord } from "../game/report.ts";
import type { GameRow } from "../data/rows.ts";
import { readAnswer, type Answer } from "./answer.ts";
import * as B from "./board.ts";
import { CardIndex, readCardFile } from "./cards.ts";
import { gameKey, type GamesState, loadView, type PickedGame } from "./loadView.ts";
import { resultView, type ResultState } from "./resultView.ts";
import { sm } from "./strings.ts";
import { type Adding, editorView, type Marks, modeSwitch, NO_MARKS, type Selection } from "./view.ts";

/** Wait after the last edit before playing the boards (as the app's overlay). */
const SETTLE_MS = 300;

function byId(id: string): HTMLElement {
  const node = document.getElementById(id);
  if (!node) throw new Error(`#${id} is missing`);
  return node;
}

const els = {
  load: byId("sim-load"),
  note: byId("sim-note"),
  mode: byId("sim-mode"),
  editor: byId("sim-editor"),
  result: byId("sim-result"),
  status: byId("sim-status"),
};

// ---------- State ----------

let cards = new CardIndex({ version: 1, source: "none", cards: [] });
let sim: B.SimState = B.emptyState();
let selected: Selection | null = null;
let adding: Adding | null = null;
let marks: Marks = NO_MARKS;
let result: ResultState = { kind: "empty" };
let games: GamesState = { kind: "loading" };
let picked: PickedGame | null = null;
let notes: string[] = [];

// ---------- Drawing ----------

/** Redraws a region and puts focus back on the control that had it. */
function draw(target: HTMLElement, nodes: VNode[]): void {
  const active = document.activeElement as HTMLElement | null;
  const focus = active && target.contains(active) ? active.dataset.focus : undefined;
  const caret = active instanceof HTMLInputElement && active.type === "search" ? active.selectionStart : null;
  mount(target, ...nodes);
  if (!focus) return;
  const again = target.querySelector<HTMLElement>(`[data-focus="${CSS.escape(focus)}"]`);
  again?.focus();
  if (again instanceof HTMLInputElement && caret !== null) again.setSelectionRange(caret, caret);
}

function drawEditor(): void {
  draw(els.mode, [modeSwitch(sim, t("mode.solo"), t("mode.duos"))]);
  draw(els.editor, editorView({ state: sim, cards, selected, adding, marks }));
}

function drawResult(): void {
  draw(els.result, resultView({ result, cards, cap: sim.cap }));
}

function drawLoad(): void {
  draw(els.load, loadView(games, picked));
}

function drawNotes(): void {
  mount(els.note, ...notes.map((n) => h("p", {}, n)));
}

// ---------- The engine, in a worker ----------

let worker: Worker | null = null;
let sent = 0;
let timer: ReturnType<typeof setTimeout> | undefined;

function startWorker(): Worker {
  const w = new Worker(new URL("./simWorker.ts", import.meta.url), { type: "module" });
  w.addEventListener("message", (event: MessageEvent<{ id?: unknown; answer?: unknown; failed?: unknown }>) => {
    const { id, answer, failed } = event.data ?? {};
    if (id !== sent) return; // an answer to boards edited since
    if (failed === true || typeof answer !== "string") return showFailed();
    showAnswer(readAnswer(answer));
  });
  w.addEventListener("error", () => {
    worker = null;
    showFailed();
  });
  return w;
}

function showFailed(): void {
  result = { kind: "failed" };
  marks = NO_MARKS;
  drawResult();
  drawEditor();
}

function marksOf(a: Answer): Marks {
  if (a.kind === "not_modelled") return { notModelled: new Set(a.notModelled), missing: new Set(), approximate: new Map(a.approximate.map((x) => [x.cardId, x.reason])) };
  if (a.kind === "no_card_data") return { notModelled: new Set(), missing: new Set(a.missing), approximate: new Map() };
  if (a.kind === "odds") return { notModelled: new Set(), missing: new Set(), approximate: new Map(a.approximate.map((x) => [x.cardId, x.reason])) };
  return NO_MARKS;
}

function showAnswer(a: Answer): void {
  result = { kind: "answer", answer: a };
  marks = marksOf(a);
  els.status.textContent = a.kind === "odds" ? sm("result.bar", { win: share(a.won, a.trials), tie: share(a.tied, a.trials), lose: share(a.lost, a.trials) }) : "";
  drawResult();
  drawEditor();
}

function play(): void {
  clearTimeout(timer);
  if (B.notReady(sim)) {
    sent += 1;
    result = { kind: "empty" };
    marks = NO_MARKS;
    drawResult();
    return;
  }
  const previous = result.kind === "answer" ? result.answer : result.kind === "running" ? result.previous : null;
  result = { kind: "running", previous };
  drawResult();
  timer = setTimeout(() => {
    sent += 1;
    try {
      worker ??= startWorker();
      worker.postMessage({ id: sent, request: B.toRequest(sim, cards) });
    } catch {
      showFailed();
    }
  }, SETTLE_MS);
}

/** A new state from an edit: redraw and play it. */
function update(next: B.SimState): void {
  if (next === sim) return;
  sim = next;
  drawEditor();
  play();
}

// ---------- Editing ----------

function sideOf(el: HTMLElement): { side: B.Side; board: number } | null {
  const side = el.dataset.side;
  const board = Number(el.dataset.board ?? "0");
  if ((side !== "player" && side !== "opponent") || !Number.isInteger(board) || !sim[side][board]) return null;
  return { side, board };
}

function onEditorClick(event: Event): void {
  const el = (event.target as HTMLElement).closest<HTMLElement>("[data-act]");
  if (!el) return;
  const act = el.dataset.act;
  if (act === "mode") {
    const mode = el.dataset.mode === "duos" ? "duos" : "solo";
    selected = null;
    adding = null;
    return update(B.setMode(sim, mode));
  }
  if (act === "retry") {
    worker?.terminate();
    worker = null;
    return play();
  }
  if (act === "board-add") {
    if (el.dataset.side === "player" || el.dataset.side === "opponent") update(B.addBoard(sim, el.dataset.side));
    return;
  }
  const at = sideOf(el);
  if (!at) return;
  const { side, board } = at;
  const key = Number(el.dataset.key);
  switch (act) {
    case "select":
      selected = selected?.key === key ? null : { side, board, key };
      adding = null;
      return drawEditor();
    case "close":
      selected = null;
      return drawEditor();
    case "tier":
      return update(B.setTier(sim, side, board, el.dataset.tier ? Number(el.dataset.tier) : null));
    case "add-open":
      adding = adding?.side === side && adding.board === board ? null : { side, board, query: "" };
      selected = null;
      drawEditor();
      if (adding) els.editor.querySelector<HTMLElement>(`[data-focus="q-${side}-${board}"]`)?.focus();
      return;
    case "add-close":
      adding = null;
      drawEditor();
      els.editor.querySelector<HTMLElement>(`[data-focus="add-${side}-${board}"]`)?.focus();
      return;
    case "pick": {
      const next = B.addMinion(sim, side, board, el.dataset.card ?? "", cards);
      adding = next[side][board].minions.length >= B.MAX_MINIONS ? null : adding && { ...adding, query: "" };
      update(next);
      els.editor.querySelector<HTMLElement>(`[data-focus="q-${side}-${board}"]`)?.focus();
      return;
    }
    case "golden":
      return update(B.toggleGolden(sim, side, board, key, cards));
    case "keyword":
      if ((B.SIM_KEYWORDS as readonly string[]).includes(el.dataset.keyword ?? "")) {
        update(B.toggleKeyword(sim, side, board, key, el.dataset.keyword as B.SimKeyword));
      }
      return;
    case "move":
      return update(B.moveMinion(sim, side, board, key, el.dataset.step === "-1" ? -1 : 1));
    case "remove":
      selected = null;
      update(B.removeMinion(sim, side, board, key));
      els.editor.querySelector<HTMLElement>(`[data-focus="add-${side}-${board}"]`)?.focus();
      return;
    case "board-remove":
      selected = null;
      adding = null;
      return update(B.removeBoard(sim, side, board));
  }
}

function onEditorInput(event: Event): void {
  const el = event.target as HTMLInputElement;
  if (el.dataset.field !== "query" || !adding) return;
  adding = { ...adding, query: el.value.slice(0, 40) };
  drawEditor();
}

function onEditorChange(event: Event): void {
  const el = event.target as HTMLInputElement;
  const field = el.dataset.field;
  if (field !== "attack" && field !== "health") return;
  const at = sideOf(el);
  const key = Number(el.dataset.key);
  const minion = at && sim[at.side][at.board].minions.find((m) => m.key === key);
  if (!at || !minion) return;
  const value = el.value === "" ? NaN : Number(el.value);
  const attack = field === "attack" ? value : minion.attack;
  const health = field === "health" ? value : minion.health;
  const next = B.setStats(sim, at.side, at.board, key, attack, health);
  // A value the field cannot hold (empty, 0 Health) snaps back to the one kept.
  if (next === sim || Number(el.value) !== (field === "attack" ? attack : health)) drawEditor();
  update(next);
}

function onResultChange(event: Event): void {
  const el = event.target as HTMLInputElement;
  if (el.dataset.field !== "cap") return;
  update(B.setCap(sim, el.value === "" ? null : Number(el.value)));
  drawResult();
}

// ---------- Loading a combat from the player's games ----------

interface Account {
  userId: string;
  download: Parameters<typeof loadGame>[0];
  list(page: number): Promise<{ rows: GameRow[]; total: number }>;
}

let account: Account | null = null;
let page = 1;
let record: GameRecord | null = null;

async function listGames(append: boolean): Promise<void> {
  if (!account) return;
  if (!append) {
    games = { kind: "loading" };
    drawLoad();
  }
  try {
    const { rows, total } = await account.list(page);
    const before = append && games.kind === "ready" ? games.rows : [];
    const all = [...before, ...rows];
    games = { kind: "ready", rows: all, total, more: all.length < total && rows.length > 0 };
  } catch {
    games = { kind: "error" };
  }
  drawLoad();
}

async function pickGame(key: string): Promise<void> {
  if (!account || games.kind !== "ready") return;
  const row = games.rows.find((r) => gameKey(r) === key);
  if (!row || !row.session || row.gameIndex === null) return;
  picked = { key, rounds: null, failed: false, loaded: null };
  record = null;
  drawLoad();
  const ref = { userId: account.userId, session: row.session, index: row.gameIndex };
  const loaded = await loadGame(account.download, ref);
  if (picked?.key !== key) return; // another game was picked meanwhile
  if (loaded.kind !== "ok") {
    picked = { ...picked, failed: true };
  } else {
    record = loaded.record;
    picked = { ...picked, rounds: B.loadableRounds(loaded.record) };
  }
  drawLoad();
}

function loadRound(n: number): void {
  if (!record || !picked) return;
  const loaded = B.fromRound(record, n, cards);
  if (!loaded) return;
  picked = { ...picked, loaded: n };
  selected = null;
  adding = null;
  notes = [sm("load.loaded", { n })];
  if (loaded.skipped > 0) notes.push(loaded.skipped === 1 ? sm("load.skipped.one") : sm("load.skipped.other", { count: loaded.skipped }));
  const boards = [...loaded.state.player, ...loaded.state.opponent];
  if (boards.some((b) => b.tier === null)) notes.push(sm("load.unknownTiers"));
  drawLoad();
  drawNotes();
  update(loaded.state);
}

function onLoadClick(event: Event): void {
  const el = (event.target as HTMLElement).closest<HTMLElement>("[data-act]");
  if (!el) return;
  if (el.dataset.act === "games-retry") void (account ? listGames(false) : signIn().catch(() => undefined));
  if (el.dataset.act === "games-more") {
    page += 1;
    void listGames(true);
  }
  if (el.dataset.act === "game" && el.dataset.game) void pickGame(el.dataset.game);
  if (el.dataset.act === "round") loadRound(Number(el.dataset.round));
}

async function signIn(): Promise<void> {
  const { supabase } = await import("../../scripts/client.ts");
  const { loadGames } = await import("../dataApi.ts");
  const user = (await supabase.auth.getSession()).data.session?.user ?? null;
  if (!user) {
    games = { kind: "signed-out" };
    return drawLoad();
  }
  // Only the signed-in player's own games: the id comes from their session, never from the address.
  account = {
    userId: user.id,
    download: (path) => supabase.storage.from("games").download(path),
    list: (p) => loadGames(supabase, user.id, { ...DEFAULT_FILTERS, page: p }),
  };
  await listGames(false);
}

// ---------- Start ----------

async function start(): Promise<void> {
  watchPictureFallback(document);
  const file = lang() === "es" ? await import("./generated/cards.es.json") : await import("./generated/cards.en.json");
  cards = new CardIndex(readCardFile(file.default));
  els.editor.addEventListener("click", onEditorClick);
  els.mode.addEventListener("click", onEditorClick);
  els.result.addEventListener("click", onEditorClick);
  els.editor.addEventListener("input", onEditorInput);
  els.editor.addEventListener("change", onEditorChange);
  els.result.addEventListener("change", onResultChange);
  els.load.addEventListener("click", onLoadClick);
  els.editor.removeAttribute("aria-busy");
  drawEditor();
  drawResult();
  drawLoad();
  await signIn().catch(() => {
    games = { kind: "error" };
    drawLoad();
  });
}

start().catch(() => showFailed());
