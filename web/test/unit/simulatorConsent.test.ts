// The account page's "improve the simulator" switch (T-547): the read, the
// owner-only save and the texts in both languages.
import { afterEach, test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { loadSimulatorConsent, makeClient, saveSimulatorConsent } from "../../src/lib/api.ts";
import { ES as ACCOUNT_STRINGS_ES } from "../../src/lib/account/strings.es.ts";
import { ACCOUNT_STRINGS } from "../../src/lib/account/strings.ts";
import { json, memoryStorage, stubFetch } from "../helpers/fetchStub.ts";

const CONFIG = { url: "http://127.0.0.1:54321", anonKey: "sb_publishable_test" };
const USER = "0f8fad5b-d9cb-469f-a165-70867728950e";
const realFetch = globalThis.fetch;

afterEach(() => {
  globalThis.fetch = realFetch;
});

test("the switch reads off when the account never set it, and its own value otherwise", async () => {
  stubFetch({ "/rest/v1/simulator_consent": () => json([]) });
  assert.equal(await loadSimulatorConsent(makeClient(CONFIG, memoryStorage()), USER), false);
  stubFetch({ "/rest/v1/simulator_consent": () => json([{ improve_simulator: true }]) });
  assert.equal(await loadSimulatorConsent(makeClient(CONFIG, memoryStorage()), USER), true);
  stubFetch({ "/rest/v1/simulator_consent": () => json([{ improve_simulator: false }]) });
  assert.equal(await loadSimulatorConsent(makeClient(CONFIG, memoryStorage()), USER), false);
});

test("a switch that cannot be read is an error, never off", async () => {
  for (const answer of [json({ message: "no" }, 500), json([{ improve_simulator: "yes" }]), json([{}])]) {
    stubFetch({ "/rest/v1/simulator_consent": () => answer });
    await assert.rejects(loadSimulatorConsent(makeClient(CONFIG, memoryStorage()), USER));
  }
});

test("saving the switch calls the owner-only function and checks its answer", async () => {
  const bodies: unknown[] = [];
  stubFetch({
    "/rest/v1/rpc/set_simulator_consent": (_url, init) => {
      bodies.push(JSON.parse(String(init.body)));
      return json(true);
    },
  });
  assert.equal(await saveSimulatorConsent(makeClient(CONFIG, memoryStorage()), true), null);
  assert.deepEqual(bodies, [{ p_improve: true }]);
  // An answer that is not the choice sent is a failed save.
  stubFetch({ "/rest/v1/rpc/set_simulator_consent": () => json(true) });
  assert.match((await saveSimulatorConsent(makeClient(CONFIG, memoryStorage()), false)) ?? "", /Could not save/);
  stubFetch({ "/rest/v1/rpc/set_simulator_consent": () => json({ code: "42501", message: "not signed in" }, 401) });
  assert.match((await saveSimulatorConsent(makeClient(CONFIG, memoryStorage()), true)) ?? "", /Sign in again/);
  stubFetch({ "/rest/v1/rpc/set_simulator_consent": () => json({ message: "down" }, 500) });
  assert.match((await saveSimulatorConsent(makeClient(CONFIG, memoryStorage()), false)) ?? "", /Could not save/);
});

test("the texts say what is used, how, and what withdrawing does, in both languages", () => {
  const keys = [
    "account.simulator",
    "account.simulator.label",
    "account.simulator.hint",
    "account.simulator.more",
    "account.simulator.how",
    "account.simulator.savedOn",
    "account.simulator.savedOff",
    "account.simulator.unchanged",
  ] as const;
  for (const key of keys) {
    assert.ok(ACCOUNT_STRINGS[key], key);
    assert.ok(ACCOUNT_STRINGS_ES[key], key);
  }
  // The line in view says what is used, that it is off by default and that it can be withdrawn;
  // the fold (T-557, D-130) carries the rest of the text.
  const en = ACCOUNT_STRINGS["account.simulator.hint"];
  assert.match(en, /Off by default/);
  assert.match(en, /board/);
  assert.match(en, /combat replays/);
  assert.match(en, /turn it off at any time/i);
  const enMore = ACCOUNT_STRINGS["account.simulator.more"];
  assert.match(enMore, /never published per player/);
  assert.match(enMore, /cannot be traced back to you/);
  const es = ACCOUNT_STRINGS_ES["account.simulator.hint"];
  assert.match(es, /Desactivado por defecto/);
  assert.match(es, /repeticiones de los combates/);
  assert.match(es, /desactivarlo cuando quieras/);
  const esMore = ACCOUNT_STRINGS_ES["account.simulator.more"];
  assert.match(esMore, /nunca se publican por jugador/);
  assert.doesNotMatch(es + esMore, /app\b|autodeclarad/, "voice guide: la aplicación, never la app");
});

test("a consent records the policy version whose \"What changed\" line introduced the simulator section", () => {
  const repo = join(import.meta.dirname, "..", "..", "..");
  const migration = readFileSync(join(repo, "supabase", "migrations", "20261012200000_simulator_consent.sql"), "utf8");
  const version = /simulator_policy_version\(\)[^$]*\$\$ select '(\d{4})-(\d{2})-(\d{2})' \$\$/.exec(migration);
  assert.ok(version, "the migration names the policy version");
  const [, year, month, day] = version;
  const months = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
  const date = `${Number(day)} ${months[Number(month) - 1]} ${year}`;
  const policy = readFileSync(join(repo, "docs", "legal", "privacy.md"), "utf8");
  const changed = policy.split("\n").filter((line) => line.startsWith(`**What changed on ${date}`));
  assert.ok(changed.some((line) => /improve the simulator/.test(line)), `a "What changed on ${date}" line names the switch`);
});
