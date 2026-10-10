import type { User } from "@supabase/supabase-js";
import {
  deleteAccount,
  exportAll,
  finishAuthLink,
  loadOwnProfile,
  saveProfile,
  setNewPassword,
  signIn,
} from "../lib/api.ts";
import { checkNewPassword } from "../lib/auth.ts";
import { hasLinkParams, parseAuthLink } from "../lib/authLink.ts";
import { isConfirmed } from "../lib/deleteAccount.ts";
import { exportFileName } from "../lib/exportData.ts";
import { checkDisplayName, publicProfilePath } from "../lib/profile.ts";
import { config, supabase } from "./client.ts";
import { busy, byId, field, say, show } from "./dom.ts";

// Every email link lands here (D-048). Read it, then take it out of the
// address before any request, so no code or token stays in the address bar
// or in the history.
const link = parseAuthLink(location.search, location.hash);
if (hasLinkParams(location.search, location.hash)) history.replaceState(null, "", location.pathname);

const pageMessage = byId("page-message");

type Section = "signed-in" | "signed-out" | "set-password" | "deleted";
const SECTIONS: Section[] = ["signed-in", "signed-out", "set-password", "deleted"];

function showOnly(section: Section): void {
  for (const id of SECTIONS) show(byId(id), id === section);
}

function setPublicLink(userId: string, isPublic: boolean): void {
  const link = byId<HTMLAnchorElement>("public-link");
  const path = publicProfilePath(userId);
  link.href = path;
  link.textContent = new URL(path, location.origin).href;
  show(byId("public-link-row"), isPublic);
}

async function setUpProfile(user: User): Promise<void> {
  const form = byId<HTMLFormElement>("profile-form");
  const message = byId("profile-message");
  const name = field(form, "display_name");
  const isPublic = field(form, "is_public");
  try {
    const profile = await loadOwnProfile(supabase, user.id);
    name.value = profile.displayName ?? "";
    isPublic.checked = profile.isPublic;
    setPublicLink(user.id, profile.isPublic);
  } catch {
    // A blank form saved now would overwrite the stored profile.
    for (const el of form.querySelectorAll<HTMLInputElement | HTMLButtonElement>("input, button")) {
      el.disabled = true;
    }
    say(message, "Could not load your profile. Reload the page to try again.", "error");
    return;
  }
  form.addEventListener("submit", (event) => {
    event.preventDefault();
    const checked = checkDisplayName(name.value);
    if (!checked.ok) {
      say(message, checked.message, "error");
      return;
    }
    void busy(form.querySelector("button") as HTMLButtonElement, async () => {
      say(message, "Saving…");
      const error = await saveProfile(supabase, user.id, {
        displayName: checked.value,
        isPublic: isPublic.checked,
      });
      if (error) {
        say(message, error, "error");
        return;
      }
      name.value = checked.value ?? "";
      setPublicLink(user.id, isPublic.checked);
      say(message, "Saved.", "ok");
    });
  });
}

function download(fileName: string, text: string): void {
  const url = URL.createObjectURL(new Blob([text], { type: "application/json" }));
  const a = document.createElement("a");
  a.href = url;
  a.download = fileName;
  a.click();
  setTimeout(() => URL.revokeObjectURL(url), 60_000);
}

function setUpExport(): void {
  const button = byId<HTMLButtonElement>("export-button");
  const message = byId("export-message");
  button.addEventListener("click", () => {
    void busy(button, async () => {
      say(message, "Preparing your file…");
      try {
        const data = await exportAll(supabase);
        download(exportFileName(new Date()), JSON.stringify(data, null, 2));
        say(message, "Downloaded.", "ok");
      } catch {
        say(message, "The export failed. Try again in a moment.", "error");
      }
    });
  });
}

function setUpDelete(user: User): void {
  const form = byId<HTMLFormElement>("delete-form");
  const message = byId("delete-message");
  const reauth = byId("reauth");
  const password = field(form, "password");
  form.addEventListener("submit", (event) => {
    event.preventDefault();
    if (!isConfirmed(field(form, "confirm").value)) {
      say(message, "Type the phrase exactly to confirm.", "error");
      return;
    }
    void busy(form.querySelector("button") as HTMLButtonElement, async () => {
      if (!reauth.hidden) {
        say(message, "Checking your password…");
        const error = await signIn(supabase, user.email ?? "", password.value);
        if (error) {
          say(message, error, "error");
          return;
        }
      }
      say(message, "Deleting…");
      const outcome = await deleteAccount(supabase, config);
      password.value = "";
      switch (outcome.kind) {
        case "deleted":
          await supabase.auth.signOut({ scope: "local" });
          showOnly("deleted");
          return;
        case "reauthenticate":
          show(reauth, true);
          password.focus();
          say(message, "Enter your password, then press Delete my account again.", "error");
          return;
        case "signed-out":
          await signedOut();
          return;
        case "error":
          say(message, outcome.message, "error");
      }
    });
  });
}

/** The server no longer accepts this session: forget it here too. */
async function signedOut(): Promise<void> {
  await supabase.auth.signOut({ scope: "local" });
  showOnly("signed-out");
}

function setUpSignOut(): void {
  const button = byId<HTMLButtonElement>("signout-button");
  button.addEventListener("click", () => {
    void busy(button, async () => {
      await supabase.auth.signOut();
      location.assign("/");
    });
  });
}

/** The form a password reset link leads to; then the account as usual. */
function setUpNewPassword(user: User): void {
  const form = byId<HTMLFormElement>("set-password-form");
  const message = byId("set-password-message");
  showOnly("set-password");
  field(form, "password").focus();
  form.addEventListener("submit", (event) => {
    event.preventDefault();
    const password = field(form, "password").value;
    const problem = checkNewPassword(password, field(form, "repeat").value);
    if (problem) {
      say(message, problem, "error");
      return;
    }
    void busy(form.querySelector("button") as HTMLButtonElement, async () => {
      say(message, "Saving…");
      const error = await setNewPassword(supabase, password);
      if (error) {
        say(message, error, "error");
        return;
      }
      form.reset();
      say(message, "");
      say(pageMessage, "Your new password is saved.", "ok");
      await showAccount(user);
    });
  });
}

async function showAccount(user: User): Promise<void> {
  byId("account-email").textContent = user.email ?? "";
  showOnly("signed-in");
  setUpExport();
  setUpDelete(user);
  setUpSignOut();
  await setUpProfile(user);
}

async function start(): Promise<void> {
  const outcome = await finishAuthLink(supabase, config, link);
  // Validates the session with the server, not only the stored one.
  const { data, error } = await supabase.auth.getUser();
  say(pageMessage, outcome.message ?? "", outcome.tone);
  if (error || !data.user) {
    await signedOut();
    return;
  }
  if (outcome.recovery) {
    setUpNewPassword(data.user);
    return;
  }
  await showAccount(data.user);
}

start().catch(() => {
  say(pageMessage, "Could not load your account. Reload the page to try again.", "error");
});
