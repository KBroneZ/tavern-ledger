// Run: deno test supabase/functions/upload-game/
// What an uploaded record may hold (T-104d, D-012, D-017).
import {
  broken,
  dataGames,
  duo,
  type Json,
  realDuos,
  realRecord,
  realSolo,
  record,
  shopGames,
  SESSION,
  solo,
} from "./testdata.ts";
import { parsePath, validateRecord, VALIDATOR_VERSION } from "./validate.ts";

function assertEquals(actual: unknown, expected: unknown, msg = ""): void {
  const a = JSON.stringify(actual);
  const e = JSON.stringify(expected);
  if (a !== e) throw new Error(`${msg}\n  actual:   ${a}\n  expected: ${e}`);
}

function refused(rec: Json, code: string, field: string, msg = "") {
  const v = validateRecord(rec, SESSION, 1);
  assertEquals(v.ok ? "accepted" : [v.code, v.field], [code, field], msg);
}

Deno.test("a real Duos record is accepted and summarised", () => {
  const v = validateRecord(record(), SESSION, 1);
  if (!v.ok) throw new Error(`refused: ${v.code} ${v.field}`);
  assertEquals(v.summary, {
    game_type: "GT_BATTLEGROUNDS_DUO",
    status: "ok",
    hero_card_id: duo[0].hero,
    final_place: 1,
    build: duo[0].build,
    played_on: "2026-10-09",
    tribes_offered: { BEAST: 4, RACE_99: 1 },
    saved_at: 1791000000,
    parser_version: "0.1.0+r1",
  });
});

Deno.test("real Solo, unsupported and old records are accepted", () => {
  for (const report of [solo[0], broken[0], broken[1]]) {
    const rec = record({ report: { ...report, index: 1 } });
    delete rec.parser; // records saved before T-107 have no parser stamp
    const v = validateRecord(rec, SESSION, 1);
    assertEquals(v.ok, true, JSON.stringify(v));
    if (v.ok) assertEquals(v.summary.parser_version, null);
  }
});

Deno.test("real games from the current parser (revision 4) are accepted", () => {
  for (const report of [realDuos[0], realSolo[0]]) {
    assertEquals(report.shop.turns.length > 0, true, "the fixture has a shop record");
    assertEquals(report.hero_select.offered.length > 0, true, "the fixture has a hero pick");
    assertEquals(report.rounds.every((r: Json) => r.result !== null), true, "results");
    const v = validateRecord(realRecord(report), SESSION, 1);
    if (!v.ok) throw new Error(`refused: ${v.code} ${v.field}`);
    assertEquals(v.summary.parser_version, "0.1.0+r4");
    assertEquals(v.summary.hero_card_id, report.hero);
  }
});

Deno.test("synthetic games with every shop action kind are accepted", () => {
  for (const report of shopGames) {
    assertEquals(report.shop.actions.length > 0, true, "the fixture has actions");
    const v = validateRecord(realRecord(report), SESSION, 1);
    if (!v.ok) throw new Error(`refused: ${v.code} ${v.field}`);
  }
});

Deno.test("synthetic games with every revision 4 field are accepted", () => {
  for (const report of dataGames) {
    const v = validateRecord(realRecord(report), SESSION, 1);
    if (!v.ok) throw new Error(`refused: ${v.code} ${v.field}`);
  }
  const kinds = dataGames.flatMap((g: Json) => g.shop.actions.map((a: Json) => a.kind));
  assertEquals(kinds.includes("pass"), true, "a pass to the teammate");
});

