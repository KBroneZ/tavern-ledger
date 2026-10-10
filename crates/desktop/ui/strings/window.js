// Words of the main window: index.html, app.js, upload.js, overlay-switch.js and settings.js (T-325). English is the source; Spanish (Spain) is complete
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
