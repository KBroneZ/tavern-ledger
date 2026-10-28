// The expected-place calibration page (T-655): the committed file is the
// empty state (no model published) and reads strictly; a published file
// (made up by the crate's test, never real data) reads only when its totals agree and no
// mean has fewer games than the floor behind it; and the page's words say
// empty, thin and unknown apart, in English and Spanish.
import { afterEach, test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { CALIBRATION_FORMAT, MIN_GAMES, parseCalibration, THIN_GAMES } from "../../src/lib/calibration/data.ts";
import { calibrationView } from "../../src/lib/calibration/view.ts";
import { setLang } from "../../src/lib/i18n.ts";

const ROOT = join(import.meta.dirname, "..", "..");
const committed = () => JSON.parse(readFileSync(join(ROOT, "src", "data", "calibration.json"), "utf8")) as unknown;
const crate = (file: string) => readFileSync(join(ROOT, "..", "crates", "expected-place", "src", file), "utf8");

afterEach(() => setLang("en"));

type Raw = Record<string, any>;

/**
 * The file `place-train` writes for made-up Duos games, saved by the crate's
 * own test (crates/expected-place/src/calibration_tests.rs, BLESS=1): build
 * 200 with 30 games, build 100 with 25 (one thin bucket), 8 more games
 * counted. Parsing it here makes a change of shape on either side fail.
 */
const published = (): Raw => JSON.parse(readFileSync(join(ROOT, "..", "crates", "expected-place", "tests", "data", "calibration_published.json"), "utf8"));
const duos = (): Raw => published().modes.duos;

// ---------- The committed file and the crate ----------

test("the committed file is the empty state: no model is published", () => {
  const data = parseCalibration(committed());
  assert.equal(data.format, CALIBRATION_FORMAT);
  assert.equal(data.date, null);
  assert.deepEqual(data.modes, { solo: null, duos: null });
});

test("the page's floors and format are the crate's", () => {
  assert.match(crate("model.rs"), new RegExp(`pub const MIN_GAMES: usize = ${MIN_GAMES};`));
  assert.match(crate("calibration.rs"), new RegExp(`pub const THIN_GAMES: usize = ${THIN_GAMES};`));
  assert.match(crate("calibration.rs"), new RegExp(`pub const FORMAT: u32 = ${CALIBRATION_FORMAT};`));
});

// ---------- Strict reading ----------

test("a published file reads", () => {
  const data = parseCalibration(published());
  assert.equal(data.date, "2026-11-02");
  assert.equal(data.modes.solo, null);
  assert.equal(data.modes.duos?.builds.length, 2);
  assert.equal(data.modes.duos?.builds[1].buckets[0].said, null);
});

test("another format, or a missing or extra key anywhere, fails", () => {
  assert.throws(() => parseCalibration({ format: 2, date: null, modes: { solo: null, duos: null } }), /format/);
  assert.throws(() => parseCalibration({ format: 1, modes: { solo: null, duos: null } }), /date/);
  assert.throws(() => parseCalibration({ format: 1, date: null, modes: { solo: null } }), /duos/);
  const cases: [string, (r: Raw) => void][] = [
    ["player", (r) => (r.player = "x")],
    ["hero", (r) => (r.modes.duos.hero = "x")],
    ["game", (r) => (r.modes.duos.builds[0].game = 1)],
    ["seat", (r) => (r.modes.duos.builds[0].buckets[0].seat = 1)],
    ["trio", (r) => (r.modes.trio = null)],
  ];
  for (const [name, change] of cases) {
    const raw = published();
    change(raw);
    assert.throws(() => parseCalibration(raw), new RegExp(name), name);
  }
});

test("totals that do not agree, or a mean with too few games behind it, fail", () => {
  const cases: [string, RegExp, (r: Raw) => void][] = [
    ["date with no model", /date/, (r) => (r.modes.duos = null)],
    ["no date with a model", /date/, (r) => (r.date = null)],
    ["not a day", /date/, (r) => (r.date = "2026-13-45")],
    ["ready below the minimum", /ready/, (r) => (r.modes.duos.ready = true)],
    ["a lower floor", /thin_games/, (r) => (r.modes.duos.thin_games = 5)],
    ["a mode under the floor", /games/, (r) => Object.assign(r.modes.duos, { games: 19, other_games: -36 })],
    ["one fold", /folds/, (r) => (r.modes.duos.folds = 1)],
    ["games do not add up", /other_games/, (r) => (r.modes.duos.other_games = 9)],
    ["rows do not add up", /rows/, (r) => (r.modes.duos.builds[0].rows = 61)],
    ["builds out of order", /newest first/, (r) => r.modes.duos.builds.reverse()],
    ["a thin build listed", /games/, (r) => Object.assign(r.modes.duos.builds[1], { games: 19 })],
    ["a missing bucket", /buckets/, (r) => r.modes.duos.builds[0].buckets.pop()],
    ["buckets out of place", /from/, (r) => (r.modes.duos.builds[0].buckets[0].from = 2)],
    ["a thin mean published", /null exactly/, (r) => Object.assign(r.modes.duos.builds[1].buckets[0], { said: 1.4, real: 2 })],
    ["a thin count published", /games/, (r) => Object.assign(r.modes.duos.builds[1].buckets[0], { rows: 10, games: 10 })],
    ["one thin count without the other", /null together/, (r) => (r.modes.duos.builds[1].buckets[0].rows = 10)],
    ["a minimum below the crate's", /min_games/, (r) => (r.modes.duos.min_games = 50)],
    ["a mean withheld with enough games", /null exactly/, (r) => Object.assign(r.modes.duos.builds[0].buckets[0], { said: null, real: null })],
    ["games without combats", /rows/, (r) => (r.modes.duos.builds[0].buckets[2].games = 1)],
    ["more games in a bucket than the build", /games/, (r) => Object.assign(r.modes.duos.builds[0].buckets[0], { games: 31, rows: 31 })],
    ["a said mean outside its bucket", /said/, (r) => (r.modes.duos.builds[0].buckets[0].said = 2.5)],
    ["a place past the last", /real/, (r) => (r.modes.duos.builds[0].buckets[0].real = 5)],
    ["a distance below zero", /mean_distance/, (r) => (r.modes.duos.mean_distance.model = -0.1)],
    ["a mistyped count", /rows/, (r) => (r.modes.duos.rows = "103")],
  ];
  for (const [name, pattern, change] of cases) {
    const raw = published();
    change(raw);
    assert.throws(() => parseCalibration(raw), pattern, name);
  }
});

test("Solo reads with seven buckets, the last one including 8", () => {
  const raw = published();
  const buckets = [1, 2, 3, 4, 5, 6, 7].map((from) => ({ from, to: from + 1, rows: 20, games: 20, said: from === 7 ? 8 : from + 0.5, real: from }));
  raw.modes.solo = { ...duos(), games: 20, rows: 140, ready: false, other_games: 0, builds: [{ build: 7, games: 20, rows: 140, buckets }] };
  const data = parseCalibration(raw);
  assert.equal(data.modes.solo?.builds[0].buckets.length, 7);
});

// ---------- Words ----------

test("the empty state says plainly that no model is published, in both languages", () => {
  const empty = parseCalibration(committed());
  const en = calibrationView(empty);
  assert.equal(en.empty, true);
  assert.equal(en.date, null);
  assert.ok(en.modes.every((m) => !m.published));
  setLang("es");
  const es = calibrationView(empty);
  assert.equal(es.empty, true);
  assert.notEqual(es.modes[1].name, en.modes[1].name);
});

test("a published mode: its totals, the distance beside two guesses, and each build's groups", () => {
  const v = calibrationView(parseCalibration(published()));
  assert.equal(v.empty, false);
  assert.equal(v.date, "2 November 2026");
  const [solo, duosView] = v.modes;
  assert.equal(solo.published, false);
  assert.match(solo.published ? "" : solo.none, /No Solo model is published yet/);
  assert.ok(duosView.published);
  if (!duosView.published) return;
  assert.match(duosView.summary, /63 games \(103 combats\).*5 parts/);
  assert.match(duosView.readiness, /Not shown to players yet.*150/);
  assert.equal(duosView.distance.value, "0.81");
  assert.match(duosView.distance.backing, /1\.10.*1\.25/);
  assert.equal(duosView.builds.length, 2);
  const [newest, older] = duosView.builds;
  assert.equal(newest.caption, "Build 200");
  assert.equal(newest.summary, "30 games, 60 combats");
  assert.deepEqual(
    newest.rows.map((r) => [r.range, r.said, r.real, r.games, r.rows, r.why]),
    [
      ["1 to 2", "1.50", "1.00", "30", "30", null],
      ["2 to 3", "2.50", "3.00", "30", "30", null],
      ["3 to 4", "—", "—", "0", "0", "none"],
    ],
  );
  assert.deepEqual([older.rows[0].said, older.rows[0].why], ["—", "fewer than 20 games"]);
  // A thin group's counts are withheld too: only "fewer than 20".
  assert.deepEqual([older.rows[0].games, older.rows[0].rows, older.rows[0].withheld], ["<20", "—", [true, true, true, true]]);
  assert.deepEqual(newest.rows[2].withheld, [true, true, false, false]);
  assert.match(duosView.other ?? "", /8 more games/);
});

test("Spanish numbers use a decimal comma and Spanish words", () => {
  setLang("es");
  const v = calibrationView(parseCalibration(published()));
  assert.equal(v.date, "2 de noviembre de 2026");
  const duosView = v.modes[1];
  assert.ok(duosView.published);
  if (!duosView.published) return;
  assert.equal(duosView.distance.value, "0,81");
  assert.equal(duosView.builds[0].caption, "Versión 200");
  assert.equal(duosView.builds[0].rows[0].range, "De 1 a 2");
  assert.equal(duosView.builds[0].rows[0].said, "1,50");
  assert.equal(duosView.builds[1].rows[0].why, "menos de 20 partidas");
  assert.match(duosView.other ?? "", /8 partidas más/);
});

test("one game left over reads in the singular; no build with enough games says so", () => {
  const raw = published();
  raw.modes.duos.games = 56;
  raw.modes.duos.other_games = 1;
  const one = calibrationView(parseCalibration(raw)).modes[1];
  assert.ok(one.published && /^1 more game,/.test(one.other ?? ""));

  const none = published();
  Object.assign(none.modes.duos, { builds: [], other_games: 63 });
  const v = calibrationView(parseCalibration(none)).modes[1];
  assert.ok(v.published && v.builds.length === 0 && /No game version has 20 games yet/.test(v.buildsNone ?? ""));
});