Deno.test("revision 3 records, without the revision 4 fields, are still accepted", () => {
  const rec = realRecord(realSolo[0]);
  rec.parser.revision = 3;
  delete rec.report.hero_select;
  delete rec.report.skin_parents;
  rec.report.rounds.forEach((r: Json) => delete r.result);
  for (const turn of rec.report.shop.turns) {
    for (const key of ["extra_gold", "sell_gold", "buy_gold", "spell_gold", "free_refreshes", "passes"]) {
      delete turn[key];
    }
  }
  rec.report.shop.actions.forEach((a: Json) => a.kind = a.kind === "pass" ? "other" : a.kind);
  const v = validateRecord(rec, SESSION, 1);
  if (!v.ok) throw new Error(`refused: ${v.code} ${v.field}`);
  // Unknown is allowed too: no result, no pick, a turn without gold numbers.
  const unknown = realRecord(realDuos[0]);
  unknown.report.rounds[0].result = null;
  unknown.report.hero_select = null;
  unknown.report.skin_parents = {};
  unknown.report.shop.turns[0].extra_gold = null;
  unknown.report.shop.turns[0].passes = null;
  assertEquals(validateRecord(unknown, SESSION, 1).ok, true);
});

Deno.test("the revision 4 fields keep to their shapes and bounds", () => {
  const hero = (r: Json) => r.hero_select.offered[0];
  const cases: Array<[(r: Json) => void, string]> = [
    [(r) => (r.rounds[0].result = "draw"), "report.rounds[0].result"],
    [(r) => (r.rounds[0].result = 1), "report.rounds[0].result"],
    [(r) => (r.hero_select.player = "Someone"), "report.hero_select.player"],
    [(r) => (r.hero_select.offered = new Array(33).fill(hero(r))), "report.hero_select.offered"],
    [(r) => (r.hero_select.offered[1] = null), "report.hero_select.offered[1]"],
    [(r) => (r.hero_select.offered[1] = "<b>x</b>"), "report.hero_select.offered[1]"],
    [(r) => (r.hero_select.rerolls = -1), "report.hero_select.rerolls"],
    [(r) => (r.hero_select.rerolls = 33), "report.hero_select.rerolls"],
    [(r) => (r.hero_select.picked = "BG_NOT_OFFERED"), "report.hero_select.picked"],
    [(r) => delete r.hero_select.offered, "report.hero_select.offered"],
    [(r) => (r.skin_parents = { BG_NOT_SHOWN_SKIN_A: 5 }), "report.skin_parents"],
    [(r) => (r.skin_parents = { [hero(r)]: 0 }), "report.skin_parents"],
    [(r) => (r.skin_parents = { [hero(r)]: 10000001 }), "report.skin_parents"],
    [(r) => (r.skin_parents = { [hero(r)]: "60011" }), "report.skin_parents"],
    [(r) => (r.skin_parents = [hero(r)]), "report.skin_parents"],
    [(r) => (r.shop.turns[0].extra_gold = -1), "report.shop.turns[0].extra_gold"],
    [(r) => (r.shop.turns[0].sell_gold = 10001), "report.shop.turns[0].sell_gold"],
    [(r) => (r.shop.turns[0].buy_gold = 1.5), "report.shop.turns[0].buy_gold"],
    [(r) => (r.shop.turns[0].spell_gold = "2"), "report.shop.turns[0].spell_gold"],
    [
      (r) => (r.shop.turns[0].free_refreshes = r.shop.turns[0].rolls + 1),
      "report.shop.turns[0].free_refreshes",
    ],
    [(r) => (r.shop.turns[0].passes = ["BG_1", 7]), "report.shop.turns[0].passes[1]"],
    [(r) => (r.shop.turns[0].passes = new Array(101).fill("BG_1")), "report.shop.turns[0].passes"],
  ];
  for (const [forge, field] of cases) {
    const rec = realRecord(realSolo[0]);
    forge(rec.report);
    refused(rec, "invalid_report", field);
  }
  // Offered heroes are heroes the report shows: their names may come along.
  const named = realRecord(realSolo[0]);
  named.report.card_names = { [hero(named.report)]: "Héroe" };
  assertEquals(validateRecord(named, SESSION, 1).ok, true);
});

Deno.test("records without a shop (older parsers) or with none (unreadable game) are accepted", () => {
  const old = realRecord(realSolo[0]);
  delete old.report.shop;
  assertEquals(validateRecord(old, SESSION, 1).ok, true);
  const none = realRecord(realSolo[0]);
  none.report.shop = null;
  assertEquals(validateRecord(none, SESSION, 1).ok, true);
});

