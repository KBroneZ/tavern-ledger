// Shop and logged actions per minute (T-204, T-205, D-046) in the recap and
// the stats. The app works out every number and source (tracker::shop); this
// file only draws them. Values are set with textContent only. Card pictures
// come from cards.js, looked up by the log's card id.
"use strict";

window.TLShop = (() => {
  const I18n = window.TLI18n;
  const t = I18n.t;
  // The word for a kind of logged action: shop.kind.<kind>, as the app writes
  // them (strings/details.js); an unknown kind shows as it is.
  const kindWord = (k) => {
    const key = `shop.kind.${k === "other" ? "misc" : k}`;
    return key in I18n.tables().en ? t(key) : k;
  };
  // A card's tooltip: its name (or id), and that it was frozen from last turn.
  const cardTitle = (id, frozen) => {
    const name = window.TLCards.name(id) || id || t("shop.card.unknown");
    return frozen ? t("shop.card.frozen", { name }) : name;
  };

  // Card labels drawn before the card data answered; named when it does.
  let unnamed = [];
  window.TLCards.onChange(() => {
    unnamed = unnamed.filter(({ box, label, id, frozen, ui }) => {
      if (!box.isConnected) return false;
      const name = window.TLCards.name(id);
      if (name === undefined) return true;
      label.textContent = name || id;
      box.title = cardTitle(id, frozen);
      ui.addTip(box, "card_data");
      return false;
    });
  });

  // A hero row's name: the card data's in the app's language (the hero's own,
  // as in the stats), else the log's.
  const heroName = (h) => (h.hero ? window.TLCards.heroName(h.hero, h.name) : h.name);

  const one = (v, digits = 1) => (v === null || v === undefined ? "—" : I18n.num(v, digits));
  const num = (v) => (v === null || v === undefined ? "—" : I18n.num(v));

  // A card picture with its name (or id) as tooltip; the id as text when there is no picture.
  function card(ui, id, extraClass) {
    const box = ui.el("span", `shop-card ${extraClass || ""}`.trim());
    const name = window.TLCards.name(id);
    const frozen = extraClass === "frozen";
    box.title = cardTitle(id, frozen);
    const img = ui.art(id, "shop-art");
    if (img) {
      img.title = t("shop.card.picture", { source: t("source.card_data.label") });
      box.append(img);
    }
    const label = ui.el("span", "shop-card-id", name || id || "?");
    box.append(label);
    if (name === undefined && id) unnamed.push({ box, label, id, frozen, ui });
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

  // A sum over the turns that have it, with "10 of 11 turns" under it when
  // some turn lacks it (extras.js turnSum).
  function sumFact(ui, label, sum) {
    return window.TLExtras.factList(ui, [{ label, ...window.TLExtras.turnSum(sum) }]).firstChild;
  }

  // The game's shop in numbers: the key ones in view, the rest folded (T-323).
  // The turns themselves are drawn by turns.js.
  function recapSummary(shop, ui, isDuos, game) {
    const section = ui.el("section", "recap-part shop");
    const heading = ui.el("h3", null, t("shop.recap.title"));
    const mark = ui.sourceMark(shop.source);
    if (mark) heading.append(mark);
    section.append(heading);
    if (shop.state !== "recorded") {
      section.append(ui.withSource(ui.el("p", "empty", shop.state === "not_recorded" ? t("shop.not_recorded") : t("shop.not_available")), "unknown"));
      return section;
    }
    const tot = shop.totals;
    if (!tot) {
      section.append(ui.withSource(ui.el("p", "empty", t("shop.recap.no_turn")), "unknown"));
      return section;
    }
    const facts = ui.el("dl", "facts");
    facts.append(
      fact(ui, t("shop.apm"), one(shop.apm), I18n.backend(shop.apm_definition)),
      fact(ui, t("shop.rolls"), tot.free_rolls ? t("shop.rolls_free", { rolls: num(tot.rolls), free: num(tot.free_rolls) }) : num(tot.rolls)),
      fact(ui, t("shop.buys_sells"), `${num(tot.buys + tot.spell_buys)} · ${num(tot.sells)}`),
      sumFact(ui, t("shop.gold_spent"), tot.gold_spent),
    );
    section.append(facts);
    const more = window.TLCompact.fold(ui, `${game}:shop-more`, t("shop.more"));
    const moreFacts = ui.el("dl", "facts");
    moreFacts.append(
      fact(ui, t("shop.actions"), shop.apm === null && tot.actions === 0 ? "—" : num(tot.actions), shop.action_kinds.map(([k, n]) => `${num(n)} ${kindWord(k)}`).join(", ")),
      fact(ui, t("shop.minutes"), one(shop.minutes)),
    );
    // Gold extras and free rolls (T-210): "not recorded" in older games, never 0.
    more.append(moreFacts, window.TLExtras.factList(ui, window.TLExtras.shopExtras(tot, isDuos)));
    const ups = shop.tier_ups.map((u) => t("shop.tier_up_item", { tier: u.tier, turn: u.turn })).join(" · ");
    more.append(ui.el("p", "note", ups ? t("shop.tier_ups", { list: ups }) : t("shop.tier_ups.none")));
    // The long definitions wait behind "About this" (T-323).
    more.append(window.TLCompact.about(ui, `${game}:shop-about`, `${I18n.backend(shop.apm_definition)} ${t("shop.frozen_note")}`));
    section.append(more);
    if (shop.ended === false) {
      section.append(ui.withSource(ui.el("p", "note", t("shop.cut_short")), "unknown"));
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
      ui.el("td", h.name || h.hero ? "hero-name" : "hero", heroName(h) || t("shop.hero.unknown")),
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
      s.not_recorded && t("shop.left.old", { n: I18n.num(s.not_recorded) }),
      s.not_counted && t("shop.left.cut", { n: I18n.num(s.not_counted) }),
    ].filter(Boolean);
    const left = notes.length ? [ui.el("p", "note shop-left", t("shop.left", { list: notes.join("; ") }))] : [];
    if (!s.games) {
      box.replaceChildren(ui.el("p", "empty", t("shop.stats.none")), ...left);
      return;
    }
    const facts = ui.el("dl", "facts");
    facts.append(
      fact(ui, t("shop.games_counted"), s.games ? I18n.num(s.games) : "—"),
      fact(ui, t("shop.apm"), one(s.apm), I18n.backend(s.apm_definition)),
      fact(ui, t("shop.rolls_game"), one(s.rolls)),
      fact(ui, t("shop.buys_game"), one(s.buys)),
      fact(ui, t("shop.gold_game"), one(s.gold_spent), window.TLExtras.averageTip(s.gold_games, s.gold_turns)),
    );
    const more = window.TLCompact.fold(ui, `stats:${mode}:shop-more`, t("shop.stats.more"));
    const moreFacts = ui.el("dl", "facts");
    moreFacts.append(fact(ui, t("shop.free_rolls_game"), one(s.free_rolls)), fact(ui, t("shop.sells_game"), one(s.sells)));
    more.append(moreFacts, window.TLExtras.factList(ui, window.TLExtras.shopExtraStats(s)));
    if (s.tier_turns.length) {
      const list = ui.el("ul", "tribe-list");
      list.append(...s.tier_turns.map((tt) => {
        const li = ui.el("li", null, t("shop.tier_turn", { tier: tt.tier, turn: one(tt.average_turn) }));
        li.title = t("shop.tier_turn.tip", { tier: tt.tier, games: I18n.plural("shop.games", tt.games) });
        ui.addTip(li, "log");
        return li;
      }));
      more.append(ui.el("p", "note", t("shop.tier_turns.note")), list);
    }
    const perHero = heroShopTable(ui, m, mode);
    if (perHero) more.append(ui.el("p", "note", t("shop.per_hero")), perHero);
    more.append(window.TLCompact.about(ui, `stats:${mode}:shop-about`, I18n.backend(s.apm_definition)));
    box.replaceChildren(facts, ...left, more);
  }

  // Each hero's shop averages, top rows first (compact.js); null with none.
  function heroShopTable(ui, m, mode) {
    const heroes = m.heroes.filter((h) => h.shop && h.shop.games > 0);
    if (!heroes.length) return null;
    const name = (h) => heroName(h) || "";
    const ranked = window.TLCompact.rank(heroes, { games: (h) => h.shop.games, name });
    const columns = [
      { label: t("shop.col.hero"), sort: name, text: true },
      { label: t("shop.col.games"), num: true, sort: (h) => h.shop.games },
      { label: t("shop.col.rolls"), num: true, sort: (h) => h.shop.rolls },
      { label: t("shop.col.buys"), num: true, sort: (h) => h.shop.buys },
      { label: t("shop.col.gold_spent"), num: true, sort: (h) => h.shop.gold_spent },
      { label: t("shop.col.apm"), num: true, sort: (h) => h.shop.apm },
    ];
    return window.TLCompact.table(ui, { key: `shop-heroes:${mode}`, columns, rows: ranked, row: (h) => heroRow(ui, h), label: t("shop.per_hero.label") });
  }

  return { recapSummary, renderStats, card, cardRow };
})();
