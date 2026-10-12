// The simulator accuracy page (T-607): the committed export and card names
// are read strictly (a missing, extra or mistyped key fails, as does another
// format), and the figures put the blocked share beside every accuracy
// number, in English and Spanish.
import { afterEach, test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { ACCURACY_FORMAT, CAUSE_IDS, parseAccuracy, parseCardNames, type Accuracy } from "../../src/lib/accuracy/data.ts";
import { blockingRows, buildsView, calibrationBars, causeRows, modeFigures } from "../../src/lib/accuracy/view.ts";
import { setLang } from "../../src/lib/i18n.ts";

const DATA = join(import.meta.dirname, "..", "..", "src", "data");
const committed = () => JSON.parse(readFileSync(join(DATA, "accuracy.json"), "utf8")) as Record<string, unknown>;
const committedNames = () => JSON.parse(readFileSync(join(DATA, "accuracy-cards.json"), "utf8")) as unknown;
const clone = <T>(value: T): T => structuredClone(value);

afterEach(() => setLang("en"));

// ---------- The committed export ----------

test("the committed export reads with the export's keys and format 3", () => {
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
  for (const format of [1, 2, "3", null]) {
    const raw = { ...committed(), format };
    assert.throws(() => parseAccuracy(raw), /format/);
  }
  assert.throws(() => parseAccuracy({ format: 1 }), /format/);
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

// ---------- Misses by cause and game builds (T-553) ----------

const withMisses = (misses: unknown) => ({ ...committed(), misses });
const withBuilds = (builds: unknown) => ({ ...committed(), builds });
const MISSES = {
  total: 8,
  causes: [
    { cause: "chance", solo: 0, duos: 4, share: 0.5 },
    { cause: "replay_minions", solo: 0, duos: 2, share: 0.25 },
    { cause: "no_replay_not_modelled", solo: 1, duos: 1, share: 0.25 },
  ],
};
const BUILDS = {
  newest: 260001,
  checked: [
    { build: 260001, count: 3 },
    { build: 253216, count: 150 },
  ],
};

test("misses and builds are read with their keys", () => {
  const data = parseAccuracy({ ...committed(), misses: MISSES, builds: BUILDS });
  assert.equal(data.misses.total, 8);
  assert.deepEqual(data.misses.causes[0], { cause: "chance", solo: 0, duos: 4, share: 0.5 });
  assert.equal(data.builds.newest, 260001);
  assert.equal(data.builds.checked.length, 2);
});

test("no misses and no builds are read as empty, not as missing", () => {
  const data = parseAccuracy({ ...committed(), misses: { total: 0, causes: [] }, builds: { newest: null, checked: [] } });
  assert.deepEqual(data.misses.causes, []);
  assert.equal(data.builds.newest, null);
});

test("misses: an unknown cause, a wrong total or share, or an extra key fails", () => {
  const bad = (change: (m: typeof MISSES) => void) => {
    const m = clone(MISSES);
    change(m);
    return () => parseAccuracy(withMisses(m));
  };
  assert.throws(bad((m) => (m.causes[0].cause = "a_new_cause")), /cause/);
  assert.throws(bad((m) => (m.total = 9)), /total/);
  assert.throws(bad((m) => (m.causes[0].share = 0.9)), /share/);
  assert.throws(bad((m) => m.causes.push({ ...m.causes[0] })), /twice/);
  assert.throws(bad((m) => (m.causes[1].solo = 5)), /total|share|largest/);
  assert.throws(bad((m) => m.causes.reverse()), /largest/);
  assert.throws(() => parseAccuracy(withMisses({ ...MISSES, game: 1 })), /game/);
  assert.throws(() => parseAccuracy(withMisses({ total: 0, causes: [{ ...MISSES.causes[0] }] })), /total/);
  const raw = withMisses(MISSES);
  delete (raw as Record<string, unknown>).misses;
  assert.throws(() => parseAccuracy(raw), /misses/);
});

test("builds: newest must lead the list; bad numbers, repeats and extra keys fail", () => {
  const bad = (change: (b: typeof BUILDS) => void) => {
    const b = clone(BUILDS);
    change(b);
    return () => parseAccuracy(withBuilds(b));
  };
  assert.throws(bad((b) => (b.newest = 253216)), /newest/);
  assert.throws(bad((b) => b.checked.reverse()), /newest|newest first/);
  assert.throws(bad((b) => (b.checked[1].build = 260001)), /twice|newest first/);
  assert.throws(bad((b) => (b.checked[0].build = -1)), /build/);
  assert.throws(bad((b) => (b.checked[0].count = 0)), /count/);
  assert.throws(() => parseAccuracy(withBuilds({ newest: null, checked: BUILDS.checked })), /newest/);
  assert.throws(() => parseAccuracy(withBuilds({ ...BUILDS, date: "2026-10-12" })), /date/);
  assert.throws(() => parseAccuracy(withBuilds({ newest: 1, checked: [{ build: 1, count: 1, session: "x" }] })), /session/);
});

test("every cause id has a name and a one-line why in both languages", () => {
  for (const lang of ["en", "es"] as const) {
    setLang(lang);
    const rows = causeRows(
      CAUSE_IDS.map((cause) => ({ cause, solo: 1, duos: 1, share: 1 / CAUSE_IDS.length })),
    );
    assert.equal(rows.length, CAUSE_IDS.length);
    for (const r of rows) {
      assert.ok(r.name.length > 0 && r.why.length > 0, `${lang} ${r.id}`);
      assert.ok(!r.name.startsWith("cause."), `${lang} ${r.id}`);
      assert.ok(r.why.length <= 140, `${lang} ${r.id}: one line`);
    }
  }
});

test("the example export's cause ids are all known to the page", () => {
  const example = JSON.parse(readFileSync(join(import.meta.dirname, "..", "..", "..", "crates", "combat-sim", "tests", "data", "accuracy_example.json"), "utf8"));
  const parsed = parseAccuracy({ ...committed(), misses: example.misses, builds: example.builds });
  for (const c of parsed.misses.causes) assert.ok((CAUSE_IDS as readonly string[]).includes(c.cause), c.cause);
});

test("a share of 99.5% or more is never written as 100%, and a tiny one is <1%", () => {
  const rows = causeRows([{ cause: "chance", solo: 995, duos: 0, share: 0.995 }, { cause: "replay_end", solo: 4, duos: 0, share: 0.0049 }]);
  assert.equal(rows[0].share, "99%");
  assert.equal(rows[1].share, "<1%");
});

test("cause rows in English and Spanish: counts and shares", () => {
  const data = parseAccuracy({ ...committed(), misses: MISSES, builds: BUILDS });
  const en = causeRows(data.misses.causes);
  assert.equal(en[0].id, "chance");
  assert.equal(en[0].name, "Chance");
  assert.equal(en[0].combats, "4");
  assert.equal(en[0].share, "50%");
  assert.equal(en[2].solo, "1");
  assert.equal(en[2].duos, "1");
  setLang("es");
  assert.equal(causeRows(data.misses.causes)[0].name, "Azar");
});

test("a share never rounds to a claim the counts do not make", () => {
  const rows = causeRows([
    { cause: "chance", solo: 999, duos: 0, share: 999 / 1000 },
    { cause: "replay_end", solo: 1, duos: 0, share: 1 / 1000 },
  ]);
  assert.equal(rows[0].share, "99%");
  assert.equal(rows[1].share, "<1%");
});

test("the builds block: newest build, the day, and each build's games", () => {
  const data = parseAccuracy({ ...committed(), misses: MISSES, builds: BUILDS });
  const en = buildsView(data.builds, data.date);
  assert.match(en.summary, /^Newest build checked: 260001, on \d+ \w+ \d{4}\.$/);
  assert.deepEqual(en.rows.map((r) => r.build), ["260001", "253216"]);
  assert.equal(en.rows[0].games, "3 games");
  assert.equal(en.rows[1].games, "150 games");
  const one = buildsView({ newest: 5, checked: [{ build: 5, count: 1 }] }, data.date);
  assert.equal(one.rows[0].games, "1 game");
  setLang("es");
  const es = buildsView(data.builds, data.date);
  assert.match(es.summary, /^Última versión comprobada: 260001, el /);
  assert.equal(es.rows[0].games, "3 partidas");
  assert.equal(buildsView({ newest: null, checked: [] }, data.date).summary, "Aún no se ha leído ninguna versión del juego.");
});
