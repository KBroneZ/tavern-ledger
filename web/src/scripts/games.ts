// The games list (/games/ for your own, /games/?id=… for a public profile)
// (T-503): filters from the address, one page of games from the data API,
// drawn in the reserved #list-body in one go (D-073).
import { loadGames, loadHeroes, profileAccess } from "../lib/dataApi.ts";
import { parseFilters, type GameFilters } from "../lib/data/filters.ts";
import { t } from "../lib/data/strings.ts";
import { parseProfileId, publicProfilePath } from "../lib/profile.ts";
import { listBody } from "../lib/view/gamesView.ts";
import { h, mount } from "../lib/view/node.ts";
import { stateBlock } from "../lib/view/pieces.ts";
import { supabase } from "./client.ts";
import { byId } from "./dom.ts";
import { watchPictureFallback } from "../lib/data/pictures.ts";

// A picture that fails to load shows its local placeholder (D-074).
watchPictureFallback(document);

const root = byId("list");
const title = byId("list-title");
const status = byId("list-status");
const body = byId("list-body");
const form = byId<HTMLFormElement>("list-filters");
const back = byId<HTMLAnchorElement>("list-back");
const heroSelect = byId<HTMLSelectElement>("list-hero");

function settle(text: string): void {
  status.textContent = text;
  root.setAttribute("aria-busy", "false");
}

/** The form ships with "Show games" off, so it cannot be sent before it shows the filters in force. */
function formReady(): void {
  byId<HTMLButtonElement>("list-apply").disabled = false;
}

/** The form shows the filters in force (the address may have been typed by hand). */
function fillForm(f: GameFilters, profileId: string | null): void {
  byId<HTMLSelectElement>("list-mode").value = f.mode;
  byId<HTMLSelectElement>("list-place").value = f.place;
  byId<HTMLInputElement>("list-from").value = f.from ?? "";
  byId<HTMLInputElement>("list-to").value = f.to ?? "";
  if (profileId) {
    const hidden = document.createElement("input");
    hidden.type = "hidden";
    hidden.name = "id";
    hidden.value = profileId;
    form.prepend(hidden);
  }
}

function fillHeroes(heroes: string[], selected: string | null): void {
  // A hero in the address with no game still shows, so the filter in force is visible.
  const list = selected && !heroes.includes(selected) ? [selected, ...heroes] : heroes;
  const options = list.map((hero) => {
    const option = document.createElement("option");
    option.value = hero;
    option.textContent = hero;
    option.selected = hero === selected;
    return option;
  });
  heroSelect.append(...options);
}

function gate(title: string, text: string): void {
  mount(body, stateBlock({ kind: "unknown", title, body: text }));
  form.hidden = true;
  back.hidden = true;
  document.title = `${title} · Tavern Ledger`;
  settle(title);
}

async function whose(profileId: string | null): Promise<string | null> {
  const { data } = await supabase.auth.getSession();
  const viewer = data.session?.user.id ?? null;
  if (profileId === null) {
    if (!viewer) {
      mount(body, stateBlock({ kind: "empty", title: t("dash.signedOut"), body: "" }, h("p", {}, h("a", { href: "/signin/" }, t("dash.signedOut.link")))));
      form.hidden = true;
      settle(t("dash.signedOut"));
      return null;
    }
    return viewer;
  }
  const access = await profileAccess(supabase, profileId, viewer);
  if (access.kind === "private") {
    gate(t("dash.private.title"), t("dash.private.body"));
    return null;
  }
  const name = access.name ?? t("dash.unnamed");
  title.textContent = `${t("list.title")}: ${name}`;
  document.title = `${t("list.title")}: ${name} · Tavern Ledger`;
  return profileId;
}

async function start(): Promise<void> {
  root.setAttribute("aria-busy", "true");
  status.textContent = t("dash.loading");
  const hasId = new URLSearchParams(location.search).has("id");
  const profileId = parseProfileId(location.search);
  if (hasId && profileId === null) {
    gate(t("dash.private.title"), t("dash.private.body"));
    return;
  }
  const { filters, invalid } = parseFilters(location.search);
  fillForm(filters, profileId);
  if (profileId) {
    back.href = publicProfilePath(profileId);
    back.textContent = t("list.back.public");
    byId<HTMLAnchorElement>("list-reset").href = `/games/?id=${encodeURIComponent(profileId)}`;
  }
  const userId = await whose(profileId);
  if (!userId) return;
  const [heroes, page] = await Promise.all([loadHeroes(supabase, userId), loadGames(supabase, userId, filters)]);
  fillHeroes(heroes, filters.hero);
  formReady();
  mount(body, ...listBody({ ...page, filters, profileId, userId, invalid }));
  settle(page.total === 0 ? t("list.none") : t("dash.loaded"));
}

function failed(): void {
  const retry = h("button", { type: "button", class: "secondary", id: "list-retry" }, t("dash.retry"));
  mount(body, stateBlock({ kind: "error", title: t("dash.error"), body: t("dash.error.body") }, retry));
  form.hidden = true;
  settle(t("dash.error"));
  byId("list-retry").addEventListener("click", () => location.reload());
}

start().catch(failed);
