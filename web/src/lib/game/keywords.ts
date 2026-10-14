// The minion keywords the parser writes (crates/bg-parser), on a board or in
// a combat replay. Its own module so the report reader and the replay reader
// share it without importing each other.
export const KEYWORDS = [
  "taunt",
  "divine_shield",
  "reborn",
  "venomous",
  "windfury",
  "stealth",
  "deathrattle",
  "rally",
] as const;
export type Keyword = (typeof KEYWORDS)[number];
