// The community hero stats' reads (T-507): public.community_heroes() (every
// mode and period in one read, so from one snapshot) and its state. The
// server picks what is shown (only heroes above the thresholds); this only
// checks the rows. Any failure throws, so the page shows its error state.
import type { SupabaseClient } from "@supabase/supabase-js";
import { DataError } from "../data/rows.ts";
import { communityHeroes, communityState, rowsWhileUnavailable, type CommunityHeroes, type CommunityState } from "./heroes.ts";

export interface CommunityStats {
  heroes: CommunityHeroes;
  state: CommunityState;
}

export async function loadCommunityStats(client: SupabaseClient): Promise<CommunityStats> {
  const [rows, state] = await Promise.all([client.rpc("community_heroes"), client.rpc("community_heroes_state")]);
  if (rows.error || state.error) throw new DataError("the request failed");
  const heroes = communityHeroes(rows.data);
  const checked = communityState(state.data);
  if (rowsWhileUnavailable(checked, heroes)) throw new DataError("rows from a snapshot too old to show");
  return { heroes, state: checked };
}
