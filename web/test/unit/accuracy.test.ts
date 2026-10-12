// The simulator accuracy page (T-607): the committed export and card names
// are read strictly (a missing, extra or mistyped key fails, as does another
// format), and the figures put the blocked share beside every accuracy
// number, in English and Spanish.
import { afterEach, test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { ACCURACY_FORMAT, parseAccuracy, parseCardNames, type Accuracy } from "../../src/lib/accuracy/data.ts";
import { blockingRows, calibrationBars, modeFigures } from "../../src/lib/accuracy/view.ts";
import { setLang } from "../../src/lib/i18n.ts";

const DATA = join(import.meta.dirname, "..", "..", "src", "data");
const committed = () => JSON.parse(readFileSync(join(DATA, "accuracy.json"), "utf8")) as Record<string, unknown>;
const committedNames = () => JSON.parse(readFileSync(join(DATA, "accuracy-cards.json"), "utf8")) as unknown;
const clone = <T>(value: T): T => structuredClone(value);

afterEach(() => setLang("en"));

// ---------- The committed export ----------

test("the committed export reads with the export's keys and format 1", () => {
  const data = parseAccuracy(committed());
  assert.equal(data.format, ACCURACY_FORMAT);
  assert.match(data.date, /^\d{4}-\d{2}-\d{2}$/);
  assert.match(data.engine_version, /^\d+\.\d+\.\d+$/);
  for (const mode of ["solo", "duos"] as const) {
    const m = data.modes[mode];
    assert.equal(m.calibration.length, 5, mode);
    assert.ok(m.scored + m.blocked + m.refused <= m.combats_with_result, mode);
    assert.ok(m.most_likely_right <= m.scored, mode);
  }
  assert.ok(data.blocking_cards.length <= 10);
});

test("the committed export holds no names, accounts, seats, heroes, games or rounds", () => {
  const text = readFileSync(join(DATA, "accuracy.json"), "utf8");
  for (const word of ["name", "battletag", "account", "seat", "hero\"", "game", "round", "player", "@"]) {
    assert.ok(!text.toLowerCase().includes(word), word);
  }
});

test("every blocking card of the committed export has a name in both languages", () => {
  const data = parseAccuracy(committed());
  const names = parseCardNames(committedNames());
  for (const card of data.blocking_cards) {
    const entry = names[card.card_id];
    assert.ok(entry, card.card_id);
    assert.ok(entry.en.length > 0 && entry.es.length > 0, card.card_id);
  }
});

// ---------- Strict reading ----------

test("a missing top-level key, mode key or bucket key fails", () => {
  const top = committed();
  delete top.trials;
  assert.throws(() => parseAccuracy(top), /trials/);

  const mode = committed();
  delete (mode.modes as Record<string, Record<string, unknown>>).duos.blocked_share;
  assert.throws(() => parseAccuracy(mode), /blocked_share/);

  const bucket = committed();
  delete ((bucket.modes as Record<string, Record<string, unknown>>).solo.calibration as Record<string, unknown>[])[2].happened;
  assert.throws(() => parseAccuracy(bucket), /happened/);

  const noMode = committed();
  delete (noMode.modes as Record<string, unknown>).solo;
  assert.throws(() => parseAccuracy(noMode), /solo/);
});

test("another format fails before anything else is read", () => {
  for (const format of [2, 0, "1", null]) {
    const raw = { ...committed(), format };
    assert.throws(() => parseAccuracy(raw), /format/);
  }
  assert.throws(() => parseAccuracy({ format: 2 }), /format/);
});

test("an extra key anywhere fails: the page shows only the export's fields", () => {
  const top = { ...committed(), player: "x" };
  assert.throws(() => parseAccuracy(top), /player/);

  const mode = committed();
  (mode.modes as Record<string, Record<string, unknown>>).duos.hero = "x";
  assert.throws(() => parseAccuracy(mode), /hero/);

  const card = committed();
  (card.blocking_cards as Record<string, unknown>[])[0].seat = 1;
  assert.throws(() => parseAccuracy(card), /seat/);

  const modes = committed();
  (modes.modes as Record<string, unknown>).arena = (modes.modes as Record<string, unknown>).solo;
  assert.throws(() => parseAccuracy(modes), /arena/);
});

test("wrong types and impossible values fail", () => {
  const cases: [string, (raw: Record<string, unknown>) => void][] = [
    ["date", (r) => (r.date = "11/10/2026")],
    ["date", (r) => (r.date = "2026-13-45")],
    ["blocked_share", (r) => ((r.modes as Record<string, Record<string, unknown>>).duos.blocked_share = 0.5)],
    ["engine_version", (r) => (r.engine_version = 4)],
    ["trials", (r) => (r.trials = -1)],
    ["card_id", (r) => ((r.blocking_cards as Record<string, unknown>[])[0].card_id = "<script>")],
    ["combats", (r) => ((r.blocking_cards as Record<string, unknown>[])[0].combats = 1.5)],
    ["scored", (r) => ((r.modes as Record<string, Record<string, unknown>>).solo.scored = "19")],
    ["mean_p_real", (r) => ((r.modes as Record<string, Record<string, unknown>>).solo.mean_p_real = 1.2)],
    ["brier", (r) => ((r.modes as Record<string, Record<string, unknown>>).solo.brier = 3)],
    ["most_likely_right", (r) => ((r.modes as Record<string, Record<string, unknown>>).solo.most_likely_right = 999)],
    ["blocked", (r) => ((r.modes as Record<string, Record<string, unknown>>).duos.blocked = 999)],
    ["calibration", (r) => ((r.modes as Record<string, Record<string, unknown>>).solo.calibration = [])],
  ];
  for (const [name, change] of cases) {
    const raw = committed();
    change(raw);
    assert.throws(() => parseAccuracy(raw), new RegExp(name), name);
  }
});

test("null shares are read when nothing was scored", () => {
  const raw = committed();
  const solo = (raw.modes as Record<string, Record<string, unknown>>).solo;
  Object.assign(solo, { combats_with_result: 0, blocked: 0, blocked_share: null, refused: 0, scored: 0, mean_p_real: null, brier: null, most_likely_right: 0 });
  solo.calibration = (solo.calibration as Record<string, unknown>[]).map((b) => ({ ...b, predictions: 0, mean_said: null, happened: null }));
  const data = parseAccuracy(raw);
  assert.equal(data.modes.solo.mean_p_real, null);
  assert.equal(data.modes.solo.blocked_share, null);
});

test("card names: a bad id or an empty name fails", () => {
  assert.throws(() => parseCardNames({ format: 1, cards: { "bad id": { en: "a", es: "b", golden: false } } }), /card id/);
  assert.throws(() => parseCardNames({ format: 1, cards: { BG1_1: { en: "", es: "b", golden: false } } }), /en/);
  assert.throws(() => parseCardNames({ format: 2, cards: {} }), /format/);
  assert.throws(() => parseCardNames({ format: 1, cards: { BG1_1: { en: "a", es: "b", golden: false, text: "x" } } }), /text/);
});

// ---------- Figures ----------

const sample = (): Accuracy => parseAccuracy(committed());

test("every mode's figures carry the blocked share next to the accuracy", () => {
  const data = sample();
  for (const lang of ["en", "es"] as const) {
    setLang(lang);
    for (const mode of ["solo", "duos"] as const) {
      const f = modeFigures(data.modes[mode], mode);
      assert.ok(f.blocked.value.length > 0, `${lang} ${mode}`);
      assert.ok(f.blocked.note.length > 0, `${lang} ${mode}`);
      assert.ok(f.right.value.length > 0 && f.meanP.value.length > 0, `${lang} ${mode}`);
    }
  }
});

test("Duos figures in English: the counts and the blocked sentence", () => {
  const raw = committed();
  const duos = (raw.modes as Record<string, Record<string, unknown>>).duos;
  Object.assign(duos, { combats_with_result: 232, blocked: 30, blocked_share: 30 / 232, scored: 202, most_likely_right: 166, mean_p_real: 0.81, brier: 0.2742 });
  const f = modeFigures(parseAccuracy(raw).modes.duos, "duos");
  assert.equal(f.right.value, "82%");
  assert.equal(f.right.backing, "166 of 202 combats");
  assert.equal(f.meanP.value, "81%");
  assert.equal(f.blocked.value, "13%");
  assert.equal(f.blocked.note, "No odds in 13% of Duos combats (30 of 232): a card is not modelled.");
  assert.equal(f.brier.value, "0.274");
});

test("Spanish figures use Spanish numbers and words", () => {
  setLang("es");
  const raw = committed();
  const duos = (raw.modes as Record<string, Record<string, unknown>>).duos;
  Object.assign(duos, { combats_with_result: 232, blocked: 30, blocked_share: 30 / 232, scored: 202, most_likely_right: 166, brier: 0.2742 });
  const f = modeFigures(parseAccuracy(raw).modes.duos, "duos");
  assert.equal(f.right.backing, "166 de 202 combates");
  assert.equal(f.blocked.value.replace(/\s/g, " "), "13 %");
  assert.equal(f.brier.value, "0,274");
  assert.match(f.blocked.note, /^Sin probabilidades en el 13\s% de los combates de Dúos \(30 de 232\)/);
});

test("nothing scored: dashes, never zero, and the blocked share still shows", () => {
  const raw = committed();
  const solo = (raw.modes as Record<string, Record<string, unknown>>).solo;
  Object.assign(solo, { combats_with_result: 0, blocked: 0, blocked_share: null, refused: 0, scored: 0, mean_p_real: null, brier: null, most_likely_right: 0 });
  solo.calibration = (solo.calibration as Record<string, unknown>[]).map((b) => ({ ...b, predictions: 0, mean_said: null, happened: null }));
  const f = modeFigures(parseAccuracy(raw).modes.solo, "solo");
  assert.equal(f.right.value, "—");
  assert.equal(f.meanP.value, "—");
  assert.equal(f.brier.value, "—");
  assert.equal(f.blocked.value, "—");
  assert.equal(f.blocked.note, "No Solo combats with a result yet.");
});

test("no combat blocked: the note says so instead of hiding the share", () => {
  const f = modeFigures(sample().modes.solo, "solo");
  assert.equal(f.blocked.value, "0%");
  assert.equal(f.blocked.note, "Odds in every Solo combat with a result (19): no card was missing.");
});

test("shares never round to a claim the counts do not make", () => {
  const raw = committed();
  const duos = (raw.modes as Record<string, Record<string, unknown>>).duos;
  Object.assign(duos, { combats_with_result: 1000, blocked: 3, blocked_share: 0.003, refused: 0, scored: 997, most_likely_right: 996 });
  const f = modeFigures(parseAccuracy(raw).modes.duos, "duos");
  assert.equal(f.blocked.value, "<1%");
  assert.match(f.blocked.note, /^No odds in <1% of Duos combats \(3 of 1,000\)/);
  assert.equal(f.right.value, "99%");
});

test("calibration bars: percents per bucket, empty buckets marked, never drawn as 0", () => {
  // The middle bucket emptied, whatever the committed export holds.
  const raw = committed();
  const solo = (raw.modes as Record<string, { calibration: Record<string, unknown>[] }>).solo;
  Object.assign(solo.calibration[2], { predictions: 0, mean_said: null, happened: null });
  const bars = calibrationBars(parseAccuracy(raw).modes.solo.calibration);
  assert.equal(bars.length, 5);
  assert.deepEqual(bars.map((b) => b.range), ["0–20%", "20–40%", "40–60%", "60–80%", "80–100%"]);
  const empty = bars[2];
  assert.equal(empty.predictions, 0);
  assert.equal(empty.said, null);
  assert.equal(empty.happened, null);
  assert.equal(empty.saidText, "—");
  const full = bars[4];
  assert.ok(full.said !== null && full.said > 0.9);
  assert.equal(full.happenedText, "100%");
});

test("blocking rows name the card in the page's language, golden marked, id kept", () => {
  const names = parseCardNames({
    format: 1,
    cards: {
      BG34_140: { en: "Expert Aviator", es: "Aviador experto", golden: false },
      BG34_140_G: { en: "Expert Aviator", es: "Aviador experto", golden: true },
    },
  });
  const cards = [
    { card_id: "BG34_140", combats: 11 },
    { card_id: "BG34_140_G", combats: 3 },
    { card_id: "BG99_1", combats: 1 },
  ];
  const en = blockingRows(cards, names);
  assert.deepEqual(en.map((r) => r.name), ["Expert Aviator", "Expert Aviator (golden)", "BG99_1"]);
  assert.deepEqual(en.map((r) => r.id), ["BG34_140", "BG34_140_G", "BG99_1"]);
  assert.deepEqual(en.map((r) => r.combats), ["11", "3", "1"]);
  setLang("es");
  assert.deepEqual(blockingRows(cards, names).map((r) => r.name), ["Aviador experto", "Aviador experto (dorado)", "BG99_1"]);
});

test("the export is not changed by reading it", () => {
  const raw = committed();
  const before = clone(raw);
  parseAccuracy(raw);
  assert.deepEqual(raw, before);
});
