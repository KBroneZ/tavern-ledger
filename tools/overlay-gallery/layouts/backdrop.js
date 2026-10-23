// A plain stand-in for the game window behind the layout round's frames:
// flat shapes where the game puts its leaderboard, tavern, boards, hero and
// hand, so a layout can be judged against what it sits next to. No game art:
// shapes only, places approximate except the leaderboard, which is where the
// app measures it (default-layout.js). Drawn in the game's 16:9 picture,
// 1600 x 900 units, centred in the window like the game.
"use strict";

(() => {
  const node = typeof module !== "undefined" && module.exports;
  const defaults = node ? require("../default-layout.js") : window.TLGalleryDefaults;

  const W = 1600;
  const H = 900;
  const SLOTS = 8;
  const COLOURS = { table: "#2a231d", edge: "#16120f", board: "#352c24", rim: "#5b4a37", shape: "#4a3e33", line: "#76624b", gold: "#6e5a2c" };

  const oval = (cx, cy) => `<ellipse cx="${cx}" cy="${cy}" rx="50" ry="64" fill="${COLOURS.shape}" stroke="${COLOURS.line}" stroke-width="3"/>`;
  const circle = (cx, cy, r) => `<circle cx="${cx}" cy="${cy}" r="${r}" fill="${COLOURS.shape}" stroke="${COLOURS.line}" stroke-width="4"/>`;
  const row = (count, cy) => Array.from({ length: count }, (_, i) => oval(W / 2 + (i - (count - 1) / 2) * 128, cy)).join("");

  // The game's leaderboard, in picture units: eight heroes, in pairs in Duos.
  function leaderboard(duos) {
    const area = defaults.leaderboardArea(W, H);
    const each = area.h / SLOTS;
    const parts = [];
    for (let i = 0; i < SLOTS; i += 1) {
      const y = area.y + i * each;
      parts.push(`<rect x="${area.x + 3}" y="${y + 3}" width="${area.w - 6}" height="${each - 6}" rx="10" fill="${COLOURS.board}" stroke="${COLOURS.line}" stroke-width="2"/>`);
      parts.push(`<circle cx="${area.x + area.w / 2}" cy="${y + each / 2}" r="${each * 0.32}" fill="${COLOURS.shape}"/>`);
    }
    if (duos) {
      for (let i = 0; i < SLOTS; i += 2) {
        parts.push(`<rect x="${area.x - 2}" y="${area.y + i * each}" width="${area.w + 4}" height="${each * 2}" rx="12" fill="none" stroke="${COLOURS.rim}" stroke-width="4"/>`);
      }
    }
    return parts.join("");
  }

  // Recruit: the tavern's keeper and minions on top, the buttons beside the
  // keeper, the player's board, hero and hand below. Combat: the opponent's
  // hero and board on top, the player's below.
  function svg(scene) {
    const recruit = scene.phase !== "combat";
    const top = recruit
      ? [circle(800, 160, 58), circle(690, 150, 30), circle(910, 150, 28), row(5, 330)].join("")
      : [circle(800, 150, 58), row(7, 330)].join("");
    const hand = recruit
      ? Array.from({ length: 4 }, (_, i) => `<rect x="${800 + (i - 1.5) * 84 - 45}" y="820" width="90" height="125" rx="10" fill="${COLOURS.shape}" stroke="${COLOURS.line}" stroke-width="3"/>`).join("")
      : "";
    return [
      `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${W} ${H}" preserveAspectRatio="xMidYMid meet" width="100%" height="100%" aria-hidden="true">`,
      `<defs><radialGradient id="tl-table" cx="50%" cy="45%" r="75%"><stop offset="0" stop-color="${COLOURS.table}"/><stop offset="1" stop-color="${COLOURS.edge}"/></radialGradient></defs>`,
      `<rect width="${W}" height="${H}" fill="url(#tl-table)"/>`,
      `<rect x="330" y="85" width="940" height="730" rx="44" fill="${COLOURS.board}" stroke="${COLOURS.rim}" stroke-width="5"/>`,
      `<line x1="400" y1="430" x2="1200" y2="430" stroke="${COLOURS.rim}" stroke-width="2" stroke-dasharray="10 12"/>`,
      top,
      row(recruit ? 6 : 7, 530),
      circle(800, 715, 62),
      circle(905, 735, 28),
      hand,
      `<rect x="1285" y="408" width="130" height="56" rx="22" fill="${recruit ? COLOURS.gold : COLOURS.shape}" stroke="${COLOURS.line}" stroke-width="3"/>`,
      leaderboard(scene.duos),
      "</svg>",
    ].join("");
  }

  // Behind everything the page draws; the bars around the picture stay black.
  function draw(win, scene) {
    if (!scene) return;
    const doc = win.document;
    const back = doc.createElement("div");
    back.id = "gallery-backdrop";
    back.setAttribute("aria-hidden", "true");
    back.style.cssText = "position:fixed;inset:0;z-index:-1;background:#000;pointer-events:none";
    back.innerHTML = svg(scene);
    doc.body.prepend(back);
  }

  const api = { svg, draw };
  if (node) module.exports = api;
  else window.TLGalleryBackdrop = api;
})();
