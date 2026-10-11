// Account deletion: the typed confirmation and how the delete-account
// function's answer is read (supabase/functions/delete-account/handler.ts).

import { at } from "./account/strings.ts";

/** The phrase to type, in the page's language (T-508). */
export const confirmPhrase = (): string => at("account.delete.phrase");

export function isConfirmed(typed: string): boolean {
  return typed.trim() === confirmPhrase();
}

export type DeleteOutcome =
  | { kind: "deleted" }
  | { kind: "reauthenticate" }
  | { kind: "signed-out" }
  | { kind: "error"; message: string };

const STEPS = new Set(["auth", "list files", "delete files", "delete rows", "delete auth user", "delete auth events"]);

/** `status` 0 means the request never got an answer. */
export function deleteOutcome(status: number, body: unknown): DeleteOutcome {
  const b = (body ?? {}) as { deleted?: { account?: unknown }; reason?: unknown; step?: unknown };
  if (status === 200) {
    return b.deleted?.account === 1
      ? { kind: "deleted" }
      : { kind: "error", message: at("delete.unexpected") };
  }
  if (status === 403 && b.reason === "reauthenticate") return { kind: "reauthenticate" };
  if (status === 401) return { kind: "signed-out" };
  if (status === 0) {
    return { kind: "error", message: at("delete.unreachable") };
  }
  if (status >= 500) {
    const known = typeof b.step === "string" && STEPS.has(b.step);
    return { kind: "error", message: known ? at("delete.unfinishedStep", { step: b.step as string }) : at("delete.unfinished") };
  }
  return { kind: "error", message: at("delete.refused") };
}
