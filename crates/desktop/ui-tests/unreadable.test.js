"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const U = require("../ui/unreadable.js");
const I18n = require("../ui/i18n.js");
require("../ui/strings/common.js");
require("../ui/strings/window.js");

const TECHNICAL = /--reparse|tavern-watch|parser|NotImplementedError|Exception/i;

test("only a game the parser could not read counts as unreadable", () => {
  assert.equal(U.isUnreadable("unsupported"), true);
  assert.equal(U.isUnreadable("ok"), false);
  assert.equal(U.isUnreadable("incomplete"), false);
});

test("an unreadable game explains itself in plain words, in both languages", () => {
  for (const lang of ["en", "es"]) {
    I18n.setLang(lang);
    const words = U.words((key) => I18n.t(key));
    for (const part of [words.note, words.title, words.what, words.todo]) {
      assert.ok(part && !part.startsWith("win."), `${lang}: a word is missing`);
      assert.doesNotMatch(part, TECHNICAL, `${lang}: technical wording in "${part}"`);
    }
    assert.match(words.todo, lang === "en" ? /Report a problem/ : /Informar de un problema/);
  }
  I18n.setLang("en");
});

test("a cause the parser words for the user is kept, a parser error name is not", () => {
  const problems = [
    "parser error: NotImplementedError",
    "the game log has too many unreadable lines (restart Hearthstone and play a new game)",
    "the game log has no times (restart Hearthstone and play a new game)",
    "Player Jane#1234 left",
  ];
  assert.deepEqual(U.userReasons(problems), problems.slice(1, 3));
  assert.deepEqual(U.userReasons(["parser error: LineTooLong"]), []);
  assert.deepEqual(U.userReasons(undefined), []);
});

test("a reason is written as a sentence", () => {
  assert.equal(U.sentence("the game log has no times (restart Hearthstone)"), "The game log has no times (restart Hearthstone).");
});
