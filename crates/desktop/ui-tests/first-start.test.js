"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const TLFirstStart = require("../ui/first-start.js");

// A dialog that records what was done to it.
function fakeDialog() {
  return {
    open: false,
    shown: 0,
    showModal() {
      this.open = true;
      this.shown += 1;
    },
    close() {
      this.open = false;
    },
  };
}

// The backend: answers by command name and records every call.
function fakeCall(answers) {
  const calls = [];
  const call = (name, args) => {
    calls.push([name, args]);
    const answer = answers[name];
    return answer instanceof Error ? Promise.reject(answer) : Promise.resolve(answer);
  };
  return { call, calls };
}

test("the question opens when no overlay choice exists yet", async () => {
  const dialog = fakeDialog();
  const { call } = fakeCall({ overlay_asks_first: true });
  assert.equal(await TLFirstStart.ask(call, dialog), true);
  assert.equal(dialog.open, true);
});

test("once a choice exists the question never opens", async () => {
  const dialog = fakeDialog();
  const { call } = fakeCall({ overlay_asks_first: false });
  assert.equal(await TLFirstStart.ask(call, dialog), false);
  assert.equal(dialog.shown, 0);
});

test("when the app cannot say, nothing is asked (the overlay stays as it is)", async () => {
  const dialog = fakeDialog();
  const { call } = fakeCall({ overlay_asks_first: new Error("ipc") });
  assert.equal(await TLFirstStart.ask(call, dialog), false);
  assert.equal(dialog.shown, 0);
});

test("each answer saves the same overlay setting as the tray and Settings", async () => {
  for (const enabled of [true, false]) {
    const dialog = fakeDialog();
    dialog.open = true;
    const { call, calls } = fakeCall({ overlay_set_enabled: { enabled } });
    assert.equal(await TLFirstStart.answer(call, dialog, enabled), null);
    assert.deepEqual(calls, [["overlay_set_enabled", { enabled }]]);
    assert.equal(dialog.open, false);
  }
});

test("an answer the app cannot take keeps the question open and says why", async () => {
  const dialog = fakeDialog();
  dialog.open = true;
  const { call } = fakeCall({ overlay_set_enabled: new Error("boom") });
  assert.equal(await TLFirstStart.answer(call, dialog, true), "Error: boom");
  assert.equal(dialog.open, true);
});
