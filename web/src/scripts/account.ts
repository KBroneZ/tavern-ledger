import type { User } from "@supabase/supabase-js";
import { deleteAccount, exportAll, loadOwnProfile, saveProfile, signIn } from "../lib/api.ts";
import { isConfirmed } from "../lib/deleteAccount.ts";
import { exportFileName } from "../lib/exportData.ts";
import { checkDisplayName, publicProfilePath } from "../lib/profile.ts";
import { config, supabase } from "./client.ts";
import { busy, byId, field, say, show } from "./dom.ts";

// Read before the Supabase client cleans the address up: an email link lands
// here with ?code=… (or an error), and the client swaps the code for a session.
const query = new URLSearchParams(location.search);
const hash = new URLSearchParams(location.hash.slice(1));
const fromEmailLink = query.has("code");
const linkError = query.has("error") || hash.has("error");

const pageMessage = byId("page-message");

function showOnly(section: "signed-in" | "signed-out" | "deleted"): void {
  for (const id of ["signed-in", "signed-out", "deleted"]) show(byId(id), id === section);
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
    say(message, "Could not load your profile. Reload the page to try again.", "error");
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
          showOnly("signed-out");
          return;
        case "error":
          say(message, outcome.message, "error");
      }
    });
  });
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

async function start(): Promise<void> {
  // Validates the session with the server (and finishes an email link).
  const { data, error } = await supabase.auth.getUser();
  if (fromEmailLink || linkError) history.replaceState(null, "", location.pathname);
  say(pageMessage, "");
  if (error || !data.user) {
    if (linkError) {
      say(pageMessage, "This link is invalid or has expired. Sign in, or ask for a new link by signing up again.", "error");
    } else if (fromEmailLink) {
      say(pageMessage, "Your email is confirmed. Sign in to continue.", "ok");
    }
    showOnly("signed-out");
    return;
  }
  const user = data.user;
  byId("account-email").textContent = user.email ?? "";
  showOnly("signed-in");
  setUpExport();
  setUpDelete(user);
  setUpSignOut();
  await setUpProfile(user);
}

start().catch(() => {
  say(pageMessage, "Could not load your account. Reload the page to try again.", "error");
});
