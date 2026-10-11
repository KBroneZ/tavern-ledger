// Every string the data pages show, in one table (T-501), so T-508 can add
// Spanish beside it without hunting through components. Placeholders are
// {name}; t() refuses a missing one rather than print "{total}".

export const STRINGS = {
  "value.none": "—",
  "unit.games.one": "game",
  "unit.games.other": "games",
  "unit.turns.one": "turn",
  "unit.turns.other": "turns",
  "backing.one": "in 1 {unit}",
  "backing.some": "in {known} of {total} {unit}",
  "backing.all": "in all {total} {unit}",
  "backing.none": "no data in {total} {unit}",
  "backing.empty": "no games yet",
  "threshold.needs": "Needs {min} games to rank (has {games})",
  "trend.better": "better than the previous period",
  "trend.worse": "worse than the previous period",
  "trend.same": "same as the previous period",

  "place.label": "{place} place",
  "place.1": "1st",
  "place.2": "2nd",
  "place.3": "3rd",
  "place.n": "{n}th",
  "place.unknown": "Place unknown",
  "place.group.first": "1st place",
  "place.group.top.solo": "top 4",
  "place.group.top.duos": "top 2",
  "place.group.bottom": "bottom half",

  "result.win": "Won",
  "result.loss": "Lost",
  "result.tie": "Tied",
  "result.unknown": "Unknown",
  "result.win.short": "W",
  "result.loss.short": "L",
  "result.tie.short": "T",
  "result.unknown.short": "?",

  "source.inferred": "inferred",
  "source.inferred.tip": "Worked out from other values, not read straight from the game's log.",
  "source.unknown": "unknown",
  "source.unknown.tip": "The log does not say. Unknown is not zero.",

  "mode.label": "Game mode",
  "mode.solo": "Solo",
  "mode.duos": "Duos",
  "detail.label": "Detail",
  "detail.simple": "Simple",
  "detail.advanced": "Advanced",

  "table.showAll": "Show all {count}",
  "table.showTop": "Show top {count}",

  "state.empty": "Nothing here yet",
  "state.empty.body": "Upload a game from the app and it shows up here.",
  "state.unknown": "Not recorded",
  "state.unknown.body": "Games saved before this was kept have no value for it.",
  "state.loading": "Loading",
  "state.error": "Could not load this",
  "state.error.body": "Try again in a moment. Nothing was changed.",

  "picture.missing": "No picture",

  "chart.table": "Show as a table",
  "chart.places.title": "Places",
  "chart.places.desc": "How many games ended in each place, {mode}.",
  "chart.places.unknown.one": "1 game with no place is left out.",
  "chart.places.unknown.other": "{count} games with no place are left out.",
  "chart.places.col.place": "Place",
  "chart.places.col.games": "Games",
  "chart.places.col.share": "Share",
  "chart.trend.title": "Place over time",
  "chart.trend.desc": "Place in each of the last {count} games, oldest first; higher on the chart is better.",
  "chart.trend.col.game": "Game",
  "chart.trend.col.place": "Place",
  "chart.turns.title": "Turns",
  "chart.turns.desc": "Each turn's combat: bars up are damage dealt, bars down damage taken.",
  "chart.turns.col.turn": "Turn",
  "chart.turns.col.result": "Result",
  "chart.turns.col.damage": "Damage",
  "chart.turns.turn": "Turn {turn}",
} as const;

export type StringKey = keyof typeof STRINGS;
export type Vars = Record<string, string | number>;

/** The string for a key with its placeholders filled. */
export function t(key: StringKey, vars: Vars = {}): string {
  const text: string | undefined = STRINGS[key];
  if (text === undefined) throw new Error(`no string ${key}`);
  return text.replaceAll(/\{([a-z]+)\}/g, (_, name: string) => {
    if (!(name in vars)) throw new Error(`string ${key} needs {${name}}`);
    return String(vars[name]);
  });
}
