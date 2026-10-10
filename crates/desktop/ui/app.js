// Tavern Ledger window: shows the local history, its stats and the tracker's status.
// It only renders: every number comes from the app (tracker::stats).
// Values are set with textContent only; nothing from the history is parsed as HTML.
"use strict";

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

// The words of the interface come from the language tables (i18n.js,
// strings/window.js); `tl` is a short name for the lookup (`t` is a common
// local name, and this file's scope is shared with the other scripts).
const tl = TLI18n.t;
const tlPlural = TLI18n.plural;

// The game types with a switch of their own; the name shown comes from `mode.<id>`.
const MODES = { GT_BATTLEGROUNDS: true, GT_BATTLEGROUNDS_DUO: true };
const modeName = (id) => (MODES[id] ? tl(`mode.${id}`) : null);

let allGames = [];
let stats = { modes: [], legend: [] };
let curious = null; // the "Curious stats" tab (T-216), all modes together
let mode = null; // chosen game type; defaults to the mode with most games
let userPicked = false;
const OTHER = "OTHER"; // any other Battlegrounds type, so no game is hidden

function modeOf(game) {
  return MODES[game.report.game_type] ? game.report.game_type : OTHER;
}

function el(tag, className, text) {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (text !== undefined && text !== null) node.textContent = String(text);
  return node;
}

// Where a value comes from (T-109). The app decides the source (tracker::provenance)
// and this file only shows it: a mark for every source but the log, and a tooltip on every value.
const SOURCES = ["log", "inferred", "entered", "leaderboard", "card_data", "possible", "unknown"];
const sourceOf = (source) => (SOURCES.includes(source) ? source : "unknown");

// The words are the interface's own (source.<key>.label and .description),
// the same ones the Rust legend carries in English.
function sourceInfo(source) {
  const key = sourceOf(source);
  return { source: key, label: tl(`source.${key}.label`), description: tl(`source.${key}.description`) };
}

function addTip(node, source) {
  const info = sourceInfo(source);
  node.title = [node.title, tl("win.source.tip", { label: info.label, description: info.description })].filter(Boolean).join("\n");
}

function sourceMark(source) {
  const key = sourceOf(source);
  if (key === "log") return null;
  const info = sourceInfo(key);
  const mark = el("span", `src src-${key}`, info.label);
  mark.title = info.description;
  return mark;
}

// Tooltip on the value, plus a visible mark when it is not straight from the log.
function withSource(node, source) {
  addTip(node, source);
  const mark = sourceMark(source);
  if (mark) node.append(mark);
  return node;
}

// "Hearthstone_2026_10_09_15_26_25" + game 2 -> "09/10/2026, 15:26 · 2" (as the language writes dates)
// (the session start time; the log has no per-game date yet)
function played(session, index) {
  const m = /^Hearthstone_(\d{4})_(\d{2})_(\d{2})_(\d{2})_(\d{2})_\d{2}$/.exec(session);
  const when = m ? TLI18n.dateTime(new Date(Number(m[1]), Number(m[2]) - 1, Number(m[3]), Number(m[4]), Number(m[5]))) : session;
  return `${when} · ${index}`;
}

// Tooltip of a game: the game build and the parser that read it. A record
// from before the parser was saved is "unknown version", not an error.
function provenance(game) {
  const p = game.parser;
  const parser = p && typeof p.version === "string" ? tl("win.prov.parser", { version: p.version, revision: p.revision }) : tl("win.prov.parserUnknown");
  const build = Number.isInteger(game.report.build) ? tl("win.prov.build", { build: game.report.build }) : tl("win.prov.buildUnknown");
  return `${build} · ${parser}`;
}

// Stats of the shown mode; an empty tally until the app answers.
function modeStats() {
  return stats.modes.find((m) => m.mode === mode) || { top_half: null, totals: {}, heroes: [], tribes: [] };
}

// top is null when the mode's place rules are unknown: no colour then.
function placeBadge(place, top) {
  if (place === null || place === undefined) return el("span", "place unknown", "—");
  const kind = top === null ? "plain" : place === 1 ? "win" : place <= top ? "top4" : "bottom4";
  return el("span", `place ${kind}`, place);
}

const dash = (value, format) => (value === null || value === undefined ? "—" : format(value));
const avgText = (v) => dash(v, (x) => TLI18n.num(x, 2));
const shareText = (v) => dash(v, (x) => TLI18n.percent(x));

// The history is a local file; tolerate hand-edited or older records.
const list = (value) => (Array.isArray(value) ? value : []);

// Every picture in this window (T-304, T-320): fetched only when it comes into
// view, so long lists and closed folds spend none of the app's hourly picture
// budget, and a picture that cannot load leaves a plain shape of the same
// size instead of a gap. The crop that hides the white bars of the source art
// is in style.css.
const ART_OPTIONS = { lazy: true, placeholder: true };

function art(id, className) {
  const img = TLCards.art(id, className, ART_OPTIONS);
  if (img) img.title = tl("win.art.tip", { source: sourceInfo("card_data").label });
  return img;
}

// The hero's portrait, looked up by the log's card id in the card data
// (T-304); none when there is no id.
const portrait = (id) => art(id, "portrait");

// Hero name as the game's log printed it (in the game's language); the card
// id when the log gave none. Never a guessed name.
function heroCell(report, source) {
  const id = typeof report.hero === "string" ? report.hero : null;
  const names = report.card_names && typeof report.card_names === "object" ? report.card_names : {};
  const name = id && typeof names[id] === "string" ? names[id] : null;
  const cell = el("td", "hero", name || id || "—");
  if (name) cell.title = id;
  const img = portrait(id);
  if (img) cell.prepend(img);
  return withSource(cell, source);
}

function note(report) {
  if (report.status === "incomplete") return tl("win.note.incomplete");
  if (report.status === "unsupported") return tl("win.note.unsupported", { problems: list(report.problems).map(TLI18n.backend).join("; ") });
  return list(report.warnings).map(TLI18n.backend).join("; ");
}

