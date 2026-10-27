// The account page's "train the expected-place model" switch (T-651, D-174):
// the read (on when never set), the owner-only save, the one-time notice and
// the texts in both languages.
import { afterEach, test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { loadTrainingChoice, makeClient, saveTrainingChoice } from "../../src/lib/api.ts";
import { ES as ACCOUNT_STRINGS_ES } from "../../src/lib/account/strings.es.ts";
import { ACCOUNT_STRINGS } from "../../src/lib/account/strings.ts";
import { NOTICE_KEY, dismissTrainingNotice, showTrainingNotice } from "../../src/lib/account/trainingNotice.ts";
import { json, memoryStorage, stubFetch } from "../helpers/fetchStub.ts";

const CONFIG = { url: "http://127.0.0.1:54321", anonKey: "sb_publishable_test" };
const USER = "0f8fad5b-d9cb-469f-a165-70867728950e";
const realFetch = globalThis.fetch;
const REPO = join(import.meta.dirname, "..", "..", "..");

afterEach(() => {
  globalThis.fetch = realFetch;
});

test("the switch reads on when the account never set it, and its own value otherwise", async () => {
  stubFetch({ "/rest/v1/training_objection": () => json([]) });
  assert.equal(await loadTrainingChoice(makeClient(CONFIG, memoryStorage()), USER), true);
  stubFetch({ "/rest/v1/training_objection": () => json([{ train_model: false }]) });
  assert.equal(await loadTrainingChoice(makeClient(CONFIG, memoryStorage()), USER), false);
  stubFetch({ "/rest/v1/training_objection": () => json([{ train_model: true }]) });
  assert.equal(await loadTrainingChoice(makeClient(CONFIG, memoryStorage()), USER), true);
});

test("a switch that cannot be read is an error, never on", async () => {
  for (const answer of [json({ message: "no" }, 500), json([{ train_model: "no" }]), json([{}])]) {
    stubFetch({ "/rest/v1/training_objection": () => answer });
    await assert.rejects(loadTrainingChoice(makeClient(CONFIG, memoryStorage()), USER));
  }
});

test("saving the switch calls the owner-only function and checks its answer", async () => {
  const bodies: unknown[] = [];
  stubFetch({
    "/rest/v1/rpc/set_training_choice": (_url, init) => {
      bodies.push(JSON.parse(String(init.body)));
      return json(false);
    },
  });
  assert.equal(await saveTrainingChoice(makeClient(CONFIG, memoryStorage()), false), null);
  assert.deepEqual(bodies, [{ p_train: false }]);
  // An answer that is not the choice sent is a failed save.
  stubFetch({ "/rest/v1/rpc/set_training_choice": () => json(false) });
  assert.match((await saveTrainingChoice(makeClient(CONFIG, memoryStorage()), true)) ?? "", /Could not save/);
  stubFetch({ "/rest/v1/rpc/set_training_choice": () => json({ code: "42501", message: "not signed in" }, 401) });
  assert.match((await saveTrainingChoice(makeClient(CONFIG, memoryStorage()), false)) ?? "", /Sign in again/);
  stubFetch({ "/rest/v1/rpc/set_training_choice": () => json({ message: "down" }, 500) });
  assert.match((await saveTrainingChoice(makeClient(CONFIG, memoryStorage()), true)) ?? "", /Could not save/);
});

test("the notice shows once to accounts made before the change, until dismissed", () => {
  const store = memoryStorage() as Storage;
  const before = "2026-10-01T12:00:00Z";
  assert.equal(showTrainingNotice(store, before), true, "an existing account sees it");
  assert.equal(showTrainingNotice(store, "2026-11-02T08:00:00Z"), false, "an account made after the change signed up under it");
  assert.equal(showTrainingNotice(store, undefined), true, "an unknown sign-up date errs on telling");
  assert.equal(showTrainingNotice(store, "not a date"), true);
  dismissTrainingNotice(store);
  assert.ok(store.getItem(NOTICE_KEY), "the dismissal is remembered");
  assert.equal(showTrainingNotice(store, before), false, "and the notice stays gone");
});

test("storage that is missing or throws never breaks the page", () => {
  const throwing = {
    getItem: () => {
      throw new Error("blocked");
    },
    setItem: () => {
      throw new Error("blocked");
    },
  } as unknown as Storage;
  assert.equal(showTrainingNotice(throwing, "2026-10-01T12:00:00Z"), true);
  assert.doesNotThrow(() => dismissTrainingNotice(throwing));
  assert.equal(showTrainingNotice(null, "2026-10-01T12:00:00Z"), true);
  assert.doesNotThrow(() => dismissTrainingNotice(null));
});

test("the texts say what is used, that it is on, and how to object, in both languages", () => {
  const keys = [
    "account.training",
    "account.training.label",
    "account.training.hint",
    "account.training.more",
    "account.training.how",
    "account.training.savedOn",
    "account.training.savedOff",
    "account.training.unchanged",
    "account.training.notice",
    "account.training.notice.link",
    "account.training.notice.dismiss",
  ] as const;
  for (const key of keys) {
    assert.ok(ACCOUNT_STRINGS[key], key);
    assert.ok(ACCOUNT_STRINGS_ES[key], key);
  }
  // The label is the one the privacy policy names, word for word.
  assert.equal(ACCOUNT_STRINGS["account.training.label"], "Use my uploaded games to train the expected-place model");
  assert.equal(ACCOUNT_STRINGS_ES["account.training.label"], "Usar mis partidas subidas para entrenar el modelo de puesto esperado");
  const en = ACCOUNT_STRINGS["account.training.hint"];
  assert.match(en, /On unless you turn it off/);
  assert.match(en, /turn it off at any time/i);
  const enMore = ACCOUNT_STRINGS["account.training.more"];
  assert.match(enMore, /only the model and totals/i);
  assert.match(enMore, /deleted the same day/);
  assert.match(enMore, /still see the expected place/);
  const es = ACCOUNT_STRINGS_ES["account.training.hint"];
  assert.match(es, /Activado salvo que lo desactives/);
  assert.match(es, /desactivarlo cuando quieras/);
  const esMore = ACCOUNT_STRINGS_ES["account.training.more"];
  assert.match(esMore, /solo se conservan el modelo y totales/i);
  assert.match(esMore, /seguirás viendo el puesto esperado/);
  const esNotice = ACCOUNT_STRINGS_ES["account.training.notice"];
  assert.doesNotMatch(es + esMore + esNotice, /\bapp\b|autodeclarad/, "voice guide: la aplicación, never la app");
});

test("a choice records the policy version whose \"What changed\" line introduced the training switch", () => {
  const migration = readFileSync(join(REPO, "supabase", "migrations", "20261027120000_training_objection.sql"), "utf8");
  const version = /training_policy_version\(\)[^$]*\$\$ select '(\d{4})-(\d{2})-(\d{2})' \$\$/.exec(migration);
  assert.ok(version, "the migration names the policy version");
  const [, year, month, day] = version;
  const months = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
  const date = `${Number(day)} ${months[Number(month) - 1]} ${year}`;
  const policy = readFileSync(join(REPO, "docs", "legal", "privacy.md"), "utf8");
  const changed = policy.split("\n").filter((line) => line.startsWith(`**What changed on ${date}`));
  assert.ok(
    changed.some((line) => line.includes(ACCOUNT_STRINGS["account.training.label"])),
    `a "What changed on ${date}" line names the switch`,
  );
});
