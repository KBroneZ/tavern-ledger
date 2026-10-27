"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const I18n = require("../ui/i18n.js");
require("../ui/strings/common.js");
require("../ui/strings/review.js");
const W = require("../ui/review-words.js");
const TLReview = require("../ui/review.js");

// Just enough of an element for the section: tag, class, text, children.
function el(tag, cls, text) {
  const node = { tag, className: cls || "", textContent: text === undefined ? "" : String(text), children: [], listeners: {}, attrs: {}, hidden: false, disabled: false };
  node.append = (...kids) => kids.forEach((k) => node.children.push(typeof k === "string" ? { tag: "#text", textContent: k, children: [] } : k));
  node.addEventListener = (type, fn) => { node.listeners[type] = fn; };
  node.setAttribute = (k, v) => { node.attrs[k] = String(v); };
  return node;
}
const ui = { el };
const text = (n) => [n.textContent, ...(n.children || []).map(text)].join(" ");
const find = (n, cls) => (n.className.split(" ").includes(cls) ? n : (n.children || []).map((c) => find(c, cls)).find(Boolean));
const tools = (over) => ({ fold: (u, key, title) => { const f = el("details", "fold"); f.append(el("summary", null, title)); return f; }, game: "g", cardName: (id) => id, card: (u, id) => el("span", "shop-card", id), report: null, onStart() {}, ...over });

const fight = (won) => ({ won, tied: 0, lost: 1 - won, trials: 1000, place: "no_model", approximate: [] });
const costly = {
  decision: { turn: 2, ms: 1, kind: "buy", taken: "BG_A", offered: ["BG_A", "BG_B"], source: null, tier: null, inferred: false },
  rebuilt: { Ok: { round: 2, actual: fight(0.4), unknown_offers: 0, alternatives: [{ card: "BG_B", approx: ["base_stats"], fight: { Ok: fight(0.6) }, delta: { score: 0.2, standard_error: 0.01, clear: true, place: null }, stop: "clear" }] } },
};
const roll = { decision: { turn: 3, ms: 2, kind: "roll", taken: null, offered: [], source: null, tier: null, inferred: false }, rebuilt: { Err: "needs_shop_playout" } };
const answer = (decisions, over) => W.read({ state: "done", stopped_for_game: false, stats: { BG_B: [4, 4] }, review: { decisions, out_of_time: false, boards: 2, trials: 2000, ...over } });

test("idle: a button starts the review, with what it measures said first", () => {
  I18n.setLang("en");
  let started = 0;
  const part = TLReview.section(ui, "idle", tools({ onStart: () => { started += 1; } }));
  assert.match(text(part), /the next fight only/);
  const button = find(part, "review-start");
  assert.ok(button, "a start button");
  assert.equal(button.textContent, "Review my choices");
  button.listeners.click();
  assert.equal(started, 1);
});

test("reviewing, game on, busy, failed and no review each say so plainly", () => {
  I18n.setLang("en");
  assert.match(text(TLReview.section(ui, "reviewing", tools())), /Reviewing… simulating/);
  assert.equal(find(TLReview.section(ui, "reviewing", tools()), "review-start"), undefined, "no second start while reviewing");
  assert.match(text(TLReview.section(ui, { state: "game_on" }, tools())), /only between games/);
  assert.match(text(TLReview.section(ui, { state: "busy" }, tools())), /Another game is being reviewed/);
  const failed = TLReview.section(ui, "failed", tools());
  assert.match(text(failed), /could not be reviewed/);
  assert.ok(find(failed, "review-start"), "a failure can be tried again");
  assert.match(text(TLReview.section(ui, { state: "no_review", why: "not_solo" }, tools())), /Solo games/);
});

test("done: the costly choices with their reason and boards, every turn behind a fold", () => {
  I18n.setLang("en");
  const report = { local_player_id: 1, rounds: [{ number: 2, entries: [{ side: "own", player_id: 1, board: [{ card_id: "BG_A", atk: 1, health: 1 }] }] }] };
  const part = TLReview.section(ui, answer([costly, roll]), tools({ report }));
  const all = text(part);
  assert.match(all, /1 of 2 choices simulated again/);
  assert.match(all, /Turn 2: you bought BG_A\. Had you bought BG_B instead/);
  assert.match(all, /BG_B is simulated at its base stats/);
  assert.ok(find(part, "review-boards"), "before and after boards");
  const fold = find(part, "fold");
  assert.match(text(fold), /Show every turn/);
  assert.match(text(fold), /Turn 3: you rolled\./);
  assert.match(text(fold), /Not simulated: it would need shops and gold/);
  assert.doesNotMatch(all, /final place is|places? lost/i, "never the cost in final place");
});

test("done with nothing costly, or stopped partway, says so", () => {
  I18n.setLang("en");
  assert.match(text(TLReview.section(ui, answer([roll]), tools())), /No choice of this game could be simulated again/);
  const partial = TLReview.section(ui, answer([roll], { out_of_time: true }), tools());
  assert.match(text(partial), /Time ran out partway through/);
  assert.match(text(partial), /No choice reviewed in time/);
});
