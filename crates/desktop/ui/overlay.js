// The overlay (T-301). The app decides every value and its source
// (tracker::live); this file only shows them. The window is click-through
// while locked, so a source is always a word on the page, never a tooltip.
// Text is set with textContent only: nothing from the log is ever treated as markup.
"use strict";

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const SOURCES = ["log", "inferred", "entered", "leaderboard", "card_data", "possible", "game", "unknown"];

let game = null;
// Where the game client is (T-314, LoadingScreen.log): "battlegrounds_loading"
// shows the overlay with the loading screen, before the log has the game.
let scene = null;
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

// The word for a source, in the language (strings/common.js).
function label(source) {
  return TLI18n.t(`source.${SOURCES.includes(source) ? source : "unknown"}.label`);
}

// The mode as the app names it; any other game type is just Battlegrounds.
function modeName(type) {
  if (type === "GT_BATTLEGROUNDS") return TLI18n.t("mode.GT_BATTLEGROUNDS");
  if (type === "GT_BATTLEGROUNDS_DUO") return TLI18n.t("mode.GT_BATTLEGROUNDS_DUO");
  return "Battlegrounds";
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
  const key = `tribe.${String(id).toUpperCase()}`;
  if (key in TLI18n.tables().en) return TLI18n.t(key);
  const text = String(id).toLowerCase();
  return text.charAt(0).toUpperCase() + text.slice(1);
}

// The card data's name in the app's language, else the log's, else the id.
function heroName(hero) {
  return TLCards.display(hero.id, hero.name);
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
      el("div", "hero", TLI18n.t("ov.status.unreadable.title")),
      el("div", null, TLI18n.t("ov.status.unreadable.body")),
    );
    return;
  }
  if (g.phase === "starting") {
    panel.append(TLI18n.t("ov.status.starting"));
    return;
  }
  const line = el("div");
  const mode = g.game_type.value ? modeName(g.game_type.value) : null;
  const names = [g.hero.value, g.is_duos ? g.teammate_hero.value : null]
    .filter(Boolean)
    .map(heroName)
    .join(" + ");
  const sep = () => el("span", "sep", "·");
  line.append(el("span", "hero", names || TLI18n.t("ov.status.heroUnknown")), sep(), mode || TLI18n.t("ov.status.modeUnknown"), sep());
  if (g.phase === "over") {
    line.append(TLI18n.t("ov.status.over"));
  } else {
    line.append(g.last_combat.value === null ? TLI18n.t("ov.status.noCombat") : TLI18n.t("ov.status.combat", { value: g.last_combat.value }));
  }
  line.append(sep(), sourced(el("span"), g.own_health), ` ${TLI18n.t("ov.hp.unit")}`);
  panel.append(line);
  if (g === EXAMPLE_GAME) panel.append(el("p", "notes", TLI18n.t("ov.status.example")));
  const notes = [...g.warnings, ...g.problems].slice(0, 2).map((w) => TLI18n.backend(w));
  if (notes.length) panel.append(el("p", "notes", notes.join(" · ")));
}