// Played with the reconnect dev tool (D-043): left out of the stats and the upload.
const devReconnectTip = () => tl("win.dev.tip");

// The game's note under its hero, small; nothing when there is none (T-323).
function noteLine(game) {
  const text = note(game.report);
  if (!text && game.dev_reconnect !== true) return null;
  const line = el("span", "row-note", text);
  if (game.dev_reconnect === true) {
    const mark = el("span", "dev-mark", tl("win.dev.mark"));
    mark.title = devReconnectTip();
    line.prepend(mark, " ");
  }
  return line;
}

// One game of the ledger. The mode is the list's own (the mode switch), so it
// is not repeated; tribes and the problem report are in the game's recap.
function row(game, top) {
  const r = game.report;
  const tr = el("tr", game.dev_reconnect === true ? "dev-reconnect" : null);
  const src = game.sources || {};
  const playedCell = el("td", "played", played(game.session, game.index));
  playedCell.title = provenance(game);
  const hero = heroCell(r, src.hero);
  // "Other" holds several game types: each row says its own.
  if (mode === OTHER) hero.append(withSource(el("span", "row-note", r.game_type || tl("win.row.unknownType")), src.mode));
  const line = noteLine(game);
  if (line) hero.append(line);
  tr.append(withSource(playedCell, src.played), hero);
  const placeCell = el("td", "num");
  placeCell.append(placeBadge(r.final_place, top));
  const open = el("td", "actions");
  open.append(recapButton(game));
  tr.append(
    withSource(placeCell, src.place),
    withSource(el("td", "num", r.final_health ?? "—"), src.health),
    withSource(el("td", "num", list(r.rounds).length || "—"), src.rounds),
    open,
  );
  return tr;
}

const LEDGER_LIMIT = 8;

function ledgerTable(games, top) {
  const columns = [
    { label: tl("win.col.played") },
    { label: tl("win.col.hero") },
    { label: tl("win.col.place"), num: true, sort: (g) => g.report.final_place, low: true },
    { label: tl("win.col.health"), num: true, sort: (g) => g.report.final_health },
    { label: tl("win.col.rounds"), num: true, sort: (g) => list(g.report.rounds).length || null },
    { label: "", head: () => el("span", "visually-hidden", tl("win.ledger.openRecap")) },
  ];
  return TLCompact.table(SHOP_UI, {
    key: `ledger:${mode}`, columns, rows: games, row: (g) => row(g, top), limit: LEDGER_LIMIT, label: tl("win.ledger.label"),
  });
}

function topLabel(top) {
  return top === null ? tl("win.top.half") : tl("win.top.n", { n: top });
}

// "—" when no game counts: no games is unknown, not 0.
function renderTally(m, devGames) {
  const t = m.totals;
  const src = m.totals_sources || {};
  const set = (id, value, source) => {
    const node = document.getElementById(id);
    node.textContent = value;
    node.title = "";
    addTip(node, source);
    const mark = sourceMark(source);
    document.getElementById(`${id}-src`).replaceChildren(...(mark ? [mark] : []));
  };
  document.getElementById("stat-top-label").textContent = topLabel(m.top_half);
  set("stat-games", t.games || "—", src.games);
  set("stat-avg", avgText(t.average_place), src.average_place);
  set("stat-top", shareText(t.top_half_share), src.top_half_share);
  set("stat-wins", dash(t.wins, String), src.wins);
  const notCounted = (t.games || 0) - (t.placed || 0);
  const note = document.getElementById("not-counted");
  note.hidden = notCounted === 0 && devGames === 0;
  const parts = [
    t.incomplete && tl("win.tally.incomplete", { n: t.incomplete }),
    t.unsupported && tl("win.tally.unsupported", { n: t.unsupported }),
  ].filter(Boolean);
  const why = m.top_half === null ? tl("win.tally.rulesUnknown") : parts.join(", ");
  const counted = { counted: notCounted, games: t.games, why };
  note.textContent = notCounted === 0 ? "" : tl(why ? "win.tally.notCountedWhy" : "win.tally.notCounted", counted);
  if (devGames > 0) note.textContent += (notCounted === 0 ? "" : " ") + tl("win.tally.dev", { n: devGames });
}

function heroRow(h, top) {
  const tr = el("tr");
  const cell = el("td", h.name ? "hero-name" : "hero", h.name || h.hero || tl("win.hero.unknown"));
  if (h.variants.length) cell.title = h.variants.join("\n");
  const img = portrait(h.hero);
  if (img) cell.prepend(img);
  withSource(cell, h.name_source);
  withSource(cell, h.grouping);
  const t = h.tally;
  const src = h.tally_sources || {};
  tr.append(
    cell,
    withSource(el("td", "num", t.games), src.games),
    withSource(el("td", "num", avgText(t.average_place)), src.average_place),
    withSource(el("td", "num", top === null ? "—" : shareText(t.top_half_share)), src.top_half_share),
    withSource(el("td", "num", dash(t.wins, String)), src.wins),
  );
  return tr;
}

// The log's race names (NEUTRAL is a minion with no tribe, ALL one with every
// tribe) in the language; one the tables do not know shows capitalised.
const tribeLabel = (name) => {
  const key = `tribe.${name}`;
  return key in TLI18n.tables().en ? tl(key) : name.charAt(0) + name.slice(1).toLowerCase();
};

// Lobby tribes entered by hand (T-303). The app owns the fixed list, the rules
// and the saving; this file shows them. Always marked "entered by you", and
// never merged into the tribes seen in the tavern.
let lobby = null;
let tribesTarget = null; // { kind: "game", session, index } or { kind: "pending" }

const gameKey = (g) => `${g.session}#${g.index}`;
const sameList = (a, b) => a.length === b.length && a.every((t, i) => t === b[i]);

function entryOf(target) {
  if (!lobby) return null;
  if (target.kind === "pending") return lobby.pending;
  const found = lobby.games.find((g) => g.session === target.session && g.index === target.index);
  return found ? found.tribes : null;
}

function tribeChips(tribes) {
  return tribes.map((t) => {
    const li = el("li", null, tribeLabel(t));
    li.title = t;
    return withSource(li, "entered");
  });
}

