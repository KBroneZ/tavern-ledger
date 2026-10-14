// The overlay (T-301). The app decides every value and its source
// (tracker::live); this file only shows them. The window is click-through
// while locked, so a source is always a word on the page, never a tooltip.
// Text is set with textContent only: nothing from the log is ever treated as markup.
"use strict";

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const SOURCES = ["log", "inferred", "unknown"];
// Up to this many triples are drawn as stars; more as a star and the number.
const MAX_STARS = 3;

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
  // Nothing of the heroes, mode, combat or health in words during a game
  // (T-339): the game shows all of it already.
  if (g.phase === "over") panel.append(el("div", null, TLI18n.t("ov.status.over")));
  if (g === EXAMPLE_GAME) panel.append(el("p", "notes", TLI18n.t("ov.status.example")));
  const notes = [...g.warnings, ...g.problems].slice(0, 2).map((w) => TLI18n.backend(w));
  if (notes.length) panel.append(el("p", "notes", notes.join(" · ")));
  // Warnings and problems always show: a failure is never silent.
  panel.classList.toggle("empty", g.phase !== "over" && g !== EXAMPLE_GAME && !notes.length);
}

// The tribe row (T-339, D-148): the lobby's tribes as short chips, only the
// ones read from the game. With none, one short line says why.
function tribeRow(g) {
  const panel = TLWords.tribesPanel(g);
  if (!panel.tribes.length) return el("p", "tribe-reason", panel.reason);
  const row = el("div", "tribe-chips");
  row.setAttribute("aria-label", TLI18n.t("ov.slot.tribes"));
  for (const t of panel.tribes) {
    // The short name is seen, the full one read out.
    const chip = el("span", "chip");
    const short = el("span", null, tribeShort(t));
    short.setAttribute("aria-hidden", "true");
    chip.append(short, el("span", "visually-hidden", tribeName(t)));
    row.append(chip);
  }
  return row;
}

// A short tribe name that fits a chip; the full name when there is none.
function tribeShort(id) {
  const key = `ov.tribeShort.${String(id).toUpperCase()}`;
  return key in TLI18n.tables().en ? TLI18n.t(key) : tribeName(id);
}

function renderTribes(g) {
  const panel = document.getElementById("tribes");
  const row = tribeRow(g);
  panel.replaceChildren(row);
  panel.classList.remove("empty");
  panel.hidden = false;
}

// The counters panel (T-612): the player's own running totals, one line each
// Hidden when there is nothing to show: null (unknown) and [] (nothing applies) both leave no empty box.
function renderCounters(g) {
  const panel = document.getElementById("counters");
  if (!TLCounters.ENABLED) {
    panel.hidden = true;
    return;
  }
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
    seen.append(main);
    item.append(seen, el("span", "visually-hidden", r.said));
    list.append(item);
  }
  panel.replaceChildren(list);
}

// A minion's Dark Gifts (T-528, D-107): a small mark
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

