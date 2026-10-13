// First: the page's language, before any module draws text (T-508).
import "../lib/i18n/boot.ts";
import { at } from "../lib/account/strings.ts";
import { requestDesktopLink } from "../lib/api.ts";
import { checkEmail } from "../lib/auth.ts";
import { parseDesktopRequest } from "../lib/desktopLink.ts";
import { captchaIn } from "./captcha.ts";
import { config } from "./client.ts";
import { busy, byId, clearInvalid, failField, field, ready, say, show } from "./dom.ts";

// The app's request is in the fragment (lib/desktopLink.ts). Read once, then
// taken out of the address, so the loopback address and the email do not
// stay in the history or in a screenshot of the address bar.
const request = parseDesktopRequest(location.hash);
if (location.hash) history.replaceState(null, "", location.pathname + location.search);

if (request) {
  const form = byId<HTMLFormElement>("desktop-form");
  const message = byId("desktop-message");
  const email = field(form, "email");
  email.value = request.email;
  show(form, true);
  show(byId("desktop-not-from-app"), false);
  // Drawn once the form is shown, so it gets the form's width.
  const captcha = captchaIn(form.querySelector<HTMLElement>("[data-captcha]") as HTMLElement);

  form.addEventListener("submit", (event) => {
    event.preventDefault();
    clearInvalid(form);
    const problem = checkEmail(email.value);
    if (problem) {
      failField(email, message, problem);
      return;
    }
    const check = captcha.take();
    if ("problem" in check) {
      say(message, check.problem, "error");
      return;
    }
    void busy(form.querySelector('button[type="submit"]') as HTMLButtonElement, async () => {
      say(message, at("desktop.sending"));
      const outcome = await requestDesktopLink(config, { ...request, email: email.value }, check.token);
      say(message, outcome.message, outcome.kind === "error" ? "error" : "ok");
    }).finally(() => captcha.reset());
  });
  ready(form);
}
