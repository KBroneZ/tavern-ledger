// The facts of the release the download page and the home page's Download
// button point to (T-558). They are not typed by hand: `python tools/release.py
// facts vX.Y.Z` reads the published release from GitHub (each file's size and
// GitHub's own SHA-256) and writes releases.json, newest first, and a daily
// check compares the live files with it (.github/workflows/release-check.yml).
// Everything else (file name, tag, addresses) follows from the version.
import releases from "./releases.json" with { type: "json" };

const REPO = "https://github.com/KBroneZ/tavern-ledger";

interface Recorded {
  version: string;
  tag: string;
  file: string;
  bytes: number;
  sha256: string;
  date: string;
  sums: boolean;
}

/** The newest recorded release; a malformed record stops the build. */
function newest(list: readonly Recorded[]): Recorded {
  const r = list[0];
  const ok =
    r !== undefined &&
    /^\d+\.\d+\.\d+$/.test(r.version) &&
    r.tag === `v${r.version}` &&
    r.file === `TavernLedger_${r.version}_x64-setup.exe` &&
    Number.isInteger(r.bytes) &&
    r.bytes > 0 &&
    /^[0-9a-f]{64}$/.test(r.sha256) &&
    /^\d{4}-\d{2}-\d{2}$/.test(r.date) &&
    typeof r.sums === "boolean";
  if (!ok) throw new Error("web/src/lib/site/releases.json: the newest release is not well formed");
  return r;
}

const CURRENT = newest(releases);

export const RELEASE = {
  version: CURRENT.version,
  /** The git tag of the GitHub release. */
  tag: CURRENT.tag,
  /** The installer's file name, as uploaded to the release. */
  file: CURRENT.file,
  /** Size of the file in bytes. */
  bytes: CURRENT.bytes,
  /** SHA-256 of the file, lower case. */
  sha256: CURRENT.sha256,
  /** The day it was published (UTC), YYYY-MM-DD. */
  date: CURRENT.date,
  /** Whether the release carries a signed SHA256SUMS file. */
  sums: CURRENT.sums,
} as const;

/** Direct link to the installer. */
export const installerUrl = (): string => `${REPO}/releases/download/${RELEASE.tag}/${RELEASE.file}`;

/** The release's list of SHA-256 hashes and its minisign signature; null for a release without them (v0.1.0). */
export const sumsUrls = (): { list: string; signature: string } | null =>
  RELEASE.sums
    ? {
        list: `${REPO}/releases/download/${RELEASE.tag}/SHA256SUMS`,
        signature: `${REPO}/releases/download/${RELEASE.tag}/SHA256SUMS.minisig`,
      }
    : null;

/** The release's own page on GitHub (notes and files). */
export const releasePageUrl = (): string => `${REPO}/releases/tag/${RELEASE.tag}`;

/** Every release. */
export const releasesUrl = (): string => `${REPO}/releases`;

/** The project's page on GitHub. */
export const repoUrl = (): string => REPO;

/** Megabytes (1 MB = 1,048,576 bytes, as Windows shows file sizes) with one decimal. */
export const sizeMb = (bytes: number): number => Math.round((bytes / 1_048_576) * 10) / 10;
