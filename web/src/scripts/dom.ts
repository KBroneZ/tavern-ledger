// DOM helpers. Text only ever goes in through textContent.

export function byId<T extends HTMLElement = HTMLElement>(id: string): T {
  const el = document.getElementById(id);
  if (!el) throw new Error(`missing element #${id}`);
  return el as T;
}

export function field(form: HTMLFormElement, name: string): HTMLInputElement {
  const el = form.elements.namedItem(name);
  if (!(el instanceof HTMLInputElement)) throw new Error(`missing field ${name}`);
  return el;
}

export function show(el: HTMLElement, visible: boolean): void {
  el.hidden = !visible;
}

export type Tone = "info" | "ok" | "error";

export function say(el: HTMLElement, text: string, tone: Tone = "info"): void {
  el.textContent = text;
  el.classList.toggle("error", tone === "error");
  el.classList.toggle("ok", tone === "ok");
}

export function cell(text: string, className?: string): HTMLTableCellElement {
  const td = document.createElement("td");
  td.textContent = text;
  if (className) td.className = className;
  return td;
}

/** Runs `work` with the button disabled, so a form cannot be sent twice. */
export async function busy<T>(button: HTMLButtonElement, work: () => Promise<T>): Promise<T> {
  button.disabled = true;
  try {
    return await work();
  } finally {
    button.disabled = false;
  }
}
