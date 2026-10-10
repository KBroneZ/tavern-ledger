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
const SOURCES = ["log", "inferred", "entered", "leaderboard", "card_data", "unknown"];
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

// The hero's portrait, looked up by the log's card id in the card data
// (T-304); none when there is no id or no image (cards.js removes it).
function portrait(id) {
  const img = TLCards.art(id, "portrait");
  if (img) img.title = `Picture: ${sourceInfo("card_data").label}. © Blizzard Entertainment.`;
  return img;
}

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
  if (report.status === "incomplete") return "Game not finished in the log";
  if (report.status === "unsupported") return "Could not be read: " + list(report.problems).join("; ");
  return list(report.warnings).join("; ");
}

// Played with the reconnect dev tool (D-043): left out of the stats and the upload.
const DEV_RECONNECT_TIP = "Played with the reconnect dev tool: not counted in the stats and never uploaded.";

function noteCell(game) {
  const cell = el("td", "note", note(game.report));
  if (game.dev_reconnect === true) {
    const mark = el("span", "dev-mark", "Dev reconnect");
    mark.title = DEV_RECONNECT_TIP;
    cell.prepend(mark, " ");
  }
  return cell;
}

function row(game, top) {
  const r = game.report;
  const tr = el("tr", game.dev_reconnect === true ? "dev-reconnect" : null);
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
    noteCell(game),
    reportCell(game),
  );
  return tr;
}

function topLabel(top) {
  return top === null ? "Top half" : `Top ${top}`;
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
    t.incomplete && `${t.incomplete} not finished in the log`,
    t.unsupported && `${t.unsupported} could not be read`,
  ].filter(Boolean);
  const why = m.top_half === null ? "place rules unknown for this mode" : parts.join(", ");
  note.textContent = notCounted === 0 ? "" : `${notCounted} of ${t.games} games not counted for places` + (why ? ` (${why})` : "") + ".";
  if (devGames > 0) note.textContent += `${notCounted === 0 ? "" : " "}${devGames} played with the reconnect dev tool left out.`;
}

function heroRow(h, top) {
  const tr = el("tr");
  const cell = el("td", h.name ? "hero-name" : "hero", h.name || h.hero || "Unknown hero");
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

// The log's race names: NEUTRAL is a minion with no tribe, ALL one with every tribe.
const TRIBE_LABELS = { NEUTRAL: "No tribe", ALL: "All tribes" };
const tribeLabel = (name) => TRIBE_LABELS[name] || name.charAt(0) + name.slice(1).toLowerCase();

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

function renderLobby() {
  const state = document.getElementById("lobby-state");
  const current = document.getElementById("lobby-current");
  const enter = document.getElementById("lobby-enter");
  const clear = document.getElementById("lobby-clear");
  enter.hidden = clear.hidden = true;
  current.replaceChildren();
  if (!lobby) {
    state.textContent = "Loading…";
    return;
  }
  if (!lobby.available) {
    state.textContent = lobby.problem || "Entering tribes is not available: this window does not own the history.";
    return;
  }
  const target = lobby.in_progress
    ? { kind: "game", session: lobby.in_progress.session, index: lobby.in_progress.index }
    : { kind: "pending" };
  const entered = entryOf(target);
  if (lobby.in_progress) {
    state.textContent = entered
      ? "A game is in progress. Tribes entered by you for it:"
      : "A game is in progress. Pick the five tribes you see on your screen at hero select.";
  } else {
    state.textContent = entered
      ? "No game in progress. These tribes wait for the next game:"
      : "No game in progress. You can pick the tribes now and they will go to the next game.";
  }
  current.replaceChildren(...(entered ? tribeChips(entered) : []));
  enter.textContent = entered ? "Change" : lobby.in_progress ? "Enter tribes" : "Enter tribes for the next game";
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
    error.textContent = String(err);
    error.hidden = false;
    return false;
  }
}

function tribesTargetText(target) {
  if (target.kind === "pending") return "For the next game that starts.";
  const game = allGames.find((g) => g.session === target.session && g.index === target.index);
  return game ? `For game ${played(game.session, game.index)}.` : "For the game in progress.";
}

