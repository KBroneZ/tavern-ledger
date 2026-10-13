// Combat replays in the recap (T-215, D-099): each combat the game played
// back on screen, step by step, from the game's report (parser revision 8,
// `rounds[].replay`). Both boards with the recap's pictures; play and pause,
// and in Advanced step back and forward, the speed and every step in words.
// What the log says happened, never a simulation. A combat the game did not
// play back has none ("not shown"); a game saved before revision 8 has no
// replays at all ("not recorded"). `combats`, `frames` and `stepWords` are
// plain data in, plain data out (ui-tests/replay.test.js), and so are the
// pace (`beats`) and the motion's geometry (T-560, D-132). Since parser
// revision 13 (T-575) a step is a beat of the record: every event one log
// block gave plays at once, an attack is always a step of its own, and the
// opening's start-of-combat buffs play together; damage names its source
// and keywords given or used mid-combat show as marks. The website's
// replay.ts does the same.
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
  /** The keywords a token wears as a mark, as the game shows them on the board. */
  const MARKED = ["taunt", "reborn", "venomous", "windfury", "stealth"];
  /** Keywords a keyword step may give or take (Divine Shield has its own step). */
  const STEP_KEYWORDS = ["taunt", "reborn", "venomous", "windfury", "stealth", "deathrattle", "rally"];
  const CAUSES = ["attack", "start_of_combat", "deathrattle", "avenge", "rally", "hero_power", "trinket", "effect"];
  /** Playback speeds; at 1× each step stays for its beat (BEAT_MS). */
  const SPEEDS = [0.5, 1, 2, 4];
  // How long each kind of step stays at 1× before the next one plays (T-560,
  // D-132), the website's too (web/src/lib/game/replay.ts; a test keeps them
  // equal). Set from the game's own playback on the owner's 73 combats: about
  // 1.2 s from one attack to the next with no death, 1.3 s with one, 2.1 s
  // with two or more. An attack's beat ends as it hits.
  const BEAT_MS = {
    start: 900,
    attack: 600,
    damage: 260,
    shield: 120,
    stats: 220,
    death: 300,
    summon: 380,
    trigger: 480,
    hero_damage: 1200,
    board_in: 800,
    keyword: 220,
    unknown: 0,
  };
  /** What the log writes between an attack and its first damage (a buff, a shield) plays at the impact. */
  const AT_IMPACT_MS = 60;
  /** The parser's bound on events per combat (crates/bg-parser/src/replay.rs). */
  const MAX_EVENTS = 2000;

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
      gifts: list(m.gifts).filter((g) => typeof g === "string"),
      gift_unknown: m.gift_unknown === true,
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
      case "keyword":
        return on(e.minion) && STEP_KEYWORDS.includes(e.keyword) && typeof e.on === "boolean";
      default:
        return false;
    }
  }

  // The boards as the combat starts: every minion before `start` on its side.
  function startState(replay) {
    const minions = list(replay && replay.minions).map((m) => (m && typeof m === "object" ? m : {}));
    const start = isInt(replay && replay.start) ? Math.min(Math.max(replay.start, 0), minions.length) : 0;
    const st = { minions, tiles: new Map(), boards: { own: [], opponent: [] }, hero: { own: 0, opponent: 0 }, heroes: { own: null, opponent: null }, dying: [], incoming: new Set() };
    for (let ref = 0; ref < start; ref++) {
      const side = minions[ref].side;
      if (!SIDES.includes(side)) continue;
      st.tiles.set(ref, tileOf(minions[ref], ref));
      st.boards[side].push(ref);
    }
    return st;
  }

  // Plays event `e` on the state; returns the step's marks per minion.
  function applyEvent(st, e) {
    const marks = {};
    const mark = (ref, what) => (marks[ref] = { ...(marks[ref] || {}), ...what });
    const set = (ref, what) => st.tiles.set(ref, { ...st.tiles.get(ref), ...what });
    switch (e.kind) {
      case "attack":
        st.incoming.clear();
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
      case "stats": {
        // A buff or a heal shows what it gave (the website's gain mark).
        const was = st.tiles.get(e.minion);
        const gain = isInt(was.atk) && isInt(was.health) ? { atk: e.atk - was.atk, health: e.health - was.health } : null;
        set(e.minion, { atk: e.atk, health: e.health });
        mark(e.minion, gain && gain.atk >= 0 && gain.health >= 0 && gain.atk + gain.health > 0 ? { buffed: true, gain } : { buffed: true });
        break;
      }
      case "death":
        mark(e.minion, { dying: true });
        st.dying.push(e.minion);
        break;
      case "summon": {
        const m = st.minions[e.minion];
        const board = st.boards[m.side];
        // Slot 1 is the left-most; a slot of 0 (none in the log) goes left
        // too, but a board coming in fills from the left.
        const unplaced = !isInt(m.position) || (m.position <= 0 && st.incoming.has(m.side));
        const at = unplaced ? board.length : m.position - 1;
        st.tiles.set(e.minion, tileOf(m, e.minion));
        board.splice(Math.min(Math.max(at, 0), board.length), 0, e.minion);
        mark(e.minion, { summoned: true });
        break;
      }
      case "trigger":
        if (isInt(e.source) && st.tiles.has(e.source)) mark(e.source, { triggered: true });
        break;
      case "hero_damage":
        st.hero[e.side] += e.amount;
        break;
      case "keyword": {
        const now = st.tiles.get(e.minion).keywords;
        if (e.on !== now.includes(e.keyword)) {
          set(e.minion, { keywords: e.on ? [...now, e.keyword] : now.filter((k) => k !== e.keyword) });
          mark(e.minion, e.on ? { gained: [e.keyword] } : { lost: [e.keyword] });
        }
        break;
      }
      case "board_in":
        // A Duos board coming in brings its hero and replaces whatever is
        // still drawn on its side: apps before parser revision 11 left the
        // shop board there.
        st.heroes[e.side] = typeof e.hero === "string" ? e.hero : null;
        st.incoming.add(e.side);
        st.boards[e.side].forEach((ref) => st.tiles.delete(ref));
        st.boards[e.side] = [];
        break;
      default:
        break;
    }
    return marks;
  }

  const snapshot = (st, events, marks) => ({
    event: events[0] || null,
    events,
    hero: { ...st.hero },
    heroes: { ...st.heroes },
    own: st.boards.own.map((ref) => ({ ...st.tiles.get(ref), ...(marks[ref] || {}) })),
    opponent: st.boards.opponent.map((ref) => ({ ...st.tiles.get(ref), ...(marks[ref] || {}) })),
  });

  // Every board state of a replay: the start, then one per step. A frame has
  // both boards (tiles left to right), the hero damage so far, the step's
  // event and its marks (attacker, defender, hurt, summoned, dying).
  // `skipped` counts the events this page could not play; a hand-edited
  // file is cut at the parser's own bound.
  function frames(replay) {
    const st = startState(replay);
    const out = [snapshot(st, [], {})];
    let skipped = 0;
    const grouped = steps(replay);
    grouped.forEach((step) => {
      // Who died at the last step leaves the board now.
      st.dying.forEach((ref) => SIDES.forEach((s) => (st.boards[s] = st.boards[s].filter((x) => x !== ref))));
      st.dying.forEach((ref) => st.tiles.delete(ref));
      st.dying = [];
      const played = [];
      const marks = {};
      step.forEach((e) => {
        if (!playable(e, st.minions, st.tiles)) {
          skipped += 1;
          return;
        }
        const got = applyEvent(st, e);
        Object.keys(got).forEach((ref) => (marks[ref] = together(marks[ref], got[ref])));
        played.push(e);
      });
      if (played.length) out.push(snapshot(st, played, marks));
    });
    out.skipped = skipped;
    out.grouped = beatsOf(replay) !== null;
    return out;
  }

  // Marks of several events on one minion, together: damage adds up, keywords gather.
  function together(a, b) {
    if (!a) return b;
    const out = { ...a, ...b };
    if (a.hurt && b.hurt) out.hurt = a.hurt + b.hurt;
    if (a.gain && b.gain) out.gain = { atk: a.gain.atk + b.gain.atk, health: a.gain.health + b.gain.health };
    ["gained", "lost"].forEach((k) => {
      if (a[k] && b[k]) out[k] = [...new Set([...a[k], ...b[k]])];
    });
    return out;
  }

  // The record's beats (revision 13): each at least one event, adding up to
  // the events; null in an older record, or one this page cannot trust.
  function beatsOf(replay) {
    const runs = replay && replay.beats;
    const events = list(replay && replay.events).slice(0, MAX_EVENTS);
    if (!Array.isArray(runs) || !runs.every((n) => isInt(n) && n > 0)) return null;
    return runs.reduce((a, b) => a + b, 0) === events.length ? runs : null;
  }

  // A start-of-combat effect that only changes stats, shields or keywords.
  function onlyBuffs(run) {
    const sot = (e) => e.kind === "trigger" && e.trigger === "start_of_combat";
    return run.length > 0 && sot(run[0]) && run.every((e) => sot(e) || ["stats", "shield", "keyword"].includes(e.kind));
  }

  // The steps of a replay, each the events played at once: a beat of the
  // record, with an attack always a step of its own, and before the first
  // attack the start-of-combat buffs of the whole table together; one event
  // each in an older record.
  function steps(replay) {
    const events = list(replay && replay.events).slice(0, MAX_EVENTS).map((e) => (e && typeof e === "object" ? e : {}));
    const runs = beatsOf(replay);
    if (!runs) return events.map((e) => [e]);
    const out = [];
    let opening = true;
    const push = (run) => {
      const last = out[out.length - 1];
      if (opening && last && onlyBuffs(last) && onlyBuffs(run)) last.push(...run);
      else out.push(run);
    };
    let at = 0;
    runs.forEach((n) => {
      let run = [];
      events.slice(at, at + n).forEach((e) => {
        if (e.kind !== "attack") {
          run.push(e);
          return;
        }
        if (run.length) push(run);
        opening = false;
        out.push([e]);
        run = [];
      });
      if (run.length) push(run);
      at += n;
    });
    return out;
  }

  // A beat's length at 1× (revision 13): each kind of step in it once, at
  // its own beat; damage counts for at most two minions.
  function groupBeat(events) {
    const counts = new Map();
    events.forEach((e) => {
      const kind = Object.prototype.hasOwnProperty.call(BEAT_MS, e.kind) ? e.kind : "unknown";
      counts.set(kind, (counts.get(kind) || 0) + 1);
    });
    let ms = 0;
    counts.forEach((n, kind) => (ms += BEAT_MS[kind] * (kind === "damage" ? Math.min(n, 2) : 1)));
    return ms;
  }

  // Each frame's beat at 1×: by groupBeat for a record with beats, else as before.
  function frameBeats(all) {
    if (!all.grouped) return beats(all.map((f) => f.event));
    return all.map((f) => (f.event ? groupBeat(f.events) : BEAT_MS.start));
  }

  // Each step's beat at 1× (the steps' events, null for the start), with the
  // attack's damage on its impact.
  function beats(events) {
    let landing = false;
    return events.map((e) => {
      const kind = e ? (Object.prototype.hasOwnProperty.call(BEAT_MS, e.kind) ? e.kind : "unknown") : "start";
      if (kind === "attack") landing = true;
      else if (kind === "damage" || kind === "death" || kind === "hero_damage") landing = false;
      else if (landing) return AT_IMPACT_MS;
      return BEAT_MS[kind];
    });
  }

  // --- Motion (T-560, D-132): the attacker winds up, flies to its target and
  // back, the target shakes as it is hit, a board that loses or gains a
  // minion slides its others into place. Web Animations on transform only;
  // the boards are measured once before a step and once after it, and none
  // of it plays with reduced motion. The website's replayMotion.ts does the same.
  const IMPACT_MS = BEAT_MS.attack;
  const ATTACK_MS = IMPACT_MS + 300;
  const HIT_MS = 260;
  const SLIDE_MS = 260;
  /** Where the attacker stops: this share of the target's half-size along the attack short of its centre (on it, at any distance and angle). */
  const STOP = 0.5;
  const WIND_UP_PX = 8;
  const EASE_OUT = "cubic-bezier(0.23, 1, 0.32, 1)";
  const EASE_IN_OUT = "cubic-bezier(0.77, 0, 0.175, 1)";
  // One decimal, and never "-0px".
  const px = (v) => `${Math.round(v * 10) / 10 + 0}px`;
  const translate = (x, y) => `translate(${px(x)}, ${px(y)})`;
  const centre = (b) => ({ x: b.x + b.width / 2, y: b.y + b.height / 2 });

  // The attacker's flight from box `from` to box `to`, as keyframes over ATTACK_MS.
  function lungeFrames(from, to) {
    const a = centre(from);
    const b = centre(to);
    const dx = b.x - a.x;
    const dy = b.y - a.y;
    const length = Math.hypot(dx, dy) || 1;
    const hit = IMPACT_MS / ATTACK_MS;
    const stop = STOP * ((Math.abs(dx) / length) * (to.width / 2) + (Math.abs(dy) / length) * (to.height / 2));
    const reach = Math.max(length - stop, length * 0.25) / length;
    return [
      { offset: 0, transform: translate(0, 0), easing: EASE_OUT },
      { offset: hit * 0.3, transform: `${translate((-dx / length) * WIND_UP_PX, (-dy / length) * WIND_UP_PX)} scale(1.08)`, easing: EASE_IN_OUT },
      { offset: hit, transform: `${translate(dx * reach, dy * reach)} scale(1.08)`, easing: EASE_OUT },
      { offset: 1, transform: translate(0, 0) },
    ];
  }

  // How far a minion moved between two boxes; null when it did not (under a pixel).
  function flipDelta(before, after) {
    const dx = before.x - after.x;
    const dy = before.y - after.y;
    return Math.abs(dx) < 1 && Math.abs(dy) < 1 ? null : { dx, dy };
  }

  function boxOf(el) {
    const r = el.getBoundingClientRect();
    return { x: r.left, y: r.top, width: r.width, height: r.height };
  }

  // Where each element is now: read before a step changes the boards.
  function measure(els) {
    const out = new Map();
    for (const el of els) if (el.isConnected) out.set(el, boxOf(el));
    return out;
  }

  // Stops the motion of this kind already running on `el`, so the new one starts from where it is.
  function restart(el, id) {
    el.getAnimations().forEach((a) => {
      if (a.id === id) a.cancel();
    });
  }

  const MOTIONS = ["replay-lunge", "replay-hit", "replay-slide"];

  // Stops every motion on `els`: a jump or a step back draws the step as it is.
  function still(els) {
    for (const el of els) {
      el.getAnimations().forEach((a) => {
        if (MOTIONS.includes(a.id)) a.cancel();
      });
      delete el.dataset.lunging;
    }
  }

  // The lunge running on each element: only its own end clears the mark.
  const lunging = new WeakMap();

  // `attacker` flies to `target` and back, drawn above its neighbours and the
  // other board meanwhile (`data-lunging`).
  // Where an element sits in the layout: its box less the translation a
  // running motion adds (a board still sliding into place after a death or
  // a summon), so an attack aims at where its target is (the website's layoutBox).
  function placeOf(el) {
    const b = boxOf(el);
    const t = getComputedStyle(el).transform;
    const m = t && t !== "none" ? new DOMMatrixReadOnly(t) : null;
    return { x: b.x - (m ? m.m41 : 0), y: b.y - (m ? m.m42 : 0), width: b.width, height: b.height };
  }

  function lunge(attacker, target, speed) {
    const from = placeOf(attacker);
    const to = placeOf(target);
    restart(attacker, "replay-lunge");
    restart(attacker, "replay-slide");
    attacker.dataset.lunging = "";
    const motion = attacker.animate(lungeFrames(from, to), { duration: ATTACK_MS / speed, id: "replay-lunge" });
    lunging.set(attacker, motion);
    const done = () => {
      if (lunging.get(attacker) === motion) delete attacker.dataset.lunging;
    };
    motion.addEventListener("finish", done);
    motion.addEventListener("cancel", done);
  }

  function hit(el, speed) {
    restart(el, "replay-hit");
    const shake = [0, -4, 4, -2, 0].map((x) => ({ transform: translate(x, 0) }));
    el.animate(shake, { duration: HIT_MS / speed, delay: IMPACT_MS / speed, easing: "ease-in-out", id: "replay-hit" });
  }

  function slide(before, els, speed) {
    els.forEach((el) => {
      const was = before.get(el);
      const delta = was ? flipDelta(was, boxOf(el)) : null;
      if (!delta) return;
      restart(el, "replay-slide");
      el.animate([{ transform: translate(delta.dx, delta.dy) }, { transform: translate(0, 0) }], { duration: SLIDE_MS / speed, easing: EASE_OUT, id: "replay-slide" });
    });
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

  // A damage in words: with its source since revision 13 ("from their
  // Cleave Hound (attack)"), or "an unknown source"; as before without one.
  function damageWords(e, m, heroText) {
    const base = { m: m(e.minion), n: I18n.num(e.amount), left: I18n.num(e.health) };
    const lethal = e.health <= 0;
    if (e.source === undefined) return t(lethal ? "replay.ev.damage_lethal" : "replay.ev.damage", base);
    const src = e.source;
    if (src === null || typeof src !== "object" || !CAUSES.includes(src.how)) {
      return t(lethal ? "replay.ev.damage_unknown_lethal" : "replay.ev.damage_unknown", base);
    }
    const cardOk = typeof src.card_id === "string" && /^[A-Za-z0-9_]{1,64}$/.test(src.card_id);
    if (!isInt(src.minion) && !cardOk) return t(lethal ? "replay.ev.damage_unknown_lethal" : "replay.ev.damage_unknown", base);
    const from = isInt(src.minion) ? m(src.minion) : t(`replay.ev.card_${src.side === "own" ? "own" : "opponent"}`, { name: heroText(src.card_id) });
    return t(lethal ? "replay.ev.damage_by_lethal" : "replay.ev.damage_by", { ...base, src: from, how: t(`replay.how.${src.how}`) });
  }

  // A frame's step in words: each of its events, in order; the start for none.
  function frameWords(f, minionName, heroText) {
    if (!f || !f.events || !f.events.length) return t("replay.start");
    return f.events.map((e) => stepWords(e, minionName, heroText)).join(". ");
  }

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
        return capital(damageWords(e, m, heroText));
      case "keyword":
        return capital(t(e.on ? "replay.ev.keyword_on" : "replay.ev.keyword_off", { m: m(e.minion), k: t(`keyword.${e.keyword}`) }));
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
  // The game whose recap was drawn last: playback resumes after a redraw of
  // the same recap, never by itself when coming back from another game.
  let lastGame = null;

  function remembered(game, shown) {
    const saved = memory.get(game);
    const same = game === lastGame;
    lastGame = game;
    const round = saved && shown.some((c) => c.round === saved.round) ? saved.round : shown[shown.length - 1].round;
    const kept = saved && saved.round === round;
    return { round, step: kept ? saved.step : 0, playing: Boolean(kept && same && saved.playing) };
  }

  function nameOf(replay, h) {
    const minions = list(replay.minions);
    return (ref) => {
      const m = minions[ref] || {};
      const name = typeof m.card_id === "string" ? h.cardName(m.card_id) || m.card_id : t("shop.card.unknown");
      return t(m.side === "own" ? "replay.your" : "replay.their", { name });
    };
  }

  // A minion as the game page draws it (D-131): an oval portrait, Attack in
  // a gold diamond and Health in a red drop, a gold ring when golden, a halo
  // while it has Divine Shield.
  function minionTile(ui, tile, h) {
    const box = ui.el("span", "replay-minion");
    // Each keyword's mark, shown by the stylesheet while the token has it.
    const keys = ui.el("span", "replay-keys");
    keys.setAttribute("aria-hidden", "true");
    MARKED.forEach((k) => keys.append(ui.el("i", `replay-k replay-k-${k}`)));
    box.append(keys);
    const frame = ui.el("span", "replay-frame");
    const img = h.art(tile.id, "replay-art");
    frame.append(img || ui.el("span", "replay-art art-missing"));
    box.append(frame);
    const stats = ui.el("span", "replay-stats");
    stats.setAttribute("aria-hidden", "true");
    stats.append(ui.el("span", "replay-atk"), ui.el("span", "replay-hp"));
    // The words a screen reader reads for the tile (the picture has none).
    // Dark Gifts (T-528): they do not change during a combat.
    const gifts = Turns.giftMarks(tile);
    if (gifts.length) {
      const line = ui.el("span", "replay-gifts");
      line.setAttribute("aria-hidden", "true");
      gifts.forEach((g) => line.append(Turns.giftMark(ui, g)));
      box.append(line);
    }
    box.append(stats, ui.el("span", "visually-hidden replay-words"));
    return box;
  }

  function updateTile(box, tile, h) {
    const flags = ["golden", "shield", "attacker", "defender", "dying", "summoned", "buffed", "popped", "triggered"];
    flags.forEach((f) => box.classList.toggle(f, Boolean(tile[f])));
    MARKED.forEach((k) => {
      box.classList.toggle(`has-${k}`, tile.keywords.includes(k));
      box.classList.toggle(`gains-${k}`, Boolean(tile.gained && tile.gained.includes(k)));
      box.classList.toggle(`loses-${k}`, Boolean(tile.lost && tile.lost.includes(k)));
    });
    // Damage past 0 (overkill) shows as 0, as on the game's board.
    const atk = tile.atk === null ? "?" : I18n.num(tile.atk);
    const health = tile.health === null ? "?" : I18n.num(Math.max(tile.health, 0));
    box.querySelector(".replay-atk").textContent = atk;
    box.querySelector(".replay-hp").textContent = health;
    box.querySelectorAll(".replay-hurt, .replay-gain").forEach((old) => old.remove());
    // The hit as the game's splat; a buff or a heal as a green one.
    const splat = (cls, text) => {
      const s = document.createElement("span");
      s.className = cls;
      s.setAttribute("aria-hidden", "true");
      const b = document.createElement("b");
      b.textContent = text;
      s.append(b);
      box.append(s);
    };
    if (tile.hurt) splat("replay-hurt", `−${I18n.num(tile.hurt)}`);
    else if (tile.gain) splat("replay-gain", `+${I18n.num(tile.gain.atk)}/+${I18n.num(tile.gain.health)}`);
    const name = tile.id ? h.cardName(tile.id) || tile.id : t("shop.card.unknown");
    const gifts = Turns.giftMarks(tile).map((g) => g.title);
    const marks = [tile.golden && t("keyword.golden"), ...tile.keywords.map((k) => (`keyword.${k}` in I18n.tables().en ? t(`keyword.${k}`) : k)), ...gifts].filter(Boolean);
    const words = [`${name} ${atk}/${health}`, marks.join(", ")].filter(Boolean).join(" · ");
    box.title = words;
    box.querySelector(".replay-words").textContent = words;
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

  // Theirs on top, yours below, as in the game.
  function buildStage(ui) {
    const stage = ui.el("div", "replay-stage");
    const sides = {};
    SIDES.slice().reverse().forEach((side) => {
      const row = ui.el("div", `replay-side ${side}`);
      const badge = heroBadge(ui);
      const board = ui.el("div", "replay-board");
      row.append(badge, board);
      sides[side] = { board, badge, tiles: new Map() };
      stage.append(row);
    });
    return { stage, sides };
  }

  // Draws frame `f` on the stage, reusing each minion's tile (no picture
  // loads twice). `heroes` are the round's heroes before any board comes in.
  // A board is rebuilt only when its minions change, so a running motion or
  // a mark's animation is not restarted. Returns the minions drawn.
  function drawStage(ui, sides, f, heroes, h) {
    const drawn = [];
    SIDES.forEach((side) => {
      const s = sides[side];
      const nodes = f[side].map((tile) => {
        if (!s.tiles.has(tile.ref)) s.tiles.set(tile.ref, minionTile(ui, tile, h));
        const node = s.tiles.get(tile.ref);
        updateTile(node, tile, h);
        return node;
      });
      const kids = s.board.children;
      const same = nodes.length === kids.length && nodes.every((n, i) => kids[i] === n);
      const stillEmpty = !nodes.length && kids.length === 1 && kids[0].classList.contains("replay-empty");
      if (!same && !stillEmpty) {
        s.board.replaceChildren(...nodes);
        if (!nodes.length) s.board.append(ui.el("span", "replay-empty", t("turns.board.empty")));
      }
      setHero(ui, s.badge, f.heroes[side] || heroes[side], h);
      s.badge.querySelector(".replay-hero-hit").textContent = f.hero[side] ? `−${I18n.num(f.hero[side])}` : "";
      drawn.push(...nodes);
    });
    return drawn;
  }

  // The step's motion, once the boards are drawn: the board sliding into
  // place, an attack, the winner's hero hitting the loser's.
  function move(sides, f, before, drawn, speed) {
    slide(before, drawn, speed);
    const e = f.event;
    const tileOfRef = (ref) => SIDES.map((side) => sides[side].tiles.get(ref)).find(Boolean);
    if (e && e.kind === "attack" && e.defender !== null) {
      const a = tileOfRef(e.attacker);
      const d = tileOfRef(e.defender);
      if (a && d && a.isConnected && d.isConnected) {
        lunge(a, d, speed);
        hit(d, speed);
      }
    } else if (e && e.kind === "hero_damage") {
      const face = (side) => sides[side].badge.querySelector(".replay-hero-face");
      lunge(face(e.side === "own" ? "opponent" : "own"), face(e.side), speed);
      hit(face(e.side), speed);
    }
  }

  // Play and pause for everyone; back, forward, the start and the speed in Advanced.
  function buildControls(ui) {
    const box = ui.el("div", "replay-controls");
    const c = {
      box,
      restart: button(ui, "replay-btn adv-only", "⏮", t("replay.restart")),
      back: button(ui, "replay-btn adv-only", "‹", t("replay.back")),
      play: button(ui, "replay-btn replay-play", "", null),
      forward: button(ui, "replay-btn adv-only", "›", t("replay.forward")),
      select: document.createElement("select"),
    };
    SPEEDS.forEach((s) => {
      const o = document.createElement("option");
      o.value = String(s);
      o.textContent = t("replay.speed_x", { n: I18n.num(s) });
      o.selected = s === speed;
      c.select.append(o);
    });
    const speedBox = ui.el("label", "replay-speed adv-only", `${t("replay.speed")} `);
    speedBox.append(c.select);
    box.append(c.restart, c.back, c.play, c.forward, speedBox);
    return c;
  }

  // Every step in words, numbered from 0 (the start) like the counter.
  function buildSteps(ui, all, words, h, round, go) {
    const fold = h.fold(ui, `${h.game}:replay-steps:${round}`, t("replay.steps", { n: I18n.num(all.length - 1) }), "replay-steps adv-only");
    const ol = ui.el("ol", "replay-list");
    ol.setAttribute("start", "0");
    const items = all.map((f, i) => {
      const li = ui.el("li");
      const b = button(ui, "replay-step", words(f), null);
      b.addEventListener("click", () => go(i, false));
      li.append(b);
      return li;
    });
    ol.append(...items);
    fold.append(ol);
    return { fold, items };
  }

  // The playback clock: `draw(how)` after every step, each step staying for
  // its beat; it stops by itself once the player has left the page. "move":
  // one step on, played; "still": a jump, a step back or a player just
  // built, drawn as it is; "keep": the same step (a pause), left as it is.
  function clock(state, last, box, beat, draw) {
    let timer = null;
    const later = () => setTimeout(tick, beat[state.step] / speed);
    function tick() {
      timer = null;
      if (!box.isConnected || !state.playing) return;
      if (state.step >= last) {
        // The end: the last step has played already.
        state.playing = false;
        draw("keep");
        return;
      }
      state.step += 1;
      draw("move");
      if (state.playing) timer = later();
    }
    return function go(step, playing, fresh = false) {
      if (timer) clearTimeout(timer);
      timer = null;
      const next = Math.min(Math.max(step, 0), last);
      const how = !fresh && next === state.step + 1 ? "move" : fresh || next !== state.step ? "still" : "keep";
      state.step = next;
      state.playing = playing;
      draw(how);
      if (playing) timer = later();
    };
  }

  // The player of one combat.
  function player(ui, combat, outcome, heroes, h, state) {
    const all = frames(combat.replay);
    const name = nameOf(combat.replay, h);
    const words = (f) => frameWords(f, name, (id) => h.heroText({ id }));
    const box = ui.el("div", "replay-player");
    const head = ui.el("div", "replay-head");
    const result = ui.el("span", `chip chip-${outcome.outcome}`, t(`outcome.${outcome.outcome}`));
    ui.addTip(result, outcome.source);
    const counter = ui.el("span", "replay-counter");
    head.append(result, ui.el("span", "replay-title", t("replay.combat_of", { turn: combat.round, who: outcome.who || t("turns.opponent_unknown") })), counter);
    const { stage, sides } = buildStage(ui);
    const caption = ui.el("p", "replay-caption");
    const c = buildControls(ui);
    const last = all.length - 1;
    let go = null;
    const steps = buildSteps(ui, all, words, h, combat.round, (i, p) => go(i, p));
    const reduced = () => window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    const movers = () => [...SIDES.flatMap((side) => [...sides[side].tiles.values()]), ...stage.querySelectorAll(".replay-hero-face")];
    const draw = (how) => {
      const moving = how === "move" && !reduced();
      if (how === "still" || (how === "move" && !moving)) still(movers());
      if (how !== "keep") stage.classList.toggle("still", !moving);
      // Measured before the boards change and once after, never in between.
      const before = moving ? measure(SIDES.flatMap((side) => [...sides[side].tiles.values()])) : null;
      const drawn = drawStage(ui, sides, all[state.step], heroes, h);
      if (before) move(sides, all[state.step], before, drawn, speed);
      caption.textContent = words(all[state.step]);
      // Each hit's tooltip says what dealt it, as the step's words do.
      const tips = new Map();
      all[state.step].events.forEach((e) => {
        if (e.kind === "damage") tips.set(e.minion, [...(tips.get(e.minion) || []), stepWords(e, name, (id) => h.heroText({ id }))]);
      });
      tips.forEach((lines, ref) => {
        const tile = SIDES.map((side) => sides[side].tiles.get(ref)).find(Boolean);
        const hurt = tile && tile.querySelector(".replay-hurt");
        if (hurt) hurt.title = lines.join(". ");
      });
      // Read aloud on a step or a pause only, not at every step of playback.
      caption.setAttribute("aria-live", state.playing ? "off" : "polite");
      counter.textContent = t("replay.step_of", { n: I18n.num(state.step), total: I18n.num(last) });
      c.play.textContent = state.playing ? "⏸" : "▶";
      c.play.setAttribute("aria-label", t(state.playing ? "replay.pause" : "replay.play"));
      c.play.title = c.play.getAttribute("aria-label");
      c.back.disabled = c.restart.disabled = state.step === 0;
      c.forward.disabled = state.step === last;
      steps.items.forEach((li, i) => li.classList.toggle("now", i === state.step));
      memory.set(h.game, { ...state });
    };
    go = clock(state, last, box, frameBeats(all), draw);
    // At the end, play starts over.
    c.play.addEventListener("click", () => (state.playing ? go(state.step, false) : go(state.step >= last ? 0 : state.step, true)));
    c.back.addEventListener("click", () => go(state.step - 1, false));
    c.forward.addEventListener("click", () => go(state.step + 1, false));
    c.restart.addEventListener("click", () => go(0, false));
    c.select.addEventListener("change", () => {
      speed = SPEEDS.includes(Number(c.select.value)) ? Number(c.select.value) : 1;
      if (state.playing) go(state.step, true);
    });
    box.append(head, stage, caption, c.box);
    if (!combat.replay.complete) box.append(ui.withSource(ui.el("p", "note", t("replay.cut")), "unknown"));
    if (all.skipped) box.append(ui.withSource(ui.el("p", "note", I18n.plural("replay.skipped", all.skipped)), "unknown"));
    box.append(steps.fold);
    go(state.step, state.playing, true);
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

  const api = { SPEEDS, BEAT_MS, IMPACT_MS, ATTACK_MS, combats, frames, steps, beats, groupBeat, frameBeats, lungeFrames, flipDelta, stepWords, frameWords, heroesOf, outcomeOf, section };
  if (typeof module !== "undefined" && module.exports) module.exports = api;
  else window.TLReplay = api;
})();