function renderTribes(g) {
  const panel = document.getElementById("tribes");
  panel.replaceChildren(el("h2", null, TLI18n.t("ov.slot.tribes")));
  const seen = el("div", "row");
  seen.append(el("span", "what", TLI18n.t("ov.tribes.seen")));
  const seenMark = mark(g.tribes_source);
  if (seenMark) seen.append(seenMark);
  seen.append(
    el(
      "div",
      "tribe-list",
      g.tribes.length ? g.tribes.map((t) => `${tribeName(t.tribe)} ${t.offers}`).join(" · ") : TLI18n.t("ov.tribes.none"),
    ),
  );
  panel.append(seen);
  // The lobby's tribes the tavern confirmed with the card data (D-050): the
  // log has no list of them.
  const lobby = g.lobby_tribes;
  // Read from the game (D-063), when the user turned that on.
  if (lobby && lobby.basis === "game") {
    const row = el("div", "row");
    row.append(el("span", "what", TLI18n.t("ov.tribes.lobbyGame")));
    const gameMark = mark(lobby.source);
    if (gameMark) row.append(gameMark);
    row.append(el("div", "tribe-list", lobby.tribes.map(tribeName).join(" · ")));
    panel.append(row);
  }
  if (g.game_tribes && g.game_tribes.note) panel.append(el("p", "notes", TLI18n.backend(g.game_tribes.note)));
  if (lobby && lobby.basis === "tavern_confirmed") {
    const row = el("div", "row");
    row.append(el("span", "what", lobby.complete ? TLI18n.t("ov.tribes.lobbyAll") : TLI18n.t("ov.tribes.lobbyPart", { count: lobby.tribes.length })));
    const lobbyMark = mark(lobby.source);
    if (lobbyMark) row.append(lobbyMark);
    row.append(el("div", "tribe-list", lobby.tribes.map(tribeName).join(" · ")));
    panel.append(row);
  }
  if (g.entered_tribes.length) {
    const entered = el("div", "row");
    entered.append(el("span", "what", TLI18n.t("ov.tribes.entered")));
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
    record.append(el("span", "none", TLI18n.t("ov.record.none")));
    return record;
  }
  const r = opponent.record;
  record.append(
    el("span", "w", TLI18n.t("ov.record.won", { value: r.won })),
    " ",
    el("span", "l", TLI18n.t("ov.record.lost", { value: r.lost })),
    ` ${TLI18n.t("ov.record.tie", { value: r.tie })}`,
  );
  if (r.unknown) record.append(" ", unknown(), ` ×${r.unknown}`);
  const recordMark = mark(opponent.record_source);
  if (recordMark) record.append(recordMark);
  return record;
}

function renderBoards(opponent) {
  const wrap = el("div", "board");
  if (!opponent.boards.length) {
    wrap.append(el("span", "none", TLI18n.t("ov.board.none")), mark(opponent.boards_source) || "");
    return [wrap];
  }
  return opponent.boards.map((board) => {
    const row = el("div", "board");
    const who = opponent.boards.length > 1 && board.hero ? `${heroName(board.hero)} · ` : "";
    row.append(el("span", "when", `${who}${TLWords.seenWords(board.round, current().turn.value)}`));
    if (!board.minions.length) row.append(el("span", "none", TLI18n.t("ov.board.empty")));
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
      const marks = keywordMarks(m.keywords);
      if (marks) chip.append(marks);
      row.append(chip);
    }
    if (named) row.append(mark("card_data"));
    if (unnamed) row.append(mark("unknown"));
    return row;
  });
}

// A minion's keywords (T-208): short letters, each read out by its full
// name; the legend spells them out. Nothing when the record did not keep them.
function keywordMarks(keywords) {
  const marks = TLWords.keywordMarks(keywords);
  if (!marks || !marks.length) return null;
  const line = el("span", "kws adv");
  for (const k of marks) {
    const mark = el("span", `kw kw-${k.key}`, k.short);
    mark.setAttribute("aria-hidden", "true");
    line.append(mark);
  }
  line.append(el("span", "visually-hidden", marks.map((k) => k.full).join(", ")));
  return line;
}

// The leaderboard's triples and trinkets of each of the opponent's heroes
// (T-312), as its tooltip shows them now.
function renderShown(opponent) {
  if (opponent.shown_source !== "log" || !Array.isArray(opponent.shown) || !opponent.shown.length) {
    const none = el("div", "seat-line none", TLI18n.t("ov.shown.none"));
    none.append(mark("unknown") || "");
    return [none];
  }
  return opponent.shown.map((seat) => {
    const w = TLWords.seatWords(seat, TLCards.name);
    const i = opponent.seats.indexOf(seat.seat);
    const hero = opponent.seats.length > 1 && opponent.heroes.length === opponent.seats.length ? opponent.heroes[i] : null;
    const line = el("div", "seat-line", `${hero ? `${heroName(hero)} · ` : ""}${w.triples}`);
    // Trinket names are Advanced (T-333); Simple says how many.
    const names = el("span", "adv");
    if (w.trinkets.length) {
      line.append(el("span", "simple-only", ` · ${TLI18n.plural("ov.trinkets", w.trinkets.length)}`));
      names.append(` · ${TLI18n.t("ov.shown.trinkets")} `);
      w.trinkets.forEach((t, n) => {
        if (n) names.append(", ");
        names.append(el("span", t.known ? "trinket" : "trinket unnamed", t.hero_power ? TLI18n.t("ov.shown.heroPower", { text: t.text }) : t.text));
      });
    } else {
      names.append(` · ${TLI18n.t("ov.shown.noTrinket")}`);
    }
    if (w.named) names.append(mark("card_data"));
    if (w.unknown) names.append(mark("unknown"));
    line.append(names);
    return line;
  });
}

