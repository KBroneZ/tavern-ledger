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

/**
 * Enables a form's submit button. The pages ship it disabled, so a form
 * sent before its script runs cannot fall back to a plain GET that would
 * put the email (and password) in the address.
 */
export function ready(form: HTMLFormElement): void {
  for (const button of form.querySelectorAll<HTMLButtonElement>('button[type="submit"]')) {
    button.disabled = false;
  }
}

/**
 * Says an error and ties it to the field that holds it: `aria-invalid` and
 * `aria-describedby` on the field, then focus on it.
 */
export function failField(input: HTMLInputElement, message: HTMLElement, text: string): void {
  say(message, text, "error");
  input.setAttribute("aria-invalid", "true");
  input.setAttribute("aria-describedby", message.id);
  input.focus();
}

/** Takes the error ties off every field of the form (call before a new check). */
export function clearInvalid(form: HTMLFormElement): void {
  for (const input of form.querySelectorAll<HTMLElement>("[aria-invalid]")) {
    input.removeAttribute("aria-invalid");
    input.removeAttribute("aria-describedby");
  }
}

/** Moves focus to an element that is not a control (a heading or a message). */
export function moveFocus(el: HTMLElement): void {
  el.setAttribute("tabindex", "-1");
  el.focus();
}

/**
 * Runs `work` with the button marked busy, so a form cannot be sent twice.
 * The button is not `disabled`: a disabled button loses focus, and keyboard
 * and screen reader users would land back on the page start. A second call
 * while busy does nothing.
 */
export async function busy<T>(button: HTMLButtonElement, work: () => Promise<T>): Promise<T | undefined> {
  if (button.getAttribute("aria-disabled") === "true") return undefined;
  button.setAttribute("aria-disabled", "true");
  button.setAttribute("aria-busy", "true");
  try {
    return await work();
  } finally {
    button.removeAttribute("aria-disabled");
    button.removeAttribute("aria-busy");
  }
}
