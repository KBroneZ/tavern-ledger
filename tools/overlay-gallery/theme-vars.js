// The overlay's theme colours, copied from crates/desktop/src/overlay_layout.rs
// (`Theme::palette` and `css_vars`, at the default opacity). The page has no
// colours without them. ui-tests/overlay-gallery.test.js fails when a name or
// a value here differs from the Rust source.
"use strict";

(() => {
  const THEME_VARS = {
    glass: {
      "--panel-bg": "rgba(30, 22, 18, 0.90)",
      "--panel-solid": "#1e1612",
      "--text": "#f7efe6",
      "--dim": "#cbbdaf",
      "--accent": "#ffd08a",
      "--won": "#8fe0a8",
      "--lost": "#ff9a8a",
      "--amber": "#ffb24a",
      "--info": "#a9c8ff",
      "--muted": "#c4b6a8",
      "--danger": "#ffd0c6",
      "--border": "#9a6a30",
    },
    contrast: {
      "--panel-bg": "rgba(0, 0, 0, 0.90)",
      "--panel-solid": "#000000",
      "--text": "#ffffff",
      "--dim": "#eaeaea",
      "--accent": "#fff000",
      "--won": "#7aff7a",
      "--lost": "#ffb0a8",
      "--amber": "#ffc400",
      "--info": "#8adcff",
      "--muted": "#dcdcdc",
      "--danger": "#ffc0c0",
      "--border": "#ffffff",
    },
  };
  const THEMES = [
    { id: "glass", label: "Glass (dark)" },
    { id: "contrast", label: "High contrast" },
  ];
  const api = { THEME_VARS, THEMES };
  if (typeof module !== "undefined" && module.exports) module.exports = api;
  else window.TLGalleryThemes = api;
})();
