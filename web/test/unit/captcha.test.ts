import { test } from "node:test";
import assert from "node:assert/strict";
import { captchaSize, startCaptcha, type TurnstileApi, type TurnstileOptions } from "../../src/lib/captcha.ts";
import { ACCOUNT_STRINGS } from "../../src/lib/account/strings.ts";
import { setLang } from "../../src/lib/i18n.ts";

const SITE_KEY = "1x00000000000000000000AA";

/** A stand-in for Cloudflare's widget that records what the page asked for. */
function fakeTurnstile(id: string | null = "w1") {
  const calls = { renders: [] as TurnstileOptions[], resets: [] as string[] };
  const api: TurnstileApi = {
    render(_container, options) {
      calls.renders.push(options);
      return id;
    },
    reset(widgetId) {
      calls.resets.push(widgetId);
    },
  };
  const options = () => calls.renders[0];
  return { api, calls, options };
}

const container = {} as HTMLElement;
const settle = () => new Promise((resolve) => setImmediate(resolve));

test("the widget is drawn dark, in the page's language, managed by Cloudflare, with no hidden field", async () => {
  setLang("es");
  const { api, options } = fakeTurnstile();
  startCaptcha(Promise.resolve(api), container, SITE_KEY, "es");
  await settle();
  const o = options();
  assert.equal(o.sitekey, SITE_KEY);
  assert.equal(o.theme, "dark");
  assert.equal(o.language, "es");
  assert.equal(o.size, "flexible");
  assert.equal(o["response-field"], false);
  assert.equal(o["refresh-expired"], "auto");
  setLang("en");
});

test("no token yet: the form is told to wait, never sent", async () => {
  setLang("en");
  const { api } = fakeTurnstile();
  const captcha = startCaptcha(Promise.resolve(api), container, SITE_KEY, "en");
  assert.deepEqual(captcha.take(), { problem: ACCOUNT_STRINGS["captcha.wait"] }, "while Cloudflare's script loads");
  await settle();
  assert.deepEqual(captcha.take(), { problem: ACCOUNT_STRINGS["captcha.wait"] }, "drawn, not solved");
});

test("a solved check gives its token until the form resets it after the try", async () => {
  const { api, calls, options } = fakeTurnstile("widget-7");
  const captcha = startCaptcha(Promise.resolve(api), container, SITE_KEY, "en");
  await settle();
  options().callback("0.token-from-cloudflare");
  assert.deepEqual(captcha.take(), { token: "0.token-from-cloudflare" });
  captcha.reset();
  assert.deepEqual(calls.resets, ["widget-7"], "the widget runs again for the next try");
  assert.deepEqual(captcha.take(), { problem: ACCOUNT_STRINGS["captcha.wait"] }, "a token is never sent twice");
});

test("an expired or timed-out token is dropped", async () => {
  const { api, options } = fakeTurnstile();
  const captcha = startCaptcha(Promise.resolve(api), container, SITE_KEY, "en");
  await settle();
  options().callback("tok");
  options()["expired-callback"]();
  assert.ok("problem" in captcha.take());
  options().callback("tok2");
  options()["timeout-callback"]();
  assert.ok("problem" in captcha.take());
});

test("a widget error or a script that never loads says how to get past it", async () => {
  setLang("en");
  const { api, options } = fakeTurnstile();
  const captcha = startCaptcha(Promise.resolve(api), container, SITE_KEY, "en");
  await settle();
  assert.equal(options()["error-callback"]("110200"), true, "handled: Cloudflare does not throw");
  assert.deepEqual(captcha.take(), { problem: ACCOUNT_STRINGS["captcha.failed"] });
  // Solved after all (Cloudflare retries): the token is used.
  options().callback("tok");
  assert.deepEqual(captcha.take(), { token: "tok" });

  const blocked = startCaptcha(Promise.reject(new Error("blocked")), container, SITE_KEY, "en");
  await settle();
  assert.deepEqual(blocked.take(), { problem: ACCOUNT_STRINGS["captcha.failed"] });
  blocked.reset();

  const noWidget = startCaptcha(Promise.resolve(fakeTurnstile(null).api), container, SITE_KEY, "en");
  await settle();
  assert.deepEqual(noWidget.take(), { problem: ACCOUNT_STRINGS["captcha.failed"] });
});

test("a widget that throws while drawn, or a token Cloudflare never gives, says the check failed", async () => {
  setLang("en");
  const throwing: TurnstileApi = {
    render() {
      throw new Error("TurnstileError: invalid sitekey");
    },
    reset() {},
  };
  const captcha = startCaptcha(Promise.resolve(throwing), container, SITE_KEY, "en");
  await settle();
  assert.deepEqual(captcha.take(), { problem: ACCOUNT_STRINGS["captcha.failed"] });

  const { api, options } = fakeTurnstile();
  const odd = startCaptcha(Promise.resolve(api), container, SITE_KEY, "en");
  await settle();
  options().callback("a b");
  assert.deepEqual(odd.take(), { problem: ACCOUNT_STRINGS["captcha.failed"] });
});

test("reset after an error runs the widget again and waits for its new answer", async () => {
  setLang("en");
  const { api, calls, options } = fakeTurnstile();
  const captcha = startCaptcha(Promise.resolve(api), container, SITE_KEY, "en");
  await settle();
  options()["error-callback"]("300030");
  assert.deepEqual(captcha.take(), { problem: ACCOUNT_STRINGS["captcha.failed"] });
  captcha.reset();
  assert.deepEqual(calls.resets, ["w1"]);
  assert.deepEqual(captcha.take(), { problem: ACCOUNT_STRINGS["captcha.wait"] });
  // A reset before the widget is drawn does nothing and breaks nothing.
  const early = startCaptcha(new Promise<TurnstileApi>(() => {}), container, SITE_KEY, "en");
  early.reset();
  assert.deepEqual(early.take(), { problem: ACCOUNT_STRINGS["captcha.wait"] });
});

test("a token with spaces, controls or absurd length is not sent", async () => {
  const { api, options } = fakeTurnstile();
  const captcha = startCaptcha(Promise.resolve(api), container, SITE_KEY, "en");
  await settle();
  for (const bad of ["", "a b", "a\nb", "x".repeat(2049)]) {
    options().callback(bad);
    assert.ok("problem" in captcha.take(), JSON.stringify(bad.slice(0, 10)));
  }
});

test("a form narrower than the widget gets the compact one, so a 320 px phone never scrolls sideways", async () => {
  assert.equal(captchaSize(300), "flexible");
  assert.equal(captchaSize(640), "flexible");
  assert.equal(captchaSize(256), "compact");
  assert.equal(captchaSize(0), "compact", "not laid out yet: the small one is safe");
  const { api, options } = fakeTurnstile();
  startCaptcha(Promise.resolve(api), container, SITE_KEY, "en", "compact");
  await settle();
  assert.equal(options().size, "compact");
});