Deno.test("the shop keeps to its shapes and bounds", () => {
  const cases: Array<[(s: Json) => void, string]> = [
    [(s) => (s.player = "Someone"), "report.shop.player"],
    [(s) => (s.turns[0].opponent_shop = []), "report.shop.turns[0].opponent_shop"],
    [(s) => (s.turns[0].offers[0].card_id = "<b>x</b>"), "report.shop.turns[0].offers[0].card_id"],
    [(s) => (s.turns[0].offers[0].name = "Card"), "report.shop.turns[0].offers[0].name"],
    [(s) => (s.turns[0].offers[0].roll = 99), "report.shop.turns[0].offers[0].roll"],
    [(s) => (s.turns[0].free_rolls = s.turns[0].rolls + 1), "report.shop.turns[0].free_rolls"],
    [(s) => (s.turns[0].buys = ["BG_1", 7]), "report.shop.turns[0].buys[1]"],
    [(s) => (s.turns[0].gold = -1), "report.shop.turns[0].gold"],
    [(s) => (s.turns[0].tier = 8), "report.shop.turns[0].tier"],
    [(s) => (s.turns[0].end_ms = s.turns[0].start_ms - 1), "report.shop.turns[0].end_ms"],
    [(s) => delete s.turns[0].rolls, "report.shop.turns[0].rolls"],
    [(s) => (s.tier_ups[0].tier = 1), "report.shop.tier_ups[0].tier"],
    [(s) => (s.actions[0].kind = "typed_text"), "report.shop.actions[0].kind"],
    [(s) => (s.actions[0].ms = 24 * 60 * 60 * 1000 + 1), "report.shop.actions[0].ms"],
    [(s) => (s.actions = new Array(3001).fill(s.actions[0])), "report.shop.actions"],
    [(s) => (s.ended = "yes"), "report.shop.ended"],
    [(s) => (s.turns = new Array(8).fill(null).map((_, i) => ({
      ...s.turns[0],
      turn: i + 1,
      offers: new Array(400).fill(s.turns[0].offers[0]),
    }))), "report.shop.turns"],
  ];
  for (const [forge, field] of cases) {
    const rec = realRecord(realSolo[0]);
    forge(rec.report.shop);
    refused(rec, "invalid_report", field);
  }
  const named = realRecord(realSolo[0]);
  named.report.shop.turns[0].offers[0].card_id = "Someone#1234";
  const v = validateRecord(named, SESSION, 1);
  assertEquals(v.ok ? "accepted" : v.code, "player_name");
});

Deno.test("health maps may be empty (unknown) or missing (older records)", () => {
  const rec = realRecord(realDuos[0]);
  rec.report.start_health = {};
  rec.report.rounds[0].health_after = {};
  delete rec.report.rounds[1].health_after;
  delete rec.report.start_health;
  assertEquals(validateRecord(rec, SESSION, 1).ok, true);
});

Deno.test("health maps hold only players of this game and sane health", () => {
  const sixteen = Object.fromEntries(Array.from({ length: 17 }, (_, i) => [String(i + 1), 30]));
  const bad: Array<[unknown, string]> = [
    [null, "null"],
    [[30, 30], "an array"],
    ["30", "a string"],
    [{ "9": 30 }, "a player not in the game"],
    [{ "0": 30 }, "player 0"],
    [{ "01": 30 }, "a padded id"],
    [{ "1.0": 30 }, "a decimal id"],
    [{ "Someone": 30 }, "a name as key"],
    [{ "1": -1 }, "negative health"],
    [{ "1": 100001 }, "absurd health"],
    [{ "1": 30.5 }, "a fraction"],
    [{ "1": "30" }, "health as text"],
    [{ "1": null }, "null health"],
    [{ "1": { "hp": 30 } }, "an object as health"],
    [sixteen, "more than 16 entries"],
  ];
  for (const [value, why] of bad) {
    const start = realRecord(realDuos[0]);
    start.report.start_health = value;
    refused(start, "invalid_report", "report.start_health", `start_health: ${why}`);
    const after = realRecord(realDuos[0]);
    after.report.rounds[2].health_after = value;
    refused(after, "invalid_report", "report.rounds[2].health_after", `health_after: ${why}`);
  }
});

