"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const I18n = require("../ui/i18n.js");
require("../ui/strings/common.js");
require("../ui/strings/review.js");
const W = require("../ui/review-words.js");

const nameOf = (id) => ({ BG_A: "Alpha", BG_B: "Beta", BG_C: "Gamma", BG_T1: "Old Trinket", BG_T2: "New Trinket" })[id] || id;

const fight = (won, over) => ({ won, tied: 0, lost: 1 - won, trials: 1000, minion_damage_dealt: null, minion_damage_taken: null, place: "no_model", approximate: [], ...over });
const delta = (score, over) => ({ score, standard_error: 0.01, clear: true, minion_damage_dealt: null, minion_damage_taken: null, place: null, ...over });
const alt = (card, won, actualWon, over) => ({ card, approx: ["base_stats"], fight: { Ok: fight(won) }, delta: delta(won - actualWon), stop: "clear", ...over });
const decision = (turn, kind, taken, offered, over) => ({ turn, ms: turn * 1000, kind, taken, offered, source: null, tier: null, inferred: false, ...over });
const rebuilt = (round, actualWon, alternatives, over) => ({ Ok: { round, actual: fight(actualWon), alternatives, unknown_offers: 0, ...over } });
const done = (decisions, over) => ({ state: "done", stopped_for_game: false, review: { decisions, out_of_time: false, boards: 3, trials: 9000, ...over } });

const minion = (card, atk, health) => ({ card_id: card, atk, health, golden: false, keywords: [] });
const round = (number, own) => ({ number, entries: [{ side: "opponent", player_id: 2, board: [minion("BG_X", 3, 3)] }, { side: "own", player_id: 1, board: own }] });
const report = { local_player_id: 1, rounds: [round(2, [minion("BG_C", 2, 2)]), round(3, [minion("BG_A", 5, 6), minion("BG_C", 2, 2)])] };

test("the app's answer is read defensively; one out of shape is a failure, not an empty review", () => {
  assert.equal(W.read(null), null);
  assert.equal(W.read({ state: "what" }), null);
  assert.equal(W.read({ state: "done", review: { decisions: "x" } }), null);
  assert.equal(W.read(done([{ decision: { turn: "3", kind: "buy" }, rebuilt: { Err: "no_alternative" } }])), null, "a decision out of shape fails the whole answer");
  assert.equal(W.read(done([{ decision: decision(3, "buy", "BG_A", ["BG_A"]), rebuilt: { Ok: { round: 3, actual: { won: 2 }, alternatives: [] } } }])), null, "a share above 1");
  assert.deepEqual(W.read({ state: "game_on" }), { state: "game_on" });
  assert.deepEqual(W.read({ state: "busy" }), { state: "busy" });
  assert.deepEqual(W.read({ state: "idle" }), { state: "idle" });
  assert.deepEqual(W.read({ state: "reviewing", done: 2, total: 5 }), { state: "reviewing", progress: { done: 2, total: 5 } }, "this game, already being reviewed");
  const odd = W.read(done([{ decision: decision(3, "buy", "BG_A", ["BG_A", "x<y"]), rebuilt: rebuilt(3, 0.4, [alt("<img src=x onerror=alert(1)>", 0.6, 0.4)]) }]));
  assert.ok(odd, "an alternative's odd card id does not fail the whole review");
  assert.equal(odd.decisions[0].rebuilt.alternatives[0].card, null, "it is unnamed, as a decision's");
  I18n.setLang("en");
  assert.match(W.altLine(odd.decisions[0], odd.decisions[0].rebuilt.alternatives[0], nameOf), /^a card the log does not name: clearly better/);
  assert.deepEqual(W.read({ state: "no_review", why: { no_shop: "missing" } }), { state: "no_review", why: "no_shop" });
  assert.deepEqual(W.read({ state: "no_review", why: "not_solo" }), { state: "no_review", why: "not_solo" });

  const read = W.read(done([
    { decision: decision(3, "buy", "BG_A", ["BG_A", "BG_B", null]), rebuilt: rebuilt(3, 0.4, [alt("BG_B", 0.6, 0.4)], { unknown_offers: 1 }) },
    { decision: decision(3, "roll", null, ["BG_A"]), rebuilt: { Err: "needs_shop_playout" } },
    { decision: decision(4, "buy", "BG_C", ["BG_C", "BG_A"]), rebuilt: { Err: { actual: "out_of_time" } } },
  ], { out_of_time: true }));
  assert.equal(read.state, "done");
  assert.equal(read.outOfTime, true);
  assert.equal(read.decisions.length, 3);
  assert.equal(read.decisions[0].rebuilt.unknownOffers, 1);
  assert.equal(read.decisions[0].rebuilt.alternatives[0].delta.clear, true);
  assert.equal(read.decisions[1].notRebuilt, "needs_shop_playout");
  assert.equal(read.decisions[2].notRebuilt, "actual.out_of_time");
});

