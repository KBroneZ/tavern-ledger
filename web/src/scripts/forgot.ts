// First: the page's language, before any module draws text (T-508).
import "../lib/i18n/boot.ts";
import { at } from "../lib/account/strings.ts";
import { requestPasswordReset } from "../lib/api.ts";
import { checkEmail } from "../lib/auth.ts";
import { captchaIn } from "./captcha.ts";
import { supabase } from "./client.ts";
import { busy, byId, clearInvalid, failField, field, ready, say } from "./dom.ts";

// The reset link comes back to the account page, the one return address
// the project allows (design notes): the English
// address; public/lang.js then takes a viewer who chose Spanish to
// /es/account/ with the link intact (T-508).
const RETURN_PAGE = "/account/";

const form = byId<HTMLFormElement>("reset-form");
const message = byId("reset-message");
const captcha = captchaIn(form.querySelector<HTMLElement>("[data-captcha]") as HTMLElement);

form.addEventListener("submit", (event) => {
  event.preventDefault();
  const email = field(form, "email").value;
  clearInvalid(form);
  const problem = checkEmail(email);
  if (problem) {
    failField(field(form, "email"), message, problem);
    return;
  }
  void busy(form.querySelector('button[type="submit"]') as HTMLButtonElement, async () => {
    const check = captcha.take();
    if ("problem" in check) {
      say(message, check.problem, "error");
      return;
    }
    say(message, at("forgot.sending"));
    try {
      const returnTo = new URL(RETURN_PAGE, location.origin).href;
      const outcome = await requestPasswordReset(supabase, email, returnTo, check.token);
      say(message, outcome.message, outcome.kind === "error" ? "error" : "ok");
      if (outcome.kind === "check-email") form.reset();
    } finally {
      // A token works once: the next try needs a new one.
      captcha.reset();
    }
  });
});
ready(form);
