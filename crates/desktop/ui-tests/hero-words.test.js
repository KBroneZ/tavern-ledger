"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const I18n = require("../ui/i18n.js");
require("../ui/strings/common.js");
require("../ui/strings/overlay.js");
const heroes = require("../ui/hero-words.js");

const nameOf = (hero) => hero.name || hero.id;
const hero = (id, own) => ({ hero: { id, name: `Name ${id}` }, own });
const own = (games, placed, average_place, top_half_share) => ({ games, placed, average_place, top_half_share });
const game = (list, extra = {}) => ({ phase: "starting", is_duos: false, hero_offer: { heroes: list, rerolls: 1 }, ...extra });

test("each hero on offer is a card in the game's order, with its own numbers", () => {
  const panel = heroes.heroCards(game([hero("A", own(5, 5, 3.2, 0.6)), hero("B", null)]), nameOf);
  assert.equal(panel.title, "Heroes on offer");
  assert.equal(panel.rerolls, "1 reroll");
  assert.deepEqual(panel.cards.map((c) => c.name), ["Name A", "Name B"]);
  assert.deepEqual(
    panel.cards[0].own.figures.map((f) => [f.key, f.value, f.label]),
    [
      ["games", "5", "games"],
      ["avg", "3.2", "avg"],
      ["top", "60%", "top 4"],
    ],
  );
  assert.equal(panel.cards[0].said, "Name A: 5 games, avg 3.2, top 4 in 60% of them");
});

test("a hero with no games says so and has no numbers, never a zero", () => {
  const panel = heroes.heroCards(game([hero("B", null)]), nameOf);
  assert.equal(panel.cards[0].own, null);
  assert.equal(panel.cards[0].said, "Name B: No games yet");
  // An own record with no game in it is the same as none.
  assert.equal(heroes.ownWords(own(0, 0, null, null), 4), null);
  assert.equal(heroes.ownWords(undefined, 4), null);
});

test("a number the app did not give is left out", () => {
  const figures = (o) => heroes.ownWords(o, 4).figures.map((f) => f.key);
  assert.deepEqual(figures(own(2, 0, null, null)), ["games"], "no placed game: only the games");
  assert.deepEqual(figures(own(2, 2, 4, null)), ["games", "avg"]);
  assert.deepEqual(figures(own(2, 2, null, 0.5)), ["games", "top"]);
  assert.deepEqual(figures(own(2, 2, 0, 5)), ["games"], "values out of range are not shown");
});

test("one game is a game, and Duos counts the top two", () => {
  const solo = heroes.ownWords(own(1, 1, 2, 1), 4);
  assert.equal(solo.figures[0].label, "game");
  const duos = heroes.heroCards(game([hero("A", own(3, 3, 2, 1))], { is_duos: true }), nameOf);
  assert.equal(duos.cards[0].own.figures[2].label, "top 2");
});

test("no panel unless the game is at hero select with heroes on offer", () => {
  assert.equal(heroes.heroCards(null, nameOf), null);
  assert.equal(heroes.heroCards(game([]), nameOf), null);
  assert.equal(heroes.heroCards(game([hero("A", null)], { phase: "playing" }), nameOf), null);
  assert.equal(heroes.heroCards({ phase: "starting", hero_offer: null }, nameOf), null);
});

test("the panel follows the language and has no tier or affinity until the app gives them", () => {
  I18n.setLang("es");
  try {
    const panel = heroes.heroCards(game([hero("A", own(1, 1, 3, 0.5))]), nameOf);
    assert.equal(panel.title, "Héroes a elegir");
    assert.equal(panel.rerolls, "1 cambio");
    assert.deepEqual(panel.cards[0].own.figures.map((f) => f.label), ["partida", "media", "top 4"]);
    assert.deepEqual(Object.keys(panel.cards[0]).sort(), ["hero", "name", "own", "said"]);
  } finally {
    I18n.setLang("en");
  }
});
