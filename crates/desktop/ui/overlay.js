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
// The lobby panel's values (T-338, overlay_lobby): shown on the
// Battlegrounds lobby screen with no game on.
let lobbyView = null;
const LOBBY_REFRESH_MS = 5000;

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
  // Advanced: heroes, mode, last combat and health in words. Simple shows
  // nothing of it during a game (T-339): the game shows all of it already.
  const line = el("div", "adv");
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
  if (g.phase === "over") panel.append(el("div", "simple-only", TLI18n.t("ov.status.over")));
  if (g === EXAMPLE_GAME) panel.append(el("p", "notes", TLI18n.t("ov.status.example")));
  const notes = [...g.warnings, ...g.problems].slice(0, 2).map((w) => TLI18n.backend(w));
  if (notes.length) panel.append(el("p", "notes", notes.join(" · ")));
  // Warnings and problems always show: a failure is never silent.
  panel.classList.toggle("simple-empty", g.phase !== "over" && g !== EXAMPLE_GAME && !notes.length);
}

// The Simple tribe row (T-339): the lobby's tribes as short chips, one
// "inferred" mark when the tavern confirmed them, "2/5" when not all are known.
function simpleTribeRow(g) {
  const known = TLWords.simpleTribes(g);
  if (!known) return null;
  const row = el("div", "tribe-chips simple-only");
  row.setAttribute("aria-label", TLI18n.t("ov.slot.tribes"));
  for (const t of known.tribes) {
    // The short name is seen, the full one read out.
    const chip = el("span", "chip");
    const short = el("span", null, tribeShort(t));
    short.setAttribute("aria-hidden", "true");
    chip.append(short, el("span", "visually-hidden", tribeName(t)));
    row.append(chip);
  }
  if (known.count !== null) row.append(el("span", "chip-count", TLI18n.t("ov.tribes.count", { count: known.count })));
  if (known.inferred) row.append(mark("inferred"));
  return row;
}

// A short tribe name that fits a chip; the full name when there is none.
function tribeShort(id) {
  const key = `ov.tribeShort.${String(id).toUpperCase()}`;
  return key in TLI18n.tables().en ? TLI18n.t(key) : tribeName(id);
}

function renderTribes(g) {
  const panel = document.getElementById("tribes");
  const simple = simpleTribeRow(g);
  const full = el("div", "adv");
  panel.replaceChildren(...[simple, full].filter(Boolean));
  panel.classList.toggle("simple-empty", !simple);
  full.append(el("h2", null, TLI18n.t("ov.slot.tribes")));
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
  full.append(seen);
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
    full.append(row);
  }
  if (g.game_tribes && g.game_tribes.note) full.append(el("p", "notes", TLI18n.backend(g.game_tribes.note)));
  if (lobby && lobby.basis === "tavern_confirmed") {
    const row = el("div", "row");
    row.append(el("span", "what", lobby.complete ? TLI18n.t("ov.tribes.lobbyAll") : TLI18n.t("ov.tribes.lobbyPart", { count: lobby.tribes.length })));
    const lobbyMark = mark(lobby.source);
    if (lobbyMark) row.append(lobbyMark);
    row.append(el("div", "tribe-list", lobby.tribes.map(tribeName).join(" · ")));
    full.append(row);
  }
  if (g.entered_tribes.length) {
    const entered = el("div", "row");
    entered.append(el("span", "what", TLI18n.t("ov.tribes.entered")));
    const enteredMark = mark(g.entered_tribes_source);
    if (enteredMark) entered.append(enteredMark);
    entered.append(el("div", "tribe-list", g.entered_tribes.map(tribeName).join(" · ")));
    full.append(entered);
  }
  panel.hidden = false;
}

