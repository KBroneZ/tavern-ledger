// The simulator's data (T-606): the card data from HearthstoneJSON's shape,
// boards from a round of a stored game, the edits, the engine's request and
// the fixture round trip (report file → boards → engine → answer). Only
// synthetic cards and games (D-038, D-044).
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { extractCards } from "../../tools/simCards.ts";
import { CardIndex, fold, readCardFile } from "../../src/lib/sim/cards.ts";
import * as B from "../../src/lib/sim/board.ts";
import { readRecord } from "../../src/lib/game/report.ts";
import { loadEngine } from "../../src/lib/sim/engine.ts";
import { readAnswer } from "../../src/lib/sim/answer.ts";
import { WASM_PATH } from "../../tools/simStamp.ts";
import { duosReport, INDEX, record, SESSION, soloReport } from "./gameFixtures.ts";

const FIXTURES = join(import.meta.dirname, "..", "fixtures", "sim-cards");
const en = JSON.parse(readFileSync(join(FIXTURES, "enUS.json"), "utf8")) as unknown[];
const es = JSON.parse(readFileSync(join(FIXTURES, "esES.json"), "utf8")) as unknown[];
const cards = new CardIndex(extractCards(en, en, "fixture"));

/** A stored game with the engine's real card ids in place of the made-up ones. */
function game(report: Record<string, unknown>, revision?: number) {
  const text = JSON.stringify(report).replaceAll("SYN_M_01", "BGS_119").replaceAll("SYN_M_02", "BG28_300").replaceAll("SYN_M_04", "BG_ICC_026t")
    .replaceAll("SYN_M_05", "BG29_611").replaceAll("SYN_M_06", "BG25_009");
  const read = readRecord(record(JSON.parse(text), revision === undefined ? {} : { parser: { version: "0.1.0", revision } }), SESSION, INDEX);
  assert.ok(read.ok);
  return read.record;
}

test("the card data keeps Battlegrounds minions only, with facts, names and golden twins", () => {
  const file = extractCards(en, es, "fixture");
  const ids = file.cards.map((c) => c.i);
  assert.ok(ids.includes("BGS_119") && ids.includes("TB_BaconUps_159"));
  assert.ok(!ids.includes("CORE_SYN") && !ids.includes("BG_SPELL_SYN") && !ids.includes("BG_BAD_SYN"));
  const spark = file.cards.find((c) => c.i === "BGS_119");
  assert.deepEqual(spark, { i: "BGS_119", t: 1, a: 2, h: 1, r: ["ELEMENTAL"], k: ["DIVINE_SHIELD", "WINDFURY"], n: "Chispa sintética", p: 1, g: "TB_BaconUps_159" });
  // Only the engine's keywords: Deathrattle is card text.
  assert.deepEqual(file.cards.find((c) => c.i === "BG28_300")?.k, []);
  assert.throws(() => extractCards({}, [], "x"));
});

test("a card that fails a check is left out, never half-kept (as crates/cards)", () => {
  const base = { id: "BG_X", name: "X", type: "MINION", set: "BATTLEGROUNDS", attack: 1, health: 1 };
  const bad = [
    { ...base, techLevel: 8 },
    { ...base, techLevel: "1" },
    { ...base, races: ["beast"] },
    { ...base, races: "BEAST" },
    { ...base, races: ["A", "B", "C", "D", "E"] },
    { ...base, mechanics: "TAUNT" },
    { ...base, health: 0 },
    { ...base, attack: 1001 },
    { ...base, id: "BG X" },
    { ...base, name: "a‮b" },
    { ...base, name: "" },
  ];
  const unique = bad.map((c, i) => (c.id === "BG X" ? c : { ...c, id: `BG_X${i}` }));
  assert.deepEqual(extractCards(unique, unique, "x").cards, []);
  assert.equal(extractCards([{ ...base }], [], "x").cards[0].n, "X", "a name missing in the language falls back to English");
});

