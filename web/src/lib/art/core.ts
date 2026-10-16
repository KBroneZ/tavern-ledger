// When the minions' drawing loads and what a page does without it (T-372,
// D-150). No page API in here: runtime.ts gives it the pieces (how to load the
// drawing, the page's minions, where to warn), so the rules can be tested.
//
// The rules: nothing loads on a page with no minion; until the drawing is
// ready, and for good if it cannot be had, a minion keeps the page's own look
// (an empty slot takes no room), so nothing is ever half drawn or a broken
// image; the page is warned at most once about the textures, whatever goes
// wrong (the drawing's own reports included); leaving the page lets
// everything go, and a load that ends after that is dropped.
import { type ArtLike, paintHost } from "./paint.ts";

/** The shared drawing's handle, as far as the page needs it. */
export interface ArtHandle extends ArtLike {
  ready: Promise<void>;
  destroy(): void;
}

export interface CoreEnv {
  /** Loads the drawing and checks the set can be reached; null when the drawing cannot start; rejects when the set is out of reach. `log` takes the drawing's own trouble reports. */
  loadDrawing(log: (message: string) => void): Promise<ArtHandle | null>;
  warn(message: string): void;
  /** The minions to draw under a root (the page, or a part of it). */
  hosts(root: ParentNode): HTMLElement[];
  /** Every minion of the page, for when the drawing becomes ready. */
  allHosts(): HTMLElement[];
  /** Registers the call to make when the page goes away. */
  onLeave(release: () => void): void;
}

type State = "idle" | "loading" | "ready" | "off";

export interface MinionCore {
  paint(root: ParentNode): void;
  release(): void;
  /** The drawing's own trouble reports: the page's one warning, or nothing. */
  warn(message: string): void;
}

export function createMinionCore(env: CoreEnv): MinionCore {
  let state: State = "idle";
  let art: ArtHandle | null = null;
  let warned = false;
  let watching = false;
  /** A newer start (or a leave) makes an older load stop. */
  let generation = 0;

  function warn(message: string): void {
    if (warned) return;
    warned = true;
    env.warn(message);
  }

  async function load(mine: number): Promise<void> {
    try {
      const made = await env.loadDrawing(warn);
      if (!made) throw new Error("the drawing could not start");
      await made.ready;
      if (mine !== generation) return made.destroy();
      art = made;
      state = "ready";
      for (const host of env.allHosts()) paintHost(host, made);
    } catch (error) {
      if (mine !== generation) return;
      state = "off";
      const why = error instanceof Error ? error.message : "unknown";
      warn(`the game's minion textures could not be loaded, minions are drawn without them (${why})`);
    }
  }

  function release(): void {
    generation += 1;
    if (art) art.destroy();
    art = null;
    state = "idle";
  }

  function paint(root: ParentNode): void {
    const hosts = env.hosts(root);
    if (hosts.length === 0 || state === "off") return;
    if (!watching) {
      watching = true;
      env.onLeave(release);
    }
    if (state === "idle") {
      state = "loading";
      void load(generation);
    }
    if (state !== "ready" || !art) return;
    for (const host of hosts) paintHost(host, art);
  }

  return { paint, release, warn };
}