// The counters panel (T-612): the player's own running totals, one line each
// (Advanced adds where the game shows it). Hidden when there is nothing to
// show: null (unknown) and [] (nothing applies) both leave no empty box.
function renderCounters(g) {
  const panel = document.getElementById("counters");
  // While the layout is being edited the panel shows example rows, so it can be placed in a real game too.
  let rows = TLCounters.rows(g.counters);
  if (!rows.length && editing) rows = TLCounters.rows(EXAMPLE_GAME.counters);
  panel.hidden = !rows.length;
  if (!rows.length) {
    panel.replaceChildren();
    return;
  }
  const list = el("ul", "counter-list");
  for (const r of rows) {
    const item = el("li", "counter");
    // The figures are seen; the sentence is read out instead of the pieces.
    const seen = el("span", "counter-seen");
    seen.setAttribute("aria-hidden", "true");
    const main = el("span", "counter-main");
    main.append(el("span", "counter-label", r.label), el("b", "counter-value", r.value));
    seen.append(main, el("span", "counter-where adv", r.where));
    item.append(seen, el("span", "visually-hidden", r.said));
    list.append(item);
  }
  panel.replaceChildren(list);
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
      const gifts = giftMarks(m);
      if (gifts) chip.append(gifts);
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

// A minion's Dark Gifts (T-528, D-107), in Simple and Advanced: a small mark
// per gift, its name and summary as the tooltip and read out in words.
// Nothing when it has none.
function giftMarks(m) {
  const gifts = TLCards.gifts(m);
  if (!gifts.length) return null;
  const line = el("span", "gifts");
  for (const g of gifts) {
    const mark = el("span", g.id ? "gift-mark" : "gift-mark is-unknown", g.id ? "✦" : "✦?");
    mark.title = g.title;
    mark.setAttribute("aria-hidden", "true");
    line.append(mark);
  }
  line.append(el("span", "visually-hidden", gifts.map((g) => g.title).join("; ")));
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
// One opponent in the Simple panel (T-339): the heroes' portraits with
// their tier, the health and the triples when above 0; the game's next mark
// as a lit row. No names (they are on the hover card) and no unknowns.
function simpleOpponent(opponent) {
  const r = TLWords.simpleRow(opponent);
  const row = el("li", `opp${r.next ? " next" : ""}${r.out ? " out" : ""}`);
  const said = [];
  const faces = el("span", "faces");
  for (const f of r.faces) {
    const face = el("span", "face");
    const art = f.hero ? TLCards.art(f.hero.id, "portrait") : null;
    face.append(art || el("span", "portrait blank"));
    if (f.tier !== null) {
      const tier = el("b", "face-tier", f.tier);
      tier.setAttribute("aria-hidden", "true");
      face.append(tier);
    }
    faces.append(face);
    said.push(`${f.hero ? heroName(f.hero) : TLI18n.t("ov.status.heroUnknown")}${f.tier !== null ? `, ${TLI18n.t("ov.board.tier", { value: f.tier })}` : ""}`);
  }
  row.append(faces);
  const nums = el("span", "nums");
  if (r.next) {
    const word = TLWords.nextLabels(current()).badge;
    nums.append(el("span", "opp-next", word));
  }
  if (r.health !== null) {
    const hp = el("b", "opp-hp", r.health);
    hp.setAttribute("aria-hidden", "true");
    nums.append(hp);
    said.push(`${r.health} ${TLI18n.t("ov.hp.unit")}`);
  }
  if (r.triples > 0) {
    const tr = el("span", "opp-triples", `★${r.triples}`);
    tr.setAttribute("aria-hidden", "true");
    nums.append(tr);
    said.push(TLI18n.plural("ov.triples", r.triples));
  }
  row.append(nums, el("span", "visually-hidden", said.join("; ")));
  return row;
}

// The combat odds against the next opponent (T-605), or why there are none.
// Simple: the three shares and the board's age; Advanced adds the damage and
// the cards played with a gap. Null when there is nothing to say.
function renderOdds(g, advanced) {
  const v = TLOdds.oddsView(g.odds, advanced, (id) => TLCards.name(id) || null);
  if (!v) return null;
  const box = el("div", `odds ${v.kind}`);
  box.title = v.about;
  if (v.kind === "note") {
    box.append(el("p", "odds-note", v.text));
  } else {
    const grid = el("div", "odds-shares");
    for (const s of v.shares) {
      const cell = el("span", `odds-share ${s.key}`);
      cell.append(el("span", "odds-label", s.label), el("b", "odds-value", s.value));
      grid.append(cell);
    }
    grid.setAttribute("aria-hidden", "true");
    box.append(grid);
    // A game build the rules were not checked against warns here (D-145).
    if (v.warning) box.append(el("p", "odds-warning", v.warning));
    // "≈" when a card is played with a gap; Advanced names them.
    if (v.age) box.append(el("p", "odds-age", v.approx ? `≈ ${v.age}` : v.age));
  }
  for (const text of v.lines) box.append(el("p", "odds-line", text));
  box.append(el("span", "visually-hidden", v.said));
  return box;
}

function renderOpponents(g) {
  const panel = document.getElementById("opponents");
  const list = el("ol", "opp-list simple-only");
  list.setAttribute("aria-label", TLI18n.t("ov.slot.opponents"));
  for (const opponent of g.opponents) {
    list.append(simpleOpponent(opponent));
    // Under the next opponent's line (T-605).
    const odds = opponent.next ? renderOdds(g, false) : null;
    if (odds) {
      const item = el("li", "odds-row");
      item.append(odds);
      list.append(item);
    }
  }
  const full = el("div", "adv");
  // The damage cap the game shows (T-315), above the opponents in both modes.
  const cap = TLWords.damageCap(g);
  const capLine = cap ? el("p", "damage-cap", cap.text) : null;
  if (capLine) capLine.title = cap.about;
  panel.replaceChildren(...[capLine, list, full].filter(Boolean));
  panel.classList.toggle("simple-empty", !g.opponents.length && !cap);
  full.append(el("h2", null, TLI18n.t("ov.slot.opponents")));
  if (!g.opponents.length) {
    full.append(el("div", "row", TLI18n.t("ov.opponents.noLobby")));
  }
  // The next opponent the game marks (T-313), with the last board met.
  const next = TLWords.nextOpponent(g.opponents);
  if (next) {
    const block = el("div", "opponent next-block");
    const head = opponentHead(next);
    // The heading already says "next".
    const badge = head.querySelector(".next-badge");
    if (badge) badge.remove();
    const odds = renderOdds(g, true);
    block.append(el("h3", null, TLWords.nextLabels(g).heading), head, ...(odds ? [odds] : []), ...renderShown(next), ...renderBoards(next));
    full.append(block);
  }
  for (const opponent of g.opponents) {
    const item = el("div", "opponent compact");
    item.append(opponentHead(opponent));
    full.append(item);
  }
  // Boards are only on the hover card: say when the hover cannot work yet.
  const hint =
    g.leaderboard_slots === null || g.leaderboard_slots === undefined
      ? TLI18n.t("ov.opponents.hintNoOrder")
      : TLI18n.t("ov.opponents.hint");
  full.append(el("p", "hint", hint));
  panel.hidden = false;
}

// The lobby panel (T-338): the own rating with the last game's change, and
// the day's summary. Advanced also marks the rating "from the game".
function renderLobby(v) {
  const panel = document.getElementById("lobby");
  panel.replaceChildren();
  const rows = TLLobby.ratingRows(v);
  for (const r of rows) {
    const row = el("p", "lobby-rating");
    row.append(el("span", "lobby-mode", r.label), el("b", "lobby-value", r.value));
    if (r.change) {
      const change = el("span", `lobby-change ${r.up ? "up" : "down"}`, r.change);
      change.title = TLI18n.t("lobby.change.label");
      row.append(change, el("span", "visually-hidden", TLI18n.t("lobby.change.label")));
    }
    panel.append(row);
  }
  if (rows.length) {
    const source = mark("game");
    if (source) {
      source.classList.add("adv");
      panel.append(source);
    }
  }
  const note = TLLobby.ratingNote(v, document.body.dataset.detail === "advanced");
  if (note) panel.append(el("p", "notes", note));
  // Each part of a summary line stays whole; lines break only between parts.
  for (const line of TLLobby.summaryLines(v)) {
    const p = el("p", "lobby-today");
    line.split(" · ").forEach((part, i) => {
      if (i) p.append(" · ");
      p.append(el("span", "nowrap", part));
    });
    panel.append(p);
  }
  panel.hidden = false;
}

function render() {
  const legend = document.getElementById("legend");
  const g = current();
  const lobby = document.getElementById("lobby");
  // Only on the lobby screen with no game on; while arranging, the example.
  const lobbyShown = !game && (editing ? EXAMPLE_LOBBY : lobbyView && lobbyView.in_lobby ? lobbyView : null);
  if (lobbyShown) renderLobby(lobbyShown);
  else lobby.hidden = true;
  if (!g && lobbyShown) {
    for (const id of ["status", "counters", "tribes", "opponents", "legend"]) document.getElementById(id).hidden = true;
    if (window.TLHover) window.TLHover.render();
    return;
  }
  document.getElementById("status").hidden = false;
  if (!g) {
    const status = document.getElementById("status");
    status.className = "panel status";
    status.textContent = scene === "battlegrounds_loading" ? TLI18n.t("ov.status.loading") : TLI18n.t("ov.status.waiting");
    document.getElementById("counters").hidden = true;
    document.getElementById("tribes").hidden = true;
    document.getElementById("opponents").hidden = true;
    legend.hidden = true;
    if (window.TLHover) window.TLHover.render();
    return;
  }
  renderStatus(g);
  const showPanels = g.phase === "playing" || g.phase === "over";
  document.getElementById("counters").hidden = !showPanels;
  document.getElementById("tribes").hidden = !showPanels;
  document.getElementById("opponents").hidden = !showPanels;
  legend.hidden = !showPanels;
  if (showPanels) {
    renderCounters(g);
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
  // The lobby panel only matters with no game on, and a problem with it
  // must not hide a game: it is asked apart and its failure leaves it off.
  const lobbyNow = fresh ? null : await invoke("overlay_lobby", TLLobby.dayRange(new Date())).catch(() => null);
  if (mine !== latest) return;
  game = fresh;
  scene = typeof where === "string" ? where : null;
  lobbyView = lobbyNow;
  if (window.TLHover) window.TLHover.setFailed(false);
  render();
}

// Text follows the screen (T-334): the base size is a share of the window's
// height, which covers the game's picture (overlay-scale.js).
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
  // The rating is read on the lobby screen without an event: asked again
  // while the lobby panel is up.
  setInterval(() => {
    if (lobbyView && lobbyView.in_lobby) refresh().catch(showError);
  }, LOBBY_REFRESH_MS);
}

// The window cannot be clicked: say it on the page, never throw silently, and
// never leave old panels under the message as if they were current.
function showError(error) {
  const panel = document.getElementById("status");
  panel.className = "panel status unreadable";
  panel.textContent = TLI18n.t("ov.error.update", { error });
  for (const id of ["counters", "tribes", "opponents", "legend", "lobby"]) document.getElementById(id).hidden = true;
  if (window.TLHover) window.TLHover.setFailed(true);
}

// A script error must show on the page too: nobody can open a console on this window.
window.addEventListener("error", (e) => showError(e.message));
window.addEventListener("unhandledrejection", (e) => showError(e.reason));

start().catch(showError);