test("search finds cards by any word, without accents, tavern cards first, never golden twins", () => {
  assert.deepEqual(cards.search("spark").map((c) => c.i), ["BGS_119"]);
  assert.deepEqual(cards.search("SYNTHETIC skull").map((c) => c.i), ["BG28_300"]);
  assert.deepEqual(cards.search("").length, 0);
  const spanish = new CardIndex(extractCards(en, es, "fixture"));
  assert.deepEqual(spanish.search("craneo").map((c) => c.i), ["BG28_300"]);
  assert.equal(fold("Cráneo"), "craneo");
  // Tavern minions (p) first.
  const all = cards.search("synthetic");
  assert.ok(all.findIndex((c) => !c.p) > all.findLastIndex((c) => c.p === 1));
  assert.equal(cards.twin("BGS_119"), "TB_BaconUps_159");
  assert.equal(cards.twin("TB_BaconUps_159"), "BGS_119");
  assert.ok(cards.isGolden("BG28_300_G"));
  assert.equal(cards.name("NOPE"), "NOPE");
  assert.equal(readCardFile({ version: 2 }).cards.length, 0);
});

test("a Solo round gives both boards, the player's tier from the shop and the others unknown", () => {
  const rec = game(soloReport());
  assert.deepEqual(B.loadableRounds(rec).map((r) => r.number), [1, 2], "round 3 has no own board");
  const loaded = B.fromRound(rec, 2, cards);
  assert.ok(loaded);
  const { state, skipped } = loaded;
  assert.equal(state.mode, "solo");
  assert.equal(state.player.length, 1);
  assert.equal(state.player[0].tier, 2, "turn 2's shop tier");
  assert.equal(state.opponent[0].tier, null);
  assert.deepEqual(state.player[0].minions.map((m) => [m.cardId, m.attack, m.health, m.golden]), [["BGS_119", 2, 3, false], ["BG28_300", 3, 3, true]]);
  assert.deepEqual(state.player[0].minions[1].keywords, ["taunt", "divine_shield"]);
  // The opponent's minion with no card id is left out and counted.
  assert.equal(skipped, 1);
  // Poisonous is not saved: the card's own one is kept (BG29_611 has it printed).
  assert.deepEqual(state.opponent[0].minions[0].keywords, ["poisonous"]);
  assert.equal(B.fromRound(rec, 3, cards), null);
  assert.equal(B.fromRound(rec, 99, cards), null);
});

test("a Duos round gives each team's boards in the order they came in, one per player", () => {
  const rec = game(duosReport());
  const loaded = B.fromRound(rec, 1, cards);
  assert.ok(loaded);
  const { state } = loaded;
  assert.equal(state.mode, "duos");
  assert.deepEqual(state.player.map((b) => b.minions.map((m) => m.cardId)), [[], ["BGS_119"]]);
  // The game's second copy of player 4 (no board) is not a third board.
  assert.deepEqual(state.opponent.map((b) => b.minions.map((m) => m.cardId)), [["BG_ICC_026t"], ["BG29_611", "BG25_009"]]);
  // No shop in this record: every tier unknown.
  assert.ok([...state.player, ...state.opponent].every((b) => b.tier === null));
});

test("a player logged twice with a board counts once, and only the local player gets the shop's tier", () => {
  const report = duosReport() as { rounds: { entries: unknown[] }[]; shop: unknown };
  const mine = { side: "own", player_id: 2, hero: "SYN_HERO_A", board: [{ card_id: "SYN_M_01", atk: 1, health: 1, position: 0, golden: false, keywords: [] }] };
  report.rounds[0].entries = [mine, { ...mine }, ...report.rounds[0].entries.slice(1)];
  report.shop = { turns: [{ turn: 1, tier: 4 }], tier_ups: [], actions: [], start_ms: 0, end_ms: 1 };
  const loaded = B.fromRound(game(report), 1, cards);
  assert.ok(loaded);
  assert.deepEqual(loaded.state.player.map((b) => [b.tier, b.minions.length]), [[4, 1], [null, 1]]);
});

