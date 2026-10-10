import { requestPasswordReset } from "../lib/api.ts";
import { checkEmail } from "../lib/auth.ts";
import { supabase } from "./client.ts";
import { busy, byId, clearInvalid, failField, field, ready, say } from "./dom.ts";

// The reset link comes back to the account page, the one return address
// the project allows (docs/research/deploy.md, section 10.3).
const ACCOUNT_PAGE = "/account/";

const form = byId<HTMLFormElement>("reset-form");
const message = byId("reset-message");

form.addEventListener("submit", (event) => {
  event.preventDefault();
  const email = field(form, "email").value;
  clearInvalid(form);
  const problem = checkEmail(email);
  if (problem) {
    failField(field(form, "email"), message, problem);
    return;
  }
  void busy(form.querySelector("button") as HTMLButtonElement, async () => {
    say(message, "Sending…");
    const outcome = await requestPasswordReset(supabase, email, new URL(ACCOUNT_PAGE, location.origin).href);
    say(message, outcome.message, outcome.kind === "error" ? "error" : "ok");
    if (outcome.kind === "check-email") form.reset();
  });
});
ready(form);
