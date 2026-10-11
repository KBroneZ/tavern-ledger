// First: the page's language, before any module draws text (T-508).
import "../lib/i18n/boot.ts";
import { at } from "../lib/account/strings.ts";
import { href } from "../lib/i18n.ts";
import type { User } from "@supabase/supabase-js";
import {
  deleteAccount,
  exportAll,
  finishAuthLink,
  loadCommunityConsent,
  loadOwnProfile,
  saveCommunityConsent,
  saveProfile,
  setNewPassword,
  signIn,
} from "../lib/api.ts";
import { checkNewPassword, newPasswordField } from "../lib/auth.ts";
import { hasLinkParams, parseAuthLink } from "../lib/authLink.ts";
import { isConfirmed } from "../lib/deleteAccount.ts";
import { exportFileName } from "../lib/exportData.ts";
import { checkDisplayName, publicProfilePath } from "../lib/profile.ts";
import { config, supabase } from "./client.ts";
import { busy, byId, clearInvalid, failField, field, moveFocus, say, show } from "./dom.ts";

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

/** Shows a section that replaces what the user was using, and puts focus on its text. */
function swapTo(section: Section, focusOn: string): void {
  showOnly(section);
  moveFocus(byId(focusOn));
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
    say(message, at("account.profile.loadFailed"), "error");
    return;
  }
  form.addEventListener("submit", (event) => {
    event.preventDefault();
    clearInvalid(form);
    const checked = checkDisplayName(name.value);
    if (!checked.ok) {
      failField(name, message, checked.message);
      return;
    }
    void busy(form.querySelector("button") as HTMLButtonElement, async () => {
      say(message, at("account.saving"));
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
      say(message, at("account.saved"), "ok");
    });
  });
}

/** The community hero stats switch (T-507): its own form, apart from the public profile. */
async function setUpCommunity(user: User): Promise<void> {
  const form = byId<HTMLFormElement>("community-form");
  const message = byId("community-message");
  const box = field(form, "hero_stats");
  // The choice as the server last confirmed it.
  let saved: boolean;
  try {
    saved = await loadCommunityConsent(supabase, user.id);
    box.checked = saved;
  } catch {
    // Saving an unknown state could turn the switch off, or on, by mistake.
    for (const el of form.querySelectorAll<HTMLInputElement | HTMLButtonElement>("input, button")) {
      el.disabled = true;
    }
    say(message, at("account.setting.loadFailed"), "error");
    return;
  }
  form.addEventListener("submit", (event) => {
    event.preventDefault();
    const on = box.checked;
    void busy(form.querySelector("button") as HTMLButtonElement, async () => {
      say(message, at("account.saving"));
      const error = await saveCommunityConsent(supabase, on);
      if (error) {
        // Show the choice the server last confirmed, never the one that failed.
        box.checked = saved;
        say(message, at("account.community.unchanged", { error }), "error");
        return;
      }
      saved = on;
      say(message, on ? at("account.community.savedOn") : at("account.community.savedOff"), "ok");
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
      say(message, at("account.export.preparing"));
      try {
        const data = await exportAll(supabase);
        download(exportFileName(new Date()), JSON.stringify(data, null, 2));
        say(message, at("account.export.done"), "ok");
      } catch {
        say(message, at("account.export.failed"), "error");
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
    clearInvalid(form);
    if (!isConfirmed(field(form, "confirm").value)) {
      failField(field(form, "confirm"), message, at("account.delete.typeExactly"));
      return;
    }
    void busy(form.querySelector("button") as HTMLButtonElement, async () => {
      if (!reauth.hidden) {
        say(message, at("account.delete.checking"));
        const error = await signIn(supabase, user.email ?? "", password.value);
        if (error) {
          failField(password, message, error);
          return;
        }
      }
      say(message, at("account.delete.deleting"));
      const outcome = await deleteAccount(supabase, config);
      password.value = "";
      switch (outcome.kind) {
        case "deleted":
          await supabase.auth.signOut({ scope: "local" });
          swapTo("deleted", "deleted-message");
          return;
        case "reauthenticate":
          show(reauth, true);
          password.focus();
          say(message, at("account.delete.needPassword"), "error");
          return;
        case "signed-out":
          await signedOut(true);
          return;
        case "error":
          say(message, outcome.message, "error");
      }
    });
  });
}

/** The server no longer accepts this session: forget it here too. */
async function signedOut(fromAction = false): Promise<void> {
  await supabase.auth.signOut({ scope: "local" });
  if (fromAction) swapTo("signed-out", "signed-out-message");
  else showOnly("signed-out");
}

function setUpSignOut(): void {
  const button = byId<HTMLButtonElement>("signout-button");
  button.addEventListener("click", () => {
    void busy(button, async () => {
      await supabase.auth.signOut();
      location.assign(href("/"));
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
    clearInvalid(form);
    const problem = checkNewPassword(password, field(form, "repeat").value);
    if (problem) {
      failField(field(form, newPasswordField(password)), message, problem);
      return;
    }
    void busy(form.querySelector("button") as HTMLButtonElement, async () => {
      say(message, at("account.saving"));
      const error = await setNewPassword(supabase, password);
      if (error) {
        say(message, error, "error");
        return;
      }
      form.reset();
      say(message, "");
      say(pageMessage, at("account.password.saved"), "ok");
      await showAccount(user);
      moveFocus(byId("account-heading"));
    });
  });
}

async function showAccount(user: User): Promise<void> {
  byId("account-email").textContent = user.email ?? "";
  showOnly("signed-in");
  setUpExport();
  setUpDelete(user);
  setUpSignOut();
  await Promise.all([setUpProfile(user), setUpCommunity(user)]);
}

async function start(): Promise<void> {
  const outcome = await finishAuthLink(supabase, config, link);
  // Validates the session with the server, not only the stored one.
  const { data, error } = await supabase.auth.getUser();
  say(pageMessage, outcome.message ?? "", outcome.tone);
  if (error || !data.user) {
    if (outcome.recovery) {
      say(pageMessage, at("auth.resetExpired"), "error");
    }
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
  say(pageMessage, at("account.loadFailed"), "error");
});