test("an edit that changes nothing gives the same state (no new run)", () => {
  let s = B.addMinion(B.emptyState(), "player", 0, "BGS_119", cards);
  const key = s.player[0].minions[0].key;
  assert.equal(B.setMode(s, "solo"), s);
  assert.equal(B.setCap(s, null), s);
  assert.equal(B.setTier(s, "player", 0, null), s);
  assert.equal(B.setStats(s, "player", 0, key, 2, 1), s);
  s = B.setCap(s, 10);
  assert.equal(B.setCap(s, 10), s);
});

test("edits return new states and keep the rules: seven minions, tiers 1 to 7, stats in range", () => {
  let s = B.emptyState();
  const before = s;
  for (let i = 0; i < 9; i++) s = B.addMinion(s, "player", 0, "BGS_119", cards);
  assert.equal(s.player[0].minions.length, 7);
  assert.equal(before.player[0].minions.length, 0, "the old state is untouched");
  assert.deepEqual(s.player[0].minions[0].keywords, ["divine_shield", "windfury"]);
  assert.equal(B.addMinion(s, "opponent", 0, "NOPE", cards), s);
  const [a, b] = s.player[0].minions;
  s = B.moveMinion(s, "player", 0, b.key, -1);
  assert.equal(s.player[0].minions[0].key, b.key);
  assert.equal(B.moveMinion(s, "player", 0, b.key, -1).player[0].minions[0].key, b.key, "the left end stays");
  s = B.setStats(s, "player", 0, a.key, -5, 0);
  assert.deepEqual([s.player[0].minions[1].attack, s.player[0].minions[1].health], [0, 1]);
  s = B.setStats(s, "player", 0, a.key, 1e12, NaN);
  assert.deepEqual([s.player[0].minions[1].attack, s.player[0].minions[1].health], [B.MAX_STAT, 1]);
  s = B.toggleKeyword(s, "player", 0, a.key, "taunt");
  assert.deepEqual(s.player[0].minions[1].keywords, ["taunt", "divine_shield", "windfury"], "in the engine's order");
  s = B.toggleKeyword(s, "player", 0, a.key, "taunt");
  assert.ok(!s.player[0].minions[1].keywords.includes("taunt"));
  s = B.removeMinion(s, "player", 0, a.key);
  assert.equal(s.player[0].minions.length, 6);
  assert.equal(B.setTier(s, "player", 0, 8).player[0].tier, null);
  assert.equal(B.setTier(s, "player", 0, 4).player[0].tier, 4);
  assert.equal(B.setCap(s, 0).cap, null);
  assert.equal(B.setCap(s, 15).cap, 15);
});

test("golden on and off swaps the card for its twin, with the twin's stats while untouched", () => {
  let s = B.addMinion(B.emptyState(), "player", 0, "BGS_119", cards);
  const key = s.player[0].minions[0].key;
  s = B.toggleGolden(s, "player", 0, key, cards);
  assert.deepEqual(s.player[0].minions[0], { ...s.player[0].minions[0], cardId: "TB_BaconUps_159", golden: true, attack: 4, health: 2 });
  s = B.setStats(s, "player", 0, key, 9, 9);
  s = B.toggleGolden(s, "player", 0, key, cards);
  assert.deepEqual([s.player[0].minions[0].cardId, s.player[0].minions[0].golden, s.player[0].minions[0].attack], ["BGS_119", false, 9], "edited stats stay");
});