// Every redraw keeps focus where it was (T-317), see focus-keeper.js.
function renderLobby() {
  TLFocus.keep(document, renderLobbyNow);
}

function renderLobbyNow() {
  const state = document.getElementById("lobby-state");
  const current = document.getElementById("lobby-current");
  const enter = document.getElementById("lobby-enter");
  const clear = document.getElementById("lobby-clear");
  enter.hidden = clear.hidden = true;
  current.replaceChildren();
  if (!lobby) {
    state.textContent = tl("win.lobby.loading");
    return;
  }
  if (!lobby.available) {
    state.textContent = TLI18n.backend(lobby.problem || "Entering tribes is not available: this window does not own the history.");
    return;
  }
  const target = lobby.in_progress
    ? { kind: "game", session: lobby.in_progress.session, index: lobby.in_progress.index }
    : { kind: "pending" };
  const entered = entryOf(target);
  if (lobby.in_progress) {
    state.textContent = entered
      ? tl("win.lobby.state.liveEntered")
      : tl("win.lobby.state.livePick");
  } else {
    state.textContent = entered
      ? tl("win.lobby.state.idleEntered")
      : tl("win.lobby.state.idlePick");
  }
  current.replaceChildren(...(entered ? tribeChips(entered) : []));
  enter.textContent = entered ? tl("win.lobby.change") : lobby.in_progress ? tl("win.lobby.enter") : tl("win.lobby.enterNext");
  enter.onclick = () => openTribesDialog(target);
  enter.hidden = false;
  clear.hidden = !entered;
  clear.onclick = () => editTribes({ kind: target.kind === "pending" ? "clear_pending" : "clear", ...stripKind(target) }, "lobby-error");
}

const stripKind = ({ kind, ...rest }) => (kind === "pending" ? {} : rest);

async function refreshLobby() {
  lobby = await invoke("lobby_tribes_view");
  renderLobby();
  if (document.getElementById("tribes-dialog").open) renderTribesDialog();
}

async function editTribes(action, errorId) {
  const error = document.getElementById(errorId);
  error.hidden = true;
  try {
    lobby = await invoke("edit_lobby_tribes", { action });
    renderLobby();
    await refreshGames();
    return true;
  } catch (err) {
    error.textContent = TLI18n.backend(String(err));
    error.hidden = false;
    return false;
  }
}

// A dialog gives focus back to whatever opened it. The history is redrawn
// meanwhile, so that button may be a new one: it is found again by its key.
const dialogOpeners = new Map();

function showDialog(dialog) {
  if (dialog.open) return;
  dialogOpeners.set(dialog.id, TLFocus.note(document));
  dialog.showModal();
}

document.querySelectorAll("dialog").forEach((dialog) => {
  dialog.addEventListener("close", () => {
    TLFocus.restore(document, dialogOpeners.get(dialog.id));
    dialogOpeners.delete(dialog.id);
  });
});

function tribesTargetText(target) {
  if (target.kind === "pending") return tl("win.tribes.forNext");
  const game = allGames.find((g) => g.session === target.session && g.index === target.index);
  return game ? tl("win.tribes.forGame", { game: played(game.session, game.index) }) : tl("win.tribes.forLive");
}

function pickedTribes() {
  return [...document.querySelectorAll("#tribes-choices input:checked")].map((box) => box.value);
}

function updateTribesCount() {
  const need = lobby ? lobby.lobby_size : 5;
  const picked = pickedTribes().length;
  document.getElementById("tribes-count").textContent = tl("win.tribes.count", { picked, need });
  document.getElementById("tribes-save").disabled = picked !== need;
  // At the limit, the rest stay off, so a sixth tribe cannot be picked by mistake.
  document.querySelectorAll("#tribes-choices input").forEach((box) => {
    box.disabled = picked >= need && !box.checked;
  });
}

function moveTargets(target) {
  const withEntry = new Set(lobby.games.map(gameKey));
  const options = allGames
    .filter((g) => !withEntry.has(gameKey(g)) && !(g.session === target.session && g.index === target.index))
    .map((g) => ({ session: g.session, index: g.index, text: played(g.session, g.index) }));
  const live = lobby.in_progress;
  if (live && !withEntry.has(gameKey(live)) && !(live.session === target.session && live.index === target.index)) {
    options.unshift({ session: live.session, index: live.index, text: tl("win.tribes.gameLive") });
  }
  return options;
}

function renderTribesDialog() {
  TLFocus.keep(document, renderTribesDialogNow);
}

function renderTribesDialogNow() {
  const target = tribesTarget;
  if (!target || !lobby) return;
  const entered = entryOf(target);
  document.getElementById("tribes-target").textContent = tribesTargetText(target);
  document.getElementById("tribes-legend").textContent = tl("win.tribes.pick", { n: lobby.lobby_size });
  const keep = new Set(document.getElementById("tribes-choices").childElementCount ? pickedTribes() : entered || []);
  document.getElementById("tribes-choices").replaceChildren(...lobby.choices.map((choice) => {
    const label = el("label", "choice");
    const box = el("input");
    box.type = "checkbox";
    box.value = choice.id;
    box.setAttribute("data-focus", `tribe:${choice.id}`);
    box.checked = keep.has(choice.id);
    box.addEventListener("change", updateTribesCount);
    label.append(box, el("span", null, tribeLabel(choice.id)));
    return label;
  }));
  document.getElementById("tribes-clear").hidden = !entered;
  const moves = entered && target.kind === "game" ? moveTargets(target) : [];
  document.getElementById("tribes-move").hidden = moves.length === 0;
  document.getElementById("tribes-move-to").replaceChildren(...moves.map((m, i) => {
    const option = el("option", null, m.text);
    option.value = String(i);
    return option;
  }));
  document.getElementById("tribes-move-to").moves = moves;
  updateTribesCount();
}

function openTribesDialog(target) {
  if (!lobby || !lobby.available) return;
  tribesTarget = target;
  document.getElementById("tribes-choices").replaceChildren();
  const error = document.getElementById("tribes-error");
  error.hidden = true;
  renderTribesDialog();
  showDialog(document.getElementById("tribes-dialog"));
}

