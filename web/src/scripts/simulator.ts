// The simulator page's entry (/simulator/): small on purpose. The page's
// script (the boards, the engine's worker, the sign-in client) loads after
// the first paint, so the words come first (T-554). The table's room is
// reserved in the page, so nothing moves when it is drawn (D-112).

// First: the page's language, before any module draws text (T-508).
import "../lib/i18n/boot.ts";
import { sm } from "../lib/sim/strings.ts";
import { afterPaint } from "../lib/view/afterPaint.ts";

afterPaint()
  .then(() => import("../lib/sim/main.ts"))
  .catch(() => {
    // The script itself could not be loaded: say so instead of leaving a table that does nothing.
    const status = document.getElementById("sim-status");
    if (status) status.textContent = sm("result.failed");
  });