Deno.test("a player id counts when the report shows it anywhere", () => {
  // Solo lobby is 1-8; drop player 8 from the lobby but keep it in a combat.
  const rec = realRecord(realSolo[0]);
  const pid = rec.report.rounds[0].entries.find((e: Json) => e.side === "opponent").player_id;
  rec.report.lobby = rec.report.lobby.filter((p: Json) => p.player_id !== pid);
  assertEquals(validateRecord(rec, SESSION, 1).ok, true, "a player seen in combat");
  rec.report.rounds.forEach((r: Json) =>
    r.entries.forEach((e: Json) => {
      if (e.player_id === pid) e.player_id = null;
    })
  );
  refused(rec, "invalid_report", "report.start_health", "a player seen nowhere");
});

Deno.test("a name in a health map is refused as a player name", () => {
  const rec = realRecord(realDuos[0]);
  rec.report.start_health = { "Someone#1234": 30 };
  const v = validateRecord(rec, SESSION, 1);
  assertEquals(v.ok ? "accepted" : v.code, "player_name");
});

Deno.test("the validator version is a positive integer, 4 or more since revision 4", () => {
  assertEquals(Number.isInteger(VALIDATOR_VERSION) && VALIDATOR_VERSION >= 4, true);
});

Deno.test("session and index must match the address", () => {
  refused(record({ session: "Hearthstone_2026_10_09_18_30_01" }), "invalid_report", "session");
  refused(record({ index: 2 }), "invalid_report", "index");
  const rec = record();
  rec.report.index = 2;
  refused(rec, "invalid_report", "report.index");
});

Deno.test("a BattleTag-shaped string anywhere is refused as a player name", () => {
  const places: Array<(r: Json) => void> = [
    (r) => (r.report.card_names[r.report.hero] = "Someone#1234"),
    (r) => (r.report.card_names[r.report.hero] = "Someone＃12345"),
    (r) => (r.report.warnings = ["build 1 not tested Someone#1234"]),
    (r) => (r.report.rounds[0].entries[0].hero = "Someone#1234"),
    (r) => (r.report.hero = "Player #4321"),
  ];
  for (const forge of places) {
    const rec = record();
    forge(rec);
    const v = validateRecord(rec, SESSION, 1);
    assertEquals(v.ok ? "accepted" : v.code, "player_name", forge.toString());
  }
});

Deno.test("fields outside the allowed set are refused", () => {
  const extra: Array<[(r: Json) => void, string]> = [
    [(r) => (r.mmr = 8000), "mmr"],
    [(r) => (r.report.rating = 8000), "report.rating"],
    [(r) => (r.report.player_name = "Someone"), "report.player_name"],
    [(r) => (r.report.lobby[0].name = "Someone"), "report.lobby[0].name"],
    [(r) => (r.report.lobby[0].battle_tag = "Someone"), "report.lobby[0].battle_tag"],
    [(r) => (r.report.rounds[0].entries[0].account = 1), "report.rounds[0].entries[0].account"],
    [(r) => (r.parser.extra = 1), "parser.extra"],
    [(r) => (r.report.rounds[0].entries[0].board = [{
      card_id: "X",
      atk: 1,
      health: 1,
      position: 0,
      golden: false,
      name: "Bob",
    }]), "report.rounds[0].entries[0].board[0].name"],
  ];
  for (const [forge, field] of extra) {
    const rec = record();
    forge(rec);
    refused(rec, "invalid_report", field);
  }
});

