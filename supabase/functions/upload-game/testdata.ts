// Records for the upload-game tests: real parser reports (the shared
// fixtures in crates/bg-parser/tests/data) wrapped as the desktop app keeps them.
import duo from "../../../crates/bg-parser/tests/data/duo_game.json" with { type: "json" };
import solo from "../../../crates/bg-parser/tests/data/solo_local_eliminated.json" with {
  type: "json",
};
import broken from "../../../crates/bg-parser/tests/data/broken_then_ok.json" with {
  type: "json",
};

export const SESSION = "Hearthstone_2026_10_09_18_30_00";

// deno-lint-ignore no-explicit-any
export type Json = any;

/** A record as the desktop app keeps it, with a report like a real one. */
export function record(overrides: Json = {}): Json {
  const report = structuredClone(duo[0]) as Json;
  report.card_names = {
    [report.hero]: "Señor supremo Colmillosauro",
    [report.lobby[1].hero]: "El Rey Exánime",
  };
  report.shop_tribes = { BEAST: 4, RACE_99: 1 };
  return {
    session: SESSION,
    index: 1,
    saved_at: 1791000000,
    parser: { version: "0.1.0", revision: 1 },
    report,
    ...overrides,
  };
}

export { broken, duo, solo };
