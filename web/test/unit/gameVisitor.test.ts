// The game viewer talks to a game's own player as "you" and to anyone else
// (anonymous, or another account) about "the player" (D-102).
import { afterEach, test } from "node:test";
import assert from "node:assert/strict";
import { setLang } from "../../src/lib/i18n.ts";
import { g, GAME_STRINGS, isVisitor, setSeenByVisitor, type GameStringKey } from "../../src/lib/game/strings.ts";
import { ES } from "../../src/lib/game/strings.es.ts";

afterEach(() => {
  setSeenByVisitor(false);
  setLang("en");
});

const TWINS = Object.keys(GAME_STRINGS)
  .filter((k) => k.endsWith(".visitor"))
  .map((k) => k.slice(0, -".visitor".length) as GameStringKey);

test("the owner reads \"you\"; a visitor reads \"the player\" for every twinned string", () => {
  assert.ok(TWINS.length > 0);
  assert.equal(g("turns.dealt", { n: 3 }), "you dealt 3 damage");
  assert.equal(g("lobby.you"), "you");
  setSeenByVisitor(true);
  assert.equal(g("turns.dealt", { n: 3 }), "dealt 3 damage");
  assert.equal(g("lobby.you"), "this player");
  assert.match(g("replay.about"), /the player's screen/);
});

test("no visitor string says you, your or tu in either language", () => {
  for (const key of TWINS) {
    assert.doesNotMatch(GAME_STRINGS[`${key}.visitor` as GameStringKey], /\byou(r)?\b/i, key);
    assert.doesNotMatch(ES[`${key}.visitor` as GameStringKey], /(^|[^\p{L}])(tu|tus|tú|ti)([^\p{L}]|$)/iu, key);
  }
});

test("a visitor's twin keeps the placeholders of the owner's string", () => {
  const holes = (text: string) => [...text.matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort();
  for (const key of TWINS) {
    assert.deepEqual(holes(GAME_STRINGS[`${key}.visitor` as GameStringKey]), holes(GAME_STRINGS[key]), key);
    assert.deepEqual(holes(ES[`${key}.visitor` as GameStringKey]), holes(ES[key]), key);
  }
});

test("Spanish: the visitor reads \"el jugador\"", () => {
  setLang("es");
  setSeenByVisitor(true);
  assert.equal(g("lobby.you"), "este jugador");
});

test("only the game's own account is not a visitor: signed out and other accounts are", () => {
  const owner = "0f8fad5b-d9cb-469f-a165-70867728950e";
  assert.equal(isVisitor(undefined, owner), true, "signed out");
  assert.equal(isVisitor("11111111-d9cb-469f-a165-70867728950e", owner), true, "another account");
  assert.equal(isVisitor(owner, owner), false, "the owner");
  assert.equal(isVisitor(owner.toUpperCase(), owner), false, "case does not matter");
});
