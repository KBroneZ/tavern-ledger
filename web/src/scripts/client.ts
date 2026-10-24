/// <reference path="../env.d.ts" />
// The one Supabase client of the browser pages.
import { makeClient } from "../lib/api.ts";
import { readConfig } from "../lib/config.ts";
import { browserDevice, forget, forgetOthers } from "../lib/pwa/saved.ts";

const settings = readConfig({
  PUBLIC_SUPABASE_URL: import.meta.env.PUBLIC_SUPABASE_URL,
  PUBLIC_SUPABASE_ANON_KEY: import.meta.env.PUBLIC_SUPABASE_ANON_KEY,
});
// The pages only load these scripts when accounts are open.
if (!settings) throw new Error("Accounts are closed: no Supabase settings in this build");
export const config = settings;

export const supabase = makeClient(config);

// Signing out, here or in another tab, also removes the games saved on this device and turns saving off (lib/pwa/saved.ts).
// A session that starts for someone else than the one whose games are saved also removes them. The promise is returned so the library waits for it.
supabase.auth.onAuthStateChange((event, session) => {
  const device = browserDevice();
  if (!device) return;
  const removal =
    event === "SIGNED_OUT" ? forget(device) : session && (event === "SIGNED_IN" || event === "INITIAL_SESSION") ? forgetOthers(device, session.user.id) : null;
  return removal?.catch((error) => console.warn("Tavern Ledger: the games saved on this device were not removed.", error));
});
