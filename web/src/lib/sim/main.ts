/// <reference path="../../env.d.ts" />
// The combat simulator page's script (T-606, D-090; the duel table, T-526).
// It keeps the boards, redraws both halves of the table after each edit,
// and plays the boards in a Web Worker (the engine is crates/sim-wasm
// built to WebAssembly), so the page never
// freezes. Signed in, it lists the player's own games and loads a round
// from one; signed out, the editor works with nothing read.
// First: the page's language, before any module draws text (T-508).
import "../i18n/boot.ts";
import { DEFAULT_FILTERS } from "../data/filters.ts";
import { share } from "../data/format.ts";
import { paintMinions } from "../art/runtime.ts";
import { watchPictureFallback } from "../data/pictures.ts";
import { t } from "../data/strings.ts";
import { lang } from "../i18n.ts";
import { h, mount, type VNode } from "../view/node.ts";
import { loadGame } from "../game/load.ts";
import type { GameRecord } from "../game/report.ts";
import { GAME_FILE_COLUMNS, type GameRow } from "../data/rows.ts";
import { readAnswer, type Answer } from "./answer.ts";
import * as B from "./board.ts";
import { watchTipEscape } from "../game/minionTip.ts";
import { cardNames, loadCardNames, NO_KIT_LISTS, useCardNames } from "../game/names.ts";
import { CardIndex, readCardFile } from "./cards.ts";
import { type Holding, socSwitch } from "./kitView.ts";
import type { KitKind } from "./kit.ts";
import { gameKey, type GamesState, loadView, type PickedGame } from "./loadView.ts";
import { resultView, type ResultState } from "./resultView.ts";
import { sm } from "./strings.ts";
import { type Adding, type Marks, modeSwitch, NO_MARKS, type Selection, sideView } from "./view.ts";

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
  /** The whole table: both halves, the mode switch and the odds. */
  table: byId("sim-table"),
  opponent: byId("sim-opponent"),
  player: byId("sim-player"),
  result: byId("sim-result"),
  status: byId("sim-status"),
};

// ---------- State ----------

let cards = new CardIndex({ version: 1, source: "none", cards: [] });
let sim: B.SimState = B.emptyState();
let selected: Selection | null = null;
let adding: Adding | null = null;
let holding: Holding | null = null;
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
  // The game's own drawing for the minions, once its textures are in (art/runtime.ts).
  paintMinions(target);
  if (!focus) return;
  const again = target.querySelector<HTMLElement>(`[data-focus="${CSS.escape(focus)}"]`);
  again?.focus();
  if (again instanceof HTMLInputElement && caret !== null) again.setSelectionRange(caret, caret);
}

function drawEditor(): void {
  const props = { state: sim, cards, selected, adding, marks, lists: cardNames().kit ?? NO_KIT_LISTS, holding };
  draw(els.mode, [modeSwitch(sim, t("mode.solo"), t("mode.duos")), socSwitch(sim)]);
  draw(els.opponent, sideView("opponent", props));
  draw(els.player, sideView("player", props));
}

/** Puts focus on the first control of the table that matches. */
function focusOn(selector: string): void {
  els.table.querySelector<HTMLElement>(selector)?.focus();
}

function drawResult(): void {
  draw(els.result, resultView({ result, cards, cap: sim.cap, kitIds: new Set(B.allKitCards(sim)) }));
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
  els.status.textContent = sm("result.failed");
  drawResult();
  drawEditor();
}