test("only clear differences in the alternative's favour count as a cost; the three biggest are shown", () => {
  const read = W.read(done([
    { decision: decision(1, "buy", "BG_A", ["BG_A", "BG_B"]), rebuilt: rebuilt(1, 0.5, [alt("BG_B", 0.9, 0.5, { delta: delta(0.4, { clear: false }), stop: "max_trials" })]) },
    { decision: decision(2, "buy", "BG_A", ["BG_A", "BG_B", "BG_C"]), rebuilt: rebuilt(2, 0.5, [alt("BG_B", 0.6, 0.5), alt("BG_C", 0.8, 0.5)]) },
    { decision: decision(3, "buy", "BG_A", ["BG_A", "BG_B"]), rebuilt: rebuilt(3, 0.5, [alt("BG_B", 0.2, 0.5)]) },
    { decision: decision(4, "sell", "BG_C", []), rebuilt: rebuilt(4, 0.3, [alt("BG_C", 0.5, 0.3, { approx: ["earlier_stats"] })]) },
    { decision: decision(5, "choose", "BG_A", ["BG_A", "BG_B"]), rebuilt: rebuilt(5, 0.1, [alt("BG_B", 0.2, 0.1)]) },
    { decision: decision(6, "choose", "BG_A", ["BG_A", "BG_B"]), rebuilt: rebuilt(6, 0.1, [alt("BG_B", 0.2, 0.1)]) },
    { decision: decision(7, "buy", "BG_A", ["BG_A", "BG_B"]), rebuilt: rebuilt(7, 0.5, [alt("BG_B", 0, 0.5, { fight: { Err: "no_card_data" }, delta: null, stop: null })]) },
  ]));
  assert.equal(W.costOf(read.decisions[0]), null, "not clear: never costly, however big");
  assert.equal(W.costOf(read.decisions[2]), null, "clearly worse: not a cost");
  assert.equal(W.costOf(read.decisions[6]), null, "not simulated: not a cost");
  assert.equal(W.costOf(read.decisions[1]).alt.card, "BG_C", "the best alternative of a choice");
  const top = W.top(read.decisions);
  assert.deepEqual(top.map((t) => t.decision.turn), [2, 4, 5], "by cost, then the earlier turn");
  assert.equal(W.top(read.decisions, 1).length, 1);
});

test("before and after boards: the card taken swapped for the alternative, or the minion sold put back", () => {
  const swap = W.read(done([{ decision: decision(3, "buy", "BG_A", ["BG_A", "BG_B"]), rebuilt: rebuilt(3, 0.4, [alt("BG_B", 0.6, 0.4)]) }])).decisions[0];
  const b = W.boards(report, swap, swap.rebuilt.alternatives[0], { BG_B: { atk: 4, health: 4 } });
  assert.deepEqual(b.before.map((m) => m.card), ["BG_A", "BG_C"]);
  assert.deepEqual(b.after.map((m) => [m.card, m.atk, m.health]), [["BG_B", 4, 4], ["BG_C", 2, 2]], "at its card data's stats, in the same place");
  const unknownStats = W.boards(report, swap, swap.rebuilt.alternatives[0], {});
  assert.equal(unknownStats.after[0].atk, null, "stats the card data lacks are unknown, not made up");

  const sold = W.read(done([{ decision: decision(3, "sell", "BG_C", []), rebuilt: rebuilt(3, 0.4, [alt("BG_C", 0.6, 0.4)]) }])).decisions[0];
  const soldReport = { local_player_id: 1, rounds: [round(2, [minion("BG_A", 1, 1), minion("BG_C", 2, 2)]), round(3, [minion("BG_A", 5, 6)])] };
  const k = W.boards(soldReport, sold, sold.rebuilt.alternatives[0], {});
  assert.deepEqual(k.after.map((m) => [m.card, m.atk]), [["BG_A", 5], ["BG_C", 2]], "kept at its place and stats from the fight before");

  const trinket = W.read(done([{ decision: decision(3, "choose", "BG_T1", ["BG_T1", "BG_T2"]), rebuilt: rebuilt(3, 0.4, [alt("BG_T2", 0.6, 0.4, { approx: [] })]) }])).decisions[0];
  assert.deepEqual(W.boards(report, trinket, trinket.rebuilt.alternatives[0], {}), { trinket: true, before: report.rounds[1].entries[1].board.map(W.minionOf) });
  assert.equal(W.boards({ rounds: [] }, swap, swap.rebuilt.alternatives[0], {}), null, "no saved board: no boards");
});

