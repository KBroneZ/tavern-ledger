// Shop and logged actions per minute (T-204, T-205, D-046) in the recap and
// the stats. The app works out every number and source (tracker::shop); this
// file only draws them. Values are set with textContent only. Card pictures
// come from cards.js, looked up by the log's card id.
"use strict";

window.TLShop = (() => {
  const KIND_WORDS = {
    roll: "rolls", buy: "buys", buy_spell: "spell buys", sell: "sells", freeze: "freezes",
    unfreeze: "unfreezes", tier_up: "tier-ups", hero_power: "hero powers", play: "cards played",
    move: "minions moved", choose: "picks", other: "other",
  };
  const NOT_RECORDED = "Not recorded: this game was saved by an older version of the app. "
    + "Run tavern-watch --reparse while its log is still on disk to add it.";
  const NOT_AVAILABLE = "No shop record: the log of this game could not be read.";

  // Card labels drawn before the card data answered; named when it does.
  let unnamed = [];
  window.TLCards.onChange(() => {
    unnamed = unnamed.filter(({ box, label, id, suffix, ui }) => {
      if (!box.isConnected) return false;
      const name = window.TLCards.name(id);
      if (name === undefined) return true;
      label.textContent = name || id;
      box.title = `${name || id}${suffix}`;
      ui.addTip(box, "card_data");
      return false;
    });
  });

  const one = (v, digits = 1) => (v === null || v === undefined ? "—" : v.toFixed(digits));
  const num = (v) => (v === null || v === undefined ? "—" : String(v));
  const plural = (n, word) => `${n} ${word}${n === 1 ? "" : "s"}`;

  // A card picture with its name (or id) as tooltip; the id as text when there is no picture.
  function card(ui, id, extraClass) {
    const box = ui.el("span", `shop-card ${extraClass || ""}`.trim());
    const name = window.TLCards.name(id);
    const suffix = extraClass === "frozen" ? " (frozen from last turn)" : "";
    box.title = `${name || id || "Unknown card"}${suffix}`;
    const img = window.TLCards.art(id, "shop-art");
    if (img) {
      img.title = "Picture: card database. © Blizzard Entertainment.";
      box.append(img);
    }
    const label = ui.el("span", "shop-card-id", name || id || "?");
    box.append(label);
    if (name === undefined && id) unnamed.push({ box, label, id, suffix, ui });
    ui.addTip(box, id ? "card_data" : "unknown");
    return box;
  }

  function cardRow(ui, label, ids, cls) {
    const row = ui.el("div", "shop-row");
    row.append(ui.el("span", "shop-row-label", label));
    const cards = ui.el("div", "shop-cards");
    // Bought and sold cards are ids (or null when the log has none); offers are objects.
    const isOffer = (c) => c !== null && typeof c === "object";
    cards.append(...ids.map((c) => card(ui, isOffer(c) ? c.card_id : c, cls || (isOffer(c) && c.frozen ? "frozen" : ""))));
    row.append(cards);
    return row;
  }

  function turnSummary(ui, t) {
    if (!t.in_log) return `Turn ${t.turn} · no data in the log`;
    const parts = [
      `Turn ${t.turn}`,
      `tier ${num(t.tier)}${t.tier_up ? " ▲" : ""}`,
      `gold ${num(t.gold)}`,
      `spent ${num(t.gold_spent)}`,
      `${plural(t.rolls, "roll")}${t.free_rolls ? ` (${t.free_rolls} free)` : ""}`,
      plural(t.buys.length + t.spell_buys, "buy"),
      plural(t.sells.length, "sell"),
      t.freezes ? plural(t.freezes, "freeze") : null,
      `${plural(t.actions, "action")}`,
      `${one(t.apm)} APM`,
    ];
    return parts.filter(Boolean).join(" · ");
  }

  function turnBlock(ui, t) {
    const item = ui.el("details", t.in_log ? "shop-turn" : "shop-turn gap");
    const summary = ui.el("summary", null, turnSummary(ui, t));
    ui.addTip(summary, t.in_log ? "log" : "unknown");
    item.append(summary);
    if (!t.in_log) return item;
    if (t.seconds !== null) item.append(ui.el("p", "note", `${Math.round(t.seconds)} s from this shop to the next.`));
    if (!t.shops.length) item.append(ui.withSource(ui.el("p", "empty", "No shop offers in the log for this turn."), "unknown"));
    t.shops.forEach((s) => item.append(cardRow(ui, s.roll === 0 ? "Shop" : `Roll ${s.roll}`, s.offers)));
    if (t.buys.length) item.append(cardRow(ui, "Bought", t.buys, "bought"));
    if (t.spell_buys) item.append(ui.el("p", "note", `${plural(t.spell_buys, "tavern spell")} bought.`));
    if (t.sells.length) item.append(cardRow(ui, "Sold", t.sells, "sold"));
    return item;
  }

  function fact(ui, label, value, tip) {
    const item = ui.el("div", "fact");
    const dd = ui.el("dd", null, value);
    if (tip) dd.title = tip;
    ui.addTip(dd, value === "—" ? "unknown" : "log");
    item.append(ui.el("dt", null, label), dd);
    return item;
  }

  function recapSection(shop, ui) {
    const section = ui.el("section", "recap-part shop");
    const heading = ui.el("h3", null, "Shop and actions");
    const mark = ui.sourceMark(shop.source);
    if (mark) heading.append(mark);
    section.append(heading);
    if (shop.state !== "recorded") {
      section.append(ui.withSource(ui.el("p", "empty", shop.state === "not_recorded" ? NOT_RECORDED : NOT_AVAILABLE), "unknown"));
      return section;
    }
    const t = shop.totals;
    const facts = ui.el("dl", "facts");
    facts.append(
      fact(ui, "APM (logged)", one(shop.apm), shop.apm_definition),
      fact(ui, "Actions", shop.apm === null && t.actions === 0 ? "—" : num(t.actions), shop.action_kinds.map(([k, n]) => `${n} ${KIND_WORDS[k] || k}`).join(", ")),
      fact(ui, "Rolls", `${t.rolls}${t.free_rolls ? ` (${t.free_rolls} free)` : ""}`),
      fact(ui, "Buys · sells", `${t.buys + t.spell_buys} · ${t.sells}`),
      fact(ui, "Gold spent", num(t.gold_spent)),
      fact(ui, "Minutes", one(shop.minutes)),
    );
    section.append(facts);
    section.append(ui.el("p", "note", shop.apm_definition));
    if (shop.ended === false) {
      section.append(ui.withSource(ui.el("p", "note", "The log stops before the game ends: the last turn may be cut short."), "unknown"));
    }
    const ups = shop.tier_ups.map((u) => `tier ${u.tier} on turn ${u.turn}`).join(" · ");
    section.append(ui.el("p", "note", ups ? `Tavern tier-ups: ${ups}.` : "No tavern tier-up in the log."));
    section.append(ui.el("p", "note", "Open a turn to see its shops. Frozen: kept from the last turn. Only your own shop is in the log."));
    const turns = ui.el("div", "shop-turns");
    turns.append(...shop.turns.map((x) => turnBlock(ui, x)));
    section.append(shop.turns.length ? turns : ui.el("p", "empty", "No shop turn in the log."));
    return section;
  }

  function statCell(ui, value, source, tip) {
    const td = ui.el("td", "num", value);
    if (tip) td.title = tip;
    ui.addTip(td, source);
    return td;
  }

  function heroRow(ui, h) {
    const tr = ui.el("tr");
    const s = h.shop;
    tr.append(
      ui.el("td", h.name ? "hero-name" : "hero", h.name || h.hero || "Unknown hero"),
      statCell(ui, s.games || "—", s.source),
      statCell(ui, one(s.rolls), s.source),
      statCell(ui, one(s.buys), s.source),
      statCell(ui, one(s.gold_spent), s.source),
      statCell(ui, one(s.apm), s.source),
    );
    return tr;
  }

  function renderStats(m, ui) {
    const s = m.shop;
    const box = document.getElementById("shop-stats-body");
    if (!box || !s) return;
    const parts = [];
    const facts = ui.el("dl", "facts");
    facts.append(
      fact(ui, "Games counted", s.games || "—"),
      fact(ui, "APM (logged)", one(s.apm), s.apm_definition),
      fact(ui, "Rolls / game", one(s.rolls)),
      fact(ui, "Free rolls / game", one(s.free_rolls)),
      fact(ui, "Buys / game", one(s.buys)),
      fact(ui, "Sells / game", one(s.sells)),
      fact(ui, "Gold spent / game", one(s.gold_spent)),
    );
    parts.push(facts, ui.el("p", "note", s.apm_definition));
    const notes = [
      s.not_recorded && `${s.not_recorded} saved by an older version (run tavern-watch --reparse while their logs are on disk)`,
      s.not_counted && `${s.not_counted} with the log cut short, no shop turn or an unreadable shop record`,
    ].filter(Boolean);
    if (notes.length) parts.push(ui.el("p", "note", `Not counted: ${notes.join("; ")}.`));
    if (s.tier_turns.length) {
      const list = ui.el("ul", "tribe-list");
      list.append(...s.tier_turns.map((t) => {
        const li = ui.el("li", null, `Tier ${t.tier}: turn ${one(t.average_turn)}`);
        li.title = `Average turn you reached tier ${t.tier}, over the ${plural(t.games, "game")} that reached it.`;
        ui.addTip(li, "log");
        return li;
      }));
      parts.push(ui.el("p", "note", "Average turn of each tavern tier-up:"), list);
    }
    const heroes = m.heroes.filter((h) => h.shop && h.shop.games > 0);
    if (heroes.length) {
      const table = ui.el("table");
      const head = ui.el("tr");
      ["Hero", "Games", "Rolls", "Buys", "Gold spent", "APM"].forEach((w, i) => {
        const th = ui.el("th", i ? "num" : null, w);
        th.setAttribute("scope", "col");
        head.append(th);
      });
      const thead = ui.el("thead");
      thead.append(head);
      const tbody = ui.el("tbody");
      tbody.append(...heroes.map((h) => heroRow(ui, h)));
      table.append(thead, tbody);
      parts.push(ui.el("p", "note", "Per hero, averages per game:"), table);
    }
    if (!s.games) parts.push(ui.el("p", "empty", "No game with a shop record yet."));
    box.replaceChildren(...parts);
  }

  return { recapSection, renderStats };
})();
