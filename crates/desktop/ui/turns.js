// Turns in the recap (T-321): one compact, visual row per turn (round, tavern
// tier, the combat's result and opponent, the health it cost, your board at a
// glance). Opening a turn shows both boards, the turn's shop numbers and what
// you bought and sold; the shops themselves wait behind "See shops". The app
// works out every value and source (tracker::recap, tracker::shop, the
// game's report); this file joins them by turn and draws them. A combat
// follows the shop turn of the same number (D-060). `rows`, `healthWords`,
// `summaryWords` and `shopFacts` are plain data in, plain data out
// (ui-tests/turns.test.js).
"use strict";

(() => {
  const OUTCOME = { won: "Won", lost: "Lost", tie: "Tie", unknown: "Unknown" };
  const known = (v) => v !== null && v !== undefined;
  const list = (v) => (Array.isArray(v) ? v : []);
  const plural = (n, word) => `${n} ${word}${n === 1 ? "" : "s"}`;
  const num = (v) => (known(v) ? String(v) : "—");

  const minionOf = (m) => ({
    id: typeof m.card_id === "string" ? m.card_id : null,
    atk: known(m.atk) ? m.atk : null,
    health: known(m.health) ? m.health : null,
    golden: m.golden === true,
    keywords: Array.isArray(m.keywords) ? m.keywords.filter((k) => typeof k === "string") : null,
  });

  // Your board as the combat began, from the game's report. In Duos the
  // board may be your teammate's: `mine` says whose it is.
  function ownBoard(report, number) {
    const round = list(report && report.rounds).find((r) => r && r.number === number);
    const own = list(round && round.entries).filter((e) => e && e.side === "own");
    if (!own.length) return null;
    const entry = own.find((e) => e.player_id === report.local_player_id) || own[0];
    // No board in the record is unknown; an empty list is a real empty board.
    if (!Array.isArray(entry.board)) return null;
    const minions = entry.board.map(minionOf);
    // One unknown stat makes the total unknown, never a smaller number.
    const sum = (key) => (minions.every((m) => known(m[key])) ? minions.reduce((n, m) => n + m[key], 0) : null);
    return {
      hero: typeof entry.hero === "string" ? entry.hero : null,
      mine: entry.player_id === report.local_player_id || entry.hero === report.hero,
      minions,
      totals: { count: minions.length, attack: sum("atk"), health: sum("health") },
    };
  }

  function combatAt(r, round) {
    for (const o of list(r.record)) {
      const c = list(o.combats).find((x) => x.round === round);
      if (!c) continue;
      const seen = list(o.boards).find((b) => b.round === round);
      return {
        outcome: OUTCOME[c.outcome] ? c.outcome : "unknown",
        source: c.source,
        reason: c.reason || null,
        opponents: list(o.heroes),
        board: seen ? list(seen.minions).map(minionOf) : null,
      };
    }
    const lone = list(r.unattributed).find((x) => x.round === round);
    if (!lone) return null;
    return { outcome: OUTCOME[lone.outcome] ? lone.outcome : "unknown", source: lone.source, reason: lone.reason || null, opponents: [], board: null };
  }

  function healthAt(r, round) {
    const at = (n) => {
      const p = list(r.health).find((x) => x.round === n);
      return p && Number.isInteger(p.health) ? p.health : null;
    };
    const before = at(round - 1);
    const after = at(round);
    return { before, after, change: known(before) && known(after) ? after - before : null };
  }

  // One row per turn that has a shop, a combat or a board in the record.
  function rows(r, report) {
    const shopTurns = r.shop && r.shop.state === "recorded" ? list(r.shop.turns) : [];
    const numbers = new Set();
    shopTurns.forEach((t) => numbers.add(t.turn));
    list(r.record).forEach((o) => list(o.combats).forEach((c) => numbers.add(c.round)));
    list(r.unattributed).forEach((c) => numbers.add(c.round));
    list(report && report.rounds).forEach((x) => x && Number.isInteger(x.number) && numbers.add(x.number));
    return [...numbers].filter((n) => Number.isInteger(n) && n > 0).sort((a, b) => a - b).map((turn) => {
      const shop = shopTurns.find((t) => t.turn === turn) || null;
      return {
        turn,
        shop,
        tier: shop && shop.in_log && known(shop.tier) ? { value: shop.tier, up: shop.tier_up === true } : null,
        combat: combatAt(r, turn),
        health: healthAt(r, turn),
        own: ownBoard(report, turn),
      };
    });
  }

  // "12/30", "?/30" when a stat is missing, "empty" for an empty board.
  function totalsWords(t) {
    if (!t.count) return "empty";
    return `${num(t.attack).replace("—", "?")}/${num(t.health).replace("—", "?")}`;
  }

  // False when the record is older than keywords (parser revision 5).
  const keywordsRecorded = (minions) => list(minions).every((m) => Array.isArray(m.keywords));

  // "−5", "±0", "+3" or "?" (a true minus sign, read as "minus").
  function healthWords(h) {
    if (!h || !known(h.change)) return "?";
    if (h.change === 0) return "±0";
    return h.change > 0 ? `+${h.change}` : `−${-h.change}`;
  }

  function summaryWords(row, heroText) {
    const parts = [`Turn ${row.turn}`];
    if (row.tier) parts.push(`tier ${row.tier.value}`, ...(row.tier.up ? ["up a tier"] : []));
    if (row.combat) {
      const who = row.combat.opponents.map(heroText).join(" + ");
      parts.push(`${OUTCOME[row.combat.outcome].toLowerCase()}${who ? ` against ${who}` : ""}`);
    } else parts.push("no combat in the log");
    parts.push(`health ${healthWords(row.health)}`);
    if (row.own) parts.push(`your board ${plural(row.own.totals.count, "minion")}`);
    return parts.join(", ");
  }

  function shopFacts(t) {
    if (!t || !t.in_log) return [];
    return [
      { label: "Gold", value: num(t.gold) },
      { label: "Spent", value: num(t.gold_spent) },
      { label: "Rolls", value: `${num(t.rolls)}${t.free_rolls ? ` (${t.free_rolls} free)` : ""}` },
      { label: "Buys", value: String(list(t.buys).length + (t.spell_buys || 0)) },
      { label: "Sells", value: String(list(t.sells).length) },
      { label: "APM", value: known(t.apm) ? t.apm.toFixed(1) : "—" },
    ];
  }

  // --- Drawing (the page only).
  // `ui` is the window's el/withSource/sourceMark/addTip; `hero` draws a hero
  // (portrait + name); `heroText` words one; `game` keys the open turns.

  const KEYWORD_WORDS = {
    taunt: ["Taunt", "Taunt"], divine_shield: ["Shield", "Divine Shield"], reborn: ["Reborn", "Reborn"],
    venomous: ["Venomous", "Venomous"], windfury: ["Windfury", "Windfury"], stealth: ["Stealth", "Stealth"],
    deathrattle: ["Deathrattle", "Deathrattle"], rally: ["Rally", "Rally"],
  };

  function tier(ui, t) {
    if (!t) return ui.el("span", "turn-tier none", "");
    const node = ui.el("span", "turn-tier", `Tier ${t.value}`);
    if (t.up) {
      const up = ui.el("span", "tier-up", "▲");
      up.title = "Up a tier this turn";
      node.append(up);
    }
    ui.addTip(node, "log");
    return node;
  }

  function result(ui, c, heroText, art) {
    const box = ui.el("span", "turn-combat");
    if (!c) {
      box.append(ui.el("span", "chip chip-unknown", "No combat"));
      ui.addTip(box, "unknown");
      return box;
    }
    const chip = ui.el("span", `chip chip-${c.outcome} src-${["log", "inferred"].includes(c.source) ? c.source : "unknown"}`, OUTCOME[c.outcome]);
    chip.title = c.reason ? `Unknown: ${c.reason}.` : OUTCOME[c.outcome];
    ui.addTip(chip, c.source);
    box.append(chip);
    const who = ui.el("span", "turn-opponent", c.opponents.map(heroText).join(" + ") || "opponent unknown");
    who.prepend(...c.opponents.map((h) => art(h.id, "portrait")).filter(Boolean));
    box.append(who);
    return box;
  }

  function health(ui, h, source) {
    const words = healthWords(h);
    const kind = !known(h.change) ? "unknown" : h.change < 0 ? "hurt" : h.change > 0 ? "healed" : "even";
    const node = ui.el("span", `turn-health ${kind}`, words);
    node.title = known(h.after) ? `Health ${num(h.before)} → ${h.after}.` : "The log has no health for this round.";
    ui.addTip(node, known(h.change) ? source : "unknown");
    return node;
  }

  // Your board at a glance: small pictures, then the minion count and total stats.
  function strip(ui, own, art) {
    const box = ui.el("span", "turn-strip");
    if (!own) {
      box.append(ui.el("span", "turn-strip-none", "board not in the log"));
      return box;
    }
    const faces = ui.el("span", "turn-faces");
    own.minions.slice(0, 7).forEach((m) => {
      const img = art(m.id, m.golden ? "face golden" : "face");
      if (img) faces.append(img);
    });
    const t = own.totals;
    box.append(faces, ui.el("span", "turn-totals", totalsWords(t)));
    box.title = t.count ? `${plural(t.count, "minion")}, ${num(t.attack)} attack and ${num(t.health)} health in all.` : "Empty board.";
    return box;
  }

  function minionTile(ui, m, card) {
    const tile = card(ui, m.id, m.golden ? "golden" : "");
    tile.append(ui.el("span", "minion-stats", `${num(m.atk)}/${num(m.health)}`));
    const marks = [];
    if (m.golden) marks.push(["Golden", "Golden"]);
    (m.keywords || []).forEach((k) => marks.push(KEYWORD_WORDS[k] || [k, k]));
    if (marks.length) {
      const line = ui.el("span", "tile-marks");
      marks.forEach(([short, full]) => {
        const mark = ui.el("span", "kw", short);
        mark.title = full;
        line.append(mark);
      });
      tile.append(line);
    }
    return tile;
  }

  function boardRow(ui, label, minions, card, empty) {
    const row = ui.el("div", "shop-row");
    row.append(ui.el("span", "shop-row-label", label));
    if (!minions) {
      row.append(ui.withSource(ui.el("span", "empty", empty), "unknown"));
      return row;
    }
    const cards = ui.el("div", "shop-cards");
    cards.append(...minions.map((m) => minionTile(ui, m, card)));
    if (!minions.length) cards.append(ui.el("span", "empty", "Empty board."));
    row.append(cards);
    if (minions.length && !keywordsRecorded(minions)) {
      row.append(ui.withSource(ui.el("span", "note", "Keywords not recorded for this game."), "unknown"));
    }
    return row;
  }

  function shopFactList(ui, t) {
    const dl = ui.el("dl", "turn-facts");
    dl.append(...shopFacts(t).map((f) => {
      const item = ui.el("div");
      const dd = ui.el("dd", null, f.value);
      ui.addTip(dd, f.value === "—" ? "unknown" : "log");
      item.append(ui.el("dt", null, f.label), dd);
      return item;
    }));
    return dl;
  }

  function turnNotes(t) {
    return [
      known(t.seconds) && `${Math.round(t.seconds)} s from this shop to the next`,
      t.spell_buys && `${plural(t.spell_buys, "tavern spell")} bought`,
      t.extra_gold && `${t.extra_gold} extra gold`,
      t.sell_gold && `${t.sell_gold} gold from selling`,
      t.free_refreshes && `${plural(t.free_refreshes, "free roll")} the game gave`,
    ].filter(Boolean);
  }

  function turnBody(ui, row, h) {
    const body = [];
    const own = row.own;
    const ownLabel = own && !own.mine && own.hero ? h.heroText({ id: own.hero }) : "Your board";
    body.push(boardRow(ui, ownLabel, own && own.minions, h.card, "Not in the log for this round."));
    if (row.combat) {
      const who = row.combat.opponents.map(h.heroText).join(" + ") || "Their board";
      body.push(boardRow(ui, who, row.combat.board, h.card, "Their board is not in the log for this round."));
    }
    const t = row.shop;
    if (!t || !t.in_log) {
      // Said once for the whole game when it has no shop record at all.
      if (h.shopRecorded) body.push(ui.withSource(ui.el("p", "note", "No shop record for this turn."), "unknown"));
      return body;
    }
    body.push(shopFactList(ui, t));
    if (list(t.buys).length) body.push(h.cardRow(ui, "Bought", t.buys, "bought"));
    if (list(t.sells).length) body.push(h.cardRow(ui, "Sold", t.sells, "sold"));
    if (list(t.passes).length) body.push(h.cardRow(ui, "Passed", t.passes, "passed"));
    const notes = turnNotes(t);
    if (notes.length) body.push(ui.el("p", "note", `${notes.join(" · ")}.`));
    const shops = list(t.shops);
    if (!shops.length) {
      body.push(ui.withSource(ui.el("p", "note", "No shop offers in the log for this turn."), "unknown"));
      return body;
    }
    const see = h.fold(ui, `${h.game}:shops:${row.turn}`, `See shops (${shops.length})`, "see-shops");
    see.append(...shops.map((s) => h.cardRow(ui, s.roll === 0 ? "Shop" : `Roll ${s.roll}`, s.offers)));
    body.push(see);
    return body;
  }

  function turnRow(ui, row, h, healthSource) {
    const words = summaryWords(row, h.heroText);
    const head = ui.el("span", "turn-head");
    head.setAttribute("aria-hidden", "true");
    head.append(
      ui.el("span", "turn-no", row.turn),
      tier(ui, row.tier),
      result(ui, row.combat, h.heroText, h.art),
      health(ui, row.health, healthSource),
      strip(ui, row.own, h.art),
    );
    const label = ui.el("span", "visually-hidden", words);
    const box = h.fold(ui, `${h.game}:turn:${row.turn}`, [label, head], "turn");
    return box;
  }

  function section(ui, r, report, h) {
    const part = ui.el("section", "recap-part turns");
    const heading = ui.el("h3", null, "Turns");
    part.append(heading);
    const all = rows(r, report);
    if (!all.length) {
      part.append(ui.withSource(ui.el("p", "empty", "No turn in the log."), "unknown"));
      return part;
    }
    const key = ui.el("p", "note turn-key");
    key.append("Open a turn for both boards, its shop and what you bought. Health change over each combat ");
    const mark = ui.sourceMark("inferred");
    if (mark) key.append(mark);
    key.append(".");
    part.append(key);
    const shopRecorded = Boolean(r.shop && r.shop.state === "recorded");
    if (!shopRecorded) {
      part.append(ui.withSource(ui.el("p", "note", "This game has no shop record: turns show only combats and boards."), "unknown"));
    }
    // Column names over the rows; each row's summary also says it all in words.
    const cols = ui.el("div", "turn-cols");
    cols.setAttribute("aria-hidden", "true");
    cols.append(...["Turn", "Tier", "Combat", "Health", "Your board"].map((w) => ui.el("span", null, w)));
    part.append(cols);
    const listNode = ui.el("div", "turn-list");
    const tools = { ...h, shopRecorded };
    listNode.append(...all.map((row) => {
      const box = turnRow(ui, row, tools, "inferred");
      box.append(...turnBody(ui, row, tools));
      return box;
    }));
    part.append(listNode);
    return part;
  }

  const api = { rows, healthWords, totalsWords, keywordsRecorded, summaryWords, shopFacts, section };
  if (typeof module !== "undefined" && module.exports) module.exports = api;
  else window.TLTurns = api;
})();