// One opponent in the side panel (T-339), one line each in the
// leaderboard's order; the boards are on the hover card, over the game's own
// leaderboard. The heroes' portraits with
// their tier, the health and the triples when above 0; the game's next mark
// as a lit row. No names (they are on the hover card) and no unknowns.
function opponentLine(opponent) {
  const r = TLWords.opponentRow(opponent);
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
    const word = TLWords.nextLabel(current());
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

// One side of the odds bar: the range of health taken on a win (left) or
// of damage taken on a loss (right), with its small word.
function oddsRange(side, range) {
  const node = el("span", `odds-range ${side}`);
  node.append(el("span", "odds-range-label", TLI18n.t(`ov.odds.range${side === "win" ? "Win" : "Lose"}Label`)), el("b", "odds-range-value", range.text));
  node.setAttribute("aria-hidden", "true");
  return node;
}

// The combat odds bar at the top centre (T-367, T-605): win, tie and lose
// shares as one bar, the health range of a win to its left and the damage
// range of a loss to its right (only when the app knows them), or the reason
// there are no odds. While arranging, example odds fill an empty bar.
function renderOdds(g) {
  const panel = document.getElementById("odds");
  let v = TLOdds.oddsView(g.odds);
  if (!v && editing) v = TLOdds.oddsView(EXAMPLE_GAME.odds);
  panel.classList.toggle("empty", !v);
  panel.hidden = false;
  if (!v) {
    panel.replaceChildren();
    return;
  }
  const box = el("div", `odds ${v.kind}`);
  box.title = v.about;
  if (v.kind === "note") {
    box.append(el("p", "odds-note", v.text));
  } else {
    const bar = el("div", "odds-bar-row");
    bar.setAttribute("aria-hidden", "true");
    if (v.ranges.win) bar.append(oddsRange("win", v.ranges.win));
    const track = el("div", "odds-shares");
    for (const s of v.shares) {
      const cell = el("span", `odds-share ${s.key}`);
      cell.style.flexGrow = String(Math.max(Number.parseFloat(s.value) || 0, 1));
      cell.append(el("span", "odds-label", s.label), el("b", "odds-value", s.value));
      track.append(cell);
    }
    bar.append(track);
    if (v.ranges.lose) bar.append(oddsRange("lose", v.ranges.lose));
    box.append(bar);
    // A game build the rules were not checked against warns here (D-145); "≈" when a card is played with a gap.
    if (v.warningShort) box.append(el("p", "odds-warning", v.warningShort));
    if (v.age) box.append(el("p", "odds-age", v.approx ? `≈ ${v.age}` : v.age));
  }
  box.append(el("span", "visually-hidden", v.said));
  panel.replaceChildren(box);
}

// Where the game's leaderboard is, in the window's own pixels (the app's
// overlay_settings); null until known. The marks follow it (T-368).
let boardArea = null;

// Over each opponent's hero in the game's leaderboard, on its left (T-368):
// the remaining health and one star per triple. The right edge of the mark
// is the leaderboard's left edge; the room to its left is kept for the quest
// mark. No place or board size known: no mark.
function renderMarks(g) {
  const root = document.getElementById("marks");
  root.replaceChildren();
  if (!g || !boardArea) return;
  for (const m of TLWords.opponentMarks(g)) {
    const each = boardArea.h / m.slots;
    const mark = el("div", `opp-mark${m.out ? " out" : ""}`);
    mark.style.top = `${boardArea.y + each * (m.place - 1) + each / 2}px`;
    mark.style.right = `calc(100% - ${boardArea.x}px)`;
    const said = [];
    if (m.triples > 0) {
      const stars = el("span", "mark-stars", m.triples <= MAX_STARS ? "★".repeat(m.triples) : `★${m.triples}`);
      stars.setAttribute("aria-hidden", "true");
      mark.append(stars);
      said.push(TLI18n.plural("ov.triples", m.triples));
    }
    if (m.health !== null) {
      const hp = el("b", "mark-hp", m.health);
      hp.setAttribute("aria-hidden", "true");
      mark.append(hp);
      said.push(`${m.health} ${TLI18n.t("ov.hp.unit")}`);
    }
    if (!said.length) continue;
    mark.append(el("span", "visually-hidden", said.join(", ")));
    root.append(mark);
  }
}

// One hero on offer (T-365): its portrait, name and the player's own numbers
// from their history, as figures with small labels; "no games yet" when there
// are none. The two empty slots at the right are for the community's tier and
// the tribe affinity, which come later: nothing is drawn in them now.
function heroCard(c) {
  const item = el("li", "hero-card");
  const face = el("span", "face");
  face.setAttribute("aria-hidden", "true");
  const art = TLCards.art(c.hero.id, "portrait");
  face.append(art || el("span", "portrait blank"));
  // The figures are seen; the sentence after them is read out instead.
  const main = el("span", "hero-main");
  main.setAttribute("aria-hidden", "true");
  main.append(el("b", "hero-name", c.name));
  if (c.own) {
    const figures = el("span", "hero-figures");
    for (const f of c.own.figures) {
      const fig = el("span", `fig fig-${f.key}`);
      fig.append(el("b", "fig-value", f.value), el("span", "fig-label", f.label));
      figures.append(fig);
    }
    main.append(figures);
  } else {
    main.append(el("span", "hero-none", TLI18n.t("ov.heroes.none")));
  }
  const slots = el("span", "hero-slots");
  for (const name of ["tier", "affinity"]) {
    const slot = el("span", `hero-slot hero-${name}`);
    slot.dataset.slot = name;
    slot.hidden = true;
    slots.append(slot);
  }
  item.append(face, main, slots, el("span", "visually-hidden", c.said));
  return item;
}

// The hero select panel: the lobby's tribes from the game at its head, then
// one card per hero on offer, updated on each reroll. Only while the game is
// at hero select; while arranging, an example fills it. Returns whether it shows.
function renderHeroes(g) {
  const panel = document.getElementById("heroes");
  let view = TLHeroes.heroCards(g, heroName);
  const example = !view && editing;
  if (example) view = TLHeroes.heroCards({ ...EXAMPLE_GAME, phase: "starting", hero_offer: EXAMPLE_HERO_OFFER }, heroName);
  panel.hidden = !view;
  if (!view) {
    panel.replaceChildren();
    return false;
  }
  const head = el("div", "heroes-head");
  head.append(el("b", "heroes-title", view.title));
  if (view.rerolls) head.append(el("span", "heroes-rerolls", view.rerolls));
  const list = el("ol", "hero-list");
  list.setAttribute("aria-label", view.title);
  for (const c of view.cards) list.append(heroCard(c));
  panel.replaceChildren(head, tribeRow(example ? EXAMPLE_GAME : g), list);
  return true;
}

function renderOpponents(g) {
  const panel = document.getElementById("opponents");
  const list = el("ol", "opp-list");
  list.setAttribute("aria-label", TLI18n.t("ov.slot.opponents"));
  for (const opponent of g.opponents) {
    list.append(opponentLine(opponent));
  }
  // The damage cap the game shows (T-315), above the opponents.
  const cap = TLWords.damageCap(g);
  const capLine = cap ? el("p", "damage-cap", cap.text) : null;
  if (capLine) capLine.title = cap.about;
  panel.replaceChildren(...[capLine, list].filter(Boolean));
  panel.classList.toggle("empty", !g.opponents.length && !cap);
  panel.hidden = false;
}

// One game of the day as a tile (T-366): its hero's portrait as the
// background, the place on it and the rating won or lost, green up or red
// down (a mark as well as a colour). A value the app does not give is left
// out. The sentence is read out instead of the pieces.
function recapTile(t) {
  const tile = el("li", `recap-tile${t.duos ? " duos" : ""}`);
  const seen = el("span", "recap-seen");
  seen.setAttribute("aria-hidden", "true");
  const art = t.hero ? TLCards.art(t.hero.id, "portrait") : null;
  const back = art || el("span", "portrait blank");
  back.classList.add("recap-back");
  seen.append(back);
  if (t.duos) seen.append(el("span", "recap-mode", t.mode));
  seen.append(el("b", "recap-place", t.place ? t.place : "–"));
  if (t.change) seen.append(el("span", `recap-change ${t.up ? "up" : "down"}`, t.change));
  tile.append(seen, el("span", "visually-hidden", t.said));
  return tile;
}

// The lobby panel (T-338, T-366): the own rating per mode with the one the
// day started with, the day's last games as tiles, and the day's summary.
function renderLobby(v) {
  const panel = document.getElementById("lobby");
  panel.replaceChildren();
  for (const r of TLLobby.ratingRows(v)) {
    const row = el("p", "lobby-rating");
    const seen = el("span", "lobby-seen");
    seen.setAttribute("aria-hidden", "true");
    seen.append(el("span", "lobby-mode", r.label), el("b", "lobby-value", r.value));
    if (r.net) seen.append(el("span", `lobby-change ${r.up ? "up" : "down"}`, r.net));
    if (r.start) seen.append(el("span", "lobby-from", TLI18n.t("lobby.from", { value: r.start })));
    row.append(seen, el("span", "visually-hidden", r.said));
    panel.append(row);
  }
  const note = TLLobby.ratingNote(v);
  if (note) panel.append(el("p", "notes", note));
  const tiles = TLLobby.recapTiles(v, heroName);
  if (tiles.length) {
    const list = el("ol", "recap-tiles");
    list.setAttribute("aria-label", TLI18n.t("lobby.tiles.label"));
    for (const t of tiles) list.append(recapTile(t));
    panel.append(list);
  }
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
    for (const id of ["status", "heroes", "counters", "tribes", "opponents", "odds", "legend"]) document.getElementById(id).hidden = true;
    renderMarks(null);
    if (window.TLHover) window.TLHover.render();
    return;
  }
  document.getElementById("status").hidden = false;
  if (!g) {
    const status = document.getElementById("status");
    status.className = "panel status";
    status.textContent = scene === "battlegrounds_loading" ? TLI18n.t("ov.status.loading") : TLI18n.t("ov.status.waiting");
    document.getElementById("heroes").hidden = true;
    document.getElementById("counters").hidden = true;
    document.getElementById("tribes").hidden = true;
    document.getElementById("opponents").hidden = true;
    document.getElementById("odds").hidden = true;
    legend.hidden = true;
    renderMarks(null);
    if (window.TLHover) window.TLHover.render();
    return;
  }
  renderStatus(g);
  // At hero select the heroes panel says where the game is; its line goes.
  // Unless the user switched the heroes panel off: then the status line stays.
  const heroesSlot = document.getElementById("heroes").closest(".slot");
  if (renderHeroes(g) && g.phase === "starting" && !heroesSlot.hidden) document.getElementById("status").hidden = true;
  const showPanels = g.phase === "playing" || g.phase === "over";
  document.getElementById("counters").hidden = !showPanels;
  document.getElementById("tribes").hidden = !showPanels;
  document.getElementById("opponents").hidden = !showPanels;
  document.getElementById("odds").hidden = !showPanels;
  legend.hidden = !showPanels;
  renderMarks(showPanels ? g : null);
  if (showPanels) {
    renderCounters(g);
    renderTribes(g);
    renderOpponents(g);
    renderOdds(g);
  }
  if (window.TLHover) window.TLHover.render();
}

