"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const { nudge, STEP, BIG_STEP } = require("../ui/overlay-keys.js");

const screen = { w: 1920, h: 1080 };
const panel = { minW: 160, maxW: 800, minH: 40, grabH: 32 };
const board = { minW: 12, maxW: 1920, minH: 40, keepInside: true };
const key = (name, extra = {}) => ({ key: name, shiftKey: false, altKey: false, ...extra });
const r = { x: 100, y: 100, w: 300, h: 200 };

test("an arrow moves a panel by one step", () => {
  assert.deepEqual(nudge(r, key("ArrowRight"), screen, panel), { ...r, x: 100 + STEP });
  assert.deepEqual(nudge(r, key("ArrowUp"), screen, panel), { ...r, y: 100 - STEP });
});

test("Shift makes the step bigger", () => {
  assert.deepEqual(nudge(r, key("ArrowLeft", { shiftKey: true }), screen, panel), { ...r, x: 100 - BIG_STEP });
});

test("Alt with an arrow resizes instead of moving", () => {
  assert.deepEqual(nudge(r, key("ArrowRight", { altKey: true }), screen, panel), { ...r, w: 300 + STEP });
  assert.deepEqual(nudge(r, key("ArrowUp", { altKey: true, shiftKey: true }), screen, panel), { ...r, h: 200 - BIG_STEP });
});

test("a panel cannot leave the screen, as with the mouse", () => {
  assert.equal(nudge({ ...r, x: 4 }, key("ArrowLeft"), screen, panel).x, 0);
  assert.equal(nudge({ ...r, x: 1915 - 300 }, key("ArrowRight", { shiftKey: true }), screen, panel).x, 1920 - 300);
  assert.equal(nudge({ ...r, y: 1070 }, key("ArrowDown", { shiftKey: true }), screen, panel).y, 1080 - 32, "a grab strip stays on screen");
});

test("a panel keeps its size limits", () => {
  assert.equal(nudge({ ...r, w: 165 }, key("ArrowLeft", { altKey: true, shiftKey: true }), screen, panel).w, 160);
  assert.equal(nudge({ ...r, w: 795 }, key("ArrowRight", { altKey: true, shiftKey: true }), screen, panel).w, 800);
  assert.equal(nudge({ ...r, h: 45 }, key("ArrowUp", { altKey: true, shiftKey: true }), screen, panel).h, 40);
});

test("the leaderboard box stays wholly inside the window", () => {
  const b = { x: 1900, y: 1000, w: 100, h: 80 };
  assert.equal(nudge(b, key("ArrowDown", { shiftKey: true }), screen, board).y, 1000, "already at the bottom edge");
  assert.equal(nudge(b, key("ArrowRight"), screen, board).x, 1820, "pulled back inside the screen");
});

test("other keys do nothing, and an arrow at the limit returns the same rectangle", () => {
  assert.equal(nudge(r, key("Enter"), screen, panel), null);
  assert.equal(nudge(r, key("a"), screen, panel), null);
  const edge = { ...r, x: 0 };
  assert.equal(nudge(edge, key("ArrowLeft"), screen, panel), edge);
});
