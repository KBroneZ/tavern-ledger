import { test } from "node:test";
import assert from "node:assert/strict";
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { EXPIRED_COLUMNS, expiredHeading, loadExpired, type ReadSummary, renderExpired } from "../../src/lib/game/expired.ts";
import { GAME_STRINGS } from "../../src/lib/game/strings.ts";
import type { VNode } from "../../src/lib/view/node.ts";
import { INDEX, SESSION, USER } from "./gameFixtures.ts";

const REF = { userId: USER, session: SESSION, index: INDEX };
const ROOT = join(import.meta.dirname, "..", "..", "..");

const ROW = {
  session: SESSION,
  game_index: INDEX,
  mode: "solo",
  status: "ok",
  played_on: "2026-08-01",
  hero: "BG22_HERO_002",
  counted_place: 3,
  rounds: 12,
  combats_won: 7,
  combats_lost: 4,
  combats_inferred: 0,
  triples: 2,
  apm: 9.5,
  gold_spent: 110,
  file_expired_at: "2026-09-01T03:17:00+00:00",
};

function serve(data: unknown, error: unknown = null): { read: ReadSummary; refs: unknown[] } {
  const refs: unknown[] = [];
  return {
    refs,
    read: async (ref) => {
      refs.push(ref);
      return { data, error };
    },
  };
}

function text(node: VNode | string | null): string {
  if (node === null) return "";
  if (typeof node === "string") return node;
  return node.children.map((c) => text(c as VNode | string | null)).join(" ");
}

test("expired: a row whose file has expired gives its summary", async () => {
  const { read, refs } = serve([ROW]);
  const result = await loadExpired(read, REF);
  assert.equal(result.kind, "expired");
  assert.deepEqual(refs, [REF]);
  if (result.kind !== "expired") return;
  assert.equal(result.row.place, 3);
  assert.equal(result.row.rounds, 12);
  assert.equal(result.row.playedOn, "2026-08-01");
});

test("expired: no row, or a row whose file is kept, is not expired", async () => {
  for (const data of [[], [{ ...ROW, file_expired_at: null }], [{ ...ROW, file_expired_at: "" }], [null], ["x"]]) {
    assert.deepEqual(await loadExpired(serve(data).read, REF), { kind: "none" }, JSON.stringify(data));
  }
});

test("expired: a row for another game is not this one", async () => {
  assert.deepEqual(await loadExpired(serve([{ ...ROW, game_index: INDEX + 1 }]).read, REF), { kind: "none" });
  assert.deepEqual(await loadExpired(serve([{ ...ROW, session: "Hearthstone_2026_01_01_00_00_00" }]).read, REF), { kind: "none" });
});

test("expired: a failed request is an error (worth a retry), never 'not available'", async () => {
  assert.deepEqual(await loadExpired(serve(null, { message: "down" }).read, REF), { kind: "error" });
  assert.deepEqual(await loadExpired(serve({ not: "rows" }).read, REF), { kind: "error" });
  const throwing: ReadSummary = async () => {
    throw new TypeError("network");
  };
  assert.deepEqual(await loadExpired(throwing, REF), { kind: "error" });
});

test("expired: the view says the details have expired and shows the summary", async () => {
  const result = await loadExpired(serve([ROW]).read, REF);
  assert.equal(result.kind, "expired");
  if (result.kind !== "expired") return;
  const nodes = renderExpired(result.row);
  const all = nodes.map(text).join(" ");
  assert.ok(all.includes(GAME_STRINGS["state.expired"]), "the plain line");
  assert.equal(GAME_STRINGS["state.expired"], "Full details are kept 28 days; this game's have expired.");
  assert.ok(all.includes(GAME_STRINGS["state.expired.caption"]), "the table's caption");
  assert.ok(all.includes("12"), "the rounds");
  assert.ok(all.includes("7–4"), "the combats");
  assert.ok(!/error|not available/i.test(all), "no error words");
  assert.match(expiredHeading(result.row), /3rd/);
  assert.equal(expiredHeading({ ...result.row, place: null }).includes("·"), false);
});

test("expired: reads the mark the database adds to game_summaries", () => {
  assert.ok(EXPIRED_COLUMNS.split(",").includes("file_expired_at"));
  const dir = join(ROOT, "supabase", "migrations");
  const sql = readdirSync(dir)
    .filter((f) => f.endsWith(".sql"))
    .map((f) => readFileSync(join(dir, f), "utf8"))
    .join("\n");
  assert.match(sql, /create or replace view public\.game_summaries[\s\S]*?g\.file_expired_at\s*\nfrom public\.games g/);
});
