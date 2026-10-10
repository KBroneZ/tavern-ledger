// The parser data round in the window (T-206, T-209, T-210, D-055): where
// each combat result comes from, the hero pick, and the shop's gold extras, in
// the recap and the stats. The app works out every number and source
// (tracker::recap, tracker::hero_pick, tracker::shop); this file only words
// and draws them. The word functions are plain data in, plain data out, so
// they are tested without a page (ui-tests/extras.test.js).
"use strict";

(() => {
  const REPARSE = "re-read it with tavern-watch --reparse while its log is still on disk";
  const REPARSE_MANY = "re-read them with tavern-watch --reparse while their logs are still on disk";
  const NOT_RECORDED = "not recorded";
  const INCOMPLETE = "not every turn";
  const INCOMPLETE_TIP = "The game kept this, but the log lacks it for at least one turn (often turn 1's gold), so there is no total.";
  const known = (v) => v !== null && v !== undefined;
  const plural = (n, word) => `${n} ${word}${n === 1 ? "" : "s"}`;
  const one = (v) => (known(v) ? v.toFixed(1) : "—");

  // One line on where this game's combat results come from.
  function resultsLine(results) {
    if (!results) return { text: "", source: "unknown" };
    const { from_log: log, inferred, unknown, recorded } = results;
    if (log + inferred + unknown === 0) return { text: "No combat in the log.", source: "unknown" };
    const parts = [
      log && `${log} from the game's log`,
      inferred && `${inferred} inferred from health`,
      unknown && `${unknown} unknown`,
    ].filter(Boolean);
    let text = `Results: ${parts.join(", ")}.`;
    if (!recorded) text += ` The game's own results are not recorded for this game: ${REPARSE}.`;
    return { text, source: inferred ? "inferred" : log ? "log" : "unknown" };
  }

  // The log's name, else the card data's (English), else the card id.
  const cardName = (id) => (typeof window !== "undefined" && window.TLCards ? window.TLCards.name(id) : null);
  const heroWords = (h) => (h && (h.name || cardName(h.id) || h.id)) || "Unknown hero";

  // The hero pick of one game, worded; `empty` is set when there is none to show.
  function heroPick(pick) {
    if (!pick || pick.state === "not_recorded") {
      return { empty: `Hero pick not recorded: this game was saved by an older version of the app; ${REPARSE}.` };
    }
    if (pick.state !== "recorded") return { empty: "The log has no hero choice for this game." };
    const pickedId = pick.picked && pick.picked.id;
    return {
      empty: null,
      offered: pick.offered.map((h) => ({ id: h.id, text: heroWords(h), named: Boolean(h.name), picked: h.id === pickedId })),
      rerolls: pick.rerolls === 0 ? "No reroll" : plural(pick.rerolls, "reroll"),
      picked: pick.picked ? heroWords(pick.picked) : "The log does not say which hero you picked.",
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
      if (known(value)) return { label, value: String(value), source: "log", tip };
      const partial = Boolean(key) && kept(key);
      return { label, value: partial ? INCOMPLETE : NOT_RECORDED, source: "unknown", tip: partial ? INCOMPLETE_TIP : tip };
    };
    const facts = [
      fact("Extra gold", totals.extra_gold, "Gold beyond each turn's own and beyond sells: cards, trinkets, refunds.", "extra_gold"),
      fact("Gold from selling", totals.sell_gold, null, "sell_gold"),
      fact("Gold beyond the fixed income", totals.gold_beyond_income, "Extra gold plus gold from selling.", "extra_gold"),
      fact("Gold on minions", totals.buy_gold, totals.buys ? `${plural(totals.buys, "minion")} bought.` : null, "buy_gold"),
      fact("Gold on tavern spells", totals.spell_gold, null, "spell_gold"),
      fact("Free rolls the game gave", totals.free_refreshes,
        "Rolls that used a free roll shown on the roll button. Other rolls that spent no gold came from cards or effects.", "free_refreshes"),
    ];
    if (isDuos) facts.push(fact("Cards passed to your teammate", totals.passes, null, "passes"));
    return facts;
  }

  // The extras averaged per game in the stats, each with the games it covers.
  function shopExtraStats(s) {
    if (!s) return [];
    const over = (n) => (n ? `Average over ${plural(n, "game")} with this record.` : "No game has this record yet.");
    const avg = (label, value, games) => ({
      label,
      value: games ? one(value) : NOT_RECORDED,
      source: games && known(value) ? "log" : "unknown",
      tip: over(games),
    });
    return [
      avg("Extra gold / game", s.extra_gold, s.extras_games),
      avg("Gold from selling / game", s.sell_gold, s.extras_games),
      avg("Gold beyond income / game", s.gold_beyond_income, s.extras_games),
      avg("Free rolls given / game", s.free_refreshes, s.free_refresh_games),
      avg("Cards passed / game", s.passes, s.extras_games),
    ];
  }

  const percent = (v) => (known(v) ? `${Math.round(v * 100)}%` : "—");

  // Rows of the per-hero pick table, and a note on the games left out.
  function heroPickStats(picks) {
    if (!picks) return { rows: [], note: "", empty: "No hero pick in the log yet." };
    const rows = picks.heroes.map((h) => ({
      hero: h.hero,
      text: h.name || h.hero,
      offered: String(h.offered),
      picked: String(h.picked),
      rate: percent(h.pick_rate),
      place: known(h.average_place_when_picked) ? h.average_place_when_picked.toFixed(2) : "—",
      placeTip: `Over ${plural(h.placed_when_picked, "game")} with a place that counts.`,
      sort: { offered: h.offered, picked: h.picked, rate: h.pick_rate, place: h.average_place_when_picked },
    }));
    const left = [
      picks.not_recorded && `${picks.not_recorded} saved by an older version (${REPARSE_MANY})`,
      picks.no_data && `${picks.no_data} with no hero choice or no pick in the log`,
    ].filter(Boolean);
    return {
      rows,
      note: `Over ${plural(picks.games, "game")} whose hero pick is in the log.` + (left.length ? ` Not counted: ${left.join("; ")}.` : ""),
      empty: rows.length ? null : "No hero pick in the log yet.",
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
    const heading = ui.el("h3", null, "Hero pick");
    const mark = ui.sourceMark(pick ? pick.source : "unknown");
    if (mark) heading.append(mark);
    section.append(heading);
    const p = heroPick(pick);
    if (p.empty) {
      section.append(ui.withSource(ui.el("p", "empty", p.empty), "unknown"));
      return section;
    }
    const list = ui.el("ul", "pick-list");
    list.setAttribute("aria-label", "Heroes offered");
    list.append(...p.offered.map((h) => {
      const li = ui.el("li", h.picked ? "pick picked" : "pick");
      const label = ui.el("span", null, h.text);
      li.append(label);
      const img = portrait(h.id);
      if (img) li.prepend(img);
      if (h.picked) li.append(ui.el("span", "pick-tag", "picked"));
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
      { label: "Rerolls", value: p.rerolls, source: "log" },
      { label: "Picked", value: p.picked, source: p.pickedSource },
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
      { label: "Hero", sort: (r) => r.text, text: true },
      { label: "Offered", num: true, sort: (r) => r.sort.offered },
      { label: "Picked", num: true, sort: (r) => r.sort.picked },
      { label: "Pick rate", num: true, sort: (r) => r.sort.rate },
      { label: "Avg place when picked", num: true, sort: (r) => r.sort.place, low: true },
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
    return parts.concat(C.table(ui, { key, columns, rows, row, label: "Hero picks" }));
  }

  const api = {
    NOT_RECORDED, resultsLine, heroPick, shopExtras, shopExtraStats, heroPickStats,
    factList, heroPickSection, heroPickTable,
  };
  if (typeof module !== "undefined" && module.exports) module.exports = api;
  else window.TLExtras = api;
})();