Deno.test("hero names only for heroes the report shows, and only name-like text", () => {
  const rec = record();
  rec.report.card_names.BG99_NOT_IN_GAME = "Someone";
  refused(rec, "invalid_report", "report.card_names");
  const bad = ["", " ", "a".repeat(65), "line\nbreak", "<b>x</b>", "x@example.test", "tab\tx"];
  for (const name of bad) {
    const r = record();
    r.report.card_names[r.report.hero] = name;
    refused(r, "invalid_report", "report.card_names", JSON.stringify(name));
  }
  const ok = [
    "Mr. Bigglesworth",
    "N'Zoth",
    "E.T.C., Band Manager",
    "瓦托格尔女王",
    "ヨグ＝サロン",
    "Ини",
  ];
  for (const name of ok) {
    const r = record();
    r.report.card_names[r.report.hero] = name;
    assertEquals(validateRecord(r, SESSION, 1).ok, true, name);
  }
});

Deno.test("ids, numbers and free text keep to their shapes", () => {
  const cases: Array<[(r: Json) => void, string]> = [
    [(r) => (r.report.hero = "not an id"), "report.hero"],
    [(r) => (r.report.status = "not_battlegrounds"), "report.status"],
    [(r) => (r.report.game_type = "GT_BATTLEGROUNDS_FRIENDLY"), "report.game_type"],
    [(r) => (r.report.final_place = 5), "report.final_place"],
    [(r) => (r.report.final_place = 1.5), "report.final_place"],
    [(r) => (r.report.build = "253216"), "report.build"],
    [(r) => (r.report.lobby[0].player_id = 1e12), "report.lobby[0].player_id"],
    [(r) => (r.report.shop_tribes = { "beast": 1 }), "report.shop_tribes"],
    [(r) => (r.report.shop_tribes = { BEAST: -1 }), "report.shop_tribes"],
    [(r) => (r.report.warnings = ["<script>"]), "report.warnings[0]"],
    [(r) => (r.report.not_in_log = ["MMR", "rating 8000"]), "report.not_in_log[1]"],
    [(r) => (r.report.rounds[0].entries[0].side = "both"), "report.rounds[0].entries[0].side"],
    [(r) => (r.report.lobby = Array(17).fill(r.report.lobby[0])), "report.lobby"],
    [(r) => (r.saved_at = -1), "saved_at"],
    [(r) => (r.parser = { version: "latest", revision: 1 }), "parser.version"],
    [(r) => (r.report = []), "report"],
  ];
  for (const [forge, field] of cases) {
    const rec = record();
    forge(rec);
    refused(rec, "invalid_report", field, forge.toString());
  }
  refused("not an object", "invalid_report", "record");
  refused(record({ session: undefined }), "invalid_report", "session");
});

Deno.test("Solo places go up to 8, Duos places up to 4", () => {
  const rec = record({ report: { ...solo[0], index: 1, final_place: 8 } });
  assertEquals(validateRecord(rec, SESSION, 1).ok, true);
  const duos = record();
  duos.report.final_place = 4;
  assertEquals(validateRecord(duos, SESSION, 1).ok, true);
});

Deno.test("a session name with an impossible date is refused", () => {
  const session = "Hearthstone_2026_02_31_18_30_00";
  const v = validateRecord(record({ session }), session, 1);
  assertEquals(v.ok ? "accepted" : v.field, "session");
});

Deno.test("only /v1/games/<session>/<index> with a strict session and index", () => {
  const base = "http://fn.test/upload-game/v1/games";
  assertEquals(parsePath(`${base}/${SESSION}/1`), { session: SESSION, index: 1 });
  assertEquals(parsePath(`${base}/${SESSION}/1000`), { session: SESSION, index: 1000 });
  for (
    const bad of [
      `${base}/${SESSION}/0`,
      `${base}/${SESSION}/1001`,
      `${base}/${SESSION}/01`,
      `${base}/${SESSION}/1/`,
      `${base}/${SESSION}/1?x=1`,
      `${base}/Hearthstone_2026_10_09_18_30/1`,
      `${base}/..%2F${SESSION}/1`,
      `${base}/${SESSION}%2F../1`,
      `http://fn.test/upload-game/v1/game/${SESSION}/1`,
      `${base}/${SESSION}`,
    ]
  ) {
    assertEquals(parsePath(bad), null, bad);
  }
});
