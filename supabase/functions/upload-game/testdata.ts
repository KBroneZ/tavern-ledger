// Records for the upload-game tests: real parser reports (the shared
// fixtures in crates/bg-parser/tests/data) wrapped as the desktop app keeps them.
import duo from "../../../crates/bg-parser/tests/data/duo_game.json" with { type: "json" };
import solo from "../../../crates/bg-parser/tests/data/solo_local_eliminated.json" with {
  type: "json",
};
import broken from "../../../crates/bg-parser/tests/data/broken_then_ok.json" with {
  type: "json",
};
// Real games from the user's own logs (T-108), as the current parser writes
// them: with start_health and rounds[].health_after (parser revision 2), the
// shop record (revision 3), and the combat results, hero pick, skin links and
// shop extras (revision 4).
import realDuos from "../../../crates/bg-parser/tests/data/real/b253216_duos.json" with {
  type: "json",
};
import realSolo from "../../../crates/bg-parser/tests/data/real/b253216_solo.json" with {
  type: "json",
};

// The Rust parser's reports of the synthetic shop games (every action kind),
// kept current by crates/bg-parser/tests/shop.rs.
import shopGames from "../../../crates/bg-parser/tests/data/rust/solo_shop_games.json" with {
  type: "json",
};
// The Rust parser's reports of the synthetic revision 4 games (every new
// field), kept current by crates/bg-parser/tests/data_round.rs.
import dataGames from "../../../crates/bg-parser/tests/data/rust/data_round_games.json" with {
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

/** A record of a real game as the current desktop app uploads it. */
export function realRecord(report: Json): Json {
  return {
    session: SESSION,
    index: 1,
    saved_at: 1791000000,
    parser: { version: "0.1.0", revision: 4 },
    report: { ...structuredClone(report), index: 1 },
  };
}

export { broken, dataGames, duo, realDuos, realSolo, shopGames, solo };
