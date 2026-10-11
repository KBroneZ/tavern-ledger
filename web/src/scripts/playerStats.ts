// The heroes page (/player-heroes/) and the curious stats page (/curious/)
// (T-505): yours when signed in, or a public profile's with ?id=…, with the
// dashboard's privacy (a private profile and a missing one look the same).
// Every update lands in one go inside the reserved #pp-body (D-073).
import { profileAccess, type ProfileAccess } from "../lib/dataApi.ts";
import { t } from "../lib/data/strings.ts";
import { watchPictureFallback } from "../lib/data/pictures.ts";
import { parseSort, withSort, type HeroSort, type HeroStat } from "../lib/heroes/heroes.ts";
import { loadCurious, loadHeroStats } from "../lib/heroes/load.ts";
import { curiousAbout, curiousEmpty, curiousPanels, heroesAbout, heroesEmpty, heroesPanels } from "../lib/heroes/views.ts";
import { parseProfileId, publicProfilePath } from "../lib/profile.ts";
import { h, mount, type VNode } from "../lib/view/node.ts";
import { stateBlock } from "../lib/view/pieces.ts";
import { supabase } from "./client.ts";
import { byId, moveFocus } from "./dom.ts";

// A picture that fails to load shows its local placeholder (D-074).
watchPictureFallback(document);

const root = byId("pp");
const heroesPage = root.dataset.page === "heroes";
const title = byId("pp-title");
const back = byId<HTMLAnchorElement>("pp-back");
const status = byId("pp-status");
const gate = byId("pp-gate");
const panels = byId("pp-modes");
const note = byId("pp-note");
const about = byId("pp-about");
const switches = byId("pp-switches");

// Not busy first: some screen readers skip a live region's change inside a busy element.
function settle(text: string): void {
  root.setAttribute("aria-busy", "false");
  status.textContent = text;
}

/** A whole-page message (signed out, private, empty, error) in place of the panels. */
function showGate(node: VNode, announce: string): void {
  mount(gate, node);
  gate.hidden = false;
  panels.hidden = true;
  switches.classList.add("is-off");
  settle(announce);
}

function privateGate(): void {
  back.hidden = true;
  title.textContent = t("dash.private.title");
  showGate(stateBlock({ kind: "unknown", title: t("dash.private.title"), body: t("dash.private.body") }), t("dash.private.title"));
  document.title = `${t("dash.private.title")} · Tavern Ledger`;
}

/** Whose page this is (null on the owner's own), or undefined when the page already said why there is nothing to show. */
async function subject(): Promise<{ userId: string; profileId: string | null; access: ProfileAccess } | undefined> {
  const { data } = await supabase.auth.getSession();
  const viewer = data.session?.user.id ?? null;
  const hasId = new URLSearchParams(location.search).has("id");
  if (!hasId) {
    if (!viewer) {
      const link = h("a", { href: "/signin/" }, t("dash.signedOut.link"));
      showGate(stateBlock({ kind: "empty", title: t("dash.signedOut"), body: "" }, h("p", {}, link)), t("dash.signedOut"));
      return undefined;
    }
    return { userId: viewer, profileId: null, access: await profileAccess(supabase, viewer, viewer) };
  }
  const id = parseProfileId(location.search);
  // A bad id, a private profile and a missing one all look the same.
  const access: ProfileAccess = id === null ? { kind: "private" } : await profileAccess(supabase, id, viewer);
  if (id === null || access.kind === "private") {
    privateGate();
    return undefined;
  }
  return { userId: id, profileId: id, access };
}

function nameTitle(access: ProfileAccess): void {
  if (access.kind === "private") return;
  const name = access.name ?? t("dash.unnamed");
  const text = heroesPage ? t("player.heroes.title.public", { name }) : t("player.curious.title.public", { name });
  title.textContent = text;
  document.title = `${text} · Tavern Ledger`;
}

function ownerNote(access: ProfileAccess): VNode[] {
  if (access.kind !== "own-private") return [];
  return [h("p", { class: "data-note" }, `${t("dash.private.own")} `, h("a", { href: "/account/" }, t("dash.private.own.link")))];
}

function emptyGate(own: boolean, extra: VNode[]): void {
  const [heading, body] = own ? [t("dash.empty.own"), t("dash.empty.own.body")] : [t("dash.empty.public"), t("dash.empty.public.body")];
  showGate(stateBlock({ kind: "empty", title: heading, body }, ...extra), heading);
}

function showPanels(nodes: VNode[], extra: VNode[], aboutNodes: VNode[]): void {
  mount(note, ...extra);
  mount(panels, ...nodes);
  mount(about, ...aboutNodes);
  gate.hidden = true;
  panels.hidden = false;
  switches.classList.remove("is-off");
  settle(t("dash.loaded"));
}

/** Re-sorts on a sort button's click, keeps the sort in the address and focus on the button. */
function watchSort(rows: HeroStat[], profileId: string | null): void {
  panels.addEventListener("click", (event) => {
    const button = (event.target as HTMLElement).closest<HTMLButtonElement>("button[data-sort]");
    const value = button?.dataset.sort;
    if (value !== "games" && value !== "avg") return;
    if (button?.getAttribute("aria-pressed") === "true") return;
    const panel = button?.closest<HTMLElement>("[data-mode-panel]")?.dataset.modePanel;
    history.replaceState(history.state, "", withSort(location.href, value));
    // A re-sort keeps every fold as the viewer left it (same panels, same folds, same order).
    const open = [...panels.querySelectorAll<HTMLDetailsElement>("details")].map((d) => d.open);
    mount(panels, ...heroesPanels(rows, { profileId, sort: value }));
    panels.querySelectorAll<HTMLDetailsElement>("details").forEach((d, i) => (d.open = open[i] ?? false));
    const target =
      panels.querySelector<HTMLButtonElement>(`[data-mode-panel="${panel}"] button[data-sort="${value}"]`) ??
      panels.querySelector<HTMLButtonElement>('button[data-sort][aria-pressed="true"]');
    target?.focus();
    status.textContent = t(value === "avg" ? "hstats.sorted.avg" : "hstats.sorted.games");
  });
}

async function start(): Promise<void> {
  root.setAttribute("aria-busy", "true");
  status.textContent = t("dash.loading");
  const who = await subject();
  if (!who) return;
  const { userId, profileId, access } = who;
  const own = profileId === null;
  if (!own) {
    back.href = publicProfilePath(userId);
    back.textContent = t("player.back.public");
    nameTitle(access);
  }
  const extra = ownerNote(access);
  if (heroesPage) {
    const rows = await loadHeroStats(supabase, userId);
    if (heroesEmpty(rows)) return emptyGate(own, extra);
    const sort: HeroSort = parseSort(location.search);
    showPanels(heroesPanels(rows, { profileId, sort }), extra, [heroesAbout()]);
    watchSort(rows, profileId);
    return;
  }
  const data = await loadCurious(supabase, userId);
  if (curiousEmpty(data)) return emptyGate(own, extra);
  showPanels(curiousPanels(data, userId), extra, curiousAbout());
}

function failed(): void {
  const retry = h("button", { type: "button", class: "secondary", id: "pp-retry" }, t("dash.retry"));
  showGate(stateBlock({ kind: "error", title: t("dash.error"), body: t("dash.error.body") }, retry), t("dash.error"));
  byId("pp-retry").addEventListener("click", () => {
    mount(gate, stateBlock({ kind: "loading", lines: 6 }));
    moveFocus(title);
    run();
  });
}

function run(): void {
  start().catch(failed);
}

run();
