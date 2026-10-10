// Parser round 2 in the window (T-207, T-208, T-211, T-212, D-058): each
// opponent's boards with their keywords, triples and trinkets as the
// leaderboard showed them, the player's own choices, and the trinket and Dark
// Gift stats. The app works out every value and source (tracker::recap,
// tracker::choices); this file only words and draws them. Everything new is
// folded away by default (the user's notes, T-323): a <details> the keyboard
// opens with Enter or Space, remembered across redraws. The word functions
// are plain data in, plain data out (ui-tests/round-two.test.js).
"use strict";

(() => {
  const I18n = typeof module !== "undefined" && module.exports ? require("./i18n.js") : window.TLI18n;
  const t = I18n.t;
  const known = (v) => v !== null && v !== undefined;
  const list = (v) => (Array.isArray(v) ? v : []);
  const inEnglish = (key) => key in I18n.tables().en;

  // Short marks, in words: readable without colour and read out as they are.
  // The words are keyword.<k> (common.js); only the Divine Shield mark is
  // shorter than its name. A keyword the tables lack shows in title case.
  const titleCase = (key) => key.split("_").map((w) => w.charAt(0).toUpperCase() + w.slice(1)).join(" ");

  // A minion's keywords: null when the record is older than revision 5.
  function keywordMarks(keywords) {
    if (!Array.isArray(keywords)) return null;
    return keywords.filter((k) => typeof k === "string").map((k) => {
      const full = inEnglish(`keyword.${k}`) ? t(`keyword.${k}`) : titleCase(k);
      const short = k === "divine_shield" ? t("r2.kw.shield") : full;
      return { key: k, short, full };
    });
  }

  // The kind of one choice: r2.kind.<kind> (trinket dark_gift quest
  // triple_reward discover); an unknown kind reads as Discover.
  const choiceKind = (kind) => t(inEnglish(`r2.kind.${kind}`) ? `r2.kind.${kind}` : "r2.kind.discover");

  // Why a game has no choices or leaderboard to show; null when it has.
  // `what` is "leaderboard" or "choices": r2.missing.<what>.old|none.
  function missing(state, what) {
    if (state === "recorded") return null;
    if (state === "not_recorded") return t(`r2.missing.${what}.old`, { reparse: t("r2.reparse") });
    return t(`r2.missing.${what}.none`);
  }

  // The trinkets and triple count the leaderboard showed for a seat as of a
  // shop turn (Infinity: the end). A later trinket in the same slot is an
  // upgrade and replaces the earlier one. The count starts at 0: the game
  // logs it each time it changes.
  function seatAt(seat, turn) {
    const changes = list(seat && seat.triples_by_turn).filter((t) => Array.isArray(t) && t[0] <= turn);
    const bySlot = new Map();
    list(seat && seat.trinkets).filter((t) => t.turn <= turn).forEach((t) => bySlot.set(t.slot, t));
    return { triples: changes.length ? changes[changes.length - 1][1] : 0, trinkets: [...bySlot.values()] };
  }

  const triplesText = (n, atEnd) => (n ? I18n.plural("r2.triples", n) : atEnd ? t("r2.triple.none") : t("r2.triple.none_yet"));

  // One opponent (a team in Duos): its seats' leaderboard entries and its
  // boards, worded. `leaderboard` is null when the record has none.
  function opponentDetails(opponent, choices) {
    const recorded = choices && choices.leaderboard_state === "recorded";
    const seats = recorded ? list(choices.leaderboard).filter((s) => list(opponent.seats).includes(s.seat)) : [];
    const at = (seat, turn) => {
      const entry = seats.find((s) => s.seat === seat);
      return entry ? seatAt(entry, turn) : null;
    };
    return {
      leaderboardMissing: recorded ? null : missing(choices ? choices.leaderboard_state : "not_recorded", "leaderboard"),
      end: seats.map((s) => ({ seat: s.seat, hero: s.hero, ...seatAt(s, Infinity) })),
      boards: list(opponent.boards).map((b) => ({
        round: b.round,
        seat: b.seat,
        hero: b.hero,
        // A combat follows the shop turn of the same number.
        board: at(b.seat, b.round),
        keywordsRecorded: list(b.minions).every((m) => Array.isArray(m.keywords)),
        minions: list(b.minions).map((m) => ({
          id: m.card_id,
          stats: `${known(m.atk) ? I18n.num(m.atk) : "?"}/${known(m.health) ? I18n.num(m.health) : "?"}`,
          golden: m.golden === true,
          keywords: keywordMarks(m.keywords),
        })),
      })),
    };
  }

  // The player's choices, one row each, and what they add up to.
  function choiceRows(choices) {
    const empty = missing(choices ? choices.state : "not_recorded", "choices");
    if (empty) return { empty, rows: [], trinketsTaken: [], oldGod: null };
    const rows = list(choices.choices).map((c) => ({
      turn: c.turn,
      kind: choiceKind(c.kind),
      options: list(c.offered).map((o, i) => ({ id: o.card_id, gift: o.gift, taken: c.picked === i })),
      pickKnown: known(c.picked),
    }));
    const trinketsTaken = list(choices.choices)
      .filter((c) => c.kind === "trinket" && known(c.picked))
      .map((c) => c.offered[c.picked] && c.offered[c.picked].card_id)
      .filter(Boolean);
    const oldGod = known(choices.old_god_dbf) ? { id: choices.old_god || null, dbf: choices.old_god_dbf } : null;
    return { empty: rows.length ? null : t("r2.choices.empty"), rows, trinketsTaken, oldGod };
  }

  // The per-trinket or per-Dark Gift table of the stats.
  function choiceTable(stats, key) {
    if (!stats) return { rows: [], note: "", empty: t("r2.stats.empty") };
    const rows = list(stats[key]).map((c) => ({
      id: c.card_id,
      offered: I18n.num(c.offered),
      picked: I18n.num(c.picked),
      rate: known(c.pick_rate) ? I18n.percent(c.pick_rate) : "—",
      place: known(c.average_place_when_picked) ? I18n.num(c.average_place_when_picked, 2) : "—",
      placeTip: I18n.plural("r2.place_tip", c.placed_when_picked),
      sort: { offered: c.offered, picked: c.picked, rate: c.pick_rate, place: c.average_place_when_picked },
    }));
    const left = [
      stats.not_recorded && t("r2.left.old", { n: I18n.num(stats.not_recorded), reparse: t("r2.reparse_many") }),
      stats.no_data && t("r2.left.no_data", { n: I18n.num(stats.no_data) }),
    ].filter(Boolean);
    return {
      rows,
      note: I18n.plural("r2.stats.note", stats.games) + (left.length ? ` ${t("r2.not_counted", { list: left.join("; ") })}` : ""),
      empty: rows.length ? null : t("r2.stats.none_offered"),
    };
  }

  // --- Drawing (the page only). `ui` is the window's el/withSource/sourceMark/addTip.

  const openFolds = new Set();

  // A folded block: closed until the user opens it, and kept open across redraws.
  function fold(ui, key, title, className) {
    const box = ui.el("details", className ? `fold ${className}` : "fold");
    box.dataset.fold = key;
    const summary = ui.el("summary", null, title);
    summary.setAttribute("data-focus", `fold:${key}`);
    box.append(summary);
    box.open = openFolds.has(key);
    box.addEventListener("toggle", () => (box.open ? openFolds.add(key) : openFolds.delete(key)));
    return box;
  }

  // A card's name from the card data, else its id; named later if the card
  // data answers after it is drawn (one watcher for the whole page).
  const cards = () => (typeof window !== "undefined" ? window.TLCards : null);
  function cardName(ui, id, className) {
    const node = ui.el("span", className || "card-name", (cards() && cards().name(id)) || id || t("r2.card.unknown"));
    if (id) {
      node.title = id;
      node.dataset.card = id;
    }
    return node;
  }
  if (cards()) {
    cards().onChange(() => document.querySelectorAll("[data-card]").forEach((n) => {
      const name = cards().name(n.dataset.card);
      if (name && n.textContent === n.dataset.card) n.textContent = name;
    }));
  }

  function minionItem(ui, m) {
    const li = ui.el("li", "minion");
    li.append(cardName(ui, m.id), " ", ui.el("span", "minion-stats", m.stats));
    if (m.golden) li.append(" ", ui.el("span", "kw kw-golden", t("keyword.golden")));
    (m.keywords || []).forEach((k) => {
      const mark = ui.el("span", `kw kw-${k.key}`, k.short);
      mark.title = k.full;
      li.append(" ", mark);
    });
    return li;
  }

  function leaderLine(ui, at, prefix, atEnd) {
    const p = ui.el("p", "note leader-line", `${prefix} ${triplesText(at.triples, atEnd)}`);
    if (at.trinkets.length) {
      p.append(` · ${t("r2.trinkets_label")} `);
      at.trinkets.forEach((tr, i) => {
        if (i) p.append(", ");
        p.append(tr.card_id ? cardName(ui, tr.card_id) : ui.el("span", "card-name", t("r2.card_dbf", { id: tr.dbf_id })));
      });
    }
    ui.addTip(p, "log");
    return p;
  }

  function boardBlock(ui, b, heroText, duos) {
    const block = ui.el("div", "seen-board");
    const who = duos && b.hero ? ` · ${heroText(b.hero)}` : "";
    block.append(ui.el("h5", null, `${t("r2.round", { round: b.round })}${who}`));
    if (b.board) block.append(leaderLine(ui, b.board, t("r2.leaderboard_turn")));
    if (!b.minions.length) block.append(ui.el("p", "empty", t("r2.board.empty")));
    else {
      const items = ui.el("ul", "minions");
      items.setAttribute("aria-label", t("r2.board.aria", { round: b.round }));
      items.append(...b.minions.map((m) => minionItem(ui, m)));
      block.append(items);
    }
    if (!b.keywordsRecorded) block.append(ui.withSource(ui.el("p", "note", t("r2.keywords_missing", { reparse: t("r2.reparse") })), "unknown"));
    return block;
  }

  // One fold per opponent under the record table.
  function opponentFold(ui, o, choices, heroText, duos, key) {
    const d = opponentDetails(o, choices);
    const name = list(o.heroes).map(heroText).join(" + ") || t("r2.hero.unknown");
    const end = d.end.map((s) => triplesText(s.triples, true)).join(" + ");
    const box = fold(ui, key, `${name}: ${I18n.plural("r2.boards_seen", d.boards.length)}${end ? ` · ${t("r2.opp.end", { end })}` : ""}`, "opponent-fold");
    if (d.leaderboardMissing) box.append(ui.withSource(ui.el("p", "note", d.leaderboardMissing), "unknown"));
    const heroOf = (id) => list(o.heroes).find((h) => h.id === id) || { id };
    d.end.forEach((s) => box.append(leaderLine(ui, s, duos ? t("r2.hero_at_end", { hero: heroText(heroOf(s.hero)) }) : t("r2.at_end"), true)));
    if (!d.boards.length) box.append(ui.withSource(ui.el("p", "empty", t("r2.opp.no_board")), "unknown"));
    box.append(...d.boards.map((b) => boardBlock(ui, b, heroText, duos)));
    return box;
  }

  // `game` keys the folds, so each game's recap starts folded.
  function opponentFolds(ui, r, heroText, game) {
    const wrap = ui.el("div", "opponent-folds");
    wrap.append(ui.el("h4", null, t("r2.opp.heading")));
    wrap.append(...list(r.record).map((o) => opponentFold(ui, o, r.choices, heroText, r.is_duos, `${game}:opp:${list(o.seats).join("-")}`)));
    return wrap;
  }

  function optionNode(ui, o) {
    const li = ui.el("li", o.taken ? "choice-option taken" : "choice-option");
    li.append(cardName(ui, o.id));
    if (o.gift) li.append(" + ", cardName(ui, o.gift, "card-name gift"));
    if (o.taken) li.append(" ", ui.el("span", "pick-tag", t("r2.taken")));
    return li;
  }

  function choicesSection(ui, choices, game) {
    const section = ui.el("section", "recap-part own-choices");
    const heading = ui.el("h3", null, t("r2.choices.title"));
    const mark = ui.sourceMark(choices ? choices.source : "unknown");
    if (mark) heading.append(mark);
    section.append(heading);
    const c = choiceRows(choices);
    if (c.empty && !c.rows.length && !c.oldGod) {
      section.append(ui.withSource(ui.el("p", "empty", c.empty), "unknown"));
      return section;
    }
    const facts = ui.el("dl", "facts");
    const fact = (label, value, source) => {
      const item = ui.el("div", "fact");
      const dd = ui.el("dd", null);
      dd.append(...value);
      item.append(ui.el("dt", null, label), ui.withSource(dd, source));
      return item;
    };
    const taken = c.trinketsTaken.flatMap((id, i) => (i ? [", ", cardName(ui, id)] : [cardName(ui, id)]));
    facts.append(fact(t("r2.choices.trinkets_taken"), taken.length ? taken : [t("r2.none")], "log"));
    const god = c.oldGod;
    facts.append(god
      ? fact(t("r2.old_god"), [god.id ? cardName(ui, god.id) : t("r2.card_not_in_data", { id: god.dbf })], god.id ? "card_data" : "log")
      : fact(t("r2.old_god"), [t("r2.no_data")], "unknown"));
    section.append(facts);
    if (!c.rows.length) {
      section.append(ui.withSource(ui.el("p", "empty", c.empty), "unknown"));
      return section;
    }
    const box = fold(ui, `${game}:choices`, t("r2.choices.every", { n: c.rows.length }));
    const rows = ui.el("ol", "choice-list");
    rows.append(...c.rows.map((row) => {
      const li = ui.el("li", "own-choice");
      li.append(ui.el("span", "choice-head", t("r2.choice.head", { turn: row.turn, kind: row.kind })));
      const options = ui.el("ul", "choice-options");
      options.append(...row.options.map((o) => optionNode(ui, o)));
      li.append(options);
      if (!row.pickKnown) li.append(ui.withSource(ui.el("p", "note", t("r2.choice.unsaid")), "unknown"));
      return li;
    }));
    box.append(rows);
    section.append(box);
    return section;
  }

  // Inside its fold: the top rows by times offered (ties: better average place
  // when taken, then name), "Show all" for the rest, sortable (T-323).
  function choiceTableNode(ui, stats, key, label, singular, mode) {
    const s = choiceTable(stats, key);
    const box = fold(ui, `stats:${key}`, `${label} (${s.rows.length})`);
    if (s.empty) {
      box.append(ui.el("p", "empty", s.empty));
      return box;
    }
    const nameOf = (r) => (cards() && cards().name(r.id)) || r.id || "";
    const C = window.TLCompact;
    const rows = C.rank(s.rows, { games: (r) => r.sort.offered, place: (r) => r.sort.place, name: nameOf });
    const columns = [
      { label: singular, sort: nameOf, text: true },
      { label: t("r2.col.offered"), num: true, sort: (r) => r.sort.offered },
      { label: t("r2.col.taken"), num: true, sort: (r) => r.sort.picked },
      { label: t("r2.col.rate"), num: true, sort: (r) => r.sort.rate },
      { label: t("r2.col.place"), num: true, sort: (r) => r.sort.place, low: true },
    ];
    const row = (r) => {
      const tr = ui.el("tr");
      const name = ui.el("th", "hero-name");
      name.setAttribute("scope", "row");
      name.append(cardName(ui, r.id));
      const place = ui.el("td", "num", r.place);
      place.title = r.placeTip;
      tr.append(name, ...[r.offered, r.picked, r.rate].map((v) => ui.el("td", "num", v)), place);
      tr.querySelectorAll("td").forEach((td) => ui.addTip(td, "log"));
      return tr;
    };
    box.append(C.table(ui, { key: `${key}:${mode}`, columns, rows, row, label }));
    return box;
  }

  // The stats' trinket and Dark Gift tables for one mode, folded.
  function choiceTables(ui, stats, mode = "") {
    const s = choiceTable(stats, "trinkets");
    return [
      ui.el("p", "note", s.note),
      choiceTableNode(ui, stats, "trinkets", t("r2.table.trinkets"), t("r2.table.trinket"), mode),
      choiceTableNode(ui, stats, "dark_gifts", t("r2.table.dark_gifts"), t("r2.table.dark_gift"), mode),
    ];
  }

  const api = {
    keywordMarks, seatAt, opponentDetails, choiceRows, choiceTable,
    opponentFolds, choicesSection, choiceTables,
  };
  if (typeof module !== "undefined" && module.exports) module.exports = api;
  else window.TLRoundTwo = api;
})();
