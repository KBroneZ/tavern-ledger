// Form accessibility helpers (T-318): errors tied to their fields, and a
// busy button that keeps focus. A tiny fake stands in for the DOM nodes.
import { test } from "node:test";
import assert from "node:assert/strict";
import { busy, clearInvalid, failField, moveFocus } from "../../src/scripts/dom.ts";
import { newPasswordField } from "../../src/lib/auth.ts";

class FakeEl {
  attrs = new Map<string, string>();
  focused = 0;
  textContent = "";
  classes = new Set<string>();
  classList = {
    toggle: (name: string, on: boolean) => void (on ? this.classes.add(name) : this.classes.delete(name)),
  };
  id: string;
  constructor(id = "") { this.id = id; }
  setAttribute(name: string, value: string) { this.attrs.set(name, value); }
  getAttribute(name: string) { return this.attrs.get(name) ?? null; }
  removeAttribute(name: string) { this.attrs.delete(name); }
  hasAttribute(name: string) { return this.attrs.has(name); }
  focus() { this.focused += 1; }
}
const el = (id = "") => new FakeEl(id) as unknown as HTMLElement & FakeEl;

test("failField: says the error, ties it to the field and focuses it", () => {
  const input = el("email");
  const message = el("signin-message");
  failField(input as unknown as HTMLInputElement, message, "Enter a valid email address.");
  assert.equal(message.textContent, "Enter a valid email address.");
  assert.ok(message.classes.has("error"));
  assert.equal(input.getAttribute("aria-invalid"), "true");
  assert.equal(input.getAttribute("aria-describedby"), "signin-message");
  assert.equal(input.focused, 1);
});

test("clearInvalid: removes the ties from every field of the form", () => {
  const a = el("a");
  const b = el("b");
  for (const x of [a, b]) {
    x.setAttribute("aria-invalid", "true");
    x.setAttribute("aria-describedby", "m");
  }
  const form = { querySelectorAll: () => [a, b] } as unknown as HTMLFormElement;
  clearInvalid(form);
  for (const x of [a, b]) {
    assert.equal(x.hasAttribute("aria-invalid"), false);
    assert.equal(x.hasAttribute("aria-describedby"), false);
  }
});

test("moveFocus: makes the target focusable by script only, then focuses it", () => {
  const heading = el("h");
  moveFocus(heading);
  assert.equal(heading.getAttribute("tabindex"), "-1");
  assert.equal(heading.focused, 1);
});

test("busy: the button stays focusable and keeps focus, and is not enabled twice", async () => {
  const button = el("b") as unknown as HTMLButtonElement & FakeEl;
  let seenWhileBusy: string | null = null;
  const first = busy(button, async () => {
    seenWhileBusy = button.getAttribute("aria-disabled");
    return "done";
  });
  assert.equal(button.getAttribute("aria-busy"), "true");
  assert.equal((button as unknown as { disabled?: boolean }).disabled, undefined);
  assert.equal(await first, "done");
  assert.equal(seenWhileBusy, "true");
  assert.equal(button.hasAttribute("aria-disabled"), false);
  assert.equal(button.hasAttribute("aria-busy"), false);
});

test("busy: a second call while busy does not run the work again", async () => {
  const button = el("b") as unknown as HTMLButtonElement;
  let runs = 0;
  let release: () => void = () => {};
  const first = busy(button, () => new Promise<void>((r) => { runs += 1; release = r; }));
  const second = await busy(button, async () => { runs += 1; });
  assert.equal(second, undefined);
  release();
  await first;
  assert.equal(runs, 1);
});

test("busy: re-enables the button when the work throws", async () => {
  const button = el("b") as unknown as HTMLButtonElement & FakeEl;
  await assert.rejects(busy(button, async () => { throw new Error("x"); }));
  assert.equal(button.hasAttribute("aria-disabled"), false);
});

test("newPasswordField: names the field that holds the problem", () => {
  assert.equal(newPasswordField("short"), "password");
  assert.equal(newPasswordField("a-long-enough-password"), "repeat");
});
