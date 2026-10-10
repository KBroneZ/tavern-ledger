// The parser data round in the window (T-206, T-209, T-210, D-055): where
// each combat result comes from, the hero pick, and the shop's gold extras, in
// the recap and the stats. The app works out every number and source
// (tracker::recap, tracker::hero_pick, tracker::shop); this file only words
// and draws them. The word functions are plain data in, plain data out, so
// they are tested without a page (ui-tests/extras.test.js).
"use strict";

(() => {
  const I18n = typeof module !== "undefined" && module.exports ? require("./i18n.js") : window.TLI18n;
  const t = I18n.t;
  const known = (v) => v !== null && v !== undefined;
  const one = (v) => (known(v) ? I18n.num(v, 1) : "—");

  // One line on where this game's combat results come from.
  function resultsLine(results) {
    if (!results) return { text: "", source: "unknown" };
    const { from_log: log, inferred, unknown, recorded } = results;
    if (log + inferred + unknown === 0) return { text: t("extras.results.none"), source: "unknown" };
    const parts = [
      log && t("extras.results.log", { n: I18n.num(log) }),
      inferred && t("extras.results.inferred", { n: I18n.num(inferred) }),
      unknown && t("extras.results.unknown", { n: I18n.num(unknown) }),
    ].filter(Boolean);
    let text = t("extras.results", { parts: parts.join(", ") });
    if (!recorded) text += ` ${t("extras.results.unrecorded", { reparse: t("extras.reparse") })}`;
    return { text, source: inferred ? "inferred" : log ? "log" : "unknown" };
  }

  // The log's name, else the card data's (English), else the card id.
  const cardName = (id) => (typeof window !== "undefined" && window.TLCards ? window.TLCards.name(id) : null);
  const heroWords = (h) => (h && (h.name || cardName(h.id) || h.id)) || t("extras.hero.unknown");

  // The hero pick of one game, worded; `empty` is set when there is none to show.
  function heroPick(pick) {
    if (!pick || pick.state === "not_recorded") {
      return { empty: t("extras.pick.old", { reparse: t("extras.reparse") }) };
    }
    if (pick.state !== "recorded") return { empty: t("extras.pick.none") };
    const pickedId = pick.picked && pick.picked.id;
    return {
      empty: null,
      offered: pick.offered.map((h) => ({ id: h.id, text: heroWords(h), named: Boolean(h.name), picked: h.id === pickedId })),
      rerolls: pick.rerolls === 0 ? t("extras.pick.no_reroll") : I18n.plural("extras.pick.rerolls", pick.rerolls),
      picked: pick.picked ? heroWords(pick.picked) : t("extras.pick.unsaid"),
      pickedSource: pick.picked ? "log" : "unknown",
    };
  }

  // The gold and free-roll extras of one game's shop (parser revision 4). A
  // value the record lacks says "not recorded" (an older game) or "not every
  // turn" (the log lacks it for a turn), never 0. `turns` tells the two apart.
  function shopExtras(totals, isDuos, turns) {
    if (!totals) return [];
    const kept = (key) => Array.isArray(turns) && turns.some((t) => known(t[key]));
    const fact = (label, value, tip, key) => {
      if (known(value)) return { label, value: I18n.num(value), source: "log", tip };
      const partial = Boolean(key) && kept(key);
      return { label, value: partial ? t("extras.incomplete") : t("extras.not_recorded"), source: "unknown", tip: partial ? t("extras.incomplete_tip") : tip };
    };
    const facts = [
      fact(t("extras.gold.extra"), totals.extra_gold, t("extras.gold.extra.tip"), "extra_gold"),
      fact(t("extras.gold.sell"), totals.sell_gold, null, "sell_gold"),
      fact(t("extras.gold.beyond"), totals.gold_beyond_income, t("extras.gold.beyond.tip"), "extra_gold"),
      fact(t("extras.gold.buy"), totals.buy_gold, totals.buys ? I18n.plural("extras.minions_bought", totals.buys) : null, "buy_gold"),
      fact(t("extras.gold.spell"), totals.spell_gold, null, "spell_gold"),
      fact(t("extras.free.given"), totals.free_refreshes, t("extras.free.given.tip"), "free_refreshes"),
    ];
    if (isDuos) facts.push(fact(t("extras.passes"), totals.passes, null, "passes"));
    return facts;
  }

  // The extras averaged per game in the stats, each with the games it covers.
  function shopExtraStats(s) {
    if (!s) return [];
    const over = (n) => (n ? I18n.plural("extras.avg_over", n) : t("extras.avg_none"));
    const avg = (label, value, games) => ({
      label,
      value: games ? one(value) : t("extras.not_recorded"),
      source: games && known(value) ? "log" : "unknown",
      tip: over(games),
    });
    return [
      avg(t("extras.avg.extra"), s.extra_gold, s.extras_games),
      avg(t("extras.avg.sell"), s.sell_gold, s.extras_games),
      avg(t("extras.avg.beyond"), s.gold_beyond_income, s.extras_games),
      avg(t("extras.avg.free"), s.free_refreshes, s.free_refresh_games),
      avg(t("extras.avg.passes"), s.passes, s.extras_games),
    ];
  }

  const percent = (v) => (known(v) ? I18n.percent(v) : "—");

  // Rows of the per-hero pick table, and a note on the games left out.
  function heroPickStats(picks) {
    if (!picks) return { rows: [], note: "", empty: t("extras.picks.empty") };
    const rows = picks.heroes.map((h) => ({
      hero: h.hero,
      text: h.name || h.hero,
      offered: I18n.num(h.offered),
      picked: I18n.num(h.picked),
      rate: percent(h.pick_rate),
      place: known(h.average_place_when_picked) ? I18n.num(h.average_place_when_picked, 2) : "—",
      placeTip: I18n.plural("extras.place_tip", h.placed_when_picked),
      sort: { offered: h.offered, picked: h.picked, rate: h.pick_rate, place: h.average_place_when_picked },
    }));
    const left = [
      picks.not_recorded && t("extras.left.old", { n: I18n.num(picks.not_recorded), reparse: t("extras.reparse_many") }),
      picks.no_data && t("extras.left.none", { n: I18n.num(picks.no_data) }),
    ].filter(Boolean);
    return {
      rows,
      note: I18n.plural("extras.picks.note", picks.games) + (left.length ? ` ${t("extras.not_counted", { list: left.join("; ") })}` : ""),
      empty: rows.length ? null : t("extras.picks.empty"),
    };
  }

  // --- Drawing (the page only). `ui` is the window's el/withSource/sourceMark/addTip.

  function factList(ui, facts) {
    const dl = ui.el("dl", "facts");
    dl.append(...facts.map((f) => {
      const item = ui.el("div", "fact");
      const dd = ui.el("dd", null, f.value);
      if (f.tip) dd.title = f.tip;
      item.append(ui.el("dt", null, f.label), ui.withSource(dd, f.source));
      return item;
    }));
    return dl;
  }

  function heroPickSection(pick, ui, portrait) {
    const section = ui.el("section", "recap-part hero-pick");
    const heading = ui.el("h3", null, t("extras.pick.title"));
    const mark = ui.sourceMark(pick ? pick.source : "unknown");
    if (mark) heading.append(mark);
    section.append(heading);
    const p = heroPick(pick);
    if (p.empty) {
      section.append(ui.withSource(ui.el("p", "empty", p.empty), "unknown"));
      return section;
    }
    const list = ui.el("ul", "pick-list");
    list.setAttribute("aria-label", t("extras.pick.offered"));
    list.append(...p.offered.map((h) => {
      const li = ui.el("li", h.picked ? "pick picked" : "pick");
      const label = ui.el("span", null, h.text);
      li.append(label);
      const img = portrait(h.id);
      if (img) li.prepend(img);
      if (h.picked) li.append(ui.el("span", "pick-tag", t("extras.pick.tag")));
      li.title = h.id;
      ui.addTip(li, "log");
      // Named by the card data when the log does not name it; named later if
      // the card data answers after this is drawn.
      if (!h.named && window.TLCards) {
        if (label.textContent !== h.id) ui.addTip(li, "card_data");
        else window.TLCards.onChange(() => {
          const name = li.isConnected && window.TLCards.name(h.id);
          if (name && label.textContent === h.id) {
            label.textContent = name;
            ui.addTip(li, "card_data");
          }
        });
      }
      return li;
    }));
    section.append(list, factList(ui, [
      { label: t("extras.pick.rerolls_label"), value: p.rerolls, source: "log" },
      { label: t("extras.pick.picked_label"), value: p.picked, source: p.pickedSource },
    ]));
    return section;
  }

  // Top heroes by times offered (ties: better average place when picked, then
  // name), "Show all" for the rest, sortable (T-323). `key` keeps its state.
  function heroPickTable(picks, ui, key = "hero-picks") {
    const s = heroPickStats(picks);
    const parts = [ui.el("p", "note", s.note)];
    if (s.empty) return parts.concat(ui.el("p", "empty", s.empty));
    const C = window.TLCompact;
    const rows = C.rank(s.rows, { games: (r) => r.sort.offered, place: (r) => r.sort.place, name: (r) => r.text });
    const columns = [
      { label: t("extras.col.hero"), sort: (r) => r.text, text: true },
      { label: t("extras.col.offered"), num: true, sort: (r) => r.sort.offered },
      { label: t("extras.col.picked"), num: true, sort: (r) => r.sort.picked },
      { label: t("extras.col.rate"), num: true, sort: (r) => r.sort.rate },
      { label: t("extras.col.place"), num: true, sort: (r) => r.sort.place, low: true },
    ];
    const row = (r) => {
      const tr = ui.el("tr");
      const name = ui.el("th", "hero-name", r.text);
      name.setAttribute("scope", "row");
      name.title = r.hero;
      const img = ui.art(r.hero, "portrait");
      if (img) name.prepend(img);
      const place = ui.el("td", "num", r.place);
      place.title = r.placeTip;
      tr.append(name, ...[r.offered, r.picked, r.rate].map((v) => ui.el("td", "num", v)), place);
      tr.querySelectorAll("td").forEach((td) => ui.addTip(td, "log"));
      return tr;
    };
    return parts.concat(C.table(ui, { key, columns, rows, row, label: t("extras.picks.label") }));
  }

  const api = {
    // The words for "not recorded", in the language now in use.
    get NOT_RECORDED() { return t("extras.not_recorded"); },
    resultsLine, heroPick, shopExtras, shopExtraStats, heroPickStats,
    factList, heroPickSection, heroPickTable,
  };
  if (typeof module !== "undefined" && module.exports) module.exports = api;
  else window.TLExtras = api;
})();
