// The simulator page's entry (/simulator/): small on purpose. The page's
// script (the boards, the engine's worker, the sign-in client) loads after
// the first paint, so the words come first (T-554). The table's room is
// reserved in the page, so nothing moves when it is drawn (D-112).

// First: the page's language, before any module draws text (T-508).
import "../lib/i18n/boot.ts";
import { sm } from "../lib/sim/strings.ts";
import { afterPaint } from "../lib/view/afterPaint.ts";
import { showLoadFailure } from "../lib/view/loadFailure.ts";

afterPaint()
  .then(() => import("../lib/sim/main.ts"))
  .catch(() => {
    // The script itself could not be loaded: say so, with a way to try again.
    const result = document.getElementById("sim-result");
    if (result) showLoadFailure(result, sm("result.failed"), "", sm("result.retry"));
    const status = document.getElementById("sim-status");
    if (status) status.textContent = sm("result.failed");
    for (const id of ["sim-opponent", "sim-player"]) document.getElementById(id)?.setAttribute("aria-busy", "false");
  });
