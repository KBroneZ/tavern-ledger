import { loadPublicProfile } from "../lib/api.ts";
import { parseProfileId } from "../lib/profile.ts";
import { supabase } from "./client.ts";
import { byId, cell, say, show } from "./dom.ts";

const status = byId("profile-status");

function notFound(): void {
  say(status, "");
  show(byId("not-found"), true);
  document.title = "Profile not found · Tavern Ledger";
}

async function start(): Promise<void> {
  const id = parseProfileId(location.search);
  // A bad id, a private profile and a missing one all look the same.
  if (id === null) return notFound();
  const result = await loadPublicProfile(supabase, id);
  if (result.kind === "not-found") return notFound();
  if (result.kind === "error") {
    say(status, "Could not load this profile. Try again in a moment.", "error");
    return;
  }
  const name = result.name ?? "Unnamed player";
  byId("profile-name").textContent = name;
  document.title = `${name} · Tavern Ledger`;
  byId("profile-summary").textContent = result.games.length === 0
    ? "No games uploaded yet."
    : `${result.games.length} game${result.games.length === 1 ? "" : "s"}, newest first.`;
  const rows = result.games.map((g) => {
    const tr = document.createElement("tr");
    tr.append(cell(g.date), cell(g.mode), cell(g.hero, "mono"), cell(g.place, "num"));
    return tr;
  });
  byId("games").replaceChildren(...rows);
  say(status, "");
  show(byId("profile"), true);
}

start().catch(() => say(status, "Could not load this profile. Try again in a moment.", "error"));