test("Duos boards: a second board only in Duos, and Solo keeps the first", () => {
  let s = B.addBoard(B.emptyState(), "player");
  assert.equal(s.player.length, 1);
  s = B.addBoard(B.setMode(s, "duos"), "player");
  assert.equal(s.player.length, 2);
  assert.equal(B.addBoard(s, "player").player.length, 2);
  assert.equal(B.setMode(s, "solo").player.length, 1);
  assert.equal(B.removeBoard(s, "player", 1).player.length, 1);
  assert.equal(B.removeBoard(B.removeBoard(s, "player", 1), "player", 0).player.length, 1, "never no board");
});

test("the request carries every board in order with the card facts; an empty second board is no teammate", () => {
  let s = B.setMode(B.emptyState(), "duos");
  s = B.addBoard(B.addBoard(s, "player"), "opponent");
  s = B.addMinion(s, "player", 1, "BGS_119", cards);
  s = B.addMinion(s, "opponent", 0, "BG28_300", cards);
  s = B.setCap(B.setTier(s, "opponent", 0, 3), 15);
  const req = JSON.parse(B.toRequest(s, cards));
  assert.equal(req.player.length, 2, "the player's empty first board stays: their teammate fights");
  assert.equal(req.opponent.length, 1, "an empty second board is left out");
  assert.equal(req.damage_cap, 15);
  assert.deepEqual(req.opponent[0], {
    tier: 3,
    minions: [{ card_id: "BG28_300", golden: false, attack: 1, health: 1, keywords: [], gifts: [], facts: { tier: 1, tribes: ["UNDEAD"], attack: 1, health: 1, keywords: [] } }],
  });
  assert.equal(B.notReady(B.emptyState()), "empty");
  assert.equal(B.notReady(s), null);
});

test("Dark Gifts: added once each, at most four, removed, and sent with the minion", () => {
  const golemancy = "BG36_MidGameEffect_000t61";
  let s = B.addMinion(B.emptyState(), "player", 0, "BGS_119", cards);
  const key = s.player[0].minions[0].key;
  assert.deepEqual(s.player[0].minions[0].gifts, [], "a new minion has none");
  s = B.addGift(s, "player", 0, key, golemancy);
  assert.equal(B.addGift(s, "player", 0, key, golemancy), s, "the same gift once");
  assert.equal(B.addGift(s, "player", 0, key, "bad id"), s, "a bad id changes nothing");
  for (const g of ["G1", "G2", "G3", "G4"]) s = B.addGift(s, "player", 0, key, g);
  assert.deepEqual(s.player[0].minions[0].gifts, [golemancy, "G1", "G2", "G3"], "four at most");
  const req = JSON.parse(B.toRequest(s, cards));
  assert.deepEqual(req.player[0].minions[0].gifts, [golemancy, "G1", "G2", "G3"]);
  s = B.removeGift(s, "player", 0, key, "G1");
  assert.deepEqual(s.player[0].minions[0].gifts, [golemancy, "G2", "G3"]);
  assert.equal(B.removeGift(s, "player", 0, key, "G1"), s, "removing a missing gift changes nothing");
});

test("a minion takes any Attack and Health up to the 32-bit limit, and the built engine plays it (T-621)", async () => {
  const MAX = 2 ** 31 - 1;
  assert.equal(B.MAX_STAT, MAX);
  let s = B.addMinion(B.emptyState(), "player", 0, "BGS_119", cards);
  s = B.addMinion(s, "opponent", 0, "BG_ICC_026t", cards);
  const key = s.player[0].minions[0].key;
  for (const [typed, kept] of [[99_999, 99_999], [100_000, 100_000], [MAX - 1, MAX - 1], [MAX, MAX], [MAX + 1, MAX], [1e12, MAX]]) {
    const next = B.setStats(s, "player", 0, key, typed, typed);
    assert.deepEqual([next.player[0].minions[0].attack, next.player[0].minions[0].health], [kept, kept], String(typed));
  }
  const odd = B.setStats(s, "player", 0, key, Infinity, NaN);
  assert.deepEqual([odd.player[0].minions[0].attack, odd.player[0].minions[0].health], [0, 1], "not a number: the lowest value");
  // A stored game's huge minion is loaded whole, not cut to 99,999.
  const report = soloReport();
  const board = (report.rounds as { entries: { side: string; board: { atk: number; health: number }[] }[] }[])[1].entries.find((e) => e.side === "own")!.board;
  board[0].atk = MAX;
  board[0].health = 123_456;
  const loaded = B.fromRound(game(report), 2, cards);
  assert.ok(loaded);
  assert.deepEqual([loaded.state.player[0].minions[0].attack, loaded.state.player[0].minions[0].health], [MAX, 123_456]);
  // The engine takes the request and gives the odds, with no wrap.
  const engine = await loadEngine(readFileSync(WASM_PATH));
  s = B.setStats(s, "player", 0, key, MAX, MAX);
  const answer = readAnswer(engine.run(B.toRequest(s, cards)));
  assert.ok(answer.kind === "odds" && answer.won === answer.trials, JSON.stringify(answer));
});

