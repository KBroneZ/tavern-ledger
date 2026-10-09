// Run: deno test supabase/functions/upload-game/
// What an uploaded record may hold (T-104d, D-012, D-017).
import { broken, duo, type Json, record, SESSION, solo } from "./testdata.ts";
import { parsePath, validateRecord } from "./validate.ts";

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
