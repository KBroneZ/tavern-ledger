// First: the page's language, before any module draws text (T-508).
import "../lib/i18n/boot.ts";
import { at } from "../lib/account/strings.ts";
import { signIn, signUp } from "../lib/api.ts";
import { checkCredentials, checkEmail } from "../lib/auth.ts";
import { captchaIn } from "./captcha.ts";
import { supabase } from "./client.ts";
import { busy, byId, clearInvalid, failField, field, ready, say } from "./dom.ts";
import { href, lang } from "../lib/i18n.ts";

const ACCOUNT_PAGE = href("/account/");
// Links in emails come back to the English address, the one return address
// the project allows (deploy.md, section 10.3); public/lang.js then takes a
// viewer who chose Spanish to /es/account/ with the link intact (T-508).
const RETURN_PAGE = "/account/";

function wire(
  formId: string,
  messageId: string,
  submit: (email: string, password: string, captchaToken: string, message: HTMLElement) => Promise<void>,
): void {
  const form = byId<HTMLFormElement>(formId);
  const message = byId(messageId);
  const captcha = captchaIn(form.querySelector<HTMLElement>("[data-captcha]") as HTMLElement);
  form.addEventListener("submit", (event) => {
    event.preventDefault();
    const email = field(form, "email").value;
    const password = field(form, "password").value;
    clearInvalid(form);
    const problem = checkCredentials(email, password);
    if (problem) {
      const wrong = checkEmail(email) ? field(form, "email") : field(form, "password");
      failField(wrong, message, problem);
      return;
    }
    const button = form.querySelector('button[type="submit"]') as HTMLButtonElement;
    // Inside busy(): a second press while sending takes no token and resets nothing.
    void busy(button, async () => {
      const check = captcha.take();
      if ("problem" in check) {
        say(message, check.problem, "error");
        return;
      }
      try {
        await submit(email, password, check.token, message);
      } finally {
        // A token works once: whatever the answer, the next try needs a new one.
        captcha.reset();
      }
    });
  });
  ready(form);
}

wire("signin-form", "signin-message", async (email, password, captchaToken, message) => {
  say(message, at("signin.signingIn"));
  const error = await signIn(supabase, email, password, captchaToken);
  if (error) {
    say(message, error, "error");
    return;
  }
  location.assign(ACCOUNT_PAGE);
});

// The sign-up form is only in the page while sign-ups are open.
if (document.getElementById("signup-form")) {
  wire("signup-form", "signup-message", async (email, password, captchaToken, message) => {
    say(message, at("signin.creating"));
    const returnTo = new URL(RETURN_PAGE, location.origin).href;
    const outcome = await signUp(supabase, email, password, returnTo, lang(), captchaToken);
    say(message, outcome.message, outcome.kind === "error" ? "error" : "ok");
    if (outcome.kind === "check-email") byId<HTMLFormElement>("signup-form").reset();
  });
}

// Already signed in (checked with the server, not only the stored session):
// go straight to the account.
void supabase.auth.getUser().then(({ data }) => {
  if (data.user) location.replace(ACCOUNT_PAGE);
});
