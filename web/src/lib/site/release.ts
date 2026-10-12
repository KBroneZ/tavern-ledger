// The facts of the release the download page and the home page's Download
// button point to (T-558). One place to change when a new release goes out:
// everything else (file name, tag, addresses) follows from the version.
// Values come from the GitHub release's own asset record, never typed from
// memory: size and SHA-256 are what GitHub reports for the uploaded file.

const REPO = "https://github.com/KBroneZ/tavern-ledger";

const VERSION = "0.1.0";

export const RELEASE = {
  version: VERSION,
  /** The git tag of the GitHub release. */
  tag: `v${VERSION}`,
  /** The installer's file name, as uploaded to the release. */
  file: `TavernLedger_${VERSION}_x64-setup.exe`,
  /** Size of the file in bytes. */
  bytes: 6_847_192,
  /** SHA-256 of the file, lower case. */
  sha256: "e52f3a76a4ffb7717f017e431a6e56df7d86826e95d0372e75f5e3215e9386ee",
  /** The day it was published (UTC), YYYY-MM-DD. */
  date: "2026-10-12",
} as const;

/** Direct link to the installer. */
export const installerUrl = (): string => `${REPO}/releases/download/${RELEASE.tag}/${RELEASE.file}`;

/** The release's own page on GitHub (notes and files). */
export const releasePageUrl = (): string => `${REPO}/releases/tag/${RELEASE.tag}`;

/** Every release. */
export const releasesUrl = (): string => `${REPO}/releases`;

/** The project's page on GitHub. */
export const repoUrl = (): string => REPO;

/** Megabytes (1 MB = 1,048,576 bytes, as Windows shows file sizes) with one decimal. */
export const sizeMb = (bytes: number): number => Math.round((bytes / 1_048_576) * 10) / 10;