// Answers can come back out of order: only the newest request is shown.
let latest = 0;

async function refresh() {
  const mine = ++latest;
  const [fresh, where, settings] = await Promise.all([
    invoke("overlay_state"),
    invoke("overlay_scene"),
    // Only for where the game's leaderboard is: a failure leaves the marks off.
    invoke("overlay_settings").catch(() => null),
  ]);
  // The lobby panel only matters with no game on, and a problem with it
  // must not hide a game: it is asked apart and its failure leaves it off.
  const lobbyNow = fresh ? null : await invoke("overlay_lobby", TLLobby.dayRange(new Date())).catch(() => null);
  if (mine !== latest) return;
  game = fresh;
  scene = typeof where === "string" ? where : null;
  boardArea = settings && settings.leaderboard ? settings.leaderboard.area : null;
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
  TLCards.onChange(render);
  // Listen first, so a change between the first read and the listener is not lost.
  await listen("live-changed", () => refresh().catch(showError));
  await listen("lobby-tribes-changed", () => refresh().catch(showError));
  // New card data can change the possible minions (T-307).
  await listen("cards-changed", () => refresh().catch(showError));
  await listen("overlay-settings-changed", () => refresh().catch(showError));
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
  for (const id of ["heroes", "counters", "tribes", "opponents", "odds", "legend", "lobby"]) document.getElementById(id).hidden = true;
  renderMarks(null);
  if (window.TLHover) window.TLHover.setFailed(true);
}

// A script error must show on the page too: nobody can open a console on this window.
window.addEventListener("error", (e) => showError(e.message));
window.addEventListener("unhandledrejection", (e) => showError(e.reason));

start().catch(showError);
