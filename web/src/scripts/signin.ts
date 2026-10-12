// First: the page's language, before any module draws text (T-508).
import "../lib/i18n/boot.ts";
import { at } from "../lib/account/strings.ts";
import { signIn, signUp } from "../lib/api.ts";
import { checkCredentials, checkEmail } from "../lib/auth.ts";
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
  submit: (email: string, password: string, message: HTMLElement) => Promise<void>,
): void {
  const form = byId<HTMLFormElement>(formId);
  const message = byId(messageId);
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
    const button = form.querySelector("button") as HTMLButtonElement;
    void busy(button, () => submit(email, password, message));
  });
  ready(form);
}

wire("signin-form", "signin-message", async (email, password, message) => {
  say(message, at("signin.signingIn"));
  const error = await signIn(supabase, email, password);
  if (error) {
    say(message, error, "error");
    return;
  }
  location.assign(ACCOUNT_PAGE);
});

// The sign-up form is only in the page while sign-ups are open.
if (document.getElementById("signup-form")) {
  wire("signup-form", "signup-message", async (email, password, message) => {
    say(message, at("signin.creating"));
    const outcome = await signUp(supabase, email, password, new URL(RETURN_PAGE, location.origin).href, lang());
    say(message, outcome.message, outcome.kind === "error" ? "error" : "ok");
    if (outcome.kind === "check-email") byId<HTMLFormElement>("signup-form").reset();
  });
}

// Already signed in (checked with the server, not only the stored session):
// go straight to the account.
void supabase.auth.getUser().then(({ data }) => {
  if (data.user) location.replace(ACCOUNT_PAGE);
});