test("a loaded round keeps its game's build, the request sends it and the built engine warns on a build it was not checked against", async () => {
  const engine = await loadEngine(readFileSync(WASM_PATH));
  const play = (build: number | null) => {
    const loaded = B.fromRound(game({ ...soloReport(), build }), 2, cards);
    assert.ok(loaded);
    assert.equal(loaded.state.build, build);
    const request = JSON.parse(B.toRequest(loaded.state, cards));
    assert.equal(request.build, build);
    return readAnswer(engine.run(B.toRequest(loaded.state, cards)));
  };
  const fresh = play(999_999);
  assert.equal(fresh.kind, "odds", "the odds still come back");
  assert.equal(fresh.kind === "odds" && fresh.newBuild, 999_999);
  for (const checked of [253_216, 254_477]) assert.equal((play(checked) as { newBuild: number | null }).newBuild, null, `build ${checked}`);
  assert.equal((play(null) as { newBuild: number | null }).newBuild, null, "a file that does not say its build gets no warning");
  // Boards built by hand have no build; edits keep the loaded one.
  assert.equal(B.emptyState().build, null);
  const loaded = B.fromRound(game({ ...soloReport(), build: 254_477 }), 2, cards);
  assert.ok(loaded);
  assert.equal(B.setCap(loaded.state, 15).build, 254_477);
});

test("round trip: a stored game's round, played by the built engine, gives a readable answer", async () => {
  const engine = await loadEngine(readFileSync(WASM_PATH));
  const loaded = B.fromRound(game(soloReport()), 2, cards);
  assert.ok(loaded);
  const answer = readAnswer(engine.run(B.toRequest(loaded.state, cards)));
  assert.equal(answer.kind, "odds");
  if (answer.kind !== "odds") return;
  assert.equal(answer.trials, 10_000);
  assert.equal(answer.won + answer.tied + answer.lost, 10_000);
  // The opponent's tier is unknown: when they can win, no damage number (never 0).
  assert.equal(answer.damageKnown, answer.lost === 0);
  // A card the engine does not know is named, and no odds are given.
  const blocked = B.addMinion(loaded.state, "opponent", 0, "BG_NOT_A_CARD", cards);
  const refused = readAnswer(engine.run(B.toRequest(blocked, cards)));
  assert.deepEqual(refused.kind === "not_modelled" && refused.notModelled, ["BG_NOT_A_CARD"]);
  // A Dark Gift the engine does not know blocks the same way, by its id.
  const own = loaded.state.player[0].minions[0];
  const gifted = B.addGift(loaded.state, "player", 0, own.key, "BG99_NOT_A_GIFT");
  const unknownGift = readAnswer(engine.run(B.toRequest(gifted, cards)));
  assert.deepEqual(unknownGift.kind === "not_modelled" && unknownGift.notModelled, ["BG99_NOT_A_GIFT"]);
  // The Duos round plays both teams' boards.
  const duos = B.fromRound(game(duosReport()), 1, cards);
  assert.ok(duos);
  assert.equal(readAnswer(engine.run(B.toRequest(duos.state, cards))).kind, "odds");
});