// The recap's "Tribes" button: says whether tribes were entered for its game.
function updateTribesButton(game) {
  const button = document.getElementById("recap-tribes");
  const entered = game && entryOf({ kind: "game", session: game.session, index: game.index });
  button.textContent = entered ? tl("win.recap.tribesDone") : tl("win.recap.tribes");
  button.title = entered ? tl("win.recap.tribesTip", { tribes: TLI18n.list(entered.map(tribeLabel)) }) : tl("win.recap.tribesEnter");
  button.hidden = !game || !lobby || !lobby.available;
}

async function saveTribes() {
  const target = tribesTarget;
  if (!target) return;
  const tribes = pickedTribes();
  const action = target.kind === "pending" ? { kind: "set_pending", tribes } : { kind: "set", ...stripKind(target), tribes };
  if (await editTribes(action, "tribes-error")) document.getElementById("tribes-dialog").close();
}

document.getElementById("tribes-save").addEventListener("click", saveTribes);
document.getElementById("tribes-close").addEventListener("click", () => document.getElementById("tribes-dialog").close());
document.getElementById("tribes-dialog").addEventListener("close", () => { tribesTarget = null; });
document.getElementById("tribes-clear").addEventListener("click", async () => {
  const target = tribesTarget;
  if (!target) return;
  const action = target.kind === "pending" ? { kind: "clear_pending" } : { kind: "clear", ...stripKind(target) };
  if (await editTribes(action, "tribes-error")) document.getElementById("tribes-dialog").close();
});
document.getElementById("tribes-move-go").addEventListener("click", async () => {
  const target = tribesTarget;
  const select = document.getElementById("tribes-move-to");
  const to = (select.moves || [])[Number(select.value)];
  if (!target || !to) return;
  const action = { kind: "move", session: target.session, index: target.index, to_session: to.session, to_index: to.index };
  if (await editTribes(action, "tribes-error")) document.getElementById("tribes-dialog").close();
});
document.getElementById("tribes-form").addEventListener("submit", (e) => e.preventDefault());

// The stats' long tables (T-323): top rows by games, ties broken by the better
// average place and then the name, "Show all" for the rest (compact.js).
function heroesTable(m) {
  const name = (h) => h.name || h.hero || "";
  const rows = TLCompact.rank(m.heroes, { games: (h) => h.tally.games, place: (h) => h.tally.average_place, name });
  const columns = [
    { label: tl("win.col.hero"), sort: name, text: true },
    { label: tl("win.col.games"), num: true, sort: (h) => h.tally.games },
    { label: tl("win.col.avgPlace"), num: true, sort: (h) => h.tally.average_place, low: true },
    { label: topLabel(m.top_half), num: true, sort: (h) => (m.top_half === null ? null : h.tally.top_half_share) },
    { label: tl("win.col.wins"), num: true, sort: (h) => h.tally.wins },
  ];
  return TLCompact.table(SHOP_UI, { key: `heroes:${mode}`, columns, rows, row: (h) => heroRow(h, m.top_half), label: tl("win.heroes.title") });
}

function tribesTable(m) {
  // Most games first, then most offers (as a lower-is-better number), then the name.
  const rows = TLCompact.rank(m.tribes, { games: (t) => t.games, place: (t) => -t.offers, name: (t) => tribeLabel(t.tribe) });
  const columns = [
    { label: tl("win.col.tribe"), sort: (t) => tribeLabel(t.tribe), text: true },
    { label: tl("win.col.games"), num: true, sort: (t) => t.games },
    { label: tl("win.col.offers"), num: true, sort: (t) => t.offers },
  ];
  return TLCompact.table(SHOP_UI, {
    key: `tribes:${mode}`, columns, rows, row: (t) => tribeRow(t, m.games_with_tribes, m.tribes_source), label: tl("win.tribes.seen.title"),
  });
}

function enteredTable(m, entered) {
  const rows = TLCompact.rank(entered, { games: (t) => t.games, name: (t) => tribeLabel(t.tribe) });
  const columns = [{ label: tl("win.col.tribe") }, { label: tl("win.col.games"), num: true }];
  return TLCompact.table(SHOP_UI, {
    key: `entered:${mode}`, columns, rows, row: (t) => enteredRow(t, m.games_with_entered_tribes, m.entered_tribes_source), label: tl("win.entered.title"),
  });
}

function enteredRow(t, total, source) {
  const tr = el("tr");
  const cell = el("td", null, tribeLabel(t.tribe));
  cell.title = t.tribe;
  const count = el("td", "num", tl("win.count.of", { n: t.games, total }));
  addTip(count, source);
  tr.append(withSource(cell, source), count);
  return tr;
}

function tribeRow(t, total, source) {
  const tr = el("tr");
  const cell = el("td", null, tribeLabel(t.tribe));
  cell.title = t.tribe;
  const counts = [el("td", "num", tl("win.count.of", { n: t.games, total })), el("td", "num", t.offers)];
  counts.forEach((c) => addTip(c, source));
  tr.append(withSource(cell, source), ...counts);
  return tr;
}

function renderBreakdown(m) {
  document.getElementById("heroes").replaceChildren(...(m.heroes.length ? [heroesTable(m)] : []));
  document.getElementById("heroes-empty").hidden = m.heroes.length > 0;
  document.getElementById("tribes").replaceChildren(...(m.tribes.length ? [tribesTable(m)] : []));
  document.getElementById("tribes-empty").hidden = m.tribes.length > 0;
  const entered = list(m.entered_tribes);
  document.getElementById("entered-tribes-section").hidden = entered.length === 0;
  document.getElementById("entered-tribes").replaceChildren(...(entered.length ? [enteredTable(m, entered)] : []));
  TLShop.renderStats(m, SHOP_UI, mode);
  document.getElementById("hero-picks-body").replaceChildren(...TLExtras.heroPickTable(m.hero_picks, SHOP_UI, `hero-picks:${mode}`));
  document.getElementById("choice-stats-body").replaceChildren(...TLRoundTwo.choiceTables(SHOP_UI, m.choices, mode));
}

