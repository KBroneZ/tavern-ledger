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

Deno.test("the default key in SUPABASE_SECRET_KEYS is used", () => {
  const got = serviceKeyFromEnv(env({ SUPABASE_SECRET_KEYS: JSON.stringify({ default: NEW }) }));
  assertEquals(got, { ok: true, key: NEW });
});

Deno.test("the legacy service key is never used, even when it is set", () => {
  assertEquals(serviceKeyFromEnv(env({ SUPABASE_SERVICE_ROLE_KEY: LEGACY })), {
    ok: false,
    problem: "SUPABASE_SECRET_KEYS is not set",
  });
  const got = serviceKeyFromEnv(env({
    SUPABASE_SECRET_KEYS: JSON.stringify({ default: NEW }),
    SUPABASE_SERVICE_ROLE_KEY: LEGACY,
  }));
  assertEquals(got, { ok: true, key: NEW });
});

Deno.test("a missing or broken SUPABASE_SECRET_KEYS is a clear error", () => {
  const cases: [string | undefined, string][] = [
    [undefined, "SUPABASE_SECRET_KEYS is not set"],
    ["", "SUPABASE_SECRET_KEYS is not set"],
    ["{not json", "SUPABASE_SECRET_KEYS is not JSON"],
    [JSON.stringify({ other: NEW }), "SUPABASE_SECRET_KEYS has no default key"],
    [JSON.stringify({ default: "eyJ.legacy.jwt" }), "SUPABASE_SECRET_KEYS has no default key"],
    [JSON.stringify({ default: 42 }), "SUPABASE_SECRET_KEYS has no default key"],
    [JSON.stringify([NEW]), "SUPABASE_SECRET_KEYS has no default key"],
    ["null", "SUPABASE_SECRET_KEYS has no default key"],
  ];
  for (const [raw, problem] of cases) {
    const vars: Record<string, string> = { SUPABASE_SERVICE_ROLE_KEY: LEGACY };
    if (raw !== undefined) vars.SUPABASE_SECRET_KEYS = raw;
    assertEquals(serviceKeyFromEnv(env(vars)), { ok: false, problem }, String(raw));
  }
});

Deno.test("the description names the source, never the key", () => {
  const texts = [
    describeServiceKey("sweep", { ok: true, key: NEW }),
    describeServiceKey("sweep", { ok: false, problem: "SUPABASE_SECRET_KEYS is not JSON" }),
  ];
  assertEquals(texts, [
    "sweep: service key from SUPABASE_SECRET_KEYS (new secret key)",
    "sweep: no service key: SUPABASE_SECRET_KEYS is not JSON; every request gets a 500",
  ]);
  for (const t of texts) assertEquals(t.includes(NEW) || t.includes(LEGACY), false, t);
});
