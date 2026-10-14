// The keyword marks look like the game's (D-136) but are our own SVG files,
// no game image or asset, one file each, shared by the combat replay, the
// simulator's cards and the app's replay (T-581).
import { test } from "node:test";
import assert from "node:assert/strict";
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";

const ROOT = join(import.meta.dirname, "..", "..", "..");
const WEB_ART = join(ROOT, "web", "public", "art", "keywords");
const APP_ART = join(ROOT, "crates", "desktop", "ui", "keywords");
// The app's replay never carries Poisonous (Battlegrounds dropped it for
// Venomous); the simulator's card data still can.
const APP = ["taunt.svg", "taunt-golden.svg", "venomous.svg", "venom-cloud.svg", "deathrattle.svg", "deathrattle-bones.svg", "reborn.svg", "windfury.svg"];
const ART = [...APP, "poisonous.svg"];
const css = (...parts: string[]) => readFileSync(join(ROOT, ...parts), "utf8");

/** The body of the first rule whose selector list is exactly `selector`. */
function rule(sheet: string, selector: string): string {
  const at = sheet.indexOf(`\n${selector} {`);
  assert.ok(at >= 0, selector);
  return sheet.slice(at, sheet.indexOf("}", at));
}

test("every mark is a plain SVG of our own: no script, no link, no embedded picture", () => {
  assert.deepEqual(readdirSync(WEB_ART).sort(), [...ART].sort());
  for (const name of ART) {
    const svg = readFileSync(join(WEB_ART, name), "utf8");
    assert.match(svg, /^<svg xmlns="http:\/\/www\.w3\.org\/2000\/svg" viewBox="[-\d. ]+"/, name);
    assert.doesNotMatch(svg, /<script|<image|<foreignObject|href=|url\((?!#)|\bon\w+=/i, name);
  }
});

test("the app draws the same marks as the website, byte for byte", () => {
  assert.deepEqual(readdirSync(APP_ART).sort(), [...APP].sort());
  for (const name of APP) assert.equal(readFileSync(join(APP_ART, name), "utf8"), readFileSync(join(WEB_ART, name), "utf8"), name);
});

test("the replay draws Taunt as the shield, gold on a golden minion, and Venomous as a vial under the portrait", () => {
  const sheet = css("web", "src", "components", "game", "replay.css");
  assert.match(rule(sheet, ".rp-k-taunt"), /url\("\/art\/keywords\/taunt\.svg"\)/);
  assert.match(rule(sheet, ".rp-minion.is-golden .rp-k-taunt"), /url\("\/art\/keywords\/taunt-golden\.svg"\)/);
  const venom = rule(sheet, ".rp-k-venomous");
  assert.match(venom, /url\("\/art\/keywords\/venomous\.svg"\)/);
  assert.match(venom, /left: 50%/, "centred, as the game's icons under a portrait");
  assert.match(rule(sheet, ".rp-minion.gains-venomous .rp-keys::after"), /url\("\/art\/keywords\/venom-cloud\.svg"\)/, "a green cloud when gained, as the game's");
  assert.match(rule(sheet, ".rp-k-deathrattle"), /url\("\/art\/keywords\/deathrattle\.svg"\)/);
  assert.match(rule(sheet, ".rp-minion.has-venomous .rp-k-deathrattle"), /url\("\/art\/keywords\/deathrattle-bones\.svg"\)/, "with Venomous, the vial stands in the skull's place");
  assert.match(rule(sheet, ".rp-k-reborn"), /url\("\/art\/keywords\/reborn\.svg"\)/);
  assert.match(rule(sheet, ".rp-k-windfury"), /url\("\/art\/keywords\/windfury\.svg"\)/);
});

test("the simulator's cards wear the same drawings", () => {
  const sheet = css("web", "src", "components", "sim", "sim-cards.css");
  assert.match(rule(sheet, ".sim-taunt"), /url\("\/art\/keywords\/taunt\.svg"\)/);
  assert.match(rule(sheet, ".sim .sim-card.is-golden .sim-taunt"), /url\("\/art\/keywords\/taunt-golden\.svg"\)/);
  assert.match(rule(sheet, ".sim-venom"), /url\("\/art\/keywords\/venomous\.svg"\)/);
  assert.match(rule(sheet, ".sim .sim-card.has-poisonous:not(.has-venomous) .sim-venom"), /url\("\/art\/keywords\/poisonous\.svg"\)/, "Poisonous keeps its round flask");
  assert.match(rule(sheet, ".sim-wind"), /url\("\/art\/keywords\/windfury\.svg"\)/);
  assert.match(sheet, /url\("\/art\/keywords\/reborn\.svg"\)/);
});

test("the app's replay wears the same drawings", () => {
  const sheet = css("crates", "desktop", "ui", "replay.css");
  for (const [selector, name] of [
    [".replay-k-taunt", "taunt"],
    [".replay-minion.golden .replay-k-taunt", "taunt-golden"],
    [".replay-k-venomous", "venomous"],
    [".replay-minion.gains-venomous .replay-keys::after", "venom-cloud"],
    [".replay-k-deathrattle", "deathrattle"],
    [".replay-minion.has-venomous .replay-k-deathrattle", "deathrattle-bones"],
    [".replay-k-reborn", "reborn"],
    [".replay-k-windfury", "windfury"],
  ]) assert.match(rule(sheet, selector), new RegExp(`url\\("keywords/${name}\\.svg"\\)`), selector);
});

test("the replay marks Deathrattle too, on the website and in the app", async () => {
  const { MARKED } = await import("../../src/lib/game/replayView.ts");
  assert.ok(MARKED.includes("deathrattle"));
  assert.match(css("crates", "desktop", "ui", "replay.js"), /const MARKED = \[[^\]]*"deathrattle"/);
});

test("the website and the app show, pop in and fade the same marks", () => {
  const marks = (sheet: string, prefix: string) => {
    const found = new Set<string>();
    const pattern = String.raw`\.${prefix}-minion(?::is\()?\.?(has|gains|loses)-([a-z_]+)[^{]*?\.${prefix}-k-([a-z_]+)`;
    for (const m of sheet.matchAll(new RegExp(pattern, "g"))) found.add(`${m[1]} ${m[2]} ${m[3]}`);
    return [...found].sort();
  };
  const web = marks(css("web", "src", "components", "game", "replay.css"), "rp");
  const app = marks(css("crates", "desktop", "ui", "replay.css"), "replay");
  assert.ok(web.length >= 18, String(web.length));
  assert.deepEqual(app, web);
});

test("seven minions and their gaps fit any board from 210px, the gap a quarter of a minion when there is room", () => {
  for (const sheet of [css("web", "src", "components", "game", "replay.css"), css("crates", "desktop", "ui", "replay.css")]) {
    assert.match(sheet, /clamp\(28px, calc\(100cqi \/ 8\.82\), 104px\)/);
    assert.match(sheet, /margin-inline: min\(calc\(var\(--(rp-)?tile\) \* 0\.13\), max\(1px, calc\(\(100cqi - 7 \* var\(--(rp-)?tile\)\) \/ 14\)\)\)/);
  }
  for (let board = 210; board <= 2000; board += 1) {
    const tile = Math.min(104, Math.max(28, board / 8.82));
    const margin = Math.min(tile * 0.13, Math.max(1, (board - 7 * tile) / 14));
    assert.ok(7 * (tile + 2 * margin) <= board + 0.01, `board ${board}`);
    if (board >= 247 && tile < 104) assert.ok(Math.abs(margin - tile * 0.13) < 0.01, `board ${board}: a quarter of a minion between two`);
  }
});
