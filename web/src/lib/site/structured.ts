// Structured data (schema.org) for the download and simulator pages (T-703):
// what the app and the simulator are, in the words and facts the pages
// already show. Only facts: no rating, review or download count, which the
// project does not have (marketing rule: never make up figures).
import { RELEASE, installerUrl, repoUrl } from "./release.ts";

const AUTHOR = { "@type": "Person", name: "KBroneZ", url: "https://github.com/KBroneZ" } as const;
const FREE = { "@type": "Offer", price: "0", priceCurrency: "EUR" } as const;
const LICENSE = `${repoUrl()}/blob/main/LICENSE`;

/** The desktop app, for the download page at `url` in its language. */
export function appLd(url: string, description: string) {
  return {
    "@type": "SoftwareApplication",
    name: "Tavern Ledger",
    description,
    url,
    applicationCategory: "UtilitiesApplication",
    operatingSystem: "Windows 10, Windows 11",
    softwareVersion: RELEASE.version,
    downloadUrl: installerUrl(),
    isAccessibleForFree: true,
    offers: FREE,
    license: LICENSE,
    author: AUTHOR,
    inLanguage: ["en", "es"],
    sameAs: [repoUrl()],
  };
}

/** The website's combat simulator, at `url` in its language. */
export function simulatorLd(url: string, name: string, description: string) {
  return {
    "@type": "WebApplication",
    name,
    description,
    url,
    applicationCategory: "UtilitiesApplication",
    operatingSystem: "Any",
    browserRequirements: "Requires JavaScript and WebAssembly",
    isAccessibleForFree: true,
    offers: FREE,
    license: LICENSE,
    author: AUTHOR,
  };
}
