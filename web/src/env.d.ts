// Build-time settings the pages read (see .env.example).
interface ImportMetaEnv {
  readonly PUBLIC_SUPABASE_URL?: string;
  readonly PUBLIC_SUPABASE_ANON_KEY?: string;
  readonly PUBLIC_SIGNUPS_OPEN?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
