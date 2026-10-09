/// <reference path="../env.d.ts" />
// The one Supabase client of the browser pages.
import { makeClient } from "../lib/api.ts";
import { readConfig } from "../lib/config.ts";

export const config = readConfig({
  PUBLIC_SUPABASE_URL: import.meta.env.PUBLIC_SUPABASE_URL,
  PUBLIC_SUPABASE_ANON_KEY: import.meta.env.PUBLIC_SUPABASE_ANON_KEY,
});

export const supabase = makeClient(config);
