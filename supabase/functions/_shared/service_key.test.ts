// Run: deno test supabase/functions/_shared/
import { describeServiceKey, serviceKeyFromEnv } from "./service_key.ts";

function assertEquals(actual: unknown, expected: unknown, msg = ""): void {
  const a = JSON.stringify(actual);
  const e = JSON.stringify(expected);
  if (a !== e) throw new Error(`${msg}\n  actual:   ${a}\n  expected: ${e}`);
}

const env = (vars: Record<string, string>) => (name: string) => vars[name];
const NEW = "sb_secret_new-key-for-tests";
const LEGACY = "legacy.jwt.for-tests";

Deno.test("the new secret key wins over the legacy one", () => {
  const got = serviceKeyFromEnv(env({
    SUPABASE_SECRET_KEYS: JSON.stringify({ default: NEW }),
    SUPABASE_SERVICE_ROLE_KEY: LEGACY,
  }));
  assertEquals(got, { ok: true, key: NEW, source: "secret_key" });
});

Deno.test("the new secret key alone is enough", () => {
  const got = serviceKeyFromEnv(env({ SUPABASE_SECRET_KEYS: JSON.stringify({ default: NEW }) }));
  assertEquals(got, { ok: true, key: NEW, source: "secret_key" });
});

Deno.test("without the new key the legacy service key is the fallback", () => {
  const got = serviceKeyFromEnv(env({ SUPABASE_SERVICE_ROLE_KEY: LEGACY }));
  assertEquals(got, { ok: true, key: LEGACY, source: "legacy", problem: null });
});

Deno.test("a broken SUPABASE_SECRET_KEYS falls back and says why", () => {
  const cases: [string, string][] = [
    ["{not json", "SUPABASE_SECRET_KEYS is not JSON"],
    [JSON.stringify({ other: NEW }), "SUPABASE_SECRET_KEYS has no default key"],
    [JSON.stringify({ default: "eyJ.legacy.jwt" }), "SUPABASE_SECRET_KEYS has no default key"],
    [JSON.stringify([NEW]), "SUPABASE_SECRET_KEYS has no default key"],
    ["null", "SUPABASE_SECRET_KEYS has no default key"],
  ];
  for (const [raw, problem] of cases) {
    const got = serviceKeyFromEnv(env({
      SUPABASE_SECRET_KEYS: raw,
      SUPABASE_SERVICE_ROLE_KEY: LEGACY,
    }));
    assertEquals(got, { ok: true, key: LEGACY, source: "legacy", problem }, raw);
  }
});

Deno.test("neither key is a clear error", () => {
  assertEquals(serviceKeyFromEnv(env({})), {
    ok: false,
    problem: "neither SUPABASE_SECRET_KEYS nor SUPABASE_SERVICE_ROLE_KEY is set",
  });
  assertEquals(serviceKeyFromEnv(env({ SUPABASE_SECRET_KEYS: "{not json" })), {
    ok: false,
    problem: "SUPABASE_SECRET_KEYS is not JSON, and SUPABASE_SERVICE_ROLE_KEY is not set",
  });
});

Deno.test("the description names the source, never the key", () => {
  const texts = [
    describeServiceKey("sweep", { ok: true, key: NEW, source: "secret_key" }),
    describeServiceKey("sweep", { ok: true, key: LEGACY, source: "legacy", problem: null }),
    describeServiceKey("sweep", {
      ok: true,
      key: LEGACY,
      source: "legacy",
      problem: "SUPABASE_SECRET_KEYS is not JSON",
    }),
    describeServiceKey("sweep", { ok: false, problem: "neither is set" }),
  ];
  assertEquals(texts, [
    "sweep: service key from SUPABASE_SECRET_KEYS (new secret key)",
    "sweep: service key from SUPABASE_SERVICE_ROLE_KEY (legacy fallback)",
    "sweep: service key from SUPABASE_SERVICE_ROLE_KEY (legacy fallback; SUPABASE_SECRET_KEYS is not JSON)",
    "sweep: no service key: neither is set",
  ]);
  for (const t of texts) assertEquals(t.includes(NEW) || t.includes(LEGACY), false, t);
});
