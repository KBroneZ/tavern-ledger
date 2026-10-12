// Public leaderboards (/leaderboards/) (T-506): the page's entry. It is small
// on purpose: the sign-in client, the boards' drawing and the data load only
// after the first paint (leaderboardsMain.ts), so the words come first (T-554).

// First: the page's language, before any module draws text (T-508).
import "../lib/i18n/boot.ts";
import { lt } from "../lib/leaderboards/strings.ts";
import { afterPaint } from "../lib/view/afterPaint.ts";
import { byId } from "./dom.ts";

afterPaint()
  .then(() => import("./leaderboardsMain.ts"))
  .catch(() => {
    // The script itself could not be loaded: say so instead of leaving the skeleton.
    byId("lb-status").textContent = lt("page.error");
    byId("lb").setAttribute("aria-busy", "false");
  });
