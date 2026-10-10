// The hover card (T-306, T-307). The app watches the mouse over the game's
// own leaderboard and says which place is hovered (hover.rs) and where the
// card goes: the top of the game window, centred (screen_fit.rs, T-309); this
// file only draws that opponent's card from the live state overlay.js already
// has: hero, tier, health, record, the last board met with its round in one
// row and the minions they could have (labelled "possible"): on game turns 2
// and 3 for everyone, later only for a hero not met yet (D-050). Our own slot
// shows nothing. Text is set with
// textContent only. Own scope; it uses overlay.js's drawing helpers.
"use strict";

window.TLHover = (() => {
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
      item.prepend(el("b", null, `T${c.tier} `));
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
        ? `From the lobby tribes ${list} (all 5 confirmed in the tavern)`
        : `From the ${p.tribes.length} lobby tribes confirmed in the tavern so far ${list}; the others are missing until offered or entered`;
    }
    if (p.tribes_basis === "tavern_seen") {
      return `From the tribes seen so far ${list}; other lobby tribes are missing until you enter them`;
    }
    return `From the tribes ${list}`;
  }

  function possibleBlock(opponent, p) {
    const block = el("div", "possible");
    const hero = opponent.seats.length > 1 ? heroOf(opponent, p.seat) : null;
    const tiers = p.min_tier === p.tier ? `tier ${p.tier}` : `tier ${p.min_tier}-${p.tier}`;
    const what =
      p.reason === "not_met_yet"
        ? `not met yet · tavern tier ${p.tier}: could have ${tiers}`
        : `tavern tier ${p.tier}: could have`;
    const title = el("div", "when", `${hero ? `${heroName(hero)} · ` : ""}${what}`);
    title.append(mark(p.source) || "");
    block.append(title);
    if (p.missing === "tribes_unknown") {
      block.append(el("div", "none", "Lobby tribes unknown: enter them in the app to list these. Nothing is guessed."));
      return block;
    }
    if (p.missing === "no_card_data") {
      block.append(el("div", "none", "No card data on this PC yet, so these cannot be listed."));
      return block;
    }
    const from = el("div", "none", tribesText(p));
    from.append(mark(p.tribes_source) || "");
    block.append(from);
    if (p.reason === "not_met_yet") {
      block.append(el("div", "none", "No board of theirs seen yet: a guess from their tier and the lobby tribes. The game's own tribe hint is not in the log."));
    }
    if (!p.cards.length) block.append(el("div", "none", "No pool minion matches."));
    else block.append(...cardNames(p.cards));
    return block;
  }

  // Where the app put it (screen_fit::place_card): inside the screen, cut
  // at its bottom when taller.
  function position() {
    const p = hover.card;
    card.style.left = `${Math.round(p.x)}px`;
    card.style.top = `${Math.round(p.y)}px`;
    card.style.width = `${Math.round(p.width)}px`;
    card.style.maxHeight = `${Math.round(p.max_height)}px`;
  }

  function render() {
    const g = current();
    const playing = g && g !== EXAMPLE_GAME && (g.phase === "playing" || g.phase === "over");
    if (failed || !hover || !playing) return hide();
    if (g.own_place && g.own_place.value === hover.place) return hide();
    const opponent = g.opponents.find((o) => o.place && o.place.value === hover.place);
    if (!opponent) return hide();
    card.replaceChildren(opponentHead(opponent), ...renderBoards(opponent));
    for (const p of opponent.possible || []) card.append(possibleBlock(opponent, p));
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
