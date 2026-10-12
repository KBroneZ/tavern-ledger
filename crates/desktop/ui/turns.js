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
  const I18n = typeof module !== "undefined" && module.exports ? require("./i18n.js") : window.TLI18n;
  const t = I18n.t;
  // The words of a result are outcome.<won|lost|tie|unknown> (common.js).
  const OUTCOMES = ["won", "lost", "tie", "unknown"];
  const outcomeWord = (outcome) => t(`outcome.${outcome}`);
  const known = (v) => v !== null && v !== undefined;
  const list = (v) => (Array.isArray(v) ? v : []);
  const num = (v) => (known(v) ? I18n.num(v) : "—");

  const minionOf = (m) => ({
    id: typeof m.card_id === "string" ? m.card_id : null,
    atk: known(m.atk) ? m.atk : null,
    health: known(m.health) ? m.health : null,
    golden: m.golden === true,
    keywords: Array.isArray(m.keywords) ? m.keywords.filter((k) => typeof k === "string") : null,
    gifts: list(m.gifts).filter((g) => typeof g === "string"),
    gift_unknown: m.gift_unknown === true,
  });

  // Dark Gift marks of a minion (T-528): cards.js draws the words; none
  // where it is not loaded (the tests).
  const giftMarks = (m) => {
    const cards = typeof window !== "undefined" ? window.TLCards : null;
    return cards && cards.gifts ? cards.gifts(m) : [];
  };

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
        outcome: OUTCOMES.includes(c.outcome) ? c.outcome : "unknown",
        source: c.source,
        reason: c.reason || null,
        opponents: list(o.heroes),
        board: seen ? list(seen.minions).map(minionOf) : null,
      };
    }
    const lone = list(r.unattributed).find((x) => x.round === round);
    if (!lone) return null;
    return { outcome: OUTCOMES.includes(lone.outcome) ? lone.outcome : "unknown", source: lone.source, reason: lone.reason || null, opponents: [], board: null };
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

  // The combat's hit as the game dealt it (T-540); null when the record lacks it.
  function damageAt(r, round) {
    const d = list(r.damage).find((x) => x.round === round);
    return d && Number.isInteger(d.taken) && Number.isInteger(d.dealt) ? { taken: d.taken, dealt: d.dealt } : null;
  }

  // "your hero took 7 damage", "you dealt 12 damage", "no damage"; null when unknown.
  function damageWords(d) {
    if (!d) return null;
    if (d.taken > 0) return t("turns.s.took", { n: num(d.taken) });
    if (d.dealt > 0) return t("turns.s.dealt", { n: num(d.dealt) });
    return t("turns.s.no_damage");
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
        damage: damageAt(r, turn),
        own: ownBoard(report, turn),
      };
    });
  }

  // "12/30", "?/30" when a stat is missing, "empty" for an empty board.
  function totalsWords(t) {
    if (!t.count) return I18n.t("turns.empty");
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
    const parts = [t("turns.s.turn", { turn: row.turn })];
    if (row.tier) parts.push(t("turns.s.tier", { tier: row.tier.value }), ...(row.tier.up ? [t("turns.s.up")] : []));
    if (row.combat) {
      const who = row.combat.opponents.map(heroText).join(" + ");
      const outcome = outcomeWord(row.combat.outcome).toLocaleLowerCase(I18n.locale());
      parts.push(who ? t("turns.s.vs", { outcome, who }) : outcome);
    } else parts.push(t("turns.s.no_combat"));
    parts.push(t("turns.s.health", { change: healthWords(row.health) }));
    const hit = damageWords(row.damage);
    if (hit) parts.push(hit);
    if (row.own) parts.push(I18n.plural("turns.s.board", row.own.totals.count));
    return parts.join(", ");
  }

  function shopFacts(shop) {
    if (!shop || !shop.in_log) return [];
    return [
      { label: t("turns.fact.gold"), value: num(shop.gold) },
      { label: t("turns.fact.spent"), value: num(shop.gold_spent) },
      { label: t("turns.fact.rolls"), value: shop.free_rolls ? t("turns.rolls_free", { rolls: num(shop.rolls), free: num(shop.free_rolls) }) : num(shop.rolls) },
      { label: t("turns.fact.buys"), value: num(list(shop.buys).length + (shop.spell_buys || 0)) },
      { label: t("turns.fact.sells"), value: num(list(shop.sells).length) },
      { label: t("turns.fact.apm"), value: known(shop.apm) ? I18n.num(shop.apm, 1) : "—" },
    ];
  }

  // --- Drawing (the page only).
  // `ui` is the window's el/withSource/sourceMark/addTip; `hero` draws a hero
  // (portrait + name); `heroText` words one; `game` keys the open turns.

  // The short and full word of a keyword mark: keyword.<k> (common.js); only
  // the Divine Shield mark is shorter than its name.
  function keywordWords(k) {
    const full = `keyword.${k}` in I18n.tables().en ? t(`keyword.${k}`) : k;
    return [k === "divine_shield" ? t("turns.kw.shield") : full, full];
  }

  function tier(ui, level) {
    if (!level) return ui.el("span", "turn-tier none", "");
    const node = ui.el("span", "turn-tier", t("turns.tier", { tier: level.value }));
    if (level.up) {
      const up = ui.el("span", "tier-up", "▲");
      up.title = t("turns.tier_up_tip");
      node.append(up);
    }
    ui.addTip(node, "log");
    return node;
  }

  function result(ui, c, heroText, art) {
    const box = ui.el("span", "turn-combat");
    if (!c) {
      box.append(ui.el("span", "chip chip-unknown", t("turns.no_combat")));
      ui.addTip(box, "unknown");
      return box;
    }
    const chip = ui.el("span", `chip chip-${c.outcome} src-${["log", "inferred"].includes(c.source) ? c.source : "unknown"}`, outcomeWord(c.outcome));
    chip.title = c.reason ? t("turns.unknown_reason", { reason: I18n.backend(c.reason) }) : outcomeWord(c.outcome);
    ui.addTip(chip, c.source);
    box.append(chip);
    const who = ui.el("span", "turn-opponent", c.opponents.map(heroText).join(" + ") || t("turns.opponent_unknown"));
    who.prepend(...c.opponents.map((h) => art(h.id, "portrait")).filter(Boolean));
    box.append(who);
    return box;
  }

  function health(ui, h, source, damage) {
    const words = healthWords(h);
    const kind = !known(h.change) ? "unknown" : h.change < 0 ? "hurt" : h.change > 0 ? "healed" : "even";
    const node = ui.el("span", `turn-health ${kind}`, words);
    node.title = known(h.after) ? t("turns.health_tip", { before: num(h.before), after: num(h.after) }) : t("turns.health_none");
    const hit = damageWords(damage);
    if (hit) node.title += ` ${t("turns.damage_tip", { hit })}`;
    ui.addTip(node, known(h.change) ? source : "unknown");
    return node;
  }

  // Your board at a glance: small pictures, then the minion count and total stats.
  function strip(ui, own, art) {
    const box = ui.el("span", "turn-strip");
    if (!own) {
      box.append(ui.el("span", "turn-strip-none", t("turns.strip_none")));
      return box;
    }
    const faces = ui.el("span", "turn-faces");
    own.minions.slice(0, 7).forEach((m) => {
      const gifted = giftMarks(m).length > 0;
      const img = art(m.id, `face${m.golden ? " golden" : ""}${gifted ? " gifted" : ""}`);
      if (img) faces.append(img);
    });
    const totals = own.totals;
    box.append(faces, ui.el("span", "turn-totals", totalsWords(totals)));
    box.title = totals.count
      ? t("turns.strip_tip", { minions: I18n.plural("turns.minions", totals.count), attack: num(totals.attack), health: num(totals.health) })
      : t("turns.board.empty");
    return box;
  }

  // One Dark Gift: a small mark, its name and summary as the tooltip and for
  // screen readers.
  function giftMark(ui, g) {
    const mark = ui.el("span", g.id ? "kw gift-mark" : "kw gift-mark is-unknown", g.id ? "✦" : "✦?");
    mark.title = g.title;
    mark.setAttribute("aria-label", g.title);
    return mark;
  }

  function minionTile(ui, m, card) {
    const tile = card(ui, m.id, m.golden ? "golden" : "");
    tile.append(ui.el("span", "minion-stats", `${num(m.atk)}/${num(m.health)}`));
    const marks = [];
    if (m.golden) marks.push([t("keyword.golden"), t("keyword.golden")]);
    (m.keywords || []).forEach((k) => marks.push(keywordWords(k)));
    const gifts = giftMarks(m);
    if (marks.length || gifts.length) {
      const line = ui.el("span", "tile-marks");
      marks.forEach(([short, full]) => {
        const mark = ui.el("span", "kw", short);
        mark.title = full;
        line.append(mark);
      });
      gifts.forEach((g) => line.append(giftMark(ui, g)));
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
    if (!minions.length) cards.append(ui.el("span", "empty", t("turns.board.empty")));
    row.append(cards);
    if (minions.length && !keywordsRecorded(minions)) {
      row.append(ui.withSource(ui.el("span", "note", t("turns.kw_missing")), "unknown"));
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

  function turnNotes(shop) {
    return [
      known(shop.seconds) && t("turns.note.seconds", { n: I18n.num(Math.round(shop.seconds)) }),
      shop.spell_buys && I18n.plural("turns.note.spells", shop.spell_buys),
      shop.extra_gold && t("turns.note.extra_gold", { n: I18n.num(shop.extra_gold) }),
      shop.sell_gold && t("turns.note.sell_gold", { n: I18n.num(shop.sell_gold) }),
      shop.free_refreshes && I18n.plural("turns.note.free", shop.free_refreshes),
    ].filter(Boolean);
  }

  function turnBody(ui, row, h) {
    const body = [];
    const own = row.own;
    const ownLabel = own && !own.mine && own.hero ? h.heroText({ id: own.hero }) : t("turns.your_board");
    body.push(boardRow(ui, ownLabel, own && own.minions, h.card, t("turns.board.not_in_round")));
    if (row.combat) {
      const who = row.combat.opponents.map(h.heroText).join(" + ") || t("turns.their_board");
      body.push(boardRow(ui, who, row.combat.board, h.card, t("turns.board.theirs_not")));
    }
    const shop = row.shop;
    if (!shop || !shop.in_log) {
      // Said once for the whole game when it has no shop record at all.
      if (h.shopRecorded) body.push(ui.withSource(ui.el("p", "note", t("turns.no_shop")), "unknown"));
      return body;
    }
    body.push(shopFactList(ui, shop));
    if (list(shop.buys).length) body.push(h.cardRow(ui, t("turns.bought"), shop.buys, "bought"));
    if (list(shop.sells).length) body.push(h.cardRow(ui, t("turns.sold"), shop.sells, "sold"));
    if (list(shop.passes).length) body.push(h.cardRow(ui, t("turns.passed"), shop.passes, "passed"));
    const notes = turnNotes(shop);
    if (notes.length) body.push(ui.el("p", "note", `${notes.join(" · ")}.`));
    const shops = list(shop.shops);
    if (!shops.length) {
      body.push(ui.withSource(ui.el("p", "note", t("turns.no_offers")), "unknown"));
      return body;
    }
    const see = h.fold(ui, `${h.game}:shops:${row.turn}`, t("turns.see_shops", { n: shops.length }), "see-shops");
    see.append(...shops.map((s) => h.cardRow(ui, s.roll === 0 ? t("turns.shop") : t("turns.roll", { n: s.roll }), s.offers)));
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
      health(ui, row.health, healthSource, row.damage),
      strip(ui, row.own, h.art),
    );
    const label = ui.el("span", "visually-hidden", words);
    const box = h.fold(ui, `${h.game}:turn:${row.turn}`, [label, head], "turn");
    return box;
  }

  function section(ui, r, report, h) {
    const part = ui.el("section", "recap-part turns");
    const heading = ui.el("h3", null, t("turns.title"));
    part.append(heading);
    const all = rows(r, report);
    if (!all.length) {
      part.append(ui.withSource(ui.el("p", "empty", t("turns.none")), "unknown"));
      return part;
    }
    const key = ui.el("p", "note turn-key");
    key.append(`${t("turns.key")} `);
    const mark = ui.sourceMark("inferred");
    if (mark) key.append(mark);
    key.append(".");
    part.append(key);
    const shopRecorded = Boolean(r.shop && r.shop.state === "recorded");
    if (!shopRecorded) {
      part.append(ui.withSource(ui.el("p", "note", t("turns.no_shop_game")), "unknown"));
    }
    // Column names over the rows; each row's summary also says it all in words.
    const cols = ui.el("div", "turn-cols");
    cols.setAttribute("aria-hidden", "true");
    cols.append(...["turns.col.turn", "turns.col.tier", "turns.col.combat", "turns.col.health", "turns.col.board"].map((k) => ui.el("span", null, t(k))));
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

  const api = { giftMark, giftMarks, rows, combatAt, healthWords, damageWords, totalsWords, keywordsRecorded, summaryWords, shopFacts, section };
  if (typeof module !== "undefined" && module.exports) module.exports = api;
  else window.TLTurns = api;
})();
