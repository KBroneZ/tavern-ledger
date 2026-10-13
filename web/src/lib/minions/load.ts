// The pool files tools/prepare-sim.ts wrote for this build (T-578), read
// once when the pages are built. With accounts closed they are empty and no
// minion page is built: the closed site allows no outside picture.
import rawEn from "./generated/pool.en.json";
import rawEs from "./generated/pool.es.json";
import type { Lang } from "../i18n.ts";
import type { PoolFile } from "./poolFile.ts";
import { readPool } from "./view.ts";

const POOLS: Record<Lang, PoolFile> = { en: readPool(rawEn), es: readPool(rawEs) };

export const poolFor = (lang: Lang): PoolFile => POOLS[lang];

/** Whether this build has the minion pages (the card data has a pool). */
export const hasMinionPages = (): boolean => POOLS.en.minions.length > 0;
