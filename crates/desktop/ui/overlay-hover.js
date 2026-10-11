// The hover board (T-306, T-307, T-333). The app watches the mouse over the
// game's own leaderboard and says which place is hovered (hover.rs) and where
// the board goes: across the top of the game window, right of the game's own
// hover card (T-341), as wide as the
// game's own board (screen_fit.rs, hover.rs). This file draws that opponent
// from the live state overlay.js already has, laid out like the game: the
// last board met, one row of up to seven minions with attack and health, and
// the turn it was seen; the hero, tier, health, record, triples and trinkets
// (T-312, T-313). In Duos each hero of the team has its own row, matched by
// seat; since T-340 only the hero under the mouse. Advanced (T-333) adds keyword letters, trinket names, source words
// and the minions they could have (labelled "possible"): on game turns 2 and
// 3 for everyone, later only for a hero not met yet (D-050). Our own slot
// shows nothing. The live state follows what the game has played on screen
// (T-335), so the hovered place is the one the screen shows. Text is set with
// textContent only. Own scope; it uses overlay.js's drawing helpers.
"use strict";

window.TLHover = (() => {
  const I18n = window.TLI18n;
  const TLScale = window.TLScale;
  const { listen: onEvent } = window.__TAURI__.event;
  const card = document.getElementById("hover-card");
  let hover = null;
  // After an error the page shows it and draws no card from stale data.
  let failed = false;

  function hide() {
    card.hidden = true;
    card.replaceChildren();
  }

  // The hero a seat belongs to, by its index among the seats (Duos teams).
  function heroOf(opponent, seat) {
    const i = opponent.seats.indexOf(seat);
    return opponent.heroes.length === opponent.seats.length && i >= 0 ? opponent.heroes[i] : null;
  }

  function cardNames(cards) {
    const list = el("div", "possible-list");
    let named = false;
    let unnamed = false;
    for (const c of cards) {
      const name = TLCards.name(c.id);
      if (name) named = true;
      if (name === null) unnamed = true;
      const item = el("span", name === null ? "possible-card unnamed" : "possible-card", name || c.id);
      item.prepend(el("b", null, `${I18n.t("ov.tier", { value: c.tier })} `));
      list.append(item);
    }
    const marks = [named ? mark("card_data") : null, unnamed ? mark("unknown") : null].filter(Boolean);
    return [list, ...marks];
  }

  // Where the lobby tribes come from (D-050). Tribes from the tavern are only
  // those offered so far: say when the list may miss some (T-307).
  function tribesText(p) {
    const list = p.tribes.map(tribeName).join(" · ");
    if (p.tribes_basis === "tavern_confirmed") {
      return p.tribes_complete
        ? I18n.t("ov.possible.fromLobbyAll", { list })
        : I18n.t("ov.possible.fromLobbyPart", { count: p.tribes.length, list });
    }
    if (p.tribes_basis === "tavern_seen") {
      return I18n.t("ov.possible.fromSeen", { list });
    }
    return I18n.t("ov.possible.fromTribes", { list });
  }

  // Advanced only (D-050, T-307): always labelled "possible", never as seen.
  function possibleBlock(opponent, p) {
    const block = el("div", "possible adv");
    const hero = opponent.seats.length > 1 ? heroOf(opponent, p.seat) : null;
    const tiers = p.min_tier === p.tier ? I18n.t("ov.possible.tier", { tier: p.tier }) : I18n.t("ov.possible.tierRange", { min: p.min_tier, tier: p.tier });
    const what =
      p.reason === "not_met_yet"
        ? I18n.t("ov.possible.notMet", { tier: p.tier, tiers })
        : I18n.t("ov.possible.could", { tier: p.tier });
    const title = el("div", "when", `${hero ? `${heroName(hero)} · ` : ""}${what}`);
    title.append(mark(p.source) || "");
    block.append(title);
    if (p.missing === "tribes_unknown") {
      block.append(el("div", "none", I18n.t("ov.possible.noTribes")));
      return block;
    }
    if (p.missing === "no_card_data") {
      block.append(el("div", "none", I18n.t("ov.possible.noCards")));
      return block;
    }
    const from = el("div", "none", tribesText(p));
    from.append(mark(p.tribes_source) || "");
    block.append(from);
    if (p.reason === "not_met_yet") {
      block.append(el("div", "none", I18n.t("ov.possible.guess")));
    }
    if (!p.cards.length) block.append(el("div", "none", I18n.t("ov.possible.none")));
    else block.append(...cardNames(p.cards));
    return block;
  }

  // Where the app put it (screen_fit::place_card): across the top of the
  // game window, inside the screen, cut at its bottom when taller. The text
  // follows the board's width (T-334, overlay-scale.js).
  function position() {
    const p = hover.card;
    card.style.left = `${Math.round(p.x)}px`;
    card.style.top = `${Math.round(p.y)}px`;
    card.style.width = `${Math.round(p.width)}px`;
    card.style.maxHeight = `${Math.round(p.max_height)}px`;
    card.style.setProperty("--card-font", `${TLScale.cardPx(p.width)}px`);
  }

  // One minion as the game draws it (T-333): its picture, attack at the
  // bottom left and health at the bottom right, golden as a ring and a star,
  // keyword letters on top (Advanced), its name under it.
  function minionCard(m) {
    const item = el("li", m.golden ? "hb-minion golden" : "hb-minion");
    const frame = el("div", "hb-art");
    const art = TLCards.art(m.card_id, "art");
    if (art) frame.append(art);
    const stat = (n) => (n === null || n === undefined ? "?" : String(n));
    const atk = el("b", "hb-atk", stat(m.atk));
    const hp = el("b", "hb-hp", stat(m.health));
    atk.setAttribute("aria-hidden", "true");
    hp.setAttribute("aria-hidden", "true");
    frame.append(atk, hp);
    // Letters, Advanced only (keywordMarks marks them so).
    const marks = keywordMarks(m.keywords);
    if (marks) frame.append(marks);
    // Dark Gifts (T-528): a mark per gift, in Simple too.
    const gifts = giftMarks(m);
    if (gifts) frame.append(gifts);
    item.append(frame);
    const name = TLCards.name(m.card_id);
    item.append(el("i", name === null ? "hb-name unnamed" : "hb-name", name || m.card_id || "?"));
    // The numbers and the star read out in words, in the language.
    const said = I18n.t("ov.board.stats", { atk: stat(m.atk), health: stat(m.health) });
    item.append(el("span", "visually-hidden", m.golden ? `${said}, ${I18n.t("keyword.golden")}` : said));
    return { item, named: Boolean(name), unnamed: name === null };
  }

  function boardStrip(view) {
    const strip = el("ol", "hb-board");
    let named = false;
    let unnamed = false;
    for (const m of view.board.minions) {
      const c = minionCard(m);
      named = named || c.named;
      unnamed = unnamed || c.unnamed;
      strip.append(c.item);
    }
    const who = view.hero ? heroName(view.hero) : I18n.t("ov.status.heroUnknown");
    strip.setAttribute("aria-label", I18n.t("ov.board.slots", { name: who, count: view.board.minions.length }));
    const marks = [named ? mark("card_data") : null, unnamed ? mark("unknown") : null].filter(Boolean);
    return { strip, marks };
  }

  // The leaderboard's values of one hero: the tier and triple count always,
  // the trinkets as a count in Simple and by name in Advanced.
  function seatFacts(view) {
    const facts = [];
    const tier = view.tier === null ? I18n.t("ov.board.tierUnknown") : I18n.t("ov.board.tier", { value: view.tier });
    facts.push(el("span", "hb-tier", tier));
    if (!view.shown) {
      const none = el("span", "hb-fact", I18n.t("ov.shown.none"));
      none.append(mark("unknown") || "");
      facts.push(none);
      return facts;
    }
    const w = TLWords.seatWords(view.shown, TLCards.name);
    // Simple shows a triple count only above 0 (T-339).
    const some = Number.isInteger(view.shown.triples) && view.shown.triples > 0;
    facts.push(el("span", some ? "hb-fact" : "hb-fact adv", w.triples));
    if (w.trinkets.length) {
      facts.push(el("span", "hb-fact simple-only", I18n.plural("ov.trinkets", w.trinkets.length)));
      const names = el("span", "hb-fact adv");
      w.trinkets.forEach((t, n) => {
        if (n) names.append(", ");
        names.append(el("span", t.known ? "trinket" : "trinket unnamed", t.hero_power ? I18n.t("ov.shown.heroPower", { text: t.text }) : t.text));
      });
      if (w.named) names.append(mark("card_data"));
      if (w.unknown) names.append(mark("unknown"));
      facts.push(names);
    }
    return facts;
  }

  // One hero's part of the card: who (in Duos), their facts, when the board
  // was seen, then the board in one row.
  function seatBlock(opponent, view, team) {
    const block = el("section", "hb-seat");
    const line = el("div", "hb-seat-line");
    if (team) {
      const portrait = view.hero ? TLCards.art(view.hero.id, "portrait") : null;
      if (portrait) line.append(portrait);
      line.append(el("b", "hb-hero", view.hero ? heroName(view.hero) : I18n.t("ov.status.heroUnknown")));
    }
    line.append(...seatFacts(view));
    // "No board seen yet" is Advanced (T-339): Simple leaves the row out.
    const seen = el("span", view.board ? "hb-seen" : "hb-seen adv", TLWords.seenWords(view.board ? view.board.round : null, current().turn.value));
    if (!view.board) seen.append(mark(opponent.boards_source) || "");
    line.append(seen);
    block.append(line);
    if (view.board && view.board.minions.length) {
      const { strip, marks } = boardStrip(view);
      block.append(strip);
      if (marks.length) {
        const note = el("div", "hb-marks adv");
        note.append(...marks);
        block.append(note);
      }
    } else if (view.board) {
      block.append(el("div", "hb-empty", I18n.t("ov.board.empty")));
    }
    for (const p of view.possible) block.append(possibleBlock(opponent, p));
    return block;
  }

  // The card's first line: the game's mark, portrait and names, health and
  // the record against them (the whole team in Duos). `lone` is the one hero
  // shown (Solo, or the one of a Duos team under the mouse, T-340); health
  // and record stay the team's, as the game counts them.
  function cardHead(opponent, team, lone) {
    const head = el("div", "hb-head");
    if (opponent.next) head.append(el("span", "next-badge", TLWords.nextLabels(current()).badge));
    if (lone) {
      const portrait = TLCards.art(lone.id, "portrait");
      if (portrait) head.append(portrait);
    }
    const title = lone ? heroName(lone) : team ? heroes(opponent.heroes) : null;
    head.append(el("b", "hb-title", title || I18n.t("ov.status.heroUnknown")));
    const hp = el("span", "hb-health");
    hp.append(sourced(el("b"), opponent.health), ` ${I18n.t("ov.hp.unit")}`);
    // The record is Advanced (T-339), like in the side panel.
    const record = renderRecord(opponent);
    record.classList.add("adv");
    head.append(hp, record);
    return head;
  }

  function render() {
    const g = current();
    const playing = g && g !== EXAMPLE_GAME && (g.phase === "playing" || g.phase === "over");
    if (failed || !hover || !playing) return hide();
    if (g.own_place && g.own_place.value === hover.place) return hide();
    const opponent = g.opponents.find((o) => o.place && o.place.value === hover.place);
    if (!opponent) return hide();
    const all = TLWords.seatViews(opponent);
    // In Duos only the hero under the mouse (T-340).
    const views = TLWords.hoveredViews(all, hover.member);
    const team = views.length > 1;
    const lone = team ? null : views[0] ? views[0].hero : opponent.heroes[0] || null;
    card.classList.toggle("team", team);
    card.replaceChildren(cardHead(opponent, team, lone), ...views.map((v) => seatBlock(opponent, v, team)));
    card.hidden = false;
    position();
  }

  onEvent("overlay-hover", (event) => {
    hover = event.payload || null;
    render();
  }).catch((error) => showError(error));

  return {
    render,
    // overlay.js says when an update failed, and when one worked again.
    setFailed(on) {
      failed = on;
      if (on) hide();
    },
  };
})();
