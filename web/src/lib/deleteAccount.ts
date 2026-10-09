// Account deletion: the typed confirmation and how the delete-account
// function's answer is read (supabase/functions/delete-account/handler.ts).

export const CONFIRM_PHRASE = "delete my account";

export function isConfirmed(typed: string): boolean {
  return typed.trim() === CONFIRM_PHRASE;
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
      : { kind: "error", message: "The server gave an unexpected answer. Try again." };
  }
  if (status === 403 && b.reason === "reauthenticate") return { kind: "reauthenticate" };
  if (status === 401) return { kind: "signed-out" };
  if (status === 0) {
    return { kind: "error", message: "Could not reach the server. Nothing was deleted yet; try again." };
  }
  if (status >= 500) {
    const step = typeof b.step === "string" && STEPS.has(b.step) ? ` (step: ${b.step})` : "";
    return {
      kind: "error",
      message: `Deletion did not finish${step}. Try again: it picks up where it stopped.`,
    };
  }
  return { kind: "error", message: "The server refused the request. Reload the page and try again." };
}