// What shop.js and the other drawing files need to draw with the same marks,
// tooltips and pictures as this file.
const SHOP_UI = { el, withSource, sourceMark, addTip, art };

// The legend comes from the app, so its words are the ones on the marks.
function renderLegend() {
  const items = stats.legend.map((e) => {
    const mark = e.source === "log" ? el("span", "src src-log", tl("win.legend.noMark")) : sourceMark(e.source);
    const item = el("li");
    const words = (part, fallback) => (`source.${e.source}.${part}` in TLI18n.tables().en ? tl(`source.${e.source}.${part}`) : fallback);
    item.append(mark, el("span", null, `${words("label", e.label)}: ${words("description", e.description)}`));
    return item;
  });
  document.getElementById("legend-list").replaceChildren(...items);
}

// "Report a problem" (T-110): the app builds the file; the window shows it
// whole before it is saved. Nothing is sent anywhere.
let reportGame = null;

function showReportMessage(id, text) {
  const node = document.getElementById(id);
  node.textContent = text || "";
  node.hidden = !text;
}

async function openReport(game) {
  reportGame = { session: game.session, index: game.index };
  const target = reportGame;
  document.getElementById("report-text").textContent = "";
  showReportMessage("report-error", "");
  showReportMessage("report-saved", "");
  document.getElementById("report-save").disabled = true;
  showDialog(document.getElementById("report-dialog"));
  try {
    const text = await invoke("preview_problem_report", target);
    if (reportGame !== target) return;
    document.getElementById("report-text").textContent = text;
    document.getElementById("report-save").disabled = false;
  } catch (err) {
    if (reportGame === target) showReportMessage("report-error", TLI18n.backend(String(err)));
  }
}

async function saveReport() {
  const target = reportGame;
  if (!target) return;
  const save = document.getElementById("report-save");
  save.disabled = true;
  try {
    const path = await invoke("save_problem_report", target);
    if (reportGame !== target) return;
    showReportMessage("report-error", "");
    showReportMessage("report-saved", tl("win.report.saved", { path }));
  } catch (err) {
    if (reportGame !== target) return;
    showReportMessage("report-error", TLI18n.backend(String(err)));
    save.disabled = false;
  }
}

document.getElementById("report-save").addEventListener("click", saveReport);
document.getElementById("report-close").addEventListener("click", () => document.getElementById("report-dialog").close());
document.getElementById("report-dialog").addEventListener("close", () => { reportGame = null; });

// Recap of one game (T-202, T-203). The app works everything out (tracker::recap),
// including every source label; this file only draws it. Opponents are heroes
// and seats of this one game, never names.
let recapGame = null;
const OUTCOMES = ["won", "lost", "tie", "unknown"];
const outcomeWord = (outcome) => tl(`outcome.${OUTCOMES.includes(outcome) ? outcome : "unknown"}`);
const SVG_NS = "http://www.w3.org/2000/svg";

function recapButton(game) {
  const button = el("button", "report-btn", tl("win.recap.button"));
  button.type = "button";
  button.setAttribute("aria-label", tl("win.recap.buttonLabel", { game: played(game.session, game.index) }));
  button.setAttribute("data-focus", `recap:${game.session}:${game.index}`);
  button.addEventListener("click", () => openRecap(game));
  return button;
}

const heroText = (hero) => (hero && (hero.name || hero.id)) || "—";

// The hero's name with its portrait in front, when there is one.
function heroWithPortrait(hero) {
  const node = el("span", "hero-with-portrait", heroText(hero));
  const img = hero && portrait(hero.id);
  if (img) node.prepend(img);
  return node;
}

function fact(label, value, source) {
  const item = el("div", "fact");
  const dd = el("dd", null);
  dd.append(value);
  item.append(el("dt", null, label), withSource(dd, source));
  return item;
}

function statusText(status) {
  if (status === "ok") return tl("win.state.finished");
  if (status === "incomplete") return tl("win.state.incomplete");
  if (status === "unsupported") return tl("win.state.unreadable");
  return "—";
}

// The hero, the place, the mode and the date are in the recap's top bar.
function recapFacts(r) {
  const facts = el("dl", "facts");
  if (r.is_duos) facts.append(fact(tl("win.fact.teammate"), heroWithPortrait(r.teammate_hero.value), r.teammate_hero.source));
  facts.append(fact(tl("win.fact.health"), r.final_health.value ?? "—", r.final_health.source));
  facts.append(fact(tl("win.fact.rounds"), r.rounds_played.value ?? "—", r.rounds_played.source));
  if (r.status.value !== "ok") facts.append(fact(tl("win.fact.state"), statusText(r.status.value), r.status.source));
  return facts;
}

// The top bar: the hero's portrait and name, the place, the mode and the date.
function renderRecapBar(r, game) {
  const hero = r ? r.hero.value : null;
  const title = document.getElementById("recap-title");
  title.textContent = hero ? heroText(hero) : tl("win.recap.title");
  title.title = "";
  if (r) withSource(title, r.hero.source);
  const img = hero && art(hero.id, "recap-art");
  document.getElementById("recap-portrait").replaceChildren(...(img ? [img] : []));
  const kind = r ? modeName(r.game_type.value) || r.game_type.value : null;
  const sub = document.getElementById("recap-sub");
  sub.textContent = [kind, played(game.session, game.index)].filter(Boolean).join(" · ");
  sub.title = tl("win.recap.subTip");
  if (r) addTip(sub, r.game_type.source);
  const place = document.getElementById("recap-place");
  place.replaceChildren();
  place.title = "";
  if (r) {
    place.append(el("span", "recap-place-label", tl("win.recap.place")), placeBadge(r.place.value, r.top_half));
    withSource(place, r.place.source);
  }
}

function svg(tag, attrs) {
  const node = document.createElementNS(SVG_NS, tag);
  Object.entries(attrs).forEach(([k, v]) => node.setAttribute(k, String(v)));
  return node;
}

