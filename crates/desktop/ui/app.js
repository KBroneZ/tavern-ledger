// Tavern Ledger window: shows the local history, its stats and the tracker's status.
// It only renders: every number comes from the app (tracker::stats).
// Values are set with textContent only; nothing from the history is parsed as HTML.
"use strict";

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const MODES = { GT_BATTLEGROUNDS: "Solo", GT_BATTLEGROUNDS_DUO: "Duos" };

let allGames = [];
let stats = { modes: [], legend: [] };
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
const SOURCES = ["log", "inferred", "entered", "leaderboard", "unknown"];
const sourceOf = (source) => (SOURCES.includes(source) ? source : "unknown");

function sourceInfo(source) {
  const key = sourceOf(source);
  return stats.legend.find((e) => e.source === key) || { source: key, label: key, description: "" };
}

function addTip(node, source) {
  const info = sourceInfo(source);
  node.title = [node.title, `Source: ${info.label}. ${info.description}`].filter(Boolean).join("\n");
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

// "Hearthstone_2026_10_09_15_26_25" + game 2 -> "2026-10-09 15:26 · 2"
// (the session start time; the log has no per-game date yet)
function played(session, index) {
  const m = /^Hearthstone_(\d{4})_(\d{2})_(\d{2})_(\d{2})_(\d{2})_\d{2}$/.exec(session);
  const when = m ? `${m[1]}-${m[2]}-${m[3]} ${m[4]}:${m[5]}` : session;
  return `${when} · ${index}`;
}

// Tooltip of a game: the game build and the parser that read it. A record
// from before the parser was saved is "unknown version", not an error.
function provenance(game) {
  const p = game.parser;
  const parser = p && typeof p.version === "string" ? `parser ${p.version} r${p.revision}` : "parser unknown version";
  const build = Number.isInteger(game.report.build) ? `build ${game.report.build}` : "build unknown";
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
const avgText = (v) => dash(v, (x) => x.toFixed(2));
const shareText = (v) => dash(v, (x) => `${Math.round(x * 100)}%`);

// The history is a local file; tolerate hand-edited or older records.
const list = (value) => (Array.isArray(value) ? value : []);

// Hero name as the game's log printed it (in the game's language); the card
// id when the log gave none. Never a guessed name.
function heroCell(report, source) {
  const id = typeof report.hero === "string" ? report.hero : null;
  const names = report.card_names && typeof report.card_names === "object" ? report.card_names : {};
  const name = id && typeof names[id] === "string" ? names[id] : null;
  const cell = el("td", "hero", name || id || "—");
  if (name) cell.title = id;
  return withSource(cell, source);
}

function note(report) {
  if (report.status === "incomplete") return "Game not finished in the log";
  if (report.status === "unsupported") return "Could not be read: " + list(report.problems).join("; ");
  return list(report.warnings).join("; ");
}

function row(game, top) {
  const r = game.report;
  const tr = el("tr");
  const src = game.sources || {};
  const playedCell = el("td", null, played(game.session, game.index));
  playedCell.title = provenance(game);
  tr.append(
    withSource(playedCell, src.played),
    withSource(el("td", null, MODES[r.game_type] || r.game_type || "—"), src.mode),
    heroCell(r, src.hero),
  );
  const placeCell = el("td", "num");
  placeCell.append(placeBadge(r.final_place, top));
  tr.append(
    withSource(placeCell, src.place),
    withSource(el("td", "num", r.final_health ?? "—"), src.health),
    withSource(el("td", "num", list(r.rounds).length || "—"), src.rounds),
    el("td", "note", note(r)),
    reportCell(game),
  );
  return tr;
}

function topLabel(top) {
  return top === null ? "Top half" : `Top ${top}`;
}

// "—" when no game counts: no games is unknown, not 0.
function renderTally(m) {
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
  note.hidden = notCounted === 0;
  const parts = [
    t.incomplete && `${t.incomplete} not finished in the log`,
    t.unsupported && `${t.unsupported} could not be read`,
  ].filter(Boolean);
  const why = m.top_half === null ? "place rules unknown for this mode" : parts.join(", ");
  note.textContent = `${notCounted} of ${t.games} games not counted for places` + (why ? ` (${why})` : "") + ".";
}

function heroRow(h, top) {
  const tr = el("tr");
  const cell = el("td", h.name ? "hero-name" : "hero", h.name || h.hero || "Unknown hero");
  if (h.variants.length) cell.title = h.variants.join("\n");
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

// The log's race names: NEUTRAL is a minion with no tribe, ALL one with every tribe.
const TRIBE_LABELS = { NEUTRAL: "No tribe", ALL: "All tribes" };
const tribeLabel = (name) => TRIBE_LABELS[name] || name.charAt(0) + name.slice(1).toLowerCase();

function tribeRow(t, total, source) {
  const tr = el("tr");
  const cell = el("td", null, tribeLabel(t.tribe));
  cell.title = t.tribe;
  const counts = [el("td", "num", `${t.games} of ${total}`), el("td", "num", t.offers)];
  counts.forEach((c) => addTip(c, source));
  tr.append(withSource(cell, source), ...counts);
  return tr;
}

function renderBreakdown(m) {
  document.getElementById("heroes-top-label").textContent = topLabel(m.top_half);
  document.getElementById("heroes").replaceChildren(...m.heroes.map((h) => heroRow(h, m.top_half)));
  document.getElementById("heroes-empty").hidden = m.heroes.length > 0;
  document.getElementById("tribes").replaceChildren(...m.tribes.map((t) => tribeRow(t, m.games_with_tribes, m.tribes_source)));
  document.getElementById("tribes-empty").hidden = m.tribes.length > 0;
}

// The legend comes from the app, so its words are the ones on the marks.
function renderLegend() {
  const items = stats.legend.map((e) => {
    const mark = e.source === "log" ? el("span", "src src-log", "no mark") : sourceMark(e.source);
    const item = el("li");
    item.append(mark, el("span", null, `${e.label}: ${e.description}`));
    return item;
  });
  document.getElementById("legend-list").replaceChildren(...items);
}

// "Report a problem" (T-110): the app builds the file; the window shows it
// whole before it is saved. Nothing is sent anywhere.
let reportGame = null;

function reportCell(game) {
  const button = el("button", "report-btn", "Report");
  button.type = "button";
  button.setAttribute("aria-label", `Report a problem with game ${played(game.session, game.index)}`);
  button.addEventListener("click", () => openReport(game));
  const cell = el("td");
  cell.append(button);
  return cell;
}

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
  document.getElementById("report-dialog").showModal();
  try {
    const text = await invoke("preview_problem_report", target);
    if (reportGame !== target) return;
    document.getElementById("report-text").textContent = text;
    document.getElementById("report-save").disabled = false;
  } catch (err) {
    if (reportGame === target) showReportMessage("report-error", String(err));
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
    showReportMessage("report-saved", `Saved to ${path}. Nothing was sent.`);
  } catch (err) {
    if (reportGame !== target) return;
    showReportMessage("report-error", String(err));
    save.disabled = false;
  }
}

document.getElementById("report-save").addEventListener("click", saveReport);
document.getElementById("report-close").addEventListener("click", () => document.getElementById("report-dialog").close());
document.getElementById("report-dialog").addEventListener("close", () => { reportGame = null; });

// The mode with most games, Solo when there are none.
function pickDefaultMode() {
  const known = stats.modes.filter((m) => MODES[m.mode] && m.totals.games > 0);
  known.sort((a, b) => b.totals.games - a.totals.games);
  return known.length ? known[0].mode : "GT_BATTLEGROUNDS";
}

function render() {
  const games = allGames.filter((g) => modeOf(g) === mode);
  const m = modeStats();
  document.querySelector('.modes [data-mode="OTHER"]').hidden = !allGames.some((g) => modeOf(g) === OTHER);
  document.getElementById("games").replaceChildren(...games.map((g) => row(g, m.top_half)));
  const empty = document.getElementById("empty");
  empty.hidden = games.length > 0;
  empty.textContent = `No ${MODES[mode] || "other"} games yet. Play one and it will appear here when it ends.`;
  document.querySelectorAll(".modes button").forEach((b) => {
    b.setAttribute("aria-pressed", String(b.dataset.mode === mode));
  });
  renderTally(m);
  renderBreakdown(m);
  renderLegend();
}

// Refreshes can overlap; only the latest one may paint, so an older answer
// never replaces a newer one.
let refreshSeq = 0;

async function refreshGames() {
  const seq = ++refreshSeq;
  const [games, fresh] = await Promise.all([invoke("list_games"), invoke("game_stats")]);
  if (seq !== refreshSeq) return;
  allGames = games.slice().reverse(); // newest first
  stats = fresh;
  // Until the user picks a mode, show the one with most games (the history
  // may still be opening on the first call, so this runs on every refresh).
  if (!userPicked) mode = pickDefaultMode();
  render();
}

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
  if (s.problem) node.textContent = s.problem;
  else if (s.session && !s.power_log) node.textContent = `Following ${s.session}: no Power.log yet (it appears with the first game; if it never does, check log.config)`;
  else if (s.session) node.textContent = `Following ${s.session}`;
  else if (s.logs_dir) node.textContent = "Waiting for the game to write a log…";
  else node.textContent = "Starting…";
  if (s.notice) node.textContent += ` — ${s.notice}`;
  node.title = [s.logs_dir && `Logs: ${s.logs_dir}`, s.history && `History: ${s.history}`]
    .filter(Boolean).join("\n");
}

// What to change in log.config / client.config; hidden when the setup is fine.
function renderSetup(s) {
  const lines = Array.isArray(s.setup) ? s.setup : [];
  const list = document.getElementById("setup-list");
  list.replaceChildren(...lines.map((line) => el("li", null, String(line))));
  document.getElementById("setup").hidden = lines.length === 0;
}

async function main() {
  await listen("games-changed", () => refreshGames().catch(showError));
  const showStatus = (s) => { renderStatus(s); renderSetup(s); };
  await listen("status-changed", (e) => showStatus(e.payload));
  showStatus(await invoke("status"));
  await refreshGames();
}

function showError(err) {
  renderStatus({ problem: "Something went wrong: " + String(err) });
}

main().catch(showError);
