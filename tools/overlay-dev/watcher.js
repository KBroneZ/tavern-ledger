// Tells the server what changed in the watched folders. Changes that come
// close together (an editor saves in a few steps) are reported once.
"use strict";

const fs = require("node:fs");

const SETTLE_MS = 40;
const EDITOR_NOISE = /(~|\.swp|\.tmp|\.crswap)$/;

// `watches` is [{ dir, tag }]; `onChange(changes)` gets [{ tag, file }] with
// `file` relative to its folder, in forward slashes. Returns a function that
// stops everything, including a pending report.
function watchFolders(watches, onChange) {
  let pending = [];
  let timer = null;
  const flush = () => {
    timer = null;
    const changes = pending;
    pending = [];
    if (changes.length) onChange(changes);
  };
  const watchers = [];
  for (const { dir, tag } of watches) {
    try {
      const watcher = fs.watch(dir, { recursive: true }, (_event, name) => {
        if (!name || EDITOR_NOISE.test(name)) return;
        pending.push({ tag, file: String(name).replace(/\\/g, "/") });
        if (timer === null) timer = setTimeout(flush, SETTLE_MS);
      });
      watcher.on("error", (error) => console.warn(`overlay-dev: stopped watching ${dir}: ${error.message}`));
      watchers.push(watcher);
    } catch (error) {
      console.warn(`overlay-dev: cannot watch ${dir}: ${error.message}`);
    }
  }
  return () => {
    if (timer !== null) clearTimeout(timer);
    timer = null;
    pending = [];
    for (const watcher of watchers) watcher.close();
  };
}

module.exports = { watchFolders };