function marksOf(a: Answer): Marks {
  if (a.kind === "not_modelled") {
    return { notModelled: new Set(a.notModelled), missing: new Set(), approximate: new Map(a.approximate.map((x) => [x.cardId, x.reason])), reasons: new Map(a.reasons.map((x) => [x.cardId, x.reason])) };
  }
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
  // The id is taken now: an answer still on its way for the boards before
  // this edit no longer matches, even while this run waits to start.
  const id = ++sent;
  if (B.notReady(sim)) {
    result = { kind: "empty" };
    marks = NO_MARKS;
    els.status.textContent = "";
    drawResult();
    return;
  }
  const previous = result.kind === "answer" ? result.answer : result.kind === "running" ? result.previous : null;
  result = { kind: "running", previous };
  drawResult();
  timer = setTimeout(() => {
    try {
      worker ??= startWorker();
      worker.postMessage({ id, request: B.toRequest(sim, cards) });
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
    holding = null;
    return update(B.setMode(sim, mode));
  }
  if (act === "retry") {
    worker?.terminate();
    worker = null;
    // Without card data (its chunk failed to load) the page starts again.
    if (cards.size === 0) return void start().catch(() => showFailed());
    return play();
  }
  if (act === "board-add") {
    if (el.dataset.side === "player" || el.dataset.side === "opponent") update(B.addBoard(sim, el.dataset.side));
    return;
  }
  if (act === "soc") return update(B.setStartOfCombat(sim, !sim.startOfCombat));
  if (act === "kit-turn") return update(B.setUnlocked(sim, el.dataset.late === "1"));
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
      if (adding) focusOn(`[data-focus="q-${side}-${board}"]`);
      return;
    case "add-close":
      adding = null;
      drawEditor();
      focusOn(`[data-focus="add-${side}-${board}"]`);
      return;
    case "pick": {
      const next = B.addMinion(sim, side, board, el.dataset.card ?? "", cards);
      adding = next[side][board].minions.length >= B.MAX_MINIONS ? null : adding && { ...adding, query: "" };
      update(next);
      focusOn(`[data-focus="q-${side}-${board}"]`);
      return;
    }
    case "golden":
      return update(B.toggleGolden(sim, side, board, key, cards));
    case "gift-add":
      return addGift(key, side, board);
    case "gift-remove":
      update(B.removeGift(sim, side, board, key, el.dataset.gift ?? ""));
      // The chip is gone: the gift list takes focus.
      focusOn(`[data-focus="ga${key}"]`);
      return;
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
      focusOn(`[data-focus="add-${side}-${board}"]`);
      return;
    case "board-remove":
      selected = null;
      adding = null;
      holding = null;
      return update(B.removeBoard(sim, side, board));
    case "held-pick": {
      const slot = Number(el.dataset.slot);
      if (!Number.isInteger(slot)) return;
      holding = null;
      update(B.setHeld(sim, side, board, slot, el.dataset.card ?? "", cards));
      focusOn(`[data-focus="held-x-${side}-${board}-${slot}"]`);
      return;
    }
    case "held-remove": {
      const slot = Number(el.dataset.slot);
      if (!Number.isInteger(slot)) return;
      update(B.setHeld(sim, side, board, slot, null, cards));
      focusOn(`[data-focus="sim-held-${side}-${board}-${slot}"]`);
      return;
    }
  }
}

function onEditorInput(event: Event): void {
  const el = event.target as HTMLInputElement;
  if (el.dataset.field === "held-query") {
    const at = sideOf(el);
    if (!at || !Number.isInteger(Number(el.dataset.slot))) return;
    holding = { ...at, slot: Number(el.dataset.slot), query: el.value.slice(0, 40) };
    return drawEditor();
  }
  if (el.dataset.field !== "query" || !adding) return;
  adding = { ...adding, query: el.value.slice(0, 40) };
  drawEditor();
}

/** A hero power, trinket, chosen minion or held minion's stats changed (T-628). */
function onKitChange(el: HTMLInputElement | HTMLSelectElement): boolean {
  const field = el.dataset.field;
  const at = sideOf(el);
  const slot = Number(el.dataset.slot);
  if (!at || !Number.isInteger(slot)) return false;
  if (field === "kit") {
    const kind: KitKind = el.dataset.kind === "power" ? "power" : "trinket";
    // A search for the slot's old card is not one for its new card.
    holding = null;
    update(B.setKitCard(sim, at.side, at.board, kind, slot, el.value === "" ? null : el.value));
    return true;
  }
  if (field === "kit-target") {
    update(B.setKitTarget(sim, at.side, at.board, slot, el.value === "" ? null : Number(el.value)));
    return true;
  }
  if (field === "held-attack" || field === "held-health") {
    const held = sim[at.side][at.board].kit?.powers.find((c) => c.slot === slot)?.held;
    if (!held || el.value.trim() === "" || !Number.isFinite(Number(el.value))) {
      drawEditor();
      return true;
    }
    const value = Number(el.value);
    const next = B.setHeldStats(sim, at.side, at.board, slot, field === "held-attack" ? value : held.attack, field === "held-health" ? value : held.health);
    if (next === sim) drawEditor();
    else update(next);
    return true;
  }
  return false;
}

function onEditorChange(event: Event): void {
  const el = event.target as HTMLInputElement;
  if (onKitChange(el)) return;
  const field = el.dataset.field;
  if (field !== "attack" && field !== "health") return;
  const at = sideOf(el);
  const key = Number(el.dataset.key);
  const minion = at && sim[at.side][at.board].minions.find((m) => m.key === key);
  if (!at || !minion) return;
  // An empty field keeps the value it had; one out of range is clamped (setStats).
  if (el.value.trim() === "" || !Number.isFinite(Number(el.value))) return drawEditor();
  const value = Number(el.value);
  const attack = field === "attack" ? value : minion.attack;
  const health = field === "health" ? value : minion.health;
  const next = B.setStats(sim, at.side, at.board, key, attack, health);
  // A value clamped back to the one kept still redraws, so the field shows it.
  if (next === sim) return drawEditor();
  update(next);
}

/** Adds the gift chosen in the picked minion's list; at four, focus goes to its first gift. */
function addGift(key: number, side: B.Side, board: number): void {
  const list = els.table.querySelector<HTMLSelectElement>(`[data-focus="ga${key}"]`);
  if (!list || list.value === "") return list?.focus();
  update(B.addGift(sim, side, board, key, list.value));
  if (!els.table.querySelector(`[data-focus="ga${key}"]`)) focusOn(`[data-focus^="gr${key}-"]`);
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
    // A failed "More" keeps the games already shown and can be tried again.
    if (append && games.kind === "ready") {
      page -= 1;
      games = { ...games, more: true };
      notes = [sm("load.error")];
      drawNotes();
    } else {
      games = { kind: "error" };
    }
  }
  drawLoad();
}

