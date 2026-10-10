// Words of the overlay: overlay.html and the overlay scripts (T-325). English is the source; Spanish (Spain) is complete
// (ui-tests/i18n.test.js). Shared words (tribes, keywords, sources, modes,
// results, languages) are in common.js.
"use strict";

(() => {
  const table = {
    en: {
    },
    es: {
    },
  };

  const i18n = typeof module !== "undefined" && module.exports ? require("../i18n.js") : window.TLI18n;
  i18n.define(table);
  if (typeof module !== "undefined" && module.exports) module.exports = table;
})();
