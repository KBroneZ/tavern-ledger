// The game's textures for minions (T-372, D-150, D-151): where the site loads
// them from. They are hosted outside this repo, on their own Cloudflare Pages
// project; every file of a version is immutable, so a new extraction is a new
// version folder and this one line. The app asks for the same set
// (crates/cards/src/textures.rs VERSION); test/unit/minionArt.test.ts fails
// if the two differ.
import { TEXTURE_LAYOUT } from "./textureLayout.ts";

export { TEXTURE_LAYOUT };

export const TEXTURE_VERSION = "v2";
/** The one host the site's policy allows for these images (img-src, nothing broader). */
export const TEXTURE_ORIGIN = "https://assets.tavernledger.net";
export const TEXTURE_BASE = `${TEXTURE_ORIGIN}/${TEXTURE_VERSION}/`;

/**
 * The textures the site draws, by the role name the set's manifest gives
 * them; each is `<role>.webp` in the version's folder. Rally and the quest
 * icon are in the set but not drawn on the site.
 */
export const TEXTURE_ROLES = [
  "frame_normal",
  "frame_golden",
  "oval_mask",
  "taunt_normal",
  "taunt_golden",
  "divine_shield",
  "reborn_glow",
  "venomous",
  "poisonous",
  "skull",
  "trigger",
  "gem_attack",
  "gem_health",
  "wf_map_A0",
  "wf_map_A1",
  "wf_map_B0",
  "wf_map_B1",
  "wind1",
  "st_cover",
  "StealthDome_old",
  "StealthDomeMask",
  "StealthDomeShadow_D",
] as const;

const KNOWN: ReadonlySet<string> = new Set(TEXTURE_ROLES);

/** The address of a texture, or null for a role the site does not draw. */
export function textureUrl(role: string): string | null {
  return KNOWN.has(role) ? `${TEXTURE_BASE}${role}.webp` : null;
}
