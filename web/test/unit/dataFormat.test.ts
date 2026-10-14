// Logic behind the data pages' design system (T-501): number formatting,
// the "in 28 of 31 games" backing line, top N, place groups, trends,
// thresholds and the chart geometry.
import { test } from "node:test";
import assert from "node:assert/strict";
import { average, backing, barScale, columnPath, compact, placeDistribution, placeGroup, share, sparkline, threshold, topN, trend, perGameText, placeAverage } from "../../src/lib/data/format.ts";

test("compact: plain up to 9,999, then K and M with one decimal", () => {
  assert.equal(compact(0), "0");
  assert.equal(compact(1284), "1,284");
  assert.equal(compact(9999), "9,999");
  assert.equal(compact(12_900), "12.9K");
  assert.equal(compact(4_200_000), "4.2M");
  assert.equal(compact(-1500), "-1,500");
});

test("compact, average and share say a dash for an unknown value, never 0", () => {
  assert.equal(compact(null), "—");
  assert.equal(compact(Number.NaN), "—");
  assert.equal(average(null), "—");
  assert.equal(share(3, 0), "—");
  assert.equal(share(null, 10), "—");
});

test("average: two decimals; share: a whole percent", () => {
  assert.equal(average(3.4166), "3.42");
  assert.equal(average(4), "4.00");
  assert.equal(share(14, 31), "45%");
  assert.equal(share(31, 31), "100%");
  assert.equal(share(0, 31), "0%");
});

test("backing: how many games back a value, with the unknown kept apart", () => {
  assert.equal(backing(28, 31), "in 28 of 31 games");
  assert.equal(backing(31, 31), "in all 31 games");
  assert.equal(backing(1, 1), "in 1 game");
  assert.equal(backing(0, 31), "no data in 31 games");
  assert.equal(backing(0, 0), "no games yet");
  assert.equal(backing(10, 11, "turns"), "in 10 of 11 turns");
  assert.equal(backing(1, 1, "turns"), "in 1 turn");
});

test("backing refuses counts that cannot be true", () => {
  assert.throws(() => backing(5, 3), /more than/);
  assert.throws(() => backing(-1, 3), /count/);
  assert.throws(() => backing(1.5, 3), /count/);
});

test("topN: the first rows and the rest, never a 'show all' for nothing", () => {
  const rows = [1, 2, 3, 4, 5, 6, 7];
  assert.deepEqual(topN(rows, 5), { shown: [1, 2, 3, 4, 5], rest: [6, 7] });
  assert.deepEqual(topN(rows, 7), { shown: rows, rest: [] });
  assert.deepEqual(topN(rows, 10), { shown: rows, rest: [] });
  assert.deepEqual(topN([], 5), { shown: [], rest: [] });
  // One hidden row is not worth a button: show it.
  assert.deepEqual(topN(rows, 6), { shown: rows, rest: [] });
  assert.throws(() => topN(rows, 0), /at least 1/);
});

test("placeGroup: 1st, top half, bottom half per mode; anything else is unknown", () => {
  assert.equal(placeGroup(1, "solo"), "first");
  assert.equal(placeGroup(4, "solo"), "top");
  assert.equal(placeGroup(5, "solo"), "bottom");
  assert.equal(placeGroup(8, "solo"), "bottom");
  assert.equal(placeGroup(1, "duos"), "first");
  assert.equal(placeGroup(2, "duos"), "top");
  assert.equal(placeGroup(3, "duos"), "bottom");
  assert.equal(placeGroup(5, "duos"), "unknown");
  assert.equal(placeGroup(0, "solo"), "unknown");
  assert.equal(placeGroup(9, "solo"), "unknown");
  assert.equal(placeGroup(2.5, "solo"), "unknown");
  assert.equal(placeGroup(null, "solo"), "unknown");
});

test("placeDistribution: one bar per place with its share, unknown places counted apart", () => {
  const { bars, unknown, total } = placeDistribution([1, 1, 3, 4, 8, null, 9], "solo");
  assert.equal(total, 5);
  assert.equal(unknown, 2);
  assert.equal(bars.length, 8);
  assert.deepEqual(bars[0], { place: 1, count: 2, share: 0.4, group: "first" });
  assert.deepEqual(bars[1], { place: 2, count: 0, share: 0, group: "top" });
  assert.equal(bars[7].count, 1);
  assert.equal(bars[7].group, "bottom");
  assert.equal(placeDistribution([2, 3], "duos").bars.length, 4);
  assert.deepEqual(placeDistribution([], "duos").bars.map((b) => b.share), [0, 0, 0, 0]);
});

