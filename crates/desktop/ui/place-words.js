// Words for the expected-place curve in the recap (T-654): the place the
// model expects after each combat, the middle half of the places it gives,
// and the turn that cost the most with what changed most with it (not a
// proven cause). Until a model ships every turn is a dash, said in one quiet
// line. Pure: the curve as the app sends it (`expected_place`, from
// expected_place::curve::to_json) in, plain data out
// (ui-tests/place-words.test.js). Never makes up a number.
"use strict";

(() => {
  const I18n = typeof module !== "undefined" && module.exports ? require("./i18n.js") : window.TLI18n;

  const DASHES = ["no_model", "too_few_games", "other_mode", "other_format"];
  // The features a reason may name (expected_place::NAMES without solo and turn).
  const REASONS = [
    "health", "alive", "health_rank", "tavern_tier", "gold", "rolls", "buys", "minions",
    "attack_sum", "health_sum", "attack_max", "health_max", "golden", "taunt", "divine_shield",
    "reborn", "venomous", "windfury", "deathrattle", "opponent_minions", "opponent_attack_sum",
    "opponent_health_sum", "opponent_health",
  ];
  // The last place of each mode.
  const lastPlace = (duos) => (duos ? 4 : 8);

  // One point, or undefined when it is not well formed for a mode whose last place is `last`.
  function pointOf(p, last) {
    const isPlace = (x) => Number.isInteger(x) && x >= 1 && x <= last;
    const isMean = (x) => typeof x === "number" && Number.isFinite(x) && x >= 1 && x <= last;
    if (!p || !Number.isInteger(p.turn)) return undefined;
    if (p.mean === null) return { turn: p.turn, mean: null };
    if (!isMean(p.mean) || !isPlace(p.low) || !isPlace(p.high) || p.low > p.high) return undefined;
    return { turn: p.turn, mean: p.mean, low: p.low, high: p.high };
  }

  function worstText(worst, points) {
    if (!worst || !Number.isInteger(worst.index) || !points[worst.index] || points[worst.index].turn !== worst.turn) return null;
    return REASONS.includes(worst.reason)
      ? I18n.t("win.place.worstWith", { turn: worst.turn, reason: I18n.t(`win.place.reason.${worst.reason}`) })
      : I18n.t("win.place.worst", { turn: worst.turn });
  }

  const pointSaid = (p) =>
    p.mean === null
      ? I18n.t("win.place.pointUnknown", { turn: p.turn })
      : I18n.t("win.place.point", { turn: p.turn, mean: I18n.num(p.mean, 1), low: p.low, high: p.high });

  // Null when there is nothing to show; { kind: "dash", text } while the
  // model may not be shown; else { kind: "curve", last, points, worst, said }.
  // `duos`: the game's mode, whose last place bounds every place.
  function placeView(curve, duos) {
    if (!curve || typeof curve !== "object" || !Array.isArray(curve.points) || !curve.points.length) return null;
    if (curve.dash !== null && curve.dash !== undefined) {
      const id = DASHES.includes(curve.dash) ? curve.dash : "no_model";
      return { kind: "dash", text: I18n.t(`win.place.dash.${id}`) };
    }
    const last = lastPlace(duos);
    const points = curve.points.map((p) => pointOf(p, last));
    if (points.some((p) => p === undefined) || points.every((p) => p.mean === null)) return null;
    return {
      kind: "curve",
      last,
      points,
      worst: worstText(curve.worst, points),
      said: I18n.t("win.place.label", { points: points.map(pointSaid).join(", ") }),
    };
  }

  const api = { placeView, pointSaid };
  if (typeof module !== "undefined" && module.exports) module.exports = api;
  else window.TLPlace = api;
})();
