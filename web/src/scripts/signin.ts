import { signIn, signUp } from "../lib/api.ts";
import { checkCredentials } from "../lib/auth.ts";
import { supabase } from "./client.ts";
import { busy, byId, field, say } from "./dom.ts";

const ACCOUNT_PAGE = "/account/";

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
    const problem = checkCredentials(email, password);
    if (problem) {
      say(message, problem, "error");
      return;
    }
    const button = form.querySelector("button") as HTMLButtonElement;
    void busy(button, () => submit(email, password, message));
  });
}

wire("signin-form", "signin-message", async (email, password, message) => {
  say(message, "Signing in…");
  const error = await signIn(supabase, email, password);
  if (error) {
    say(message, error, "error");
    return;
  }
  location.assign(ACCOUNT_PAGE);
});

wire("signup-form", "signup-message", async (email, password, message) => {
  say(message, "Creating the account…");
  const outcome = await signUp(supabase, email, password, new URL(ACCOUNT_PAGE, location.origin).href);
  say(message, outcome.message, outcome.kind === "error" ? "error" : "ok");
  if (outcome.kind === "check-email") byId<HTMLFormElement>("signup-form").reset();
});

// Already signed in (checked with the server, not only the stored session):
// go straight to the account.
void supabase.auth.getUser().then(({ data }) => {
  if (data.user) location.replace(ACCOUNT_PAGE);
});
