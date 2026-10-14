// The blurred hero tokens in the home page's scene (T-530, D-105): a few
// hero portraits, drawn at view time from HearthstoneJSON's host (D-074).
// Which heroes: a handful of the known base heroes, the same all day.

/** A base hero card id: no hero token ("...t") and no skin. */
const BASE_HERO = /^(?:BG\d+_HERO_\d+|TB_BaconShop_HERO_\d+)$/;

/** Whole days since 1970 (UTC): the seed that changes the tokens once a day. */
export function tokenDay(now: number): number {
  return Math.floor(now / 86_400_000);
}

/** A small seeded generator (mulberry32), so a seed always gives the same order. */
function random(seed: number): () => number {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4_294_967_296;
  };
}

/** Up to `count` different base heroes from `ids`, in an order set by `seed`. */
export function pickHeroes(ids: readonly string[], count: number, seed: number): string[] {
  const pool = ids.filter((id) => BASE_HERO.test(id));
  const next = random(seed);
  // Fisher-Yates on a copy, then the first `count`.
  const order = [...pool];
  for (let i = order.length - 1; i > 0; i--) {
    const j = Math.floor(next() * (i + 1));
    [order[i], order[j]] = [order[j], order[i]];
  }
  return order.slice(0, Math.max(0, count));
}