function pickedTribes() {
  return [...document.querySelectorAll("#tribes-choices input:checked")].map((box) => box.value);
}

function updateTribesCount() {
  const need = lobby ? lobby.lobby_size : 5;
  const picked = pickedTribes().length;
  document.getElementById("tribes-count").textContent = `${picked} of ${need} picked.`;
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
    options.unshift({ session: live.session, index: live.index, text: "Game in progress" });
  }
  return options;
}

function renderTribesDialog() {
  const target = tribesTarget;
  if (!target || !lobby) return;
  const entered = entryOf(target);
  document.getElementById("tribes-target").textContent = tribesTargetText(target);
  document.getElementById("tribes-need").textContent = lobby.lobby_size;
  const keep = new Set(document.getElementById("tribes-choices").childElementCount ? pickedTribes() : entered || []);
  document.getElementById("tribes-choices").replaceChildren(...lobby.choices.map((choice) => {
    const label = el("label", "choice");
    const box = el("input");
    box.type = "checkbox";
    box.value = choice.id;
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
  const dialog = document.getElementById("tribes-dialog");
  if (!dialog.open) dialog.showModal();
}

function tribesButton(game) {
  const entered = entryOf({ kind: "game", session: game.session, index: game.index });
  const button = el("button", "report-btn", entered ? "Tribes ✓" : "Tribes");
  button.type = "button";
  button.title = entered ? `Lobby tribes entered by you: ${entered.map(tribeLabel).join(", ")}` : "Enter the lobby tribes for this game";
  button.setAttribute("aria-label", `Lobby tribes of game ${played(game.session, game.index)}`);
  button.addEventListener("click", () => openTribesDialog({ kind: "game", session: game.session, index: game.index }));
  return button;
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

function enteredRow(t, total, source) {
  const tr = el("tr");
  const cell = el("td", null, tribeLabel(t.tribe));
  cell.title = t.tribe;
  const count = el("td", "num", `${t.games} of ${total}`);
  addTip(count, source);
  tr.append(withSource(cell, source), count);
  return tr;
}

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
  const entered = list(m.entered_tribes);
  document.getElementById("entered-tribes-section").hidden = entered.length === 0;
  document.getElementById("entered-tribes").replaceChildren(
    ...entered.map((t) => enteredRow(t, m.games_with_entered_tribes, m.entered_tribes_source)),
  );
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
  const cell = el("td", "actions");
  cell.append(recapButton(game), tribesButton(game), button);
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

// Recap of one game (T-202, T-203). The app works everything out (tracker::recap),
// including every source label; this file only draws it. Opponents are heroes
// and seats of this one game, never names.
let recapGame = null;
const OUTCOME_WORDS = { won: "Won", lost: "Lost", tie: "Tie", unknown: "Unknown" };
const SVG_NS = "http://www.w3.org/2000/svg";

function recapButton(game) {
  const button = el("button", "report-btn", "Recap");
  button.type = "button";
  button.setAttribute("aria-label", `Recap of game ${played(game.session, game.index)}`);
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
  if (status === "ok") return "Finished";
  if (status === "incomplete") return "Not finished in the log";
  if (status === "unsupported") return "Could not be read";
  return "—";
}

function recapFacts(r) {
  const facts = el("dl", "facts");
  const mode = MODES[r.game_type.value] || r.game_type.value || "—";
  facts.append(fact("Mode", mode, r.game_type.source));
  facts.append(fact("Hero", heroWithPortrait(r.hero.value), r.hero.source));
  if (r.is_duos) facts.append(fact("Teammate", heroWithPortrait(r.teammate_hero.value), r.teammate_hero.source));
  facts.append(fact("Place", placeBadge(r.place.value, r.top_half), r.place.source));
  facts.append(fact("Final health", r.final_health.value ?? "—", r.final_health.source));
  facts.append(fact("Rounds played", r.rounds_played.value ?? "—", r.rounds_played.source));
  facts.append(fact("State", statusText(r.status.value), r.status.source));
  return facts;
}

function svg(tag, attrs) {
  const node = document.createElementNS(SVG_NS, tag);
  Object.entries(attrs).forEach(([k, v]) => node.setAttribute(k, String(v)));
  return node;
}

const pointWords = (p) => (p.round === 0 ? "before the first combat" : `after round ${p.round}`);

// Health after each round as a line; a round with no number breaks the line.
function healthChart(points) {
  const wrap = el("div", "chart");
  const known = points.filter((p) => Number.isInteger(p.health));
  if (!known.length) {
    wrap.append(el("p", "empty", "The log has no health for this game."));
    return wrap;
  }
  const width = 480, height = 110, pad = 14;
  const top = Math.max(...known.map((p) => p.health), 1);
  const last = Math.max(points.length - 1, 1);
  const x = (i) => pad + (i * (width - 2 * pad)) / last;
  const y = (h) => height - pad - (h * (height - 2 * pad)) / top;
  const chart = svg("svg", { viewBox: `0 0 ${width} ${height}`, role: "img", class: "health-chart" });
  chart.setAttribute("aria-label", "Health " + points.map((p) => `${pointWords(p)}: ${p.health ?? "unknown"}`).join(", "));
  chart.append(svg("line", { x1: pad, y1: y(0), x2: width - pad, y2: y(0), class: "axis" }));
  let run = [];
  const flush = () => {
    if (run.length > 1) chart.append(svg("polyline", { points: run.join(" "), class: "line" }));
    run = [];
  };
  points.forEach((p, i) => {
    if (!Number.isInteger(p.health)) return flush();
    run.push(`${x(i)},${y(p.health)}`);
    const dot = svg("circle", { cx: x(i), cy: y(p.health), r: 3, class: "dot" });
    const tip = svg("title", {});
    tip.textContent = `Health ${pointWords(p)}: ${p.health}`;
    dot.append(tip);
    chart.append(dot);
  });
  flush();
  wrap.append(chart);
  return wrap;
}

function combatChip(c) {
  const word = OUTCOME_WORDS[c.outcome] || "Unknown";
  const chip = el("span", `chip chip-${c.outcome} src-${sourceOf(c.source)}`, `R${c.round} ${word}`);
  chip.title = c.reason ? `Round ${c.round}: unknown, ${c.reason}.` : `Round ${c.round}: ${word}.`;
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
  const who = el("td", "opponent", o.heroes.map(heroText).join(" + ") || "Unknown hero");
  who.prepend(...o.heroes.map((h) => portrait(h.id)).filter(Boolean));
  who.title = o.heroes.map((h) => h.id).concat(o.seats.map((s) => `seat ${s}`)).join("\n");
  if (o.final_place.value !== null) {
    const place = el("span", "finished", `finished ${o.final_place.value}`);
    who.append(" ", withSource(place, o.final_place.source));
  }
  const chips = el("td", "chips");
  chips.append(...o.combats.map(combatChip));
  // Won, lost and tie are our reading of health changes: inferred. An unknown count is not a result.
  tr.append(who, countCell(o.won, "inferred"), countCell(o.lost, "inferred"), countCell(o.tie, "inferred"), countCell(o.unknown, "unknown"), chips);
  return tr;
}

function recordTable(r) {
  const head = el("tr");
  head.append(el("th", null, r.is_duos ? "Opposing team" : "Opponent"));
  ["Won", "Lost", "Tie"].forEach((word) => {
    const th = el("th", "num", word);
    th.append(sourceMark("inferred"));
    head.append(th);
  });
  head.append(el("th", "num", "Unknown"), el("th", null, "Rounds"));
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
  const heading = el("h3", null, "Record against each opponent");
  heading.append(sourceMark("inferred"));
  section.append(heading, el("p", "note", r.record_basis));
  section.append(r.record.length ? recordTable(r) : el("p", "empty", "No combat could be matched to an opponent."));
  if (r.unattributed.length) {
    const reasons = [...new Set(r.unattributed.map((c) => c.reason).filter(Boolean))];
    const rounds = r.unattributed.map((c) => c.round).join(", ");
    section.append(withSource(el("p", "note", `Rounds ${rounds}: opponent unknown (${reasons.join("; ")}).`), "unknown"));
  }
  if (r.missing_rounds.length) {
    section.append(withSource(el("p", "note", `Rounds the log does not have: ${r.missing_rounds.join(", ")}.`), "unknown"));
  }
  return section;
}

function tribesSection(r) {
  const section = el("section", "recap-part");
  const heading = el("h3", null, "Tribes seen in the tavern");
  heading.append(sourceMark(r.tribes_source));
  section.append(heading);
  if (!r.tribes.length) {
    section.append(el("p", "empty", "No tavern offers in the log."));
    return section;
  }
  section.append(el("p", "note", "What the tavern offered you. The log does not say which tribes were in the lobby."));
  const list = el("ul", "tribe-list");
  list.append(...r.tribes.map((t) => {
    const li = el("li", null, `${tribeLabel(t.tribe)} ${t.offers}`);
    li.title = t.tribe;
    return withSource(li, r.tribes_source);
  }));
  section.append(list);
  return section;
}

// What the user entered for this game. Shown next to the tavern list, never
// merged with it and never used to correct it, whether the two agree or not.
function enteredSection(r) {
  const section = el("section", "recap-part");
  const heading = el("h3", null, "Lobby tribes");
  heading.append(sourceMark(r.entered_tribes_source));
  section.append(heading);
  if (!r.entered_tribes.length) {
    section.append(el("p", "empty", "You did not enter the lobby tribes for this game."));
    return section;
  }
  const items = el("ul", "tribe-list");
  items.append(...tribeChips(r.entered_tribes));
  section.append(items);
  if (r.tribes.length) {
    section.append(el("p", "note", "The list above and the tavern offers are two separate sources. Tavern Ledger does not correct one with the other."));
  }
  return section;
}

function messagesSection(r) {
  const section = el("section", "recap-part");
  section.append(el("h3", null, "Parser warnings"));
  const all = r.problems.map((m) => `Problem: ${m}`).concat(r.warnings);
  if (!all.length) {
    section.append(el("p", "note", "The parser reported no warnings for this game."));
    return section;
  }
  const list = el("ul", "messages");
  list.append(...all.map((m) => el("li", null, m)));
  section.append(list);
  return section;
}

function renderRecap(r) {
  const health = el("section", "recap-part");
  health.append(withSource(el("h3", null, "Your health over the rounds"), r.health_source), healthChart(r.health));
  document.getElementById("recap-body").replaceChildren(recapFacts(r), health, recordSection(r), enteredSection(r), tribesSection(r), messagesSection(r));
}

async function openRecap(game) {
  const target = { session: game.session, index: game.index };
  recapGame = target;
  document.getElementById("recap-title").textContent = `Game recap · ${played(game.session, game.index)}`;
  document.getElementById("recap-body").replaceChildren();
  showReportMessage("recap-error", "");
  const dialog = document.getElementById("recap-dialog");
  if (!dialog.open) dialog.showModal();
  try {
    const recap = await invoke("game_recap", target);
    if (recapGame === target) renderRecap(recap);
  } catch (err) {
    if (recapGame === target) showReportMessage("recap-error", String(err));
  }
}

document.getElementById("recap-close").addEventListener("click", () => document.getElementById("recap-dialog").close());
document.getElementById("recap-dialog").addEventListener("close", () => { recapGame = null; });

// A game that just ended: show its recap, unless another dialog is open.
async function onGameFinished(finished) {
  await refreshGames();
  if (document.querySelector("dialog[open]")) return;
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
  renderTally(m, games.filter((g) => g.dev_reconnect === true).length);
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
  if (s.dev_reconnects) node.textContent += ` — ${s.dev_reconnects}`;
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

// Where card pictures come from, and whether they are there (T-304).
function renderCardStatus() {
  const node = document.getElementById("card-status");
  const s = TLCards.status();
  node.classList.toggle("problem", Boolean(s && s.problem));
  if (!s) node.textContent = "Card pictures: status not available.";
  else if (s.problem) node.textContent = `Card pictures: ${s.problem}` + (s.cards ? " The names already saved are still used." : " Card ids are shown instead.");
  else if (s.cards) node.textContent = `Card pictures: ${s.cards} cards known, checked ${new Date(s.fetched_at * 1000).toLocaleDateString()}.`;
  else node.textContent = "Card pictures: getting the card data…";
}

async function main() {
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
  renderStatus({ problem: "Something went wrong: " + String(err) });
}

main().catch(showError);
