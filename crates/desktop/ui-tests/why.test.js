"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const I18n = require("../ui/i18n.js");
require("../ui/strings/common.js");
require("../ui/strings/why.js");
const TLWhy = require("../ui/why.js");

const nameOf = (id) => (id === "BG_PLAIN" ? "Plain Card" : id);
const combat = (over) => ({ round: 3, result: "lost", odds: { won: 0.7, tied: 0, lost: 0.3 }, no_odds: null, cards: [], cause: { id: "matched" }, ...over });

test("the app's answer is read defensively: a combat out of shape is left out", () => {
  const read = TLWhy.lines([combat(), { round: "x", result: "won" }, combat({ result: "draw" }), null, combat({ odds: { won: 2, tied: 0, lost: 0 } })]);
  assert.equal(read.length, 2);
  assert.equal(read[1].odds, null, "odds out of range are not shown");
  assert.deepEqual(TLWhy.lines("not a list"), []);
  const bad = TLWhy.lines([combat({ cause: { id: "replay_stats", card: "../x", rule: "chance" } })])[0];
  assert.equal(bad.cause.card, null);
  assert.equal(bad.cause.rule, null);
});

test("each cause gets one plain line and its evidence, in English and Spanish", () => {
  const [chance, missing, approx, rule, noReplay, noData] = TLWhy.lines([
    combat({ cause: { id: "chance", attacks: 9 } }),
    combat({ cause: { id: "missing_inputs", attack: 4, rule: "replay_stats", card: "BG_PLAIN", inputs: [{ what: "blood_gems", card: "BG_PLAIN" }, { what: "blood_gems", card: "BG_OTHER" }] } }),
    combat({ cause: { id: "approximate_card", attack: 2, rule: "replay_minions", cards: ["BG_PLAIN"] } }),
    combat({ cause: { id: "replay_stats", attack: 4, rule: "replay_stats", card: "BG_PLAIN" } }),
    combat({ cause: { id: "no_replay_exact", replay: "not_saved" } }),
    combat({ odds: null, no_odds: "no_card_data", cards: ["BG_PLAIN"], cause: null }),
  ]);
  I18n.setLang("en");
  assert.equal(TLWhy.causeLine(chance, nameOf), "Chance: the replay matched until the dice.");
  assert.equal(TLWhy.causeLine(missing, nameOf), "Not seen: Blood Gem upgrades.", "each kind once");
  assert.equal(TLWhy.causeLine(approx, nameOf), "Approximate: Plain Card.");
  assert.equal(TLWhy.causeLine(rule, nameOf), "Rule differs at attack 4: Plain Card.");
  assert.equal(TLWhy.causeLine(noReplay, nameOf), "No replay saved, so no cause is given.");
  assert.equal(TLWhy.causeLine(noData, nameOf), "No odds: no card data for Plain Card.");
  assert.match(TLWhy.evidence(chance, nameOf)[0], /all 9 attacks/);
  assert.match(TLWhy.evidence(missing, nameOf)[0], /attack 4 \(stats and keywords\), on Plain Card/);
  assert.deepEqual(TLWhy.evidence(TLWhy.lines([combat()])[0], nameOf), []);
  I18n.setLang("es");
  assert.equal(TLWhy.causeLine(chance, nameOf), "Azar: la repetición coincidió hasta los dados.");
  assert.equal(TLWhy.causeLine(missing, nameOf), "No visto: las mejoras de las Gemas de sangre.");
  I18n.setLang("en");
});

test("the words match the website's game page (one vocabulary, D-127)", () => {
  const web = fs.readFileSync(path.join(__dirname, "..", "..", "..", "web", "src", "lib", "game", "strings.ts"), "utf8");
  const table = require("../ui/strings/why.js");
  for (const key of ["why.chance", "why.missing", "why.approximate", "why.rule", "why.matched", "why.replay.not_saved"]) {
    assert.ok(web.includes(`"${key}": ${JSON.stringify(table.en[key])}`), key);
  }
});
