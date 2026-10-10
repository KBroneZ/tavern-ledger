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
  const REPARSE = "re-read it with tavern-watch --reparse while its log is still on disk";
  const REPARSE_MANY = "re-read them with tavern-watch --reparse while their logs are still on disk";
  const known = (v) => v !== null && v !== undefined;
  const plural = (n, word) => `${n} ${word}${n === 1 ? "" : "s"}`;
  const list = (v) => (Array.isArray(v) ? v : []);

  // Short marks, in words: readable without colour and read out as they are.
  const KEYWORDS = {
    taunt: ["Taunt", "Taunt"],
    divine_shield: ["Shield", "Divine Shield"],
    reborn: ["Reborn", "Reborn"],
    venomous: ["Venomous", "Venomous"],
    windfury: ["Windfury", "Windfury"],
    stealth: ["Stealth", "Stealth"],
    deathrattle: ["Deathrattle", "Deathrattle"],
    rally: ["Rally", "Rally"],
  };
  const titleCase = (key) => key.split("_").map((w) => w.charAt(0).toUpperCase() + w.slice(1)).join(" ");

  // A minion's keywords: null when the record is older than revision 5.
  function keywordMarks(keywords) {
    if (!Array.isArray(keywords)) return null;
    return keywords.filter((k) => typeof k === "string").map((k) => {
      const [short, full] = KEYWORDS[k] || [titleCase(k), titleCase(k)];
      return { key: k, short, full };
    });
  }

  const CHOICE_KINDS = {
    trinket: "Trinket offer",
    dark_gift: "Dark Gift",
    quest: "Quest",
    triple_reward: "Triple reward",
    discover: "Discover",
  };

  // Why a game has no choices or leaderboard to show; null when it has.
  function missing(state, what) {
    if (state === "recorded") return null;
    if (state === "not_recorded") return `${what} not recorded: this game was saved by an older version of the app; ${REPARSE}.`;
    return `The log has no ${what.toLowerCase()} for this game.`;
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

  const triplesText = (n, atEnd) => (n ? plural(n, "triple") : atEnd ? "no triple" : "no triple yet");

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
      leaderboardMissing: recorded ? null : missing(choices ? choices.leaderboard_state : "not_recorded", "Triples and trinkets"),
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
          stats: `${known(m.atk) ? m.atk : "?"}/${known(m.health) ? m.health : "?"}`,
          golden: m.golden === true,
          keywords: keywordMarks(m.keywords),
        })),
      })),
    };
  }

  // The player's choices, one row each, and what they add up to.
  function choiceRows(choices) {
    const empty = missing(choices ? choices.state : "not_recorded", "Choices");
    if (empty) return { empty, rows: [], trinketsTaken: [], oldGod: null };
    const rows = list(choices.choices).map((c) => ({
      turn: c.turn,
      kind: CHOICE_KINDS[c.kind] || "Discover",
      options: list(c.offered).map((o, i) => ({ id: o.card_id, gift: o.gift, taken: c.picked === i })),
      pickKnown: known(c.picked),
    }));
    const trinketsTaken = list(choices.choices)
      .filter((c) => c.kind === "trinket" && known(c.picked))
      .map((c) => c.offered[c.picked] && c.offered[c.picked].card_id)
      .filter(Boolean);
    const oldGod = known(choices.old_god_dbf) ? { id: choices.old_god || null, dbf: choices.old_god_dbf } : null;
    return { empty: rows.length ? null : "No choice in the log for this game.", rows, trinketsTaken, oldGod };
  }

  // The per-trinket or per-Dark Gift table of the stats.
  function choiceTable(stats, key) {
    if (!stats) return { rows: [], note: "", empty: "No choice in the log yet." };
    const rows = list(stats[key]).map((c) => ({
      id: c.card_id,
      offered: String(c.offered),
      picked: String(c.picked),
      rate: known(c.pick_rate) ? `${Math.round(c.pick_rate * 100)}%` : "—",
      place: known(c.average_place_when_picked) ? c.average_place_when_picked.toFixed(2) : "—",
      placeTip: `Over ${plural(c.placed_when_picked, "game")} with a place that counts.`,
      sort: { offered: c.offered, picked: c.picked, rate: c.pick_rate, place: c.average_place_when_picked },
    }));
    const left = [
      stats.not_recorded && `${stats.not_recorded} saved by an older version (${REPARSE_MANY})`,
      stats.no_data && `${stats.no_data} the log could not read`,
    ].filter(Boolean);
    return {
      rows,
      note: `Over ${plural(stats.games, "game")} whose choices are in the log.` + (left.length ? ` Not counted: ${left.join("; ")}.` : ""),
      empty: rows.length ? null : "None offered in these games yet.",
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
    const node = ui.el("span", className || "card-name", (cards() && cards().name(id)) || id || "Unknown card");
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
    if (m.golden) li.append(" ", ui.el("span", "kw kw-golden", "Golden"));
    (m.keywords || []).forEach((k) => {
      const mark = ui.el("span", `kw kw-${k.key}`, k.short);
      mark.title = k.full;
      li.append(" ", mark);
    });
    return li;
  }

  function leaderLine(ui, at, prefix, atEnd) {
    const p = ui.el("p", "note leader-line", `${prefix}${triplesText(at.triples, atEnd)}`);
    if (at.trinkets.length) {
      p.append(" · trinkets: ");
      at.trinkets.forEach((t, i) => {
        if (i) p.append(", ");
        p.append(t.card_id ? cardName(ui, t.card_id) : ui.el("span", "card-name", `card ${t.dbf_id}`));
      });
    }
    ui.addTip(p, "log");
    return p;
  }

  function boardBlock(ui, b, heroText, duos) {
    const block = ui.el("div", "seen-board");
    const who = duos && b.hero ? ` · ${heroText(b.hero)}` : "";
    block.append(ui.el("h5", null, `Round ${b.round}${who}`));
    if (b.board) block.append(leaderLine(ui, b.board, "Leaderboard that turn: "));
    if (!b.minions.length) block.append(ui.el("p", "empty", "Empty board."));
    else {
      const items = ui.el("ul", "minions");
      items.setAttribute("aria-label", `Board in round ${b.round}`);
      items.append(...b.minions.map((m) => minionItem(ui, m)));
      block.append(items);
    }
    if (!b.keywordsRecorded) block.append(ui.withSource(ui.el("p", "note", `Keywords not recorded: ${REPARSE}.`), "unknown"));
    return block;
  }

  // One fold per opponent under the record table.
  function opponentFold(ui, o, choices, heroText, duos, key) {
    const d = opponentDetails(o, choices);
    const name = list(o.heroes).map(heroText).join(" + ") || "Unknown hero";
    const end = d.end.map((s) => triplesText(s.triples, true)).join(" + ");
    const box = fold(ui, key, `${name}: ${plural(d.boards.length, "board")} seen${end ? ` · ${end} at the end` : ""}`, "opponent-fold");
    if (d.leaderboardMissing) box.append(ui.withSource(ui.el("p", "note", d.leaderboardMissing), "unknown"));
    const heroOf = (id) => list(o.heroes).find((h) => h.id === id) || { id };
    d.end.forEach((s) => box.append(leaderLine(ui, s, duos ? `${heroText(heroOf(s.hero))} at the end: ` : "At the end: ", true)));
    if (!d.boards.length) box.append(ui.withSource(ui.el("p", "empty", "No board of theirs in the log."), "unknown"));
    box.append(...d.boards.map((b) => boardBlock(ui, b, heroText, duos)));
    return box;
  }

  // `game` keys the folds, so each game's recap starts folded.
  function opponentFolds(ui, r, heroText, game) {
    const wrap = ui.el("div", "opponent-folds");
    wrap.append(ui.el("h4", null, "Boards, triples and trinkets"));
    wrap.append(...list(r.record).map((o) => opponentFold(ui, o, r.choices, heroText, r.is_duos, `${game}:opp:${list(o.seats).join("-")}`)));
    return wrap;
  }

  function optionNode(ui, o) {
    const li = ui.el("li", o.taken ? "choice-option taken" : "choice-option");
    li.append(cardName(ui, o.id));
    if (o.gift) li.append(" + ", cardName(ui, o.gift, "card-name gift"));
    if (o.taken) li.append(" ", ui.el("span", "pick-tag", "taken"));
    return li;
  }

  function choicesSection(ui, choices, game) {
    const section = ui.el("section", "recap-part own-choices");
    const heading = ui.el("h3", null, "Your choices");
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
    facts.append(fact("Trinkets taken", taken.length ? taken : ["none"], "log"));
    const god = c.oldGod;
    facts.append(god
      ? fact("Old God", [god.id ? cardName(ui, god.id) : `card ${god.dbf} (not in the card data)`], god.id ? "card_data" : "log")
      : fact("Old God", ["no data"], "unknown"));
    section.append(facts);
    if (!c.rows.length) {
      section.append(ui.withSource(ui.el("p", "empty", c.empty), "unknown"));
      return section;
    }
    const box = fold(ui, `${game}:choices`, `Every choice (${c.rows.length})`);
    const rows = ui.el("ol", "choice-list");
    rows.append(...c.rows.map((row) => {
      const li = ui.el("li", "own-choice");
      li.append(ui.el("span", "choice-head", `Turn ${row.turn} · ${row.kind}`));
      const options = ui.el("ul", "choice-options");
      options.append(...row.options.map((o) => optionNode(ui, o)));
      li.append(options);
      if (!row.pickKnown) li.append(ui.withSource(ui.el("p", "note", "The log does not say which one you took."), "unknown"));
      return li;
    }));
    box.append(rows);
    section.append(box);
    return section;
  }

  // Inside its fold: the top rows by times offered (ties: better average place
  // when taken, then name), "Show all" for the rest, sortable (T-323).
  function choiceTableNode(ui, stats, key, label, mode) {
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
      { label: label.replace(/s$/, ""), sort: nameOf, text: true },
      { label: "Offered", num: true, sort: (r) => r.sort.offered },
      { label: "Taken", num: true, sort: (r) => r.sort.picked },
      { label: "Pick rate", num: true, sort: (r) => r.sort.rate },
      { label: "Avg place when taken", num: true, sort: (r) => r.sort.place, low: true },
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
      choiceTableNode(ui, stats, "trinkets", "Trinkets", mode),
      choiceTableNode(ui, stats, "dark_gifts", "Dark Gifts", mode),
    ];
  }

  const api = {
    keywordMarks, seatAt, opponentDetails, choiceRows, choiceTable,
    opponentFolds, choicesSection, choiceTables,
  };
  if (typeof module !== "undefined" && module.exports) module.exports = api;
  else window.TLRoundTwo = api;
})();