test("trend: signed change and whether it is better, for lower-is-better and higher-is-better", () => {
  assert.deepEqual(trend(-0.3, "lower"), { text: "−0.30", direction: "better", sign: -1 });
  assert.deepEqual(trend(0.25, "lower"), { text: "+0.25", direction: "worse", sign: 1 });
  assert.deepEqual(trend(0.05, "higher", "percent"), { text: "+5%", direction: "better", sign: 1 });
  assert.deepEqual(trend(0, "higher"), { text: "±0.00", direction: "same", sign: 0 });
  assert.equal(trend(null, "lower"), null);
});

test("threshold: says how many games are still needed to rank", () => {
  assert.equal(threshold(12, 20), "Needs 20 games to be ranked (has 12)");
  assert.equal(threshold(1, 20), "Needs 20 games to be ranked (has 1)");
  assert.equal(threshold(20, 20), null);
  assert.equal(threshold(31, 20), null);
});

test("sparkline: best place on top, gaps where a game has no place", () => {
  const line = sparkline([1, 8, null, 4], { width: 30, height: 14, min: 1, max: 8, invert: true });
  // x steps of 10, y from 1 (top, y=1) to 8 (bottom, y=13) inside a 1px inset.
  assert.equal(line.path, "M0 1L10 13M30 6.14");
  assert.deepEqual(
    line.points.map((p) => [p.x, p.y, p.value]),
    [
      [0, 1, 1],
      [10, 13, 8],
      [30, 6.14, 4],
    ],
  );
  assert.equal(sparkline([], { width: 30, height: 14, min: 1, max: 8 }).path, "");
  assert.equal(sparkline([3], { width: 30, height: 14, min: 1, max: 8 }).points[0].x, 30);
});

test("barScale: lengths share one scale and a non-zero value is never invisible", () => {
  const scale = barScale([0, 2, 10], 100);
  assert.equal(scale(0), 0);
  assert.equal(scale(10), 100);
  assert.equal(scale(2), 20);
  assert.equal(barScale([0.1, 1000], 100)(0.1), 2);
  assert.equal(barScale([0, 0], 100)(0), 0);
  assert.equal(barScale([-12, 6], 60)(-12), 60);
});

test("columnPath: a bar with 4px rounded data end and a square base, up or down", () => {
  assert.equal(columnPath(10, 100, 24, 40, "up"), "M10 100V64Q10 60 14 60H30Q34 60 34 64V100Z");
  assert.equal(columnPath(10, 50, 14, 20, "down"), "M10 50V66Q10 70 14 70H20Q24 70 24 66V50Z");
  // Shorter than the radius: the rounding shrinks to fit.
  assert.equal(columnPath(0, 10, 10, 2, "up"), "M0 10V10Q0 8 2 8H8Q10 8 10 10V10Z");
  assert.equal(columnPath(0, 10, 10, 0, "up"), "");
});

test("trend: a change that rounds to zero is 'same', and the sign is returned for the arrow", () => {
  assert.deepEqual(trend(-0.004, "lower"), { text: "±0.00", direction: "same", sign: 0 });
  assert.deepEqual(trend(0.004, "higher", "percent"), { text: "±0%", direction: "same", sign: 0 });
  assert.equal(trend(-0.3, "lower")?.sign, -1);
  assert.equal(trend(0.25, "lower")?.sign, 1);
});

test("sparkline: a value outside the range is a gap, never drawn at the edge", () => {
  const line = sparkline([1, 9, 0, 4], { width: 30, height: 14, min: 1, max: 8, invert: true });
  assert.deepEqual(line.points.map((p) => p.value), [1, 4]);
  assert.equal(line.path, "M0 1M30 6.14");
});

test("barScale ignores values that are not numbers", () => {
  assert.equal(barScale([Number.NaN, 10], 100)(10), 100);
  assert.equal(barScale([Number.NaN], 100)(5), 0);
});

test("an average place has one decimal; a per-game figure is whole from 10 up (T-559)", () => {
  assert.equal(placeAverage(3.418), "3.4");
  assert.equal(placeAverage(7), "7.0");
  assert.equal(placeAverage(null), "—");
  assert.equal(perGameText(62.97), "63");
  assert.equal(perGameText(137.28), "137");
  assert.equal(perGameText(2.08), "2.1");
  assert.equal(perGameText(9.96), "10.0", "just below 10 keeps its decimal");
  assert.equal(perGameText(0), "0.0");
  assert.equal(perGameText(12_345.6), "12.3K");
  assert.equal(perGameText(undefined), "—");
});
