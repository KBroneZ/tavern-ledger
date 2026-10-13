// The anti-robot check on the forms that make Supabase Auth send an email or
// check a password (T-562): Cloudflare Turnstile in managed mode. The widget
// gives a token; the form sends it with its one request (supabase-js
// `options.captchaToken`), and Supabase Auth checks it with Cloudflare. A
// token works once, so the widget is reset after every try, whatever the
// answer. The page script loads Cloudflare's script (scripts/captcha.ts);
// this module only drives the widget, so tests can use a fake one.

import { at } from "./account/strings.ts";
import type { Lang } from "./i18n.ts";

/** The parts of Cloudflare's `window.turnstile` this module uses. */
export interface TurnstileApi {
  render(container: HTMLElement, options: TurnstileOptions): string | null | undefined;
  reset(widgetId: string): void;
}

/** Widget settings, as Cloudflare names them (client-side rendering docs). */
export interface TurnstileOptions {
  sitekey: string;
  theme: "dark";
  language: Lang;
  size: CaptchaSize;
  /** No hidden form field: the token is read here and sent by the script. */
  "response-field": false;
  "refresh-expired": "auto";
  callback: (token: string) => void;
  "expired-callback": () => void;
  "timeout-callback": () => void;
  "error-callback": (code: string) => boolean;
}

/** "flexible" fills the form (300 px at least); "compact" fits a narrow phone. */
export type CaptchaSize = "flexible" | "compact";

/** The widget's size for a form `width` CSS pixels wide. */
export const captchaSize = (width: number): CaptchaSize => (width >= 300 ? "flexible" : "compact");

/** What a form gets when it asks for a token. */
export type CaptchaToken = { token: string } | { problem: string };

export interface Captcha {
  /** The token for this try, or the message to show instead. */
  take(): CaptchaToken;
  /** After every try: the token cannot be used twice. */
  reset(): void;
}

// A token is opaque; this only keeps something absurd out of the request.
const TOKEN = /^[\x21-\x7e]{1,2048}$/;

/**
 * Draws the widget in `container` once `api` is there. Until then, or if
 * Cloudflare's script could not load, `take()` says why there is no token.
 */
export function startCaptcha(
  api: Promise<TurnstileApi>,
  container: HTMLElement,
  siteKey: string,
  lang: Lang,
  size: CaptchaSize = "flexible",
): Captcha {
  let token: string | null = null;
  let failed = false;
  let widget: { api: TurnstileApi; id: string } | null = null;

  api.then(
    (turnstile) => {
      const id = turnstile.render(container, {
        sitekey: siteKey,
        theme: "dark",
        language: lang,
        size,
        "response-field": false,
        "refresh-expired": "auto",
        callback: (value) => {
          token = TOKEN.test(value) ? value : null;
          failed = false;
        },
        "expired-callback": () => {
          token = null;
        },
        "timeout-callback": () => {
          token = null;
        },
        "error-callback": () => {
          token = null;
          failed = true;
          // Handled: the widget shows its own error and the form says what to do.
          return true;
        },
      });
      if (typeof id === "string") widget = { api: turnstile, id };
      else failed = true;
    },
    () => {
      failed = true;
    },
  );

  return {
    take() {
      if (token !== null) return { token };
      return { problem: failed ? at("captcha.failed") : at("captcha.wait") };
    },
    reset() {
      token = null;
      if (widget) widget.api.reset(widget.id);
    },
  };
}
