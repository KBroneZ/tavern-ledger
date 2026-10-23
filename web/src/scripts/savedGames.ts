// "Keep my last games on this device" on the games list (T-637): the choice
// (off until the player turns it on), how many games are saved, and the button
// that deletes them. It shows only on your own list, signed in, in a browser
// that can keep files. What is saved and when is in lib/pwa/saved.ts.

// First: the page's language, before any module draws text (T-508).
import "../lib/i18n/boot.ts";
import { t } from "../lib/data/strings.ts";
import { browserDevice, eraseSaved, isOn, listSaved, turnOff, turnOn, type Device } from "../lib/pwa/saved.ts";
import { supabase } from "./client.ts";
import { byId, say, show } from "./dom.ts";

const panel = byId("saved");
const toggle = byId<HTMLInputElement>("saved-on");
const count = byId("saved-count");
const erase = byId<HTMLButtonElement>("saved-erase");
/** The signed-in player: the choice is theirs alone. */
let player = "";

async function showCount(device: Device): Promise<void> {
  const saved = (await listSaved(device)).length;
  say(count, saved === 0 ? t("saved.count.none") : saved === 1 ? t("saved.count.one") : t("saved.count.other", { count: saved }));
  erase.disabled = saved === 0;
}

async function change(device: Device, action: () => Promise<void>): Promise<void> {
  try {
    await action();
    await showCount(device);
  } catch {
    say(count, t("saved.error"), "error");
    // Saving cannot work in this browser: the choice does not stay on with nothing being saved.
    if (toggle.checked) await turnOff(device).catch(() => {});
  }
  toggle.checked = isOn(device, player);
}

async function start(): Promise<void> {
  const device = browserDevice();
  // A public profile's list is not yours: nothing to save.
  if (!device || new URLSearchParams(location.search).has("id")) return;
  const session = await supabase.auth.getSession().then(
    (answer) => answer.data.session,
    () => null,
  );
  if (!session) return;
  player = session.user.id;
  toggle.checked = isOn(device, player);
  toggle.addEventListener("change", () => void change(device, () => (toggle.checked ? turnOn(device, player) : turnOff(device))));
  erase.addEventListener("click", () => void change(device, () => eraseSaved(device)));
  await showCount(device);
  show(panel, true);
}

start().catch(() => {});
