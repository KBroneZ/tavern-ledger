/// <reference path="../env.d.ts" />
// The one Supabase client of the browser pages.
import { makeClient } from "../lib/api.ts";
import { readConfig } from "../lib/config.ts";
import { browserDevice, forget } from "../lib/pwa/saved.ts";

const settings = readConfig({
  PUBLIC_SUPABASE_URL: import.meta.env.PUBLIC_SUPABASE_URL,
  PUBLIC_SUPABASE_ANON_KEY: import.meta.env.PUBLIC_SUPABASE_ANON_KEY,
});
// The pages only load these scripts when accounts are open.
if (!settings) throw new Error("Accounts are closed: no Supabase settings in this build");
export const config = settings;

export const supabase = makeClient(config);

// Signing out, here or in another tab, also removes the games saved on this device and turns saving off (lib/pwa/saved.ts).
supabase.auth.onAuthStateChange((event) => {
  if (event !== "SIGNED_OUT") return;
  const device = browserDevice();
  if (device) forget(device).catch((error) => console.warn("Tavern Ledger: the games saved on this device were not removed.", error));
});
