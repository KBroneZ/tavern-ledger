// The overlay (T-301). The app decides every value and its source
// (tracker::live); this file only shows them. The window is click-through
// while locked, so a source is always a word on the page, never a tooltip.
// Text is set with textContent only: nothing from the log is ever treated as markup.
"use strict";

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const MODES = { GT_BATTLEGROUNDS: "Solo", GT_BATTLEGROUNDS_DUO: "Duos" };
const SOURCES = ["log", "inferred", "entered", "leaderboard", "card_data", "possible", "unknown"];

let game = null;
// While the layout is being edited and no game is on, the panels show the
// example game so there is something to arrange (overlay-example.js).
let editing = false;

function current() {
  return game || (editing ? EXAMPLE_GAME : null);
}

function setEditing(on) {
  editing = on;
  render();
}

function el(tag, className, text) {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (text !== undefined && text !== null) node.textContent = String(text);
  return node;
}

function label(source) {
  const key = SOURCES.includes(source) ? source : "unknown";
  const shown = current();
  const entry = shown && shown.legend.find((e) => e.source === key);
  return entry ? entry.label : key;
}

// The mark for a source; none for the log, whose values are the plain ones.
function mark(source) {
  const key = SOURCES.includes(source) ? source : "unknown";
  return key === "log" ? null : el("span", `src src-${key}`, label(key));
}

function unknown() {
  return el("span", "unknown-value", "?");
}

function tribeName(id) {
  const text = String(id).toLowerCase();
  return text.charAt(0).toUpperCase() + text.slice(1);
}

function heroName(hero) {
  return hero.name || hero.id;
}

function heroes(list) {
  return list.length ? list.map(heroName).join(" + ") : null;
}

function sourced(node, sourced) {
  node.append(sourced.value === null ? unknown() : String(sourced.value));
  return node;
}

function renderStatus(g) {
  const panel = document.getElementById("status");
  panel.className = "panel status";
  panel.replaceChildren();
  if (g.phase === "unreadable") {
    panel.classList.add("unreadable");
    panel.append(
      el("div", "hero", "Cannot read this game"),
      el("div", null, "The game's log is not in a form this version understands, so nothing is shown rather than a guess."),
    );
    return;
  }
  if (g.phase === "starting") {
    panel.append("Game starting… waiting for the lobby.");
    return;
  }
  const line = el("div");
  const mode = g.game_type.value ? MODES[g.game_type.value] || "Battlegrounds" : null;
  const names = [g.hero.value, g.is_duos ? g.teammate_hero.value : null]
    .filter(Boolean)
    .map(heroName)
    .join(" + ");
  const sep = () => el("span", "sep", "·");
  line.append(el("span", "hero", names || "Hero unknown"), sep(), mode || "Mode unknown", sep());
  if (g.phase === "over") {
    line.append("Game over");
  } else {
    line.append(g.last_combat.value === null ? "no combat yet" : `combat ${g.last_combat.value}`);
  }
  line.append(sep(), sourced(el("span"), g.own_health), " hp");
  panel.append(line);
  if (g === EXAMPLE_GAME) panel.append(el("p", "notes", "Example data: no game is on."));
  const notes = [...g.warnings, ...g.problems].slice(0, 2);
  if (notes.length) panel.append(el("p", "notes", notes.join(" · ")));
}

function renderTribes(g) {
  const panel = document.getElementById("tribes");
  panel.replaceChildren(el("h2", null, "Tribes"));
  const seen = el("div", "row");
  seen.append(el("span", "what", "Seen in the tavern"));
  const seenMark = mark(g.tribes_source);
  if (seenMark) seen.append(seenMark);
  seen.append(
    el(
      "div",
      "tribe-list",
      g.tribes.length ? g.tribes.map((t) => `${tribeName(t.tribe)} ${t.offers}`).join(" · ") : "none yet",
    ),
  );
  panel.append(seen);
  // The lobby's tribes the tavern confirmed with the card data (D-050): the
  // log has no list of them.
  const lobby = g.lobby_tribes;
  if (lobby && lobby.basis === "tavern_confirmed") {
    const row = el("div", "row");
    row.append(el("span", "what", lobby.complete ? "Lobby (all 5 confirmed)" : `Lobby (${lobby.tribes.length} of 5 confirmed)`));
    const lobbyMark = mark(lobby.source);
    if (lobbyMark) row.append(lobbyMark);
    row.append(el("div", "tribe-list", lobby.tribes.map(tribeName).join(" · ")));
    panel.append(row);
  }
  if (g.entered_tribes.length) {
    const entered = el("div", "row");
    entered.append(el("span", "what", "You entered"));
    const enteredMark = mark(g.entered_tribes_source);
    if (enteredMark) entered.append(enteredMark);
    entered.append(el("div", "tribe-list", g.entered_tribes.map(tribeName).join(" · ")));
    panel.append(entered);
  }
  panel.hidden = false;
}

function renderRecord(opponent) {
  const record = el("span", "record");
  if (!opponent.record) {
    record.append(el("span", "none", "not fought"));
    return record;
  }
  const r = opponent.record;
  record.append(el("span", "w", `${r.won}W`), " ", el("span", "l", `${r.lost}L`), ` ${r.tie}T`);
  if (r.unknown) record.append(" ", unknown(), ` ×${r.unknown}`);
  const recordMark = mark(opponent.record_source);
  if (recordMark) record.append(recordMark);
  return record;
}

