// First: the page's language, before any module draws text (T-508).
import "../lib/i18n/boot.ts";
import { at, type AccountKey } from "../lib/account/strings.ts";
import { href } from "../lib/i18n.ts";
import type { User } from "@supabase/supabase-js";
import {
  deleteAccount,
  exportAll,
  finishAuthLink,
  loadCommunityConsent,
  loadOwnProfile,
  loadSimulatorConsent,
  saveCommunityConsent,
  saveProfile,
  saveSimulatorConsent,
  setNewPassword,
  signIn,
} from "../lib/api.ts";
import { checkNewPassword, newPasswordField } from "../lib/auth.ts";
import { hasLinkParams, parseAuthLink } from "../lib/authLink.ts";
import { isConfirmed } from "../lib/deleteAccount.ts";
import { exportFileName } from "../lib/exportData.ts";
import { checkDisplayName, publicProfilePath } from "../lib/profile.ts";
import type { Captcha } from "../lib/captcha.ts";
import { captchaIn } from "./captcha.ts";
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

/** An opt-in switch with its own form: how to read and save it, and what to say. */
interface ConsentSwitch {
  form: string;
  message: string;
  box: string;
  load: (client: typeof supabase, userId: string) => Promise<boolean>;
  save: (client: typeof supabase, on: boolean) => Promise<string | null>;
  savedOn: AccountKey;
  savedOff: AccountKey;
  unchanged: AccountKey;
}

/** The community hero stats switch (T-507), apart from the public profile. */
const COMMUNITY: ConsentSwitch = {
  form: "community-form",
  message: "community-message",
  box: "hero_stats",
  load: loadCommunityConsent,
  save: saveCommunityConsent,
  savedOn: "account.community.savedOn",
  savedOff: "account.community.savedOff",
  unchanged: "account.community.unchanged",
};

/** The "improve the simulator" switch (T-547). */
const SIMULATOR: ConsentSwitch = {
  form: "simulator-form",
  message: "simulator-message",
  box: "improve_simulator",
  load: loadSimulatorConsent,
  save: saveSimulatorConsent,
  savedOn: "account.simulator.savedOn",
  savedOff: "account.simulator.savedOff",
  unchanged: "account.simulator.unchanged",
};

async function setUpSwitch(user: User, sw: ConsentSwitch): Promise<void> {
  const form = byId<HTMLFormElement>(sw.form);
  const message = byId(sw.message);
  const box = field(form, sw.box);
  // The choice as the server last confirmed it.
  let saved: boolean;
  try {
    saved = await sw.load(supabase, user.id);
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
      const error = await sw.save(supabase, on);
      if (error) {
        // Show the choice the server last confirmed, never the one that failed.
        box.checked = saved;
        say(message, at(sw.unchanged, { error }), "error");
        return;
      }
      saved = on;
      say(message, on ? at(sw.savedOn) : at(sw.savedOff), "ok");
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
  // Drawn only when the password is asked for: Cloudflare's script is not
  // loaded for an account page that never needs it.
  let captcha: Captcha | null = null;
  form.addEventListener("submit", (event) => {
    event.preventDefault();
    clearInvalid(form);
    if (!isConfirmed(field(form, "confirm").value)) {
      failField(field(form, "confirm"), message, at("account.delete.typeExactly"));
      return;
    }
    void busy(form.querySelector("button") as HTMLButtonElement, async () => {
      if (!reauth.hidden) {
        const check = captcha?.take() ?? { problem: at("captcha.wait") };
        if ("problem" in check) {
          say(message, check.problem, "error");
          return;
        }
        say(message, at("account.delete.checking"));
        let error: string | null;
        try {
          error = await signIn(supabase, user.email ?? "", password.value, check.token);
        } finally {
          // A token works once: the next try needs a new one.
          captcha?.reset();
        }
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
          captcha ??= captchaIn(reauth.querySelector<HTMLElement>("[data-captcha]") as HTMLElement);
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
  await Promise.all([setUpProfile(user), setUpSwitch(user, COMMUNITY), setUpSwitch(user, SIMULATOR)]);
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