test("the plain reasons say what was measured: the next fight, in the simulation (English and Spanish)", () => {
  const read = W.read(done([
    { decision: decision(5, "buy", "BG_A", ["BG_A", "BG_B"]), rebuilt: rebuilt(5, 0.44, [alt("BG_B", 0.62, 0.44)]) },
    { decision: decision(6, "sell", "BG_C", []), rebuilt: rebuilt(6, 0.3, [alt("BG_C", 0.5, 0.3, { approx: ["earlier_stats", "start_of_combat_on_board", "something_new"] })]) },
  ]));
  const [buy, sell] = read.decisions;
  I18n.setLang("en");
  const best = W.costOf(buy);
  assert.equal(W.reasonLine(buy, best.alt, nameOf), "Turn 5: you bought Alpha. Had you bought Beta instead, you win the next fight 62% of the time in the simulation, instead of 44%.");
  assert.equal(W.reasonLine(sell, W.costOf(sell).alt, nameOf), "Turn 6: you sold Gamma. Had you kept it, you win the next fight 50% of the time in the simulation, instead of 30%.");
  assert.doesNotMatch(W.reasonLine(buy, best.alt, nameOf), /place/i, "never the cost in final place");
  assert.equal(W.tieLine(buy.rebuilt, best.alt), null, "no ties: no tie line");
  assert.deepEqual(W.notes(sell, W.costOf(sell).alt, nameOf), [
    "Gamma is kept with its stats from the fight before you sold it.",
    "A card on the boards acts at the start of combat, which is not simulated; it might have changed this.",
    "Simulated with a simplification this version of the app does not describe.",
  ], "a marker this build does not know still says something");
  assert.deepEqual(W.notes(buy, { ...best.alt, approx: ["battlecry_not_played", "gold_ignored"] }, nameOf), [
    "Neither card's Battlecry, nor anything else done as it was played, is simulated again.",
    "The gold each card cost, and what was left for the rest of the turn, is not counted.",
  ]);
  I18n.setLang("es");
  assert.equal(W.reasonLine(buy, best.alt, nameOf), "Turno 5: compraste Alpha. Si hubieras comprado Beta, en la simulación ganas el siguiente combate el 62 % de las veces en vez del 44 %.");
  I18n.setLang("en");
});

test("when the wins do not move, the reasons name the losses instead of '0% instead of 0%'", () => {
  const kept = alt("BG_C", 0, 0, { fight: { Ok: fight(0, { tied: 0.57, lost: 0.43 }) }, delta: delta(0.285) });
  const sell = W.read(done([{ decision: decision(3, "sell", "BG_C", []), rebuilt: rebuilt(3, 0, [kept]) }])).decisions[0];
  const a = sell.rebuilt.alternatives[0];
  I18n.setLang("en");
  assert.equal(W.reasonLine(sell, a, nameOf), "Turn 3: you sold Gamma. Had you kept it, you lose the next fight 43% of the time in the simulation, instead of 100%.");
  assert.equal(W.altLine(sell, a, nameOf), "Keeping Gamma: clearly better in the next fight, 43% lost instead of 100%.");
  assert.equal(W.tieLine(sell.rebuilt, a), "Ties: 57% instead of 0%.");
  I18n.setLang("es");
  assert.equal(W.reasonLine(sell, a, nameOf), "Turno 3: vendiste Gamma. Si te lo hubieras quedado, en la simulación pierdes el siguiente combate el 43 % de las veces en vez del 100 %.");
  assert.equal(W.altLine(sell, a, nameOf), "Quedarte Gamma: claramente mejor en el siguiente combate, 43 % de derrotas en vez del 100 %.");
  I18n.setLang("en");
});