// "T3", or "T3+4" for a Duos team; "T?" when the log has not said.
function tiers(opponent) {
  const known = (opponent.tiers || []).map((t) => (t.value === null ? "?" : t.value));
  return TLI18n.t("ov.tier", { value: known.length ? known.join("+") : "?" });
}

// The head line of an opponent: portraits, hero names, tier, health, record.
// Shared by the compact list and the hover card (overlay-hover.js).
function opponentHead(opponent) {
  const head = el("div", "head");
  const hp = el("span", "hp");
  hp.append(sourced(el("b"), opponent.health), ` ${TLI18n.t("ov.hp.unit")}`);
  const portraits = opponent.heroes.map((h) => TLCards.art(h.id, "portrait")).filter(Boolean);
  // The game's own mark on its leaderboard (T-313), in words.
  if (opponent.next) head.append(el("span", "next-badge", TLWords.nextLabels(current()).badge));
  head.append(
    ...portraits,
    el("span", "name", heroes(opponent.heroes) || TLI18n.t("ov.status.heroUnknown")),
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
  panel.replaceChildren(el("h2", null, TLI18n.t("ov.slot.opponents")));
  if (!g.opponents.length) {
    panel.append(el("div", "row", TLI18n.t("ov.opponents.noLobby")));
  }
  // The next opponent the game marks (T-313), with the last board met.
  const next = TLWords.nextOpponent(g.opponents);
  if (next) {
    const block = el("div", "opponent next-block");
    const head = opponentHead(next);
    // The heading already says "next".
    const badge = head.querySelector(".next-badge");
    if (badge) badge.remove();
    block.append(el("h3", null, TLWords.nextLabels(g).heading), head, ...renderShown(next), ...renderBoards(next));
    panel.append(block);
  }
  for (const opponent of g.opponents) {
    const item = el("div", "opponent compact");
    item.append(opponentHead(opponent));
    panel.append(item);
  }
  // Boards are only on the hover card: say when the hover cannot work yet.
  const hint =
    g.leaderboard_slots === null || g.leaderboard_slots === undefined
      ? TLI18n.t("ov.opponents.hintNoOrder")
      : TLI18n.t("ov.opponents.hint");
  panel.append(el("p", "hint", hint));
  panel.hidden = false;
}

function render() {
  const legend = document.getElementById("legend");
  const g = current();
  if (!g) {
    const status = document.getElementById("status");
    status.className = "panel status";
    status.textContent = scene === "battlegrounds_loading" ? TLI18n.t("ov.status.loading") : TLI18n.t("ov.status.waiting");
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
  const [fresh, where] = await Promise.all([invoke("overlay_state"), invoke("overlay_scene")]);
  if (mine !== latest) return;
  game = fresh;
  scene = typeof where === "string" ? where : null;
  if (window.TLHover) window.TLHover.setFailed(false);
  render();
}

// Text follows the screen (T-334): the base size is a share of the window's
// height, which covers the game's monitor (overlay-scale.js).
function fitText() {
  document.documentElement.style.fontSize = `${TLScale.rootPx(window.innerHeight)}px`;
}

async function start() {
  fitText();
  window.addEventListener("resize", fitText);
  // Nothing is drawn before the language is known and the markup is in it.
  await TLI18n.ready();
  // The key to the keyword marks, from the same words the marks use.
  document.getElementById("legend").append(el("span", "legend-keys", ` ${TLI18n.t("ov.legend.keys", { list: TLWords.keywordLegend() })}`));
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
  panel.textContent = TLI18n.t("ov.error.update", { error });
  for (const id of ["tribes", "opponents", "legend"]) document.getElementById(id).hidden = true;
  if (window.TLHover) window.TLHover.setFailed(true);
}

// A script error must show on the page too: nobody can open a console on this window.
window.addEventListener("error", (e) => showError(e.message));
window.addEventListener("unhandledrejection", (e) => showError(e.reason));

start().catch(showError);
