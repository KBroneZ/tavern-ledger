// The dashboard (/me/) and the public profile (/profile/?id=…) (T-503): who
// may see what, then the panels drawn in the browser from the data API.
// Every update lands in one go inside the reserved #dash-body, so nothing on
// screen moves while it loads (D-073).

// First: the page's language, before any module draws text (T-508).
import "../lib/i18n/boot.ts";
import { loadDashboard, profileAccess, type ProfileAccess } from "../lib/dataApi.ts";
import type { Mode } from "../lib/data/format.ts";
import { filtersQuery, DEFAULT_FILTERS } from "../lib/data/filters.ts";
import { statsPagePath } from "../lib/heroes/heroes.ts";
import { t } from "../lib/data/strings.ts";
import { parseProfileId, publicProfilePath } from "../lib/profile.ts";
import { aboutFold, dashboardPanels, isEmpty } from "../lib/view/dashboardView.ts";
import { h, mount, type VNode } from "../lib/view/node.ts";
import { stateBlock } from "../lib/view/pieces.ts";
import { supabase } from "./client.ts";
import { byId, moveFocus } from "./dom.ts";
import { watchPictureFallback } from "../lib/data/pictures.ts";
import { href } from "../lib/i18n.ts";

// A picture that fails to load shows its local placeholder (D-074).
watchPictureFallback(document);

const root = byId("dash");
const own = root.dataset.kind === "own";
const title = byId("dash-title");
const status = byId("dash-status");
const gate = byId("dash-gate");
const panels = byId("dash-modes");
const note = byId("dash-note");
const about = byId("dash-about");
const switches = byId("dash-switches");

function settle(text: string): void {
  status.textContent = text;
  root.setAttribute("aria-busy", "false");
}

/** A whole-page message (signed out, private, empty, error) in place of the panels. */
function showGate(node: VNode, announce: string): void {
  mount(gate, node);
  gate.hidden = false;
  panels.hidden = true;
  switches.classList.add("is-off");
  settle(announce);
}

function ownerNote(access: ProfileAccess, userId: string): VNode | null {
  if (access.kind === "own-private") {
    return h("p", { class: "data-note" }, `${t("dash.private.own")} `, h("a", { href: href("/account/") }, t("dash.private.own.link")));
  }
  if (access.kind === "public" && access.own) {
    return h("p", { class: "data-note" }, `${t("dash.public.own")} `, h("a", { href: publicProfilePath(userId) }, t("dash.public.own.link")));
  }
  return null;
}

function privateGate(): void {
  showGate(stateBlock({ kind: "unknown", title: t("dash.private.title"), body: t("dash.private.body") }), t("dash.private.title"));
  document.title = `${t("dash.private.title")} · Tavern Ledger`;
}

async function viewerId(): Promise<string | null> {
  const { data } = await supabase.auth.getSession();
  return data.session?.user.id ?? null;
}

/** Whose page this is, or null when the page already said why there is nothing to show. */
async function subject(): Promise<{ userId: string; access: ProfileAccess } | null> {
  const viewer = await viewerId();
  if (own) {
    if (!viewer) {
      const link = h("a", { href: href("/signin/") }, t("dash.signedOut.link"));
      showGate(stateBlock({ kind: "empty", title: t("dash.signedOut"), body: "" }, h("p", {}, link)), t("dash.signedOut"));
      return null;
    }
    return { userId: viewer, access: await profileAccess(supabase, viewer, viewer) };
  }
  const id = parseProfileId(location.search);
  // A bad id, a private profile and a missing one all look the same.
  const access: ProfileAccess = id === null ? { kind: "private" } : await profileAccess(supabase, id, viewer);
  if (id === null || access.kind === "private") {
    privateGate();
    return null;
  }
  return { userId: id, access };
}

async function start(): Promise<void> {
  root.setAttribute("aria-busy", "true");
  status.textContent = t("dash.loading");
  const who = await subject();
  if (!who) return;
  const { userId, access } = who;
  const dash = await loadDashboard(supabase, userId);
  const gamesHref = (m: Mode) => href(`/games/${filtersQuery({ ...DEFAULT_FILTERS, mode: m }, own ? null : userId)}`);
  const profileId = own ? null : userId;
  const name = access.kind === "private" ? null : (access.name ?? t("dash.unnamed"));
  if (!own && name) {
    title.textContent = name;
    document.title = `${name} · Tavern Ledger`;
  }
  const extra = ownerNote(access, userId);
  if (isEmpty(dash)) {
    const [heading, body] = own ? [t("dash.empty.own"), t("dash.empty.own.body")] : [t("dash.empty.public"), t("dash.empty.public.body")];
    showGate(stateBlock({ kind: "empty", title: heading, body }, ...(extra ? [extra] : [])), heading);
    return;
  }
  mount(note, ...(extra ? [extra] : []));
  const heroesHref = (m: Mode) => statsPagePath("heroes", m, profileId);
  const curiousHref = (m: Mode) => statsPagePath("curious", m, profileId);
  mount(panels, ...dashboardPanels(dash, { gamesHref, owner: own, userId, heroesHref, curiousHref }));
  mount(about, aboutFold());
  gate.hidden = true;
  panels.hidden = false;
  switches.classList.remove("is-off");
  settle(t("dash.loaded"));
}

function failed(): void {
  const retry = h("button", { type: "button", class: "secondary", id: "dash-retry" }, t("dash.retry"));
  showGate(stateBlock({ kind: "error", title: t("dash.error"), body: t("dash.error.body") }, retry), t("dash.error"));
  byId("dash-retry").addEventListener("click", () => {
    mount(gate, stateBlock({ kind: "loading", lines: 6 }));
    moveFocus(title);
    run();
  });
}

function run(): void {
  start().catch(failed);
}

run();