test("ties and the expected place are shown only when there is something to show", () => {
  const withTies = alt("BG_B", 0.5, 0.4, { fight: { Ok: fight(0.5, { tied: 0.2, lost: 0.3, place: { expected: 3.4 } }) }, delta: delta(0.2, { place: -0.7 }) });
  const read = W.read(done([{ decision: decision(5, "buy", "BG_A", ["BG_A", "BG_B"]), rebuilt: rebuilt(5, 0.4, [withTies], { actual: fight(0.4, { place: { expected: 4.1 } }) }) }]));
  const d = read.decisions[0];
  const a = d.rebuilt.alternatives[0];
  assert.equal(W.tieLine(d.rebuilt, a), "Ties: 20% instead of 0%.");
  assert.equal(W.placeLine(d.rebuilt, a), "The expected-place model puts you at 3.4 instead of 4.1 going into that fight.");
  const noModel = W.read(done([{ decision: decision(5, "buy", "BG_A", ["BG_A", "BG_B"]), rebuilt: rebuilt(5, 0.4, [alt("BG_B", 0.6, 0.4)]) }])).decisions[0];
  assert.equal(W.placeLine(noModel.rebuilt, noModel.rebuilt.alternatives[0]), null, "no model: no place, never a number made up");
});

test("every choice in the full list says what happened to it in plain words", () => {
  const read = W.read(done([
    { decision: decision(2, "buy", "BG_A", ["BG_A", "BG_B", "BG_C", null]), rebuilt: rebuilt(2, 0.4, [
      alt("BG_B", 0.6, 0.4),
      alt("BG_C", 0.41, 0.4, { delta: delta(0.01, { clear: false }), stop: "close" }),
      alt("BG_X", 0.5, 0.4, { delta: delta(0.1, { clear: false }), stop: "max_trials", fight: { Ok: fight(0.5, { approximate: ["BG_A"] }) } }),
      alt("BG_Y", 0, 0.4, { fight: { Err: "start_of_combat" }, delta: null, stop: null }),
      alt("BG_Z", 0.1, 0.4),
    ], { unknown_offers: 1 }) },
    { decision: decision(3, "roll", null, []), rebuilt: { Err: "needs_shop_playout" } },
    { decision: decision(4, "tier_up", null, [], { tier: 3 }), rebuilt: { Err: "needs_shop_playout" } },
    { decision: decision(5, "buy", "BG_A", ["BG_A", "BG_B"]), rebuilt: { Err: { actual: "out_of_time" } } },
    { decision: decision(6, "buy", "BG_A", ["BG_A", "BG_B"]), rebuilt: { Err: { actual: "not_modelled" } } },
    { decision: decision(7, "choose", null, ["BG_A"]), rebuilt: { Err: "something_new" } },
  ]));
  I18n.setLang("en");
  const [buy, roll, tier, late, refused, unknown] = read.decisions;
  assert.equal(W.decisionLine(buy, nameOf), "Turn 2: you bought Alpha.");
  assert.equal(W.decisionLine(roll, nameOf), "Turn 3: you rolled.");
  assert.equal(W.decisionLine(tier, nameOf), "Turn 4: you upgraded the tavern to tier 3.");
  assert.equal(W.decisionLine(unknown, nameOf), "Turn 7: you picked a card the log does not name.");
  const alts = buy.rebuilt.alternatives.map((a) => W.altLine(buy, a, nameOf));
  assert.deepEqual(alts, [
    "Beta: clearly better in the next fight, 60% won instead of 40%.",
    "Gamma: about the same in the next fight (41% won instead of 40%).",
    "BG_X: no clear difference after 1000 simulated fights (50% won instead of 40%).",
    "BG_Y: not simulated, it acts at the start of combat, which is not simulated.",
    "BG_Z: clearly worse in the next fight, 10% won instead of 40%.",
  ]);
  assert.deepEqual(W.altNotes(buy.rebuilt.alternatives[2], nameOf), ["Simulated with a known gap: Alpha."]);
  assert.equal(W.unknownLine(buy.rebuilt), "1 card on offer is not named in the log and was not simulated.");
  assert.equal(W.notLine(roll), "Not simulated: it would need shops and gold the log never showed.");
  assert.equal(W.notLine(late), "Not reached in time.");
  assert.equal(W.notLine(refused), "Not simulated: your own board could not be simulated (a card on the boards is not in the simulator).");
  assert.equal(W.notLine(unknown), "Not simulated.", "a reason this build does not know");
  assert.equal(W.notLine({ notRebuilt: "no_board_before" }), "Not simulated: your board in the fight before was not seen, so what was new on it is not known.");
  const stalled = W.read(done([{ decision: decision(2, "buy", "BG_A", ["BG_A", "BG_B"]), rebuilt: rebuilt(2, 0.4, [alt("BG_B", 0.5, 0.4, { fight: { Ok: fight(0.5, { stalled: 3 }) } })]) }])).decisions[0];
  assert.deepEqual(W.altNotes(stalled.rebuilt.alternatives[0], nameOf), ["3 simulated fights hit the turn limit and count as ties."]);
  const kept = W.read(done([{ decision: decision(6, "sell", "BG_C", []), rebuilt: rebuilt(6, 0.3, [alt("BG_C", 0.5, 0.3)]) }])).decisions[0];
  assert.match(W.altLine(kept, kept.rebuilt.alternatives[0], nameOf), /^Keeping Gamma: clearly better/);
  I18n.setLang("es");
  assert.equal(W.notLine(late), "No se llegó a tiempo.");
  assert.match(W.altLine(buy, buy.rebuilt.alternatives[0], nameOf), /^Beta: claramente mejor en el siguiente combate, 60\s% de victorias en vez del 40\s%\.$/);
  I18n.setLang("en");
});

