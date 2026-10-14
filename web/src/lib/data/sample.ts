// Made-up data for the /style/ page (T-501), also used by test/build to check
// that the browser pieces (lib/view/pieces.ts) draw the same markup as the
// Astro components (T-503). No player's data and no game art.
import { average, share } from "./format.ts";
import type { Cell, Column } from "../view/pieces.ts";

export const soloPlaces = [3, 1, 5, 2, 4, 7, 1, 3, 6, 2, 4, 8, 3, 2, 5, 1, 4, 3, null, 6, 2, 4, 3, 5, 1, 2, 7, 4, 3, 2, 4];
export const duosPlaces = [2, 1, 3, 2, 4, 1, 2, 3, 2, 1, 4, 2];
export const recent = soloPlaces.slice(-12);

const heroes: { id: string; name: string; games: number; avg: number; top: number; best: number }[] = [
  { id: "SAMPLE_HERO_01", name: "Lantern Keeper", games: 9, avg: 2.89, top: 7, best: 1 },
  { id: "SAMPLE_HERO_02", name: "Brass Captain", games: 6, avg: 3.5, top: 4, best: 1 },
  { id: "SAMPLE_HERO_03", name: "Moss Witch", games: 4, avg: 4.25, top: 2, best: 2 },
  { id: "SAMPLE_HERO_04", name: "Tide Caller", games: 3, avg: 3.0, top: 2, best: 2 },
  { id: "SAMPLE_HERO_05", name: "Ember Cook", games: 3, avg: 5.67, top: 1, best: 3 },
  { id: "SAMPLE_HERO_06", name: "Clockwork Bard", games: 2, avg: 4.5, top: 1, best: 4 },
  { id: "SAMPLE_HERO_07", name: "Salt Merchant", games: 2, avg: 6.0, top: 0, best: 5 },
  { id: "SAMPLE_HERO_08", name: "Owl Sentry", games: 1, avg: 2.0, top: 1, best: 2 },
  { id: "SAMPLE_HERO_09", name: "Frost Tinker", games: 1, avg: 8.0, top: 0, best: 8 },
];
export const heroColumns: Column[] = [
  { label: "Hero" },
  { label: "Games", numeric: true },
  { label: "Avg place", numeric: true },
  { label: "Top 4", numeric: true },
  { label: "Best", numeric: true },
];
/** Formatted when called, in the page's language (the style page is English). */
export const heroRows = (): Cell[][] =>
  heroes.map((h) => [
  { hero: h.id, name: h.name },
  h.games,
  average(h.avg),
  share(h.top, h.games),
  { place: h.best, mode: "solo" },
]);

export const gameColumns: Column[] = [{ label: "Hero" }, { label: "Place", numeric: true }, { label: "Turns", numeric: true }, { label: "When" }];
export const gameRows: Cell[][] = [
  [{ hero: "SAMPLE_HERO_01", name: "Lantern Keeper" }, { place: 1, mode: "solo" }, 14, "2 hours ago"],
  [{ hero: "SAMPLE_HERO_03", name: "Moss Witch" }, { place: 4, mode: "solo" }, 12, "3 hours ago"],
  [{ hero: null, name: "Hero not recorded" }, { place: null, mode: "solo" }, "—", "Yesterday"],
  [{ hero: "SAMPLE_HERO_05", name: "Ember Cook", href: "/games/" }, { place: 7, mode: "solo" }, 9, "Yesterday"],
  // An id the name table does not know: the id shows in Advanced only (T-510).
  [{ hero: "SAMPLE_HERO_02", name: "Unknown hero", unknownId: "SAMPLE_HERO_02" }, { place: 2, mode: "solo" }, 11, "Last week"],
];
