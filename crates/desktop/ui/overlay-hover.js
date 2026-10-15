// The hover board (T-306, T-333). The app watches the mouse over the
// game's own leaderboard and says which place is hovered (hover.rs) and where
// the board goes: across the top of the game window, right of the game's own
// hover card (T-341), as wide as the
// game's own board (screen_fit.rs, hover.rs). This file draws that opponent
// from the live state overlay.js already has, laid out like the game: the
// last board met, one row of up to seven minions with attack and health, and
// the turn it was seen; the hero, tier, health, triples and trinkets
// (T-312, T-313). In Duos each hero of the team has its own row, matched by
// seat; since T-340 only the hero under the mouse. Our own slot shows
// nothing. The live state follows what the game has played on screen
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

  // Where the app put it (screen_fit::place_card): across the top of the
  // game's picture, centred, inside the screen, cut at its bottom when taller.
  // When the combat odds bar is up in the card's way it goes just below the
  // bar. The text follows the board's width (T-334, overlay-scale.js).
  function position() {
    const p = hover.card;
    const top = window.TLPlace.clearOf(oddsBar(), { x: p.x, y: p.y, w: p.width }, rootFont() * window.TLPlace.BAR_GAP_REM);
    card.style.left = `${Math.round(p.x)}px`;
    card.style.top = `${Math.round(top)}px`;
    card.style.width = `${Math.round(p.width)}px`;
    card.style.maxHeight = `${Math.round(Math.max(0, p.max_height - (top - p.y)))}px`;
    card.style.setProperty("--card-font", `${TLScale.cardPx(p.width)}px`);
  }

  const rootFont = () => parseFloat(getComputedStyle(document.documentElement).fontSize) || 14;

  // The odds bar's rectangle while it shows, else null.
  function oddsBar() {
    const slot = document.querySelector('.slot[data-panel="odds"]');
    if (!slot || slot.hidden) return null;
    const r = slot.getBoundingClientRect();
    return r.width > 0 && r.height > 0 ? { x: r.left, y: r.top, w: r.width, h: r.height } : null;
  }

  const stat = (n) => (n === null || n === undefined ? "?" : String(n));

  // The numbers and the star read out in words, in the language.
  function spoken(m) {
    const said = I18n.t("ov.board.stats", { atk: stat(m.atk), health: stat(m.health) });
    return el("span", "visually-hidden", m.golden ? `${said}, ${I18n.t("keyword.golden")}` : said);
  }

  // One minion as the game draws it (T-372, minion-art/): the game's frame,
  // the picture under it, the gems and a layer for each keyword, with its
  // name under it.
  function drawnMinion(m, art) {
    const item = el("li", m.golden ? "hb-minion hb-drawn golden" : "hb-minion hb-drawn");
    const frame = el("div", "hb-drawn-box");
    frame.append(art.render(m));
    // Dark Gifts (T-528): a mark per gift.
    const gifts = giftMarks(m);
    if (gifts) frame.append(gifts);
    item.append(frame);
    const name = TLCards.name(m.card_id);
    item.append(el("i", name === null ? "hb-name unnamed" : "hb-name", name || m.card_id || "?"), spoken(m));
    return item;
  }

  // The plain look, while the game's textures are not on this PC (T-333):
  // its picture, attack at the bottom left and health at the bottom right,
  // golden as a ring and a star, its name under it.
  function plainMinion(m) {
    const item = el("li", m.golden ? "hb-minion golden" : "hb-minion");
    const frame = el("div", "hb-art");
    const art = TLCards.art(m.card_id, "art");
    if (art) frame.append(art);
    const atk = el("b", "hb-atk", stat(m.atk));
    const hp = el("b", "hb-hp", stat(m.health));
    atk.setAttribute("aria-hidden", "true");
    hp.setAttribute("aria-hidden", "true");
    frame.append(atk, hp);
    // Dark Gifts (T-528): a mark per gift.
    const gifts = giftMarks(m);
    if (gifts) frame.append(gifts);
    item.append(frame);
    const name = TLCards.name(m.card_id);
    item.append(el("i", name === null ? "hb-name unnamed" : "hb-name", name || m.card_id || "?"), spoken(m));
    return item;
  }

  function minionCard(m) {
    const art = window.TLMinions;
    return art ? drawnMinion(m, art) : plainMinion(m);
  }

  function boardStrip(view) {
    const strip = el("ol", "hb-board");
    for (const m of view.board.minions) strip.append(minionCard(m));
    const who = view.hero ? heroName(view.hero) : I18n.t("ov.status.heroUnknown");
    strip.setAttribute("aria-label", I18n.t("ov.board.slots", { name: who, count: view.board.minions.length }));
    return strip;
  }

  // The leaderboard's values of one hero: the tier, the triple count above 0
  // (T-339) and the trinkets as a count.
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
    const w = TLWords.seatWords(view.shown);
    if (w.triples > 0) facts.push(el("span", "hb-fact", I18n.plural("ov.triples", w.triples)));
    if (w.trinkets > 0) facts.push(el("span", "hb-fact", I18n.plural("ov.trinkets", w.trinkets)));
    return facts;
  }

  // One hero's part of the card: who (in Duos), their facts, when the board
  // was seen, then the board in one row.
  function seatBlock(view, team) {
    const block = el("section", "hb-seat");
    const line = el("div", "hb-seat-line");
    if (team) {
      const portrait = view.hero ? TLCards.art(view.hero.id, "portrait") : null;
      if (portrait) line.append(portrait);
      line.append(el("b", "hb-hero", view.hero ? heroName(view.hero) : I18n.t("ov.status.heroUnknown")));
    }
    line.append(...seatFacts(view));
    // No words when no board was seen (T-339).
    const seen = view.board ? TLWords.seenWords(view.board.round, current().turn.value) : null;
    if (seen) line.append(el("span", "hb-seen", seen));
    block.append(line);
    if (view.board && view.board.minions.length) {
      block.append(boardStrip(view));
    } else if (view.board) {
      block.append(el("div", "hb-empty", I18n.t("ov.board.empty")));
    }
    return block;
  }

  // The card's first line: the game's mark, portrait and names, and health
  // (the whole team in Duos). `lone` is the one hero shown (Solo, or the one
  // of a Duos team under the mouse, T-340); health stays the team's, as the
  // game counts it.
  function cardHead(opponent, team, lone) {
    const head = el("div", "hb-head");
    if (opponent.next) head.append(el("span", "next-badge", TLWords.nextLabel(current())));
    if (lone) {
      const portrait = TLCards.art(lone.id, "portrait");
      if (portrait) head.append(portrait);
    }
    const title = lone ? heroName(lone) : team ? heroes(opponent.heroes) : null;
    head.append(el("b", "hb-title", title || I18n.t("ov.status.heroUnknown")));
    const hp = el("span", "hb-health");
    hp.append(sourced(el("b"), opponent.health), ` ${I18n.t("ov.hp.unit")}`);
    head.append(hp);
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
    card.replaceChildren(cardHead(opponent, team, lone), ...views.map((v) => seatBlock(v, team)));
    card.hidden = false;
    position();
  }

  // The game's textures arrived (or went): the card is drawn again.
  window.addEventListener("tl-minions", () => render());

  // A smaller or bigger window changes the text and so the odds bar's height.
  window.addEventListener("resize", () => requestAnimationFrame(() => window.TLHover && window.TLHover.reposition()));

  onEvent("overlay-hover", (event) => {
    hover = event.payload || null;
    render();
  }).catch((error) => showError(error));

  return {
    render,
    // The panels moved (the layout was drawn): the card follows the odds bar.
    reposition() {
      if (hover && !card.hidden) position();
    },
    // overlay.js says when an update failed, and when one worked again.
    setFailed(on) {
      failed = on;
      if (on) hide();
    },
  };
})();