test("the summary is honest about a review that stopped partway or found nothing", () => {
  const costly = { decision: decision(2, "buy", "BG_A", ["BG_A", "BG_B"]), rebuilt: rebuilt(2, 0.4, [alt("BG_B", 0.6, 0.4)]) };
  const even = { decision: decision(3, "buy", "BG_A", ["BG_A", "BG_B"]), rebuilt: rebuilt(3, 0.4, [alt("BG_B", 0.41, 0.4, { delta: delta(0.01, { clear: false }), stop: "close" })]) };
  const roll = { decision: decision(3, "roll", null, []), rebuilt: { Err: "needs_shop_playout" } };
  I18n.setLang("en");
  const s = W.summary(W.read(done([costly, even, roll])));
  assert.equal(s.count, "2 of 2 choices simulated again.", "a roll is never simulated, so it is not counted, as in the progress");
  const skipped = { decision: decision(4, "buy", "BG_A", ["BG_A"]), rebuilt: { Err: "no_alternative" } };
  assert.equal(W.summary(W.read(done([costly, skipped, roll]))).count, "1 of 2 choices simulated again.");
  assert.equal(s.empty, null);
  assert.equal(s.banner, null);
  assert.equal(W.summary(W.read(done([even, roll]))).empty, "No choice clearly cost you the next fight.");
  const partial = W.summary(W.read(done([even, roll], { out_of_time: true })));
  assert.equal(partial.empty, "No choice reviewed in time clearly cost you the next fight.");
  assert.match(partial.banner, /^Time ran out partway through/);
  const stopped = W.summary(W.read({ ...done([even]), stopped_for_game: true }));
  assert.match(stopped.banner, /^A game started, so the review stopped/);
  assert.equal(stopped.empty, "No choice reviewed in time clearly cost you the next fight.");
  assert.match(W.summary(W.read(done([roll]))).empty, /could be simulated again/);
  assert.equal(W.progressLine({ done: 12, total: 40 }), "Reviewing… 12 of 40 choices simulated so far.");
  assert.equal(W.progressLine(null), null);
});
test("a sale that paid for the next buy is weighed against skipping that buy (English and Spanish)", () => {
  // Turn 3: sold Gamma, then bought Beta.
  const swap = alt("BG_C", 0.62, 0.44, { approx: ["earlier_stats", "battlecry_not_played"], instead_of: "BG_B" });
  const sell = W.read(done([{ decision: decision(3, "sell", "BG_C", []), rebuilt: rebuilt(3, 0.44, [swap]) }])).decisions[0];
  const a = sell.rebuilt.alternatives[0];
  assert.equal(a.insteadOf, "BG_B");
  I18n.setLang("en");
  assert.equal(W.reasonLine(sell, a, nameOf), "Turn 3: you sold Gamma and bought Beta. Had you kept Gamma instead of buying Beta, you win the next fight 62% of the time in the simulation, instead of 44%.");
  assert.equal(W.altLine(sell, a, nameOf), "Keeping Gamma instead of buying Beta: clearly better in the next fight, 62% won instead of 44%.");
  assert.deepEqual(W.top([sell]).map((c) => c.decision.turn), [3], "skipping the buy is always affordable: it may be ranked");
  I18n.setLang("es");
  assert.equal(W.reasonLine(sell, a, nameOf), "Turno 3: vendiste Gamma y compraste Beta. Si te hubieras quedado Gamma en vez de comprar Beta, en la simulación ganas el siguiente combate el 62 % de las veces en vez del 44 %.");
  assert.equal(W.altLine(sell, a, nameOf), "Quedarte Gamma en vez de comprar Beta: claramente mejor en el siguiente combate, 62 % de victorias en vez del 44 %.");
  I18n.setLang("en");

  // The after board: Gamma back where it was, Beta gone.
  const soldReport = { local_player_id: 1, rounds: [round(2, [minion("BG_C", 2, 2), minion("BG_A", 1, 1)]), round(3, [minion("BG_A", 5, 6), minion("BG_B", 3, 3)])] };
  const b = W.boards(soldReport, sell, a, {});
  assert.deepEqual(b.before.map((m) => m.card), ["BG_A", "BG_B"]);
  assert.deepEqual(b.after.map((m) => [m.card, m.atk]), [["BG_C", 2], ["BG_A", 5]], "the same number of minions, the buy gone");
  const gone = { local_player_id: 1, rounds: [round(2, [minion("BG_C", 2, 2)]), round(3, [minion("BG_A", 5, 6)])] };
  assert.equal(W.boards(gone, sell, a, {}), null, "the buy not on the board: no boards made up");
});

