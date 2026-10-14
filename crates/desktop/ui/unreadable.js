// A game the parser could not read (T-357): the window says what happened and
// what to do in plain words. The parser's own reason (an error name) is not
// shown; the app's error log keeps it. Tested in ui-tests/unreadable.test.js.
"use strict";

(() => {
  const isUnreadable = (status) => status === "unsupported";

  // `t` looks a word up in the app's language.
  function words(t) {
    return {
      note: t("win.unreadable.note"),
      title: t("win.unreadable.title"),
      what: t("win.unreadable.what"),
      todo: t("win.unreadable.todo"),
    };
  }

  const api = { isUnreadable, words };
  if (typeof module !== "undefined" && module.exports) module.exports = api;
  else window.TLUnreadable = api;
})();
