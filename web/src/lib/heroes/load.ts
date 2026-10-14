// The heroes and curious stats pages' reads (T-505) from the data API's
// views (T-502). Row-level security decides what comes back, as on the
// dashboard; any failure throws, so the page shows its error state.
import type { SupabaseClient } from "@supabase/supabase-js";
import { rowsOf } from "../dataApi.ts";
import { overview, OVERVIEW_COLUMNS, type Overview } from "../data/rows.ts";
import { HERO_STAT_COLUMNS, heroStat, type HeroStat } from "./heroes.ts";
import { CURIOUS_RECORD_COLUMNS, curiousRecord, type CuriousRecord } from "./curious.ts";

/** Every hero row of one player, both modes. */
export async function loadHeroStats(client: SupabaseClient, userId: string): Promise<HeroStat[]> {
  return rowsOf(await client.from("user_heroes").select(HERO_STAT_COLUMNS).eq("user_id", userId)).map(heroStat);
}

export interface Curious {
  overviews: Overview[];
  records: CuriousRecord[];
}

/** One player's overview per mode (for the totals) and records with their games. */
export async function loadCurious(client: SupabaseClient, userId: string): Promise<Curious> {
  const [ov, records] = await Promise.all([
    client.from("user_overview").select(OVERVIEW_COLUMNS).eq("user_id", userId),
    client.from("user_records").select(CURIOUS_RECORD_COLUMNS).eq("user_id", userId),
  ]);
  return {
    overviews: rowsOf(ov).map(overview),
    records: rowsOf(records)
      .map(curiousRecord)
      .filter((r): r is CuriousRecord => r !== null),
  };
}
