/// <reference path="../env.d.ts" />
// Loads Cloudflare Turnstile's script, once per page and only when a form
// needs it (T-562), and draws the widget. Its origin is allowed on
// TURNSTILE_PAGES only (lib/config.ts). The script is fetched from Cloudflare
// as published: never copied or proxied (Cloudflare's own rule).
import { captchaSize, startCaptcha, type Captcha, type TurnstileApi } from "../lib/captcha.ts";
import { TURNSTILE_SCRIPT, turnstileSiteKey } from "../lib/config.ts";
import { lang } from "../lib/i18n.ts";

// The build stops without it when accounts are open, and pages with a form
// are only built then.
const SITE_KEY =
  turnstileSiteKey({
    PUBLIC_SUPABASE_URL: import.meta.env.PUBLIC_SUPABASE_URL,
    PUBLIC_SUPABASE_ANON_KEY: import.meta.env.PUBLIC_SUPABASE_ANON_KEY,
    PUBLIC_TURNSTILE_SITE_KEY: import.meta.env.PUBLIC_TURNSTILE_SITE_KEY,
  }) ?? "";

let loading: Promise<TurnstileApi> | null = null;

function loadTurnstile(): Promise<TurnstileApi> {
  loading ??= new Promise<TurnstileApi>((resolve, reject) => {
    const script = document.createElement("script");
    script.src = TURNSTILE_SCRIPT;
    script.async = true;
    script.addEventListener("load", () => {
      const api = (window as { turnstile?: TurnstileApi }).turnstile;
      if (api) resolve(api);
      else reject(new Error("Turnstile loaded without its API"));
    });
    script.addEventListener("error", () => reject(new Error("Turnstile could not load")));
    document.head.append(script);
  });
  return loading;
}

/** The widget in `container` (a `[data-captcha]` element of the form). */
export function captchaIn(container: HTMLElement): Captcha {
  return startCaptcha(loadTurnstile(), container, SITE_KEY, lang(), captchaSize(container.clientWidth));
}
