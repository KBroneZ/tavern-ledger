// Other ways to install the app (T-707): the Microsoft Store and winget. Each
// is off (null) until its listing is live, because the site never announces a
// channel that does not exist (marketing rule). Turning one on is the one-line
// change of the pull request that follows the listing; a malformed value stops
// the build.

/** The Store listing's address, e.g. https://apps.microsoft.com/detail/9ABCDEFGHJKL, once live. */
export const STORE_URL: string | null = null;
/** The winget package identifier, once winget-pkgs has merged it. */
export const WINGET_ID: string | null = null;

const STORE = /^https:\/\/apps\.microsoft\.com\/detail\/[0-9A-Z]{12}$/;
const WINGET = /^[A-Za-z0-9-]+\.[A-Za-z0-9.-]+$/;

export type Channel = { kind: "store"; url: string } | { kind: "winget"; command: string };

/** The channels that are live, Store first; throws on a malformed value. */
export function liveChannels(store: string | null = STORE_URL, winget: string | null = WINGET_ID): Channel[] {
  const live: Channel[] = [];
  if (store !== null) {
    if (!STORE.test(store)) throw new Error(`lib/site/channels.ts: not a Store listing address: ${store}`);
    live.push({ kind: "store", url: store });
  }
  if (winget !== null) {
    if (!WINGET.test(winget)) throw new Error(`lib/site/channels.ts: not a winget package identifier: ${winget}`);
    live.push({ kind: "winget", command: `winget install ${winget}` });
  }
  return live;
}
