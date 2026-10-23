// "Today and this week" as the dashboard draws it (T-361): the states, the
// links and the words, in English and Spanish. Made-up games.
import { afterEach, test } from "node:test";
import assert from "node:assert/strict";
import { periodBounds, summarizePeriod } from "../../src/lib/data/period.ts";
import { gameRow } from "../../src/lib/data/rows.ts";
import { setLang } from "../../src/lib/i18n.ts";
import { periodFailure, periodSection } from "../../src/lib/view/periodView.ts";
import { h } from "../../src/lib/view/node.ts";
import { elements, textOf, toHtml } from "../helpers/html.ts";
import { gameRaw } from "../helpers/rows.ts";

const USER = "0f8fad5b-d9cb-469f-a165-70867728950e";
const WED = "2026-10-21";
const bounds = periodBounds(WED);

afterEach(() => setLang("en"));

function game(n: number, day: string, place: number | null, mode = "solo") {
  return gameRow(
    gameRaw({
      public_id: `00000000-0000-4000-8000-${String(n).padStart(12, "0")}`,
      played_on: day,
      counted_place: place,
      mode,
      status: place === null ? "incomplete" : "ok",
    }),
  );
}

const text = (nodes: ReturnType<typeof periodSection>) => nodes.map(textOf).join(" ");

test("a week with games: counts, average and record per mode, and the week starts on Monday", () => {
  const rows = [game(3, WED, 5), game(2, WED, 1), game(1, "2026-10-19", 3), game(4, "2026-10-20", 2, "duos")];
  const said = text(periodSection(summarizePeriod(rows, bounds), USER));
  assert.match(said, /Today and this week/);
  assert.match(said, /Weeks start on Monday \(this one on Oct 19, 2026\)/);
  assert.match(said, /3 games/);
  assert.match(said, /average place 3\.0/);
  assert.match(said, /2 in the top 4, 1 in the bottom 4/);
  assert.match(said, /1 in the top 2, 0 in the bottom 2/);
  assert.match(said, /Best place · .* · place 1/);
});

test("no games says no games, in each column", () => {
  const said = text(periodSection(summarizePeriod([], bounds), USER));
  assert.match(said, /No games today\./);
  assert.match(said, /No games this week\./);
});

test("games with no finished place say so instead of an average of nothing", () => {
  const said = text(periodSection(summarizePeriod([game(1, WED, null)], bounds), USER));
  assert.match(said, /average place: no finished game/);
  assert.match(said, /record: no finished game/);
  assert.doesNotMatch(said, /average place \d/);
});

test("the rating change is said to be on the PC, never shown as a number", () => {
  const said = text(periodSection(summarizePeriod([game(1, WED, 2)], bounds), USER));
  assert.match(said, /rating change: not on the website/);
  assert.doesNotMatch(said, /rating change [+\-−]?\d/);
});

test("a game worth revisiting links to its game page for this player", () => {
  const nodes = periodSection(summarizePeriod([game(1, WED, 1)], bounds), USER);
  const link = nodes.flatMap(elements).find((e) => e.tag === "a");
  assert.ok(link);
  const href = link.attrs.find(([n]) => n === "href")?.[1] ?? "";
  assert.match(href, new RegExp(`u=${USER}`));
  assert.match(href, /id=00000000-0000-4000-8000-000000000001/);
});

test("no streaks are worded", () => {
  const html = periodSection(summarizePeriod([game(1, WED, 1), game(2, WED, 2)], bounds), USER).map(toHtml).join("");
  assert.doesNotMatch(html, /streak|racha/i);
});

test("the same summary in Spanish", () => {
  setLang("es");
  const said = text(periodSection(summarizePeriod([game(1, WED, 2), game(2, WED, 6)], bounds), USER));
  assert.match(said, /Hoy y esta semana/);
  assert.match(said, /Las semanas empiezan el lunes/);
  assert.match(said, /2 partidas/);
  assert.match(said, /puesto medio 4,0/);
  assert.match(said, /1 en los 4 primeros, 1 en los 4 últimos/);
  assert.match(said, /cambio de rating: no está en la web/);
});

test("a failure is said in its own place with a retry, and offers no numbers", () => {
  const retry = h("button", { type: "button" }, "Try again");
  const said = periodFailure(retry).map(textOf).join(" ");
  assert.match(said, /The summary could not be loaded/);
  assert.match(said, /The rest of the dashboard is fine/);
  assert.match(said, /Try again/);
});
