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
    const img = ui.art(id, "shop-art");
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

  function fact(ui, label, value, tip) {
    const item = ui.el("div", "fact");
    const dd = ui.el("dd", null, value);
    if (tip) dd.title = tip;
    ui.addTip(dd, value === "—" ? "unknown" : "log");
    item.append(ui.el("dt", null, label), dd);
    return item;
  }

  // The game's shop in numbers: the key ones in view, the rest folded (T-323).
  // The turns themselves are drawn by turns.js.
  function recapSummary(shop, ui, isDuos, game) {
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
    if (!t) {
      section.append(ui.withSource(ui.el("p", "empty", "No shop turn in the log."), "unknown"));
      return section;
    }
    const facts = ui.el("dl", "facts");
    facts.append(
      fact(ui, "APM (logged)", one(shop.apm), shop.apm_definition),
      fact(ui, "Rolls", `${t.rolls}${t.free_rolls ? ` (${t.free_rolls} free)` : ""}`),
      fact(ui, "Buys · sells", `${t.buys + t.spell_buys} · ${t.sells}`),
      fact(ui, "Gold spent", num(t.gold_spent)),
    );
    section.append(facts);
    const more = window.TLCompact.fold(ui, `${game}:shop-more`, "More shop numbers");
    const moreFacts = ui.el("dl", "facts");
    moreFacts.append(
      fact(ui, "Actions", shop.apm === null && t.actions === 0 ? "—" : num(t.actions), shop.action_kinds.map(([k, n]) => `${n} ${KIND_WORDS[k] || k}`).join(", ")),
      fact(ui, "Minutes", one(shop.minutes)),
    );
    // Gold extras and free rolls (T-210): "not recorded" in older games, never 0.
    more.append(moreFacts, window.TLExtras.factList(ui, window.TLExtras.shopExtras(t, isDuos, shop.turns)));
    const ups = shop.tier_ups.map((u) => `tier ${u.tier} on turn ${u.turn}`).join(" · ");
    more.append(ui.el("p", "note", ups ? `Tavern tier-ups: ${ups}.` : "No tavern tier-up in the log."));
    more.append(ui.el("p", "note", shop.apm_definition));
    more.append(ui.el("p", "note", "Frozen: kept from the last turn. Only your own shop is in the log."));
    section.append(more);
    if (shop.ended === false) {
      section.append(ui.withSource(ui.el("p", "note", "The log stops before the game ends: the last turn may be cut short."), "unknown"));
    }
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

  // Per mode: the key averages in view; the rest, the tier-up turns and the
  // per-hero table folded (T-323). `mode` keys the fold and the table.
  function renderStats(m, ui, mode) {
    const s = m.shop;
    const box = document.getElementById("shop-stats-body");
    if (!box || !s) return;
    // The games left out stay in view: an older game is "not recorded", not 0.
    const notes = [
      s.not_recorded && `${s.not_recorded} saved by an older version (run tavern-watch --reparse while their logs are on disk)`,
      s.not_counted && `${s.not_counted} with the log cut short, no shop turn or an unreadable shop record`,
    ].filter(Boolean);
    const left = notes.length ? [ui.el("p", "note shop-left", `Not counted: ${notes.join("; ")}.`)] : [];
    if (!s.games) {
      box.replaceChildren(ui.el("p", "empty", "No game with a shop record yet."), ...left);
      return;
    }
    const facts = ui.el("dl", "facts");
    facts.append(
      fact(ui, "Games counted", s.games || "—"),
      fact(ui, "APM (logged)", one(s.apm), s.apm_definition),
      fact(ui, "Rolls / game", one(s.rolls)),
      fact(ui, "Buys / game", one(s.buys)),
      fact(ui, "Gold spent / game", one(s.gold_spent)),
    );
    const more = window.TLCompact.fold(ui, `stats:${mode}:shop-more`, "More: gold, tier-ups and each hero");
    const moreFacts = ui.el("dl", "facts");
    moreFacts.append(fact(ui, "Free rolls / game", one(s.free_rolls)), fact(ui, "Sells / game", one(s.sells)));
    more.append(moreFacts, window.TLExtras.factList(ui, window.TLExtras.shopExtraStats(s)));
    if (s.tier_turns.length) {
      const list = ui.el("ul", "tribe-list");
      list.append(...s.tier_turns.map((t) => {
        const li = ui.el("li", null, `Tier ${t.tier}: turn ${one(t.average_turn)}`);
        li.title = `Average turn you reached tier ${t.tier}, over the ${plural(t.games, "game")} that reached it.`;
        ui.addTip(li, "log");
        return li;
      }));
      more.append(ui.el("p", "note", "Average turn of each tavern tier-up:"), list);
    }
    const perHero = heroShopTable(ui, m, mode);
    if (perHero) more.append(ui.el("p", "note", "Per hero, averages per game:"), perHero);
    more.append(ui.el("p", "note", s.apm_definition));
    box.replaceChildren(facts, ...left, more);
  }

  // Each hero's shop averages, top rows first (compact.js); null with none.
  function heroShopTable(ui, m, mode) {
    const heroes = m.heroes.filter((h) => h.shop && h.shop.games > 0);
    if (!heroes.length) return null;
    const name = (h) => h.name || h.hero || "";
    const ranked = window.TLCompact.rank(heroes, { games: (h) => h.shop.games, name });
    const columns = [
      { label: "Hero", sort: name, text: true },
      { label: "Games", num: true, sort: (h) => h.shop.games },
      { label: "Rolls", num: true, sort: (h) => h.shop.rolls },
      { label: "Buys", num: true, sort: (h) => h.shop.buys },
      { label: "Gold spent", num: true, sort: (h) => h.shop.gold_spent },
      { label: "APM", num: true, sort: (h) => h.shop.apm },
    ];
    return window.TLCompact.table(ui, { key: `shop-heroes:${mode}`, columns, rows: ranked, row: (h) => heroRow(ui, h), label: "Shop per hero" });
  }

  return { recapSummary, renderStats, card, cardRow };
})();