const pointWords = (p) => (p.round === 0 ? tl("win.chart.before") : tl("win.chart.after", { round: p.round }));

// Health after each round as a line; a round with no number breaks the line.
function healthChart(points) {
  const wrap = el("div", "chart");
  const known = points.filter((p) => Number.isInteger(p.health));
  if (!known.length) {
    wrap.append(el("p", "empty", tl("win.chart.noHealth")));
    return wrap;
  }
  // Room under the line for the round numbers and above it for the health values.
  const width = 480, height = 140, pad = 16, padTop = 24, padBottom = 30;
  const top = Math.max(...known.map((p) => p.health), 1);
  const last = Math.max(points.length - 1, 1);
  const x = (i) => pad + (i * (width - 2 * pad)) / last;
  const y = (h) => height - padBottom - (h * (height - padTop - padBottom)) / top;
  const chart = svg("svg", { viewBox: `0 0 ${width} ${height}`, role: "img", class: "health-chart" });
  chart.setAttribute("aria-label", tl("win.chart.label", { points: points.map((p) => `${pointWords(p)}: ${p.health ?? tl("win.chart.unknown")}`).join(", ") }));
  chart.append(svg("line", { x1: pad, y1: y(0), x2: width - pad, y2: y(0), class: "axis" }));
  const label = (cx, cy, text, className) => {
    const node = svg("text", { x: cx, y: cy, "text-anchor": "middle", class: className });
    node.textContent = text;
    return node;
  };
  let run = [];
  const flush = () => {
    if (run.length > 1) chart.append(svg("polyline", { points: run.join(" "), class: "line" }));
    run = [];
  };
  points.forEach((p, i) => {
    // The round under every point, the health over it: nothing is for screen readers only.
    chart.append(label(x(i), height - padBottom + 16, p.round, "round-label"));
    if (!Number.isInteger(p.health)) {
      chart.append(label(x(i), y(0) - 6, "?", "value-label unknown"));
      return flush();
    }
    run.push(`${x(i)},${y(p.health)}`);
    const dot = svg("circle", { cx: x(i), cy: y(p.health), r: 3, class: "dot" });
    const tip = svg("title", {});
    tip.textContent = tl("win.chart.point", { when: pointWords(p), health: p.health });
    dot.append(tip);
    chart.append(dot, label(x(i), y(p.health) - 8, p.health, "value-label"));
  });
  flush();
  wrap.append(chart, TLCompact.about(SHOP_UI, "health-chart", tl("win.chart.help"), tl("win.chart.howTo")));
  return wrap;
}

function combatChip(c) {
  const word = outcomeWord(c.outcome);
  const chip = el("span", `chip chip-${c.outcome} src-${sourceOf(c.source)}`, tl("win.chip.text", { round: c.round, word }));
  chip.title = c.reason ? tl("win.chip.unknownTip", { round: c.round, reason: TLI18n.backend(c.reason) }) : tl("win.chip.tip", { round: c.round, word });
  addTip(chip, c.source);
  return chip;
}

function countCell(n, source) {
  const cell = el("td", "num", n);
  addTip(cell, source);
  return cell;
}

function opponentRow(o) {
  const tr = el("tr");
  const who = el("td", "opponent", o.heroes.map(heroText).join(" + ") || tl("win.hero.unknown"));
  who.prepend(...o.heroes.map((h) => portrait(h.id)).filter(Boolean));
  who.title = o.heroes.map((h) => h.id).concat(o.seats.map((s) => tl("win.opp.seat", { n: s }))).join("\n");
  if (o.final_place.value !== null) {
    const place = el("span", "finished", tl("win.opp.finished", { place: o.final_place.value }));
    who.append(" ", withSource(place, o.final_place.source));
  }
  const chips = el("td", "chips");
  chips.append(...o.combats.map(combatChip));
  // Won, lost and tie: from the log when the game recorded each one, inferred
  // when any was worked out from health (D-055). An unknown count is not a result.
  // No result at all against them: "—", not a count of 0 wins.
  const src = o.counts_source;
  const n = (v) => (src === "unknown" ? "—" : v);
  tr.append(who, countCell(n(o.won), src), countCell(n(o.lost), src), countCell(n(o.tie), src), countCell(o.unknown, "unknown"), chips);
  return tr;
}

function recordTable(r) {
  const head = el("tr");
  head.append(el("th", null, r.is_duos ? tl("win.opp.team") : tl("win.opp.single")));
  // The mark on the headings: inferred when any result was worked out from
  // health, unknown when the game has no result at all.
  const counted = TLExtras.resultsLine(r.results).source;
  ["won", "lost", "tie"].forEach((outcome) => {
    const th = el("th", "num", outcomeWord(outcome));
    const mark = sourceMark(counted);
    if (mark) th.append(mark);
    head.append(th);
  });
  head.append(el("th", "num", outcomeWord("unknown")), el("th", null, tl("win.col.rounds")));
  head.querySelectorAll("th").forEach((th) => th.setAttribute("scope", "col"));
  const thead = el("thead");
  thead.append(head);
  const tbody = el("tbody");
  tbody.append(...r.record.map(opponentRow));
  const table = el("table", "record");
  table.append(thead, tbody);
  return table;
}

function recordSection(r) {
  const section = el("section", "recap-part");
  const results = TLExtras.resultsLine(r.results);
  const heading = el("h3", null, tl("win.record.title"));
  const mark = sourceMark(results.source);
  if (mark) heading.append(mark);
  section.append(heading);
  section.append(r.record.length ? recordTable(r) : el("p", "empty", tl("win.record.none")));
  const basis = TLCompact.about(SHOP_UI, "record", TLI18n.backend(r.record_basis), tl("win.record.basis"));
  if (results.text) basis.append(withSource(el("p", "note results-line", results.text), results.source));
  section.append(basis);
  if (r.record.length) section.append(TLRoundTwo.opponentFolds(SHOP_UI, r, heroText, recapGame ? gameKey(recapGame) : ""));
  if (r.unattributed.length) {
    const reasons = [...new Set(r.unattributed.map((c) => TLI18n.backend(c.reason)).filter(Boolean))];
    const rounds = r.unattributed.map((c) => c.round).join(", ");
    section.append(withSource(el("p", "note", tl("win.record.unattributed", { rounds, reasons: reasons.join("; ") })), "unknown"));
  }
  if (r.missing_rounds.length) {
    section.append(withSource(el("p", "note", tl("win.record.missing", { rounds: r.missing_rounds.join(", ") })), "unknown"));
  }
  return section;
}

