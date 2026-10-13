"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const Forward = require("../ui/error-forward.js");

// A window with only what the script uses: listeners by event name.
function page() {
  const listeners = {};
  return {
    addEventListener: (type, fn) => { listeners[type] = fn; },
    fire: (type, event) => listeners[type](event),
  };
}

function recorder(result = Promise.resolve()) {
  const calls = [];
  const call = (command, args) => {
    calls.push({ command, args });
    return typeof result === "function" ? result() : result;
  };
  return { calls, call };
}

test("a script error is sent with its message and where it happened", () => {
  const win = page();
  const sent = recorder();
  Forward.install(win, sent.call);
  win.fire("error", { message: "x is not defined", filename: "http://tauri.localhost/app.js?v=3", lineno: 12, colno: 5 });
  assert.deepEqual(sent.calls, [
    { command: "log_ui_error", args: { kind: "error", message: "x is not defined", place: "app.js:12:5" } },
  ]);
});

test("an unhandled rejection is sent with its reason", () => {
  const win = page();
  const sent = recorder();
  Forward.install(win, sent.call);
  win.fire("unhandledrejection", { reason: new Error("no answer") });
  win.fire("unhandledrejection", { reason: "plain text" });
  win.fire("unhandledrejection", {});
  assert.deepEqual(sent.calls.map((c) => [c.args.kind, c.args.message, c.args.place]), [
    ["rejection", "no answer", null],
    ["rejection", "plain text", null],
    ["rejection", "unknown reason", null],
  ]);
});

test("a long message is cut before it is sent", () => {
  const win = page();
  const sent = recorder();
  Forward.install(win, sent.call);
  win.fire("error", { message: "m".repeat(2000), filename: "", lineno: 0, colno: 0 });
  assert.equal(sent.calls[0].args.message.length, 300);
  assert.equal(sent.calls[0].args.place, null);
});

test("an error event without a message uses the error's own", () => {
  const win = page();
  const sent = recorder();
  Forward.install(win, sent.call);
  win.fire("error", { error: new Error("inner") });
  assert.equal(sent.calls[0].args.message, "inner");
});

test("a failing send never raises another error", async () => {
  const win = page();
  const rejecting = recorder(() => Promise.reject(new Error("denied")));
  Forward.install(win, rejecting.call);
  assert.doesNotThrow(() => win.fire("error", { message: "a" }));
  const win2 = page();
  Forward.install(win2, () => { throw new Error("no bridge"); });
  assert.doesNotThrow(() => win2.fire("error", { message: "b" }));
  await new Promise((resolve) => setImmediate(resolve));
});

test("the place is the file name, not the whole address", () => {
  assert.equal(Forward.placeOf("http://tauri.localhost/ui/replay.js#x", 3, 4), "replay.js:3:4");
  assert.equal(Forward.placeOf("", 1, 1), null);
});

test("a cut never splits an emoji", () => {
  const win = page();
  const sent = recorder();
  Forward.install(win, sent.call);
  win.fire("error", { message: "😀".repeat(400) });
  const message = sent.calls[0].args.message;
  assert.equal(Array.from(message).length, 300);
  assert.doesNotMatch(message, /[\ud800-\udbff](?![\udc00-\udfff])/);
});
