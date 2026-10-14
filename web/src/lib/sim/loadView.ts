// "Pick a game" as element trees (T-606, T-526): the signed-in player's own
// games, newest first, then the rounds of the one picked ("Pick a fight"). Signed out, a
// note and a link to sign in; the editor works either way.
import { dateText, modeName } from "../data/dashboard.ts";
import type { GameRow } from "../data/rows.ts";
import { heroName } from "../heroes/names.ts";
import { href } from "../i18n.ts";
import { cls, h, type VNode } from "../view/node.ts";
import { picture, placeBadge, stateBlock } from "../view/pieces.ts";
import type { RoundChoice } from "./board.ts";
import { sm } from "./strings.ts";

export type GamesState =
  | { kind: "signed-out" }
  | { kind: "loading" }
  | { kind: "error" }
  | { kind: "ready"; rows: GameRow[]; total: number; more: boolean };

export interface PickedGame {
  key: string;
  /** null while the record loads; [] when it has no round to load. */
  rounds: RoundChoice[] | null;
  failed: boolean;
  /** The round now in the editor, if it came from this game. */
  loaded: number | null;
}

export const gameKey = (row: Pick<GameRow, "publicId">): string | null => row.publicId;

function gameButton(row: GameRow, picked: PickedGame | null): VNode | null {
  const key = gameKey(row);
  if (!key) return null;
  const on = picked?.key === key;
  const name = heroName(row.hero);
  return h(
    "li",
    {},
    h(
      "button",
      { type: "button", class: cls("sim-game", { "is-on": on }), "data-act": "game", "data-game": key, "data-focus": `game-${key}`, "aria-pressed": on ? "true" : "false" },
      picture("hero", row.hero, name ?? "", 32, true),
      h(
        "span",
        { class: "sim-game-text" },
        h("span", {}, name ?? dateText(row.playedOn)),
        h("span", { class: "sim-dim" }, name ? `${dateText(row.playedOn)} · ${modeName(row.mode)}` : modeName(row.mode)),
      ),
      placeBadge(row.place, row.mode),
    ),
  );
}

/** A step of the main flow (pick a game, pick a fight, the odds), as a wax seal before its heading. */
export const stepSeal = (n: number): VNode => h("span", { class: "sim-step", "aria-hidden": "true" }, String(n));

const RESULT_CLASS = { won: "is-won", lost: "is-lost", tie: "is-tie" } as const;

function rounds(picked: PickedGame): VNode {
  if (picked.failed) return h("p", { class: "state state-error" }, sm("load.gameError"));
  if (picked.rounds === null) return stateBlock({ kind: "loading", title: sm("load.loading"), lines: 1 });
  if (picked.rounds.length === 0) return h("p", { class: "sim-dim" }, sm("load.noRounds"));
  return h(
    "div",
    { class: "sim-rounds-wrap" },
    h("h3", { class: "sim-sub", id: "sim-rounds-label" }, stepSeal(2), sm("load.rounds")),
    h(
      "ul",
      { class: "sim-rounds", role: "list", "aria-labelledby": "sim-rounds-label" },
      picked.rounds.map((r) =>
        h(
          "li",
          {},
          h(
            "button",
            {
              type: "button",
              class: cls("sim-round", r.result ? RESULT_CLASS[r.result] : null),
              "data-act": "round",
              "data-round": r.number,
              "data-focus": `round-${r.number}`,
              "aria-pressed": picked.loaded === r.number ? "true" : "false",
            },
            sm("load.round", { n: r.number }),
          ),
        ),
      ),
    ),
  );
}

/** Step 2 before a game is picked: kept on the page, so the seals read 1, 2, 3 in order. */
export function roundsWaiting(): VNode {
  return h("div", { class: "sim-rounds-wrap" }, h("h3", { class: "sim-sub" }, stepSeal(2), sm("load.rounds")), h("p", { class: "sim-dim" }, sm("load.roundsWait")));
}

export function loadView(games: GamesState, picked: PickedGame | null): VNode[] {
  if (games.kind === "signed-out") {
    return [h("p", { class: "sim-dim" }, sm("load.signedOut"), " ", h("a", { href: href("/signin/") }, sm("load.signIn"))), roundsWaiting()];
  }
  if (games.kind === "loading") return [stateBlock({ kind: "loading", title: sm("load.loading"), lines: 2 })];
  if (games.kind === "error") {
    return [
      h("div", { class: "state state-error" }, h("p", {}, sm("load.error")), h("button", { type: "button", "data-act": "games-retry", "data-focus": "games-retry" }, sm("load.retry"))),
    ];
  }
  if (games.rows.length === 0) return [h("p", { class: "sim-dim" }, sm("load.none"))];
  return [
    h(
      "ul",
      { class: "sim-games", role: "list", "aria-label": sm("load.games") },
      games.rows.map((r) => gameButton(r, picked)),
    ),
    games.more ? h("button", { type: "button", class: "sim-btn sim-btn-quiet", "data-act": "games-more", "data-focus": "games-more" }, sm("load.more")) : null,
    picked ? rounds(picked) : roundsWaiting(),
  ].filter((n): n is VNode => n !== null);
}