function tribesSection(r) {
  const section = el("section", "recap-part");
  const heading = el("h3", null, tl("win.tribes.seen.title"));
  heading.append(sourceMark(r.tribes_source));
  section.append(heading);
  if (!r.tribes.length) {
    section.append(el("p", "empty", tl("win.tribesSection.none")));
    return section;
  }
  section.append(el("p", "note", tl("win.tribesSection.note")));
  // The mark is on the heading once; each tribe keeps the tooltip.
  const list = el("ul", "tribe-list");
  list.append(...r.tribes.map((t) => {
    const li = el("li", null, `${tribeLabel(t.tribe)} ${t.offers}`);
    li.title = t.tribe;
    addTip(li, r.tribes_source);
    return li;
  }));
  section.append(list);
  return section;
}

// What the user entered for this game. Shown next to the tavern list, never
// merged with it and never used to correct it, whether the two agree or not.
function enteredSection(r) {
  const section = el("section", "recap-part");
  const heading = el("h3", null, tl("win.lobby.title"));
  heading.append(sourceMark(r.entered_tribes_source));
  section.append(heading);
  if (!r.entered_tribes.length) {
    section.append(el("p", "empty", tl("win.entered.none")));
    return section;
  }
  const items = el("ul", "tribe-list");
  items.append(...tribeChips(r.entered_tribes));
  section.append(items);
  if (r.tribes.length) {
    section.append(el("p", "note", tl("win.entered.note")));
  }
  return section;
}

// Folded: open only when there is something to read (T-323).
function messagesSection(r) {
  const section = el("section", "recap-part");
  const all = r.problems.map((m) => tl("win.msg.problem", { text: TLI18n.backend(m) })).concat(r.warnings.map(TLI18n.backend));
  if (!all.length) {
    section.append(el("p", "note", tl("win.msg.none")));
    return section;
  }
  const box = TLCompact.fold(SHOP_UI, `${recapGame ? gameKey(recapGame) : ""}:warnings`, tl("win.msg.title", { n: all.length }));
  const items = el("ul", "messages");
  items.append(...all.map((m) => el("li", null, m)));
  box.append(items);
  section.append(box);
  return section;
}

function renderRecap(r, game) {
  TLFocus.keep(document, () => renderRecapNow(r, game));
}

// Most telling first: health and turns, then the record, then the details.
function renderRecapNow(r, game) {
  const key = gameKey(game);
  const report = (allGames.find((g) => gameKey(g) === key) || {}).report || null;
  renderRecapBar(r, game);
  updateTribesButton(game);
  const health = el("section", "recap-part");
  health.append(withSource(el("h3", null, tl("win.recap.healthTitle")), r.health_source), healthChart(r.health));
  const top = el("div", "recap-top");
  top.append(recapFacts(r), health);
  const turnTools = { card: TLShop.card, cardRow: TLShop.cardRow, fold: TLCompact.fold, art, heroText, game: key };
  document.getElementById("recap-body").replaceChildren(
    top, TLShop.recapSummary(r.shop, SHOP_UI, r.is_duos, key), TLTurns.section(SHOP_UI, r, report, turnTools),
    recordSection(r), TLExtras.heroPickSection(r.hero_pick, SHOP_UI, portrait),
    TLRoundTwo.choicesSection(SHOP_UI, r.choices, key), enteredSection(r), tribesSection(r), messagesSection(r),
  );
}

// The recap fills the window (T-322): the X at the top right or Escape goes
// back to the list, scrolled as it was, with focus on the game's button.
const recapView = TLRecapView.create({
  doc: document,
  win: window,
  focus: TLFocus,
  view: document.getElementById("recap-view"),
  heading: document.getElementById("recap-title"),
  hide: [document.getElementById("masthead"), document.getElementById("home")],
  blocked: () => Boolean(document.querySelector("dialog[open]")),
  onClose: () => { recapGame = null; },
});

// Loads (or reloads) the recap of `game`; opens the view if it is not open.
async function openRecap(game) {
  const target = { session: game.session, index: game.index };
  const reload = recapView.isOpen() && recapGame && gameKey(recapGame) === gameKey(target);
  recapGame = target;
  if (!reload) {
    renderRecapBar(null, target);
    updateTribesButton(target);
    document.getElementById("recap-body").replaceChildren();
  }
  showReportMessage("recap-error", "");
  recapView.open(`recap:${target.session}:${target.index}`);
  try {
    const recap = await invoke("game_recap", target);
    if (recapGame === target) renderRecap(recap, target);
  } catch (err) {
    if (recapGame === target) showReportMessage("recap-error", TLI18n.backend(String(err)));
  }
}

document.getElementById("recap-close").addEventListener("click", () => recapView.close());
document.getElementById("recap-tribes").addEventListener("click", () => {
  if (recapGame) openTribesDialog({ kind: "game", session: recapGame.session, index: recapGame.index });
});
document.getElementById("recap-report").addEventListener("click", () => {
  if (recapGame) openReport(recapGame);
});

// A game that just ended: show its recap, unless a dialog or another recap is open.
async function onGameFinished(finished) {
  await refreshGames();
  if (document.querySelector("dialog[open]") || recapView.isOpen()) return;
  const game = allGames.find((g) => g.session === finished.session && g.index === finished.index);
  if (game) openRecap(game);
}

// The mode with most games, Solo when there are none.
function pickDefaultMode() {
  const known = stats.modes.filter((m) => MODES[m.mode] && m.totals.games > 0);
  known.sort((a, b) => b.totals.games - a.totals.games);
  return known.length ? known[0].mode : "GT_BATTLEGROUNDS";
}

