// Puts the shared drawing of a minion into the element that carries it
// (drawn.ts attributes, a `.tlm-slot` child). Plain DOM calls on the element
// it is given, so it needs no page; runtime.ts owns when it runs.
import { type DrawnMinion, drawnAttrs, readDrawn, TLM_ATTRS } from "./drawn.ts";

/** What of the shared drawing (minion-art.js) the painter uses. */
export interface ArtLike {
  render(minion: { card_id: string; atk: number | null; health: number | null; golden: boolean; keywords: string[] }): HTMLElement;
  update(element: HTMLElement, minion: { card_id: string; atk: number | null; health: number | null; golden: boolean; keywords: string[] }): void;
}

/**
 * Draws the minion a host carries in its slot, or updates the drawing already
 * there (so a picture is never loaded twice). False when the host has no
 * readable minion or no slot: it keeps the page's own look.
 */
export function paintHost(host: HTMLElement, art: ArtLike): boolean {
  const attrs: Record<string, string | null> = {};
  for (const name of TLM_ATTRS) attrs[name] = host.getAttribute(name);
  const minion = readDrawn(attrs);
  const slot = host.querySelector<HTMLElement>(".tlm-slot");
  if (!minion || !slot) return false;
  const drawn = {
    card_id: minion.cardId,
    atk: minion.atk,
    health: minion.health,
    golden: minion.golden,
    keywords: [...minion.keywords],
  };
  const made = slot.querySelector<HTMLElement>(".tlm");
  if (made) art.update(made, drawn);
  else slot.append(art.render(drawn));
  return true;
}

/** Gives an element already in the page the attributes of its minion's present state (a replay step). */
export function carryMinion(el: HTMLElement, minion: DrawnMinion | null): void {
  const attrs = drawnAttrs(minion);
  for (const name of TLM_ATTRS) {
    const value = attrs[name];
    if (typeof value === "string") el.setAttribute(name, value);
    else el.removeAttribute(name);
  }
}
