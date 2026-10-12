// Community hero stats (/heroes/) (T-507): the page's entry. It is small on
// purpose: the sign-in client, the drawing and the data load only after the
// first paint (communityHeroesMain.ts), so the words come first (T-554).

// First: the page's language, before any module draws text (T-508).
import "../lib/i18n/boot.ts";
import { ct } from "../lib/community/strings.ts";
import { afterPaint } from "../lib/view/afterPaint.ts";
import { showLoadFailure } from "../lib/view/loadFailure.ts";
import { byId } from "./dom.ts";

afterPaint()
  .then(() => import("./communityHeroesMain.ts"))
  .catch(() => {
    // The script itself could not be loaded: say so, with a way to try again.
    showLoadFailure(byId("ch-modes"), ct("page.error"), ct("page.error.body"), ct("page.retry"));
    byId("ch-status").textContent = ct("page.error");
    byId("ch").setAttribute("aria-busy", "false");
  });