function renderBoards(opponent) {
  const wrap = el("div", "board");
  if (!opponent.boards.length) {
    wrap.append(el("span", "none", "no board seen yet"), mark(opponent.boards_source) || "");
    return [wrap];
  }
  return opponent.boards.map((board) => {
    const row = el("div", "board");
    const who = opponent.boards.length > 1 && board.hero ? `${heroName(board.hero)} · ` : "";
    row.append(el("span", "when", `${who}seen in round ${board.round}`));
    if (!board.minions.length) row.append(el("span", "none", "empty"));
    // Names and art are looked up by the log's card id (T-304): the row says
    // so in words, and a card the card data does not know shows its id, marked unknown.
    let named = false;
    let unnamed = false;
    for (const m of board.minions) {
      const chip = el("span", m.golden ? "minion golden" : "minion");
      const stat = (n) => (n === null || n === undefined ? "?" : n);
      const name = TLCards.name(m.card_id);
      if (name) named = true;
      if (name === null) unnamed = true;
      const art = TLCards.art(m.card_id, "art");
      if (art) chip.append(art);
      const label = el("i", name === null ? "unnamed" : null, name || m.card_id || "?");
      if (name && m.card_id) label.title = m.card_id;
      chip.append(el("b", null, `${stat(m.atk)}/${stat(m.health)}`), label);
      row.append(chip);
    }
    if (named) row.append(mark("card_data"));
    if (unnamed) row.append(mark("unknown"));
    return row;
  });
}

// "T3", or "T3+4" for a Duos team; "T?" when the log has not said.
function tiers(opponent) {
  const known = (opponent.tiers || []).map((t) => (t.value === null ? "?" : t.value));
  return `T${known.length ? known.join("+") : "?"}`;
}

// The head line of an opponent: portraits, hero names, tier, health, record.
// Shared by the compact list and the hover card (overlay-hover.js).
function opponentHead(opponent) {
  const head = el("div", "head");
  const hp = el("span", "hp");
  hp.append(sourced(el("b"), opponent.health), " hp");
  const portraits = opponent.heroes.map((h) => TLCards.art(h.id, "portrait")).filter(Boolean);
  head.append(
    ...portraits,
    el("span", "name", heroes(opponent.heroes) || "Hero unknown"),
    el("span", "tier", tiers(opponent)),
    hp,
    renderRecord(opponent),
  );
  return head;
}

// Compact (T-306): one line per opponent, in the leaderboard's order. The
// boards are on the hover card, over the game's own leaderboard.
function renderOpponents(g) {
  const panel = document.getElementById("opponents");
  panel.replaceChildren(el("h2", null, "Opponents"));
  if (!g.opponents.length) {
    panel.append(el("div", "row", "The log does not show the lobby yet."));
  }
  for (const opponent of g.opponents) {
    const item = el("div", "opponent compact");
    item.append(opponentHead(opponent));
    panel.append(item);
  }
  // Boards are only on the hover card: say when the hover cannot work yet.
  const hint =
    g.leaderboard_slots === null || g.leaderboard_slots === undefined
      ? "The leaderboard's order is not in the log yet, so hovering it shows nothing for now."
      : "Hover a hero on the game's leaderboard to see the last board you met.";
  panel.append(el("p", "hint", hint));
  panel.hidden = false;
}

function render() {
  const legend = document.getElementById("legend");
  const g = current();
  if (!g) {
    const status = document.getElementById("status");
    status.className = "panel status";
    status.textContent = "Waiting for a game…";
    document.getElementById("tribes").hidden = true;
    document.getElementById("opponents").hidden = true;
    legend.hidden = true;
    if (window.TLHover) window.TLHover.render();
    return;
  }
  renderStatus(g);
  const showPanels = g.phase === "playing" || g.phase === "over";
  document.getElementById("tribes").hidden = !showPanels;
  document.getElementById("opponents").hidden = !showPanels;
  legend.hidden = !showPanels;
  if (showPanels) {
    renderTribes(g);
    renderOpponents(g);
  }
  if (window.TLHover) window.TLHover.render();
}

// Answers can come back out of order: only the newest request is shown.
let latest = 0;

async function refresh() {
  const mine = ++latest;
  const fresh = await invoke("overlay_state");
  if (mine !== latest) return;
  game = fresh;
  if (window.TLHover) window.TLHover.setFailed(false);
  render();
}

async function start() {
  TLCards.onChange(render);
  // Listen first, so a change between the first read and the listener is not lost.
  await listen("live-changed", () => refresh().catch(showError));
  await listen("lobby-tribes-changed", () => refresh().catch(showError));
  // New card data can change the possible minions (T-307).
  await listen("cards-changed", () => refresh().catch(showError));
  await refresh();
}

// The window cannot be clicked: say it on the page, never throw silently, and
// never leave old panels under the message as if they were current.
function showError(error) {
  const panel = document.getElementById("status");
  panel.className = "panel status unreadable";
  panel.textContent = `The overlay could not update (${error}).`;
  for (const id of ["tribes", "opponents", "legend"]) document.getElementById(id).hidden = true;
  if (window.TLHover) window.TLHover.setFailed(true);
}

// A script error must show on the page too: nobody can open a console on this window.
window.addEventListener("error", (e) => showError(e.message));
window.addEventListener("unhandledrejection", (e) => showError(e.reason));

start().catch(showError);
