/// <reference path="../env.d.ts" />
// The one Supabase client of the browser pages.
import { makeClient } from "../lib/api.ts";
import { readConfig } from "../lib/config.ts";

const settings = readConfig({
  PUBLIC_SUPABASE_URL: import.meta.env.PUBLIC_SUPABASE_URL,
  PUBLIC_SUPABASE_ANON_KEY: import.meta.env.PUBLIC_SUPABASE_ANON_KEY,
});
// The pages only load these scripts when accounts are open.
if (!settings) throw new Error("Accounts are closed: no Supabase settings in this build");
export const config = settings;

export const supabase = makeClient(config);
