// Words for the overlay's hero select panel (T-365): the heroes on offer and,
// for each, the player's own numbers from their history on this PC (games
// played, average place, top-half share). Pure: plain data in, plain data out,
// so it is tested without a page (ui-tests/hero-words.test.js). The page draws
// them (overlay.js) with textContent only. The app decides every value; this
// file only words them.
//
// A hero never played has no numbers and says so; a number the app did not
// give is left out, never zero. The tier and the tribe affinity from the
// community's games are not here yet: the page keeps an empty slot for each
// and nothing stands in their place.
"use strict";

(() => {
  const I18n = typeof module !== "undefined" && module.exports ? require("./i18n.js") : window.TLI18n;

  const SOLO_TOP = 4;
  const DUOS_TOP = 2;

  const isCount = (n) => Number.isInteger(n) && n > 0;
  const isShare = (n) => typeof n === "number" && Number.isFinite(n) && n >= 0 && n <= 1;
  const isPlace = (n) => typeof n === "number" && Number.isFinite(n) && n >= 1;

  // One hero's numbers as figures with their small labels, plus the sentence a
  // screen reader gets instead. Null when the player has no game with it.
  function ownWords(own, top) {
    if (!own || !isCount(own.games)) return null;
    const placed = isCount(own.placed) ? own.placed : 0;
    const figures = [{ key: "games", value: I18n.num(own.games), label: I18n.plural("ov.heroes.games", own.games) }];
    const said = [I18n.plural("lobby.games", own.games)];
    if (placed > 0 && isPlace(own.average_place)) {
      const value = I18n.num(own.average_place, 1);
      figures.push({ key: "avg", value, label: I18n.t("ov.heroes.avg") });
      said.push(I18n.t("lobby.avg", { value }));
    }
    if (placed > 0 && isShare(own.top_half_share)) {
      const value = I18n.percent(own.top_half_share);
      figures.push({ key: "top", value, label: I18n.t("ov.heroes.top", { top }) });
      said.push(I18n.t("ov.heroes.topSaid", { top, share: value }));
    }
    return { figures, said: said.join(", ") };
  }

  // The panel: one card per hero on offer, in the game's order. Null unless
  // the game is at hero select with at least one hero on offer.
  // `nameOf(hero)` gives the hero's name in the app's language.
  function heroCards(game, nameOf) {
    const offer = game && game.phase === "starting" ? game.hero_offer : null;
    const list = offer && Array.isArray(offer.heroes) ? offer.heroes.filter((h) => h && h.hero && h.hero.id) : [];
    if (!list.length) return null;
    const top = game.is_duos ? DUOS_TOP : SOLO_TOP;
    return {
      title: I18n.t("ov.heroes.title"),
      rerolls: isCount(offer.rerolls) ? I18n.plural("ov.heroes.rerolls", offer.rerolls) : null,
      cards: list.map((h) => {
        const name = nameOf(h.hero);
        const own = ownWords(h.own, top);
        const text = own ? own.said : I18n.t("ov.heroes.none");
        return { hero: h.hero, name, own, said: I18n.t("ov.heroes.said", { name, list: text }) };
      }),
    };
  }

  const api = { ownWords, heroCards };
  if (typeof module !== "undefined" && module.exports) module.exports = api;
  else window.TLHeroes = api;
})();