async function pickGame(key: string): Promise<void> {
  if (!account || games.kind !== "ready") return;
  const row = games.rows.find((r) => gameKey(r) === key);
  if (!row || !row.publicId) return;
  picked = { key, rounds: null, failed: false, loaded: null };
  record = null;
  drawLoad();
  const ref = { userId: account.userId, publicId: row.publicId };
  const loaded = await loadGame(account.download, ref, row.storagePath ?? undefined);
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
  // Without the card data a round would load with every minion left out: it says so instead.
  if (cards.size === 0) {
    els.status.textContent = sm("page.loading");
    return;
  }
  const loaded = B.fromRound(record, n, cards);
  if (!loaded) return;
  picked = { ...picked, loaded: n };
  selected = null;
  adding = null;
  notes = [sm("load.loaded", { n })];
  if (loaded.skipped > 0) notes.push(loaded.skipped === 1 ? sm("load.skipped.one") : sm("load.skipped.other", { count: loaded.skipped }));
  const boards = [...loaded.state.player, ...loaded.state.opponent];
  if (boards.some((b) => b.tier === null)) notes.push(sm("load.unknownTiers"));
  if (loaded.giftsNotRecorded) notes.push(sm("load.giftsNotRecorded"));
  if (loaded.unknownGifts > 0) notes.push(loaded.unknownGifts === 1 ? sm("load.unknownGifts.one") : sm("load.unknownGifts.other", { count: loaded.unknownGifts }));
  if (loaded.kitsNotRecorded) notes.push(sm("load.kitsNotRecorded"));
  holding = null;
  drawLoad();
  drawNotes();
  update({ ...loaded.state, cap: sim.cap });
}

function onLoadClick(event: Event): void {
  const el = (event.target as HTMLElement).closest<HTMLElement>("[data-act]");
  if (!el) return;
  if (el.dataset.act === "games-retry") {
    page = 1;
    void (account ? listGames(false) : signIn().catch(() => undefined));
  }
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
    // The page already holds this note (Simulator.astro): kept as it is, so
    // it is not drawn a second time; shown even if the page guessed "in".
    const note = els.load.querySelector(".sim-load-out");
    if (!note) return drawLoad();
    els.load.querySelector(".sim-load-wait")?.remove();
    note.classList.remove("sim-load-out");
    return;
  }
  // Only the signed-in player's own games: the id comes from their session, never from the address.
  account = {
    userId: user.id,
    download: (path) => supabase.storage.from("games").download(path),
    list: (p) => loadGames(supabase, user.id, { ...DEFAULT_FILTERS, page: p }, GAME_FILE_COLUMNS),
  };
  await listGames(false);
}

// ---------- Start ----------

/** The card data in the page's language (a script chunk from the site itself). */
async function loadCards(): Promise<void> {
  const file = lang() === "es" ? await import("./generated/cards.es.json") : await import("./generated/cards.en.json");
  cards = new CardIndex(readCardFile(file.default));
  // The names of the buff cards (spells and the like) for the cards' tooltips; without them the tooltip shows ids.
  useCardNames(await loadCardNames());
}

/** The page's listeners, once: "Try again" runs start() again without adding them twice. */
function listen(): void {
  watchPictureFallback(document);
  watchTipEscape(document);
  els.table.addEventListener("click", onEditorClick);
  els.table.addEventListener("input", onEditorInput);
  els.table.addEventListener("change", onEditorChange);
  els.table.addEventListener("change", onResultChange);
  els.load.addEventListener("click", onLoadClick);
}

async function start(): Promise<void> {
  // The games list does not need the card data: it is asked for at once.
  const listed = signIn().catch(() => {
    games = { kind: "error" };
    drawLoad();
  });
  await loadCards();
  els.opponent.removeAttribute("aria-busy");
  els.player.removeAttribute("aria-busy");
  drawEditor();
  drawResult();
  await listed;
}

/** After the page's first paint: the card data and the sign-in client load behind it. */
const afterPaint = (): Promise<void> =>
  // A tab opened in the background paints nothing yet (and pauses animation frames): it starts at once.
  document.visibilityState === "hidden" ? Promise.resolve() : new Promise((done) => requestAnimationFrame(() => setTimeout(done, 0)));

listen();
afterPaint()
  .then(start)
  .catch(() => showFailed());