function render() {
  TLFocus.keep(document, renderNow);
}

function renderNow() {
  const games = allGames.filter((g) => modeOf(g) === mode);
  const m = modeStats();
  document.querySelector('.modes [data-mode="OTHER"]').hidden = !allGames.some((g) => modeOf(g) === OTHER);
  document.getElementById("games").replaceChildren(...(games.length ? [ledgerTable(games, m.top_half)] : []));
  const empty = document.getElementById("empty");
  empty.hidden = games.length > 0;
  empty.textContent = tl("win.ledger.empty", { mode: modeName(mode) || tl("win.mode.else") });
  document.querySelectorAll(".modes button").forEach((b) => {
    b.setAttribute("aria-pressed", String(b.dataset.mode === mode));
  });
  renderTally(m, games.filter((g) => g.dev_reconnect === true).length);
  renderBreakdown(m);
  TLCurious.render(curious, SHOP_UI, document.getElementById("curious-body"));
  renderLegend();
  if (recapGame) updateTribesButton(recapGame);
}

// Refreshes can overlap; only the latest one may paint, so an older answer
// never replaces a newer one.
let refreshSeq = 0;

async function refreshGames() {
  const seq = ++refreshSeq;
  const [games, fresh, freshCurious] = await Promise.all([invoke("list_games"), invoke("game_stats"), invoke("curious_stats")]);
  if (seq !== refreshSeq) return;
  allGames = games.slice().reverse(); // newest first
  stats = fresh;
  curious = freshCurious;
  // Also on every refresh: the first one can run before the app opened the entries.
  const freshLobby = await invoke("lobby_tribes_view");
  if (seq !== refreshSeq) return;
  lobby = freshLobby;
  // Until the user picks a mode, show the one with most games (the history
  // may still be opening on the first call, so this runs on every refresh).
  if (!userPicked) mode = pickDefaultMode();
  render();
  renderLobby();
}

// "Your games", "Stats", "Curious stats" and Settings (T-216, T-323, T-324); all are
// drawn on every refresh. Curious stats count every mode and Settings none, so the
// mode switch hides on those two.
const tabIds = [...document.querySelectorAll('#view-tabs [role="tab"]')].map((t) => t.id);
// Settings are not about a mode either, and the legend is about the values.
const showModes = (tab) => {
  const bare = tabIds[tab] === "tab-curious" || tabIds[tab] === "tab-settings";
  document.getElementById("modes").hidden = bare;
  document.querySelector("details.legend").hidden = tabIds[tab] === "tab-settings";
};
showModes(TLTabs.setup(document, "view-tabs", showModes).current());
TLScrollCue.start(window, document);

document.querySelectorAll(".modes button").forEach((button) => {
  button.addEventListener("click", () => {
    mode = button.dataset.mode;
    userPicked = true;
    render();
  });
});

function renderStatus(s) {
  const node = document.getElementById("status");
  node.classList.toggle("problem", Boolean(s.problem || s.notice));
  node.classList.toggle("live", !s.problem && Boolean(s.session) && Boolean(s.power_log));
  if (s.problem) node.textContent = TLI18n.backend(s.problem);
  else if (s.session && !s.power_log) node.textContent = tl("win.status.noPowerLog", { session: s.session });
  else if (s.session && s.scene) node.textContent = tl("win.status.followingScene", { session: s.session, scene: TLI18n.backend(s.scene) });
  else if (s.session) node.textContent = tl("win.status.following", { session: s.session });
  else if (s.logs_dir) node.textContent = tl("win.status.waiting");
  else node.textContent = tl("win.status.starting");
  if (s.notice) node.textContent += ` — ${TLI18n.backend(s.notice)}`;
  if (s.dev_reconnects) node.textContent += ` — ${TLI18n.backend(s.dev_reconnects)}`;
  node.title = [s.logs_dir && tl("win.status.logs", { path: s.logs_dir }), s.history && tl("win.status.history", { path: s.history })]
    .filter(Boolean).join("\n");
}

// What to change in log.config / client.config; hidden when the setup is fine.
function renderSetup(s) {
  const lines = Array.isArray(s.setup) ? s.setup : [];
  const list = document.getElementById("setup-list");
  list.replaceChildren(...lines.map((line) => el("li", null, TLI18n.backend(String(line)))));
  document.getElementById("setup").hidden = lines.length === 0;
}

// Where card pictures come from, and whether they are there (T-304).
function renderCardStatus() {
  const node = document.getElementById("card-status");
  const s = TLCards.status();
  node.classList.toggle("problem", Boolean(s && s.problem));
  if (!s) node.textContent = tl("win.cards.unavailable");
  else if (s.problem) node.textContent = tl(s.cards ? "win.cards.problemKept" : "win.cards.problemIds", { problem: TLI18n.backend(s.problem) });
  else if (s.cards) node.textContent = tl("win.cards.known", { cards: TLI18n.num(s.cards), date: TLI18n.date(new Date(s.fetched_at * 1000)) });
  else node.textContent = tl("win.cards.getting");
}

async function main() {
  // The first draw is already in the chosen language.
  await TLI18n.ready();
  TLCards.onChange(() => {
    renderCardStatus();
    render();
  });
  await listen("games-changed", () => refreshGames().catch(showError));
  const showStatus = (s) => { renderStatus(s); renderSetup(s); };
  // The entries or the game in progress changed: the list, the stats and an open recap follow.
  await listen("lobby-tribes-changed", async () => {
    await refreshLobby();
    await refreshGames();
    if (recapGame) openRecap(recapGame).catch(showError);
  });
  await listen("game-finished", (e) => onGameFinished(e.payload).catch(showError));
  await listen("status-changed", (e) => showStatus(e.payload));
  showStatus(await invoke("status"));
  await refreshGames();
}

function showError(err) {
  const text = tl("win.error.generic", { error: TLI18n.backend(String(err)) });
  renderStatus({ problem: text });
  // The status line is hidden while a recap fills the window.
  if (recapView.isOpen()) showReportMessage("recap-error", text);
}

main().catch(showError);