test("a loaded round keeps each minion's Dark Gifts and sends them to the engine (T-528)", () => {
  const report = soloReport() as Record<string, any>;
  const own = report.rounds[1].entries[0].board;
  own[1].gifts = ["BG36_MidGameEffect_000t9", "BG36_MidGameEffect_000t13"];
  own[0].gift_unknown = true;
  const loaded = B.fromRound(game(report), 2, cards);
  assert.ok(loaded);
  const minions = loaded.state.player[0].minions;
  assert.deepEqual(minions[1].gifts, ["BG36_MidGameEffect_000t9", "BG36_MidGameEffect_000t13"]);
  assert.deepEqual(minions[0].gifts, [], "an unknown gift is not sent: the engine plays the plain card");
  assert.equal(loaded.unknownGifts, 1, "but it is counted, so the page says the odds are approximate");
  assert.equal(loaded.giftsNotRecorded, true, "the test record is an older revision");
  const current = game(report);
  const newer = B.fromRound({ ...current, revision: B.GIFTS_REVISION }, 2, cards);
  assert.equal(newer?.giftsNotRecorded, false);
  const request = JSON.parse(B.toRequest(loaded.state, cards));
  const sent = request.player[0].minions[1];
  assert.deepEqual(sent.gifts, ["BG36_MidGameEffect_000t9", "BG36_MidGameEffect_000t13"]);
});

test("a loaded round keeps each minion's buffs for its tooltip, a new minion has none, the engine never sees them (T-619)", () => {
  const report = soloReport() as Record<string, any>;
  report.rounds[1].entries[0].board[1].buffs = [{ card_id: "BG20_GEM", count: 2 }];
  const older = B.fromRound(game(report), 2, cards);
  assert.ok(older);
  assert.equal(older.state.player[0].minions[1].buffs, null, "a record before revision 14 did not read them");
  const loaded = B.fromRound(game(report, 14), 2, cards);
  assert.ok(loaded);
  assert.deepEqual(loaded.state.player[0].minions[1].buffs, [{ cardId: "BG20_GEM", count: 2 }]);
  assert.deepEqual(loaded.state.player[0].minions[0].buffs, [], "recorded, none");
  assert.deepEqual(B.addMinion(B.emptyState(), "player", 0, "BGS_119", cards).player[0].minions[0].buffs, [], "a minion added by hand has none");
  assert.doesNotMatch(B.toRequest(loaded.state, cards), /buffs|BG20_GEM/);
});

test("a loaded round keeps each board's game-long values and sends them to the engine (T-610)", () => {
  const report = soloReport() as Record<string, any>;
  report.rounds[1].entries[0].values = { blood_gem_atk: 2, blood_gem_health: 1, knights_dead: 3 };
  const loaded = B.fromRound(game(report), 2, cards);
  assert.ok(loaded);
  assert.deepEqual(loaded.state.player[0].values, { blood_gem_atk: 2, blood_gem_health: 1, knights_dead: 3 });
  assert.equal(loaded.state.opponent[0].values, undefined, "none saved: the printed numbers, as before");
  // Editing the board keeps its hero's values.
  const minion = loaded.state.player[0].minions[0];
  const edited = B.removeMinion(loaded.state, "player", 0, minion.key);
  assert.deepEqual(edited.player[0].values, loaded.state.player[0].values);
  const request = JSON.parse(B.toRequest(edited, cards));
  assert.deepEqual(request.player[0].values, { blood_gem_atk: 2, blood_gem_health: 1, knights_dead: 3 });
  assert.equal("values" in request.opponent[0], false, "a board without values sends none");
  // A board started in the editor has none.
  assert.equal("values" in JSON.parse(B.toRequest(B.emptyState(), cards)).player[0], false);
});
