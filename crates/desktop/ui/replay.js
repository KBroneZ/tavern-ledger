// Combat replays in the recap (T-215, D-099): each combat the game played
// back on screen, step by step, from the game's report (parser revision 8,
// `rounds[].replay`). Both boards with the recap's pictures; play and pause,
// and in Advanced step back and forward, the speed and every step in words.
// What the log says happened, never a simulation. A combat the game did not
// play back has none ("not shown"); a game saved before revision 8 has no
// replays at all ("not recorded"). `combats`, `frames` and `stepWords` are
// plain data in, plain data out (ui-tests/replay.test.js).
"use strict";

(() => {
  const node = typeof module !== "undefined" && module.exports;
  const I18n = node ? require("./i18n.js") : window.TLI18n;
  const Turns = node ? require("./turns.js") : window.TLTurns;
  const t = I18n.t;
  const list = (v) => (Array.isArray(v) ? v : []);
  const isInt = (v) => Number.isInteger(v);
  const SIDES = ["own", "opponent"];
  const TRIGGERS = ["start_of_combat", "deathrattle", "avenge", "rally"];
  /** Playback speeds; 1× shows a step every STEP_MS. */
  const SPEEDS = [0.5, 1, 2, 4];
  const STEP_MS = 900;

  const usable = (r) => r !== null && typeof r === "object" && Array.isArray(r.minions) && Array.isArray(r.events);

  // Which combats of a game have a replay. `recorded` is false when the game
  // was saved before replays existed (its rounds have no `replay` key).
  function combats(report) {
    const rounds = list(report && report.rounds).filter((r) => r && isInt(r.number));
    const recorded = rounds.length > 0 && rounds.every((r) => Object.prototype.hasOwnProperty.call(r, "replay"));
    const shown = [];
    const notShown = [];
    rounds.forEach((r) => {
      if (usable(r.replay)) shown.push({ round: r.number, replay: r.replay });
      else if (recorded) notShown.push(r.number);
    });
    shown.sort((a, b) => a.round - b.round);
    return { recorded, shown, notShown: notShown.sort((a, b) => a - b) };
  }

  function tileOf(m, ref) {
    const keywords = list(m.keywords).filter((k) => typeof k === "string");
    return {
      ref,
      id: typeof m.card_id === "string" ? m.card_id : null,
      atk: isInt(m.atk) ? m.atk : null,
      health: isInt(m.health) ? m.health : null,
      golden: m.golden === true,
      shield: keywords.includes("divine_shield"),
      keywords,
    };
  }

  // Is event `e` one this page can play? Every minion it names must be on a
  // board at that step (or be the one a summon brings in).
  function playable(e, minions, live) {
    if (!e || typeof e !== "object") return false;
    const on = (n) => isInt(n) && live.has(n);
    switch (e.kind) {
      case "attack":
        return on(e.attacker) && (e.defender === null || on(e.defender));
      case "damage":
        return on(e.minion) && isInt(e.amount) && isInt(e.health);
      case "shield":
        return on(e.minion) && typeof e.on === "boolean";
      case "stats":
        return on(e.minion) && isInt(e.atk) && isInt(e.health);
      case "death":
        return on(e.minion);
      case "summon":
        return isInt(e.minion) && minions[e.minion] && SIDES.includes(minions[e.minion].side) && !live.has(e.minion);
      case "trigger":
        return TRIGGERS.includes(e.trigger) && SIDES.includes(e.side);
      case "hero_damage":
        return SIDES.includes(e.side) && isInt(e.amount);
      case "board_in":
        return SIDES.includes(e.side);
      default:
        return false;
    }
  }

  // Every board state of a replay: the start, then one per step. A frame has
  // both boards (tiles left to right), the hero damage so far, the step's
  // event and its marks (attacker, defender, hurt, summoned, dying).
  // `skipped` counts the events this page could not play.
  function frames(replay) {
    const minions = list(replay && replay.minions).map((m) => (m && typeof m === "object" ? m : {}));
    const start = isInt(replay && replay.start) ? Math.min(Math.max(replay.start, 0), minions.length) : 0;
    const tiles = new Map();
    const boards = { own: [], opponent: [] };
    for (let ref = 0; ref < start; ref++) {
      const side = minions[ref].side;
      if (!SIDES.includes(side)) continue;
      tiles.set(ref, tileOf(minions[ref], ref));
      boards[side].push(ref);
    }
    const hero = { own: 0, opponent: 0 };
    // A Duos board coming in brings its hero.
    const heroes = { own: null, opponent: null };
    const shot = (event, marks) => ({
      event,
      hero: { ...hero },
      heroes: { ...heroes },
      own: boards.own.map((ref) => ({ ...tiles.get(ref), ...(marks[ref] || {}) })),
      opponent: boards.opponent.map((ref) => ({ ...tiles.get(ref), ...(marks[ref] || {}) })),
    });
    const out = [shot(null, {})];
    let dying = [];
    let skipped = 0;
    list(replay && replay.events).forEach((e) => {
      // Who died at the last step leaves the board now.
      dying.forEach((ref) => SIDES.forEach((s) => (boards[s] = boards[s].filter((x) => x !== ref))));
      dying.forEach((ref) => tiles.delete(ref));
      dying = [];
      if (!playable(e, minions, tiles)) {
        skipped += 1;
        return;
      }
      const marks = {};
      const mark = (ref, what) => (marks[ref] = { ...(marks[ref] || {}), ...what });
      const set = (ref, what) => tiles.set(ref, { ...tiles.get(ref), ...what });
      switch (e.kind) {
        case "attack":
          mark(e.attacker, { attacker: true });
          if (e.defender !== null) mark(e.defender, { defender: true });
          break;
        case "damage":
          set(e.minion, { health: e.health });
          mark(e.minion, { hurt: e.amount });
          break;
        case "shield":
          set(e.minion, { shield: e.on });
          mark(e.minion, { popped: !e.on });
          break;
        case "stats":
          set(e.minion, { atk: e.atk, health: e.health });
          mark(e.minion, { buffed: true });
          break;
        case "death":
          mark(e.minion, { dying: true });
          dying.push(e.minion);
          break;
        case "summon": {
          const tile = tileOf(minions[e.minion], e.minion);
          const board = boards[minions[e.minion].side];
          const at = isInt(minions[e.minion].position) ? minions[e.minion].position - 1 : board.length;
          tiles.set(e.minion, tile);
          board.splice(Math.min(Math.max(at, 0), board.length), 0, e.minion);
          mark(e.minion, { summoned: true });
          break;
        }
        case "trigger":
          if (isInt(e.source) && tiles.has(e.source)) mark(e.source, { triggered: true });
          break;
        case "hero_damage":
          hero[e.side] += e.amount;
          break;
        case "board_in":
          heroes[e.side] = typeof e.hero === "string" ? e.hero : null;
          break;
        default:
          break;
      }
      out.push(shot(e, marks));
    });
    out.skipped = skipped;
    return out;
  }

  // The heroes as the combat starts: the first of each side in the round.
  function heroesOf(report, round) {
    const r = list(report && report.rounds).find((x) => x && x.number === round);
    const first = (side) => {
      const e = list(r && r.entries).find((x) => x && x.side === side && typeof x.hero === "string");
      return e ? e.hero : null;
    };
    return { own: first("own"), opponent: first("opponent") };
  }

  // The combat's result as the recap reads it, and whom it was against.
  function outcomeOf(recap, round, heroText) {
    const c = Turns.combatAt(recap, round);
    if (!c) return { outcome: "unknown", source: "unknown", who: null };
    const who = c.opponents.map(heroText).join(" + ");
    return { outcome: c.outcome, source: c.source, who: who || null };
  }

  const capital = (s) => (s ? s.charAt(0).toLocaleUpperCase(I18n.locale()) + s.slice(1) : s);

  // A step in words. `minionName(ref)` names a minion of the replay ("your
  // Imp", "their Imp"); `heroText(id)` names a hero.
  function stepWords(e, minionName, heroText) {
    if (!e) return t("replay.start");
    const m = (n) => minionName(n);
    switch (e.kind) {
      case "attack":
        return capital(e.defender === null
          ? t("replay.ev.attack_alone", { a: m(e.attacker) })
          : t("replay.ev.attack", { a: m(e.attacker), b: m(e.defender) }));
      case "damage":
        return capital(e.health > 0
          ? t("replay.ev.damage", { m: m(e.minion), n: I18n.num(e.amount), left: I18n.num(e.health) })
          : t("replay.ev.damage_lethal", { m: m(e.minion), n: I18n.num(e.amount) }));
      case "shield":
        return capital(t(e.on ? "replay.ev.shield_on" : "replay.ev.shield_off", { m: m(e.minion) }));
      case "stats":
        return capital(t("replay.ev.stats", { m: m(e.minion), atk: I18n.num(e.atk), health: I18n.num(e.health) }));
      case "death":
        return capital(t("replay.ev.death", { m: m(e.minion) }));
      case "summon":
        return capital(t("replay.ev.summon", { m: m(e.minion) }));
      case "trigger": {
        const who = isInt(e.source) ? m(e.source) : t(`replay.ev.card_${e.side}`, { name: heroText(e.card_id) });
        return t(`replay.ev.trigger.${e.trigger}`, { m: who });
      }
      case "hero_damage":
        return t(`replay.ev.hero_${e.side}`, { n: I18n.num(e.amount) });
      case "board_in":
        return t(`replay.ev.board_in_${e.side}`, { hero: e.hero ? heroText(e.hero) : t("shop.hero.unknown") });
      default:
        return t("replay.ev.unknown");
    }
  }

  // --- Drawing (the page only). `h` has art, heroText, cardName and game
  // (the game's key). What is open, the step and the speed survive a redraw
  // of the recap (card names arriving, a game saved again).
  const memory = new Map();
  let speed = 1;

  function remembered(game, shown) {
    const saved = memory.get(game);
    const round = saved && shown.some((c) => c.round === saved.round) ? saved.round : shown[shown.length - 1].round;
    return { round, step: saved && saved.round === round ? saved.step : 0, playing: Boolean(saved && saved.playing && saved.round === round) };
  }

  function nameOf(replay, h) {
    const minions = list(replay.minions);
    return (ref) => {
      const m = minions[ref] || {};
      const name = typeof m.card_id === "string" ? h.cardName(m.card_id) || m.card_id : t("shop.card.unknown");
      return t(m.side === "own" ? "replay.your" : "replay.their", { name });
    };
  }

  function minionTile(ui, tile, h) {
    const box = ui.el("span", "replay-minion");
    const img = h.art(tile.id, "replay-art");
    box.append(img || ui.el("span", "replay-art art-missing"));
    box.append(ui.el("span", "replay-stats"));
    return box;
  }

  function updateTile(box, tile, h) {
    const flags = ["golden", "shield", "attacker", "defender", "dying", "summoned", "buffed", "popped", "triggered"];
    flags.forEach((f) => box.classList.toggle(f, Boolean(tile[f])));
    const stats = box.querySelector(".replay-stats");
    // Damage past 0 (overkill) shows as 0, as on the game's board.
    const health = tile.health === null ? "?" : I18n.num(Math.max(tile.health, 0));
    stats.textContent = `${tile.atk === null ? "?" : I18n.num(tile.atk)}/${health}`;
    const old = box.querySelector(".replay-hurt");
    if (old) old.remove();
    if (tile.hurt) {
      const hurt = document.createElement("span");
      hurt.className = "replay-hurt";
      hurt.textContent = `−${I18n.num(tile.hurt)}`;
      box.append(hurt);
    }
    const name = tile.id ? h.cardName(tile.id) || tile.id : t("shop.card.unknown");
    const marks = [tile.golden && t("keyword.golden"), ...tile.keywords.map((k) => (`keyword.${k}` in I18n.tables().en ? t(`keyword.${k}`) : k))].filter(Boolean);
    box.title = [`${name} ${stats.textContent}`, marks.join(", ")].filter(Boolean).join(" · ");
  }

  function heroBadge(ui) {
    const box = ui.el("span", "replay-hero");
    box.append(ui.el("span", "replay-hero-face"), ui.el("span", "replay-hero-name"), ui.el("span", "replay-hero-hit"));
    return box;
  }

  function setHero(ui, badge, id, h) {
    if (badge.dataset.hero === String(id)) return;
    badge.dataset.hero = String(id);
    const img = id && h.art(id, "portrait");
    badge.querySelector(".replay-hero-face").replaceChildren(...(img ? [img] : []));
    badge.querySelector(".replay-hero-name").textContent = id ? h.heroText({ id }) : t("shop.hero.unknown");
  }

  function button(ui, cls, text, label) {
    const b = ui.el("button", cls, text);
    b.type = "button";
    if (label) b.setAttribute("aria-label", label);
    if (label) b.title = label;
    return b;
  }

  // The player of one combat.
  function player(ui, combat, outcome, heroes, h, state) {
    const replay = combat.replay;
    const all = frames(replay);
    const name = nameOf(replay, h);
    const heroName = (id) => h.heroText({ id });
    const box = ui.el("div", "replay-player");
    const head = ui.el("div", "replay-head");
    const result = ui.el("span", `chip chip-${outcome.outcome}`, t(`outcome.${outcome.outcome}`));
    ui.addTip(result, outcome.source);
    const counter = ui.el("span", "replay-counter");
    head.append(result, ui.el("span", "replay-title", t("replay.combat_of", { turn: combat.round, who: outcome.who || t("turns.opponent_unknown") })), counter);
    const sides = {};
    const stage = ui.el("div", "replay-stage");
    SIDES.slice().reverse().forEach((side) => {
      const row = ui.el("div", `replay-side ${side}`);
      const badge = heroBadge(ui);
      const board = ui.el("div", "replay-board");
      row.append(badge, board);
      sides[side] = { board, badge, tiles: new Map() };
      stage.append(row);
    });
    const caption = ui.el("p", "replay-caption");
    caption.setAttribute("aria-live", "polite");
    const controls = ui.el("div", "replay-controls");
    const restart = button(ui, "replay-btn adv-only", "⏮", t("replay.restart"));
    const back = button(ui, "replay-btn adv-only", "◀", t("replay.back"));
    const play = button(ui, "replay-btn replay-play", "", null);
    const forward = button(ui, "replay-btn adv-only", "▶", t("replay.forward"));
    const speedBox = ui.el("label", "replay-speed adv-only", `${t("replay.speed")} `);
    const select = document.createElement("select");
    SPEEDS.forEach((s) => {
      const o = document.createElement("option");
      o.value = String(s);
      o.textContent = t("replay.speed_x", { n: I18n.num(s) });
      o.selected = s === speed;
      select.append(o);
    });
    speedBox.append(select);
    controls.append(restart, back, play, forward, speedBox);
    const steps = h.fold(ui, `${h.game}:replay-steps:${combat.round}`, t("replay.steps", { n: I18n.num(all.length - 1) }), "replay-steps adv-only");
    // Numbered from 0, the start, like the step counter.
    const ol = ui.el("ol", "replay-list");
    ol.setAttribute("start", "0");
    const items = all.map((f, i) => {
      const li = ui.el("li");
      const b = button(ui, "replay-step", stepWords(f.event, name, heroName), null);
      b.addEventListener("click", () => go(i, false));
      li.append(b);
      return li;
    });
    ol.append(...items);
    steps.append(ol);
    box.append(head, stage, caption, controls);
    if (!replay.complete) box.append(ui.withSource(ui.el("p", "note", t("replay.cut")), "unknown"));
    if (all.skipped) box.append(ui.withSource(ui.el("p", "note", I18n.plural("replay.skipped", all.skipped)), "unknown"));
    box.append(steps);

    let timer = null;
    const stop = () => {
      if (timer) clearTimeout(timer);
      timer = null;
    };
    function draw() {
      const f = all[state.step];
      SIDES.forEach((side) => {
        const s = sides[side];
        const nodes = f[side].map((tile) => {
          if (!s.tiles.has(tile.ref)) s.tiles.set(tile.ref, minionTile(ui, tile, h));
          const node = s.tiles.get(tile.ref);
          updateTile(node, tile, h);
          return node;
        });
        s.board.replaceChildren(...nodes);
        if (!nodes.length) s.board.append(ui.el("span", "replay-empty", t("turns.board.empty")));
        setHero(ui, s.badge, f.heroes[side] || heroes[side], h);
        const hit = s.badge.querySelector(".replay-hero-hit");
        hit.textContent = f.hero[side] ? `−${I18n.num(f.hero[side])}` : "";
      });
      caption.textContent = stepWords(f.event, name, heroName);
      counter.textContent = t("replay.step_of", { n: I18n.num(state.step), total: I18n.num(all.length - 1) });
      play.textContent = state.playing ? "⏸" : "▶";
      play.setAttribute("aria-label", t(state.playing ? "replay.pause" : "replay.play"));
      play.title = play.getAttribute("aria-label");
      play.setAttribute("aria-pressed", String(state.playing));
      back.disabled = state.step === 0;
      restart.disabled = state.step === 0;
      forward.disabled = state.step === all.length - 1;
      items.forEach((li, i) => li.classList.toggle("now", i === state.step));
      memory.set(h.game, { ...state });
    }
    function tick() {
      timer = null;
      if (!box.isConnected || !state.playing) return;
      if (state.step >= all.length - 1) {
        state.playing = false;
        draw();
        return;
      }
      state.step += 1;
      draw();
      timer = setTimeout(tick, STEP_MS / speed);
    }
    function go(step, playing) {
      stop();
      state.step = Math.min(Math.max(step, 0), all.length - 1);
      state.playing = playing;
      draw();
      if (playing) timer = setTimeout(tick, STEP_MS / speed);
    }
    play.addEventListener("click", () => {
      if (state.playing) return go(state.step, false);
      // At the end, play starts over.
      go(state.step >= all.length - 1 ? 0 : state.step, true);
    });
    back.addEventListener("click", () => go(state.step - 1, false));
    forward.addEventListener("click", () => go(state.step + 1, false));
    restart.addEventListener("click", () => go(0, false));
    select.addEventListener("change", () => {
      speed = SPEEDS.includes(Number(select.value)) ? Number(select.value) : 1;
      if (state.playing) go(state.step, true);
    });
    go(state.step, state.playing);
    return box;
  }

  // The recap's section: a chip per shown combat, the player of the chosen
  // one, and what has no replay. `r` is the app's recap of the game (the
  // results), `report` the game's report (the replays).
  function section(ui, r, report, h) {
    const part = ui.el("section", "recap-part replays");
    part.append(ui.el("h3", null, t("replay.title")));
    const found = combats(report);
    if (!found.recorded) {
      part.append(ui.withSource(ui.el("p", "note", t("replay.not_recorded")), "unknown"));
      return part;
    }
    if (!found.shown.length) {
      part.append(ui.withSource(ui.el("p", "note", t("replay.none")), "unknown"));
      return part;
    }
    const state = remembered(h.game, found.shown);
    const pick = ui.el("div", "replay-pick");
    pick.setAttribute("role", "group");
    pick.setAttribute("aria-label", t("replay.pick"));
    const holder = ui.el("div", "replay-holder");
    const show = (round) => {
      const combat = found.shown.find((c) => c.round === round);
      if (round !== state.round) Object.assign(state, { round, step: 0, playing: false });
      pick.querySelectorAll("button").forEach((b) => b.setAttribute("aria-pressed", String(b.dataset.round === String(round))));
      holder.replaceChildren(player(ui, combat, outcomeOf(r, round, h.heroText), heroesOf(report, round), h, state));
    };
    found.shown.forEach((c) => {
      const o = outcomeOf(r, c.round, h.heroText);
      const b = button(ui, `replay-chip chip-${o.outcome}`, t("replay.chip", { turn: c.round }), t("replay.chip_tip", { turn: c.round, outcome: t(`outcome.${o.outcome}`) }));
      b.dataset.round = String(c.round);
      b.addEventListener("click", () => show(c.round));
      pick.append(b);
    });
    part.append(pick, holder);
    if (found.notShown.length) {
      part.append(ui.withSource(ui.el("p", "note", I18n.plural("replay.not_shown", found.notShown.length, { turns: I18n.list(found.notShown.map((n) => I18n.num(n))) })), "unknown"));
    }
    part.append(ui.el("p", "note adv-only", t("replay.about")));
    show(state.round);
    return part;
  }

  const api = { SPEEDS, combats, frames, stepWords, heroesOf, outcomeOf, section };
  if (typeof module !== "undefined" && module.exports) module.exports = api;
  else window.TLReplay = api;
})();