test("a sale whose gold may have paid for a roll or an upgrade is never ranked, and says why in the full list", () => {
  const short = alt("BG_C", 0.9, 0.3, { approx: ["earlier_stats", "gold_maybe_short"] });
  const plain = alt("BG_C", 0.5, 0.3, { approx: ["earlier_stats"] });
  const read = W.read(done([
    { decision: decision(3, "sell", "BG_C", []), rebuilt: rebuilt(3, 0.3, [short]) },
    { decision: decision(4, "sell", "BG_C", []), rebuilt: rebuilt(4, 0.3, [plain]) },
  ]));
  const [maybe, kept] = read.decisions;
  assert.equal(W.costOf(maybe), null, "however big, never among the costliest");
  assert.deepEqual(W.top(read.decisions).map((c) => c.decision.turn), [4]);
  I18n.setLang("en");
  const a = maybe.rebuilt.alternatives[0];
  assert.match(W.altLine(maybe, a, nameOf), /^Keeping Gamma: clearly better/, "still listed");
  const note = "After this sale you rolled, upgraded the tavern, bought a spell or froze the shop that turn: the gold from the sale may have paid for that, so keeping the minion may have left you short. It is not ranked among the choices that cost the most.";
  assert.deepEqual(W.altNotes(a, nameOf), [note], "with its note in the full list");
  assert.deepEqual(W.altNotes(kept.rebuilt.alternatives[0], nameOf), []);
  assert.ok(W.notes(maybe, a, nameOf).includes(note));
  I18n.setLang("es");
  assert.deepEqual(W.altNotes(a, nameOf), ["Después de esta venta refrescaste, mejoraste la taberna, compraste un hechizo o congelaste la tienda en ese mismo turno: puede que el oro de la venta pagara eso, así que quedarte el esbirro quizá te habría dejado sin oro suficiente. No cuenta entre las decisiones que más te costaron."]);
  I18n.setLang("en");
});

test("a sale whose next buy cannot be told apart says why it was not simulated", () => {
  I18n.setLang("en");
  assert.equal(W.notLine({ notRebuilt: "next_buy_not_on_board" }), "Not simulated: the minion you bought after this sale was not on your board in the next fight (in hand, sold, made golden…), or the log does not name it.");
  assert.equal(W.notLine({ notRebuilt: "order_unknown" }), "Not simulated: the log does not show whether you bought a minion with the gold from this sale.");
  I18n.setLang("es");
  assert.equal(W.notLine({ notRebuilt: "next_buy_not_on_board" }), "No simulada: el esbirro que compraste después de esta venta no estaba en tu tablero en el siguiente combate (en la mano, vendido, dorado…), o el registro no lo nombra.");
  assert.equal(W.notLine({ notRebuilt: "order_unknown" }), "No simulada: el registro no muestra si compraste un esbirro con el oro de esta venta.");
  I18n.setLang("en");
});
