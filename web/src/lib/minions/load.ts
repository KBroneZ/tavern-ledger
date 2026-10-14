// The pool files tools/prepare-sim.ts wrote for this build (T-578), read
// once when the pages are built. With accounts closed they are empty and no
// minion page is built: the closed site allows no outside picture.
import rawEn from "./generated/pool.en.json";
import rawEs from "./generated/pool.es.json";
import { readConfig } from "../config.ts";
import type { Lang } from "../i18n.ts";
import type { PoolFile } from "./poolFile.ts";
import { readPool } from "./view.ts";

const POOLS: Record<Lang, PoolFile> = { en: readPool(rawEn), es: readPool(rawEs) };

export const poolFor = (lang: Lang): PoolFile => POOLS[lang];

/**
 * Whether this build has the minion pages: accounts open (the policy allows
 * the art host) and the card data has a pool, so stale files from an open
 * build never put pictures on a closed one.
 */
export const hasMinionPages = (): boolean => readConfig(import.meta.env) !== null && POOLS.en.minions.length > 0;
