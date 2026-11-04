// The one-time notice on the account page about the new use of uploaded games
// (T-651, D-174): training the expected-place model, on for every account
// under legitimate interest. Accounts made before the change see it once; the
// dismissal is remembered in this browser only. Storage that is missing or
// throws (a private window, blocked site data) never breaks the page: the
// notice then simply shows again next time.

/** The localStorage key of the dismissal; its value is the policy version the notice is about. */
export const NOTICE_KEY = "tl-training-notice";

/** The privacy policy version that introduced the use (as in the migration). */
const POLICY_VERSION = "2026-10-27";

/** Accounts made from this moment on signed up under the policy that already describes the use. */
const NOTICE_BEFORE = Date.parse("2026-10-28T00:00:00Z");

/** The browser's local storage, or null where it cannot be reached. */
export function browserStorage(): Storage | null {
  try {
    return window.localStorage;
  } catch {
    return null;
  }
}

/** Whether to show the notice to an account made at `createdAt` (ISO time; unknown errs on showing). */
export function showTrainingNotice(store: Pick<Storage, "getItem"> | null, createdAt: string | undefined): boolean {
  const made = createdAt ? Date.parse(createdAt) : Number.NaN;
  if (made >= NOTICE_BEFORE) return false;
  try {
    return store?.getItem(NOTICE_KEY) !== POLICY_VERSION;
  } catch {
    return true;
  }
}

/** Remembers that the notice was read. Does nothing where storage cannot be written. */
export function dismissTrainingNotice(store: Pick<Storage, "setItem"> | null): void {
  try {
    store?.setItem(NOTICE_KEY, POLICY_VERSION);
  } catch {
    // Nothing to keep it in: the notice shows again next time.
  }
}
