// Tavern Ledger window: shows the local history and the tracker's status.
// Values are set with textContent only; nothing from the history is parsed as HTML.
"use strict";

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const MODES = { GT_BATTLEGROUNDS: "Solo", GT_BATTLEGROUNDS_DUO: "Duos" };
// Solo places go 1-8; Duos places are per team, 1-4. Top half differs.
const TOP_HALF = { GT_BATTLEGROUNDS: 4, GT_BATTLEGROUNDS_DUO: 2 };

let allGames = [];
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

// "Hearthstone_2026_10_09_15_26_25" + game 2 -> "2026-10-09 15:26 · 2"
// (the session start time; the log has no per-game date yet)
function played(session, index) {
  const m = /^Hearthstone_(\d{4})_(\d{2})_(\d{2})_(\d{2})_(\d{2})_\d{2}$/.exec(session);
  const when = m ? `${m[1]}-${m[2]}-${m[3]} ${m[4]}:${m[5]}` : session;
  return `${when} · ${index}`;
}

function placeBadge(place, gameType) {
  if (place === null || place === undefined) return el("span", "place unknown", "—");
  const top = TOP_HALF[gameType] || 4;
  const kind = place === 1 ? "win" : place <= top ? "top4" : "bottom4";
  return el("span", `place ${kind}`, place);
}

// The history is a local file; tolerate hand-edited or older records.
const list = (value) => (Array.isArray(value) ? value : []);

function note(report) {
  if (report.status === "incomplete") return "Game not finished in the log";
  if (report.status === "unsupported") return "Could not be read: " + list(report.problems).join("; ");
  return list(report.warnings).join("; ");
}

function row(game) {
  const r = game.report;
  const tr = el("tr");
  tr.append(
    el("td", null, played(game.session, game.index)),
    el("td", null, MODES[r.game_type] || r.game_type || "—"),
    el("td", "hero", r.hero || "—"),
  );
  const placeCell = el("td", "num");
  placeCell.append(placeBadge(r.final_place, r.game_type));
  tr.append(
    placeCell,
    el("td", "num", r.final_health ?? "—"),
    el("td", "num", list(r.rounds).length || "—"),
    el("td", "note", note(r)),
  );
  return tr;
}

// Only finished games with a known place count; never a made-up number.
// Stats are per mode: Solo and Duos places are not comparable.
function renderTally(games) {
  const places = games.map((g) => g.report.final_place).filter((p) => Number.isInteger(p));
  const set = (id, value) => { document.getElementById(id).textContent = value; };
  const top = TOP_HALF[mode] || 4;
  set("stat-top-label", `Top ${top}`);
  set("stat-games", games.length || "—");
  if (places.length === 0) {
    ["stat-avg", "stat-top", "stat-wins"].forEach((id) => set(id, "—"));
    return;
  }
  const avg = places.reduce((a, b) => a + b, 0) / places.length;
  const topShare = places.filter((p) => p <= top).length / places.length;
  set("stat-avg", avg.toFixed(2));
  set("stat-top", `${Math.round(topShare * 100)}%`);
  set("stat-wins", places.filter((p) => p === 1).length);
}

function pickDefaultMode(games) {
  const counts = {};
  games.forEach((g) => { counts[g.report.game_type] = (counts[g.report.game_type] || 0) + 1; });
  const known = Object.keys(MODES).filter((t) => counts[t]);
  known.sort((a, b) => counts[b] - counts[a]);
  return known[0] || "GT_BATTLEGROUNDS";
}

function render() {
  const games = allGames.filter((g) => modeOf(g) === mode);
  document.querySelector('.modes [data-mode="OTHER"]').hidden = !allGames.some((g) => modeOf(g) === OTHER);
  document.getElementById("games").replaceChildren(...games.map(row));
  const empty = document.getElementById("empty");
  empty.hidden = games.length > 0;
  empty.textContent = `No ${MODES[mode] || "other"} games yet. Play one and it will appear here when it ends.`;
  document.querySelectorAll(".modes button").forEach((b) => {
    b.setAttribute("aria-pressed", String(b.dataset.mode === mode));
  });
  renderTally(games);
}

async function refreshGames() {
  const games = await invoke("list_games");
  allGames = games.slice().reverse(); // newest first
  // Until the user picks a mode, show the one with most games (the history
  // may still be opening on the first call, so this runs on every refresh).
  if (!userPicked) mode = pickDefaultMode(allGames);
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
  node.classList.toggle("problem", Boolean(s.problem));
  node.classList.toggle("live", !s.problem && Boolean(s.session) && Boolean(s.power_log));
  if (s.problem) node.textContent = s.problem;
  else if (s.session && !s.power_log) node.textContent = `Following ${s.session}: no Power.log yet (it appears with the first game; if it never does, check log.config)`;
  else if (s.session) node.textContent = `Following ${s.session}`;
  else if (s.logs_dir) node.textContent = "Waiting for the game to write a log…";
  else node.textContent = "Starting…";
  node.title = [s.logs_dir && `Logs: ${s.logs_dir}`, s.history && `History: ${s.history}`]
    .filter(Boolean).join("\n");
}

async function main() {
  await listen("games-changed", () => refreshGames().catch(showError));
  await listen("status-changed", (e) => renderStatus(e.payload));
  renderStatus(await invoke("status"));
  await refreshGames();
}

function showError(err) {
  renderStatus({ problem: "Something went wrong: " + String(err) });
}

main().catch(showError);
