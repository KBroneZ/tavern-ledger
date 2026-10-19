"use strict";
// A sticky bar covers what scrolls under it, so its background is opaque: a
// see-through one shows the text behind it as a ghost (T-359).
const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");

const UI = path.join(__dirname, "..", "ui");
const sheets = fs.readdirSync(UI).filter((f) => f.endsWith(".css"));

// Every top-level rule block: selector and body.
function rules(css) {
  const clean = css.replace(/\/\*[\s\S]*?\*\//g, "");
  return [...clean.matchAll(/([^{}]+)\{([^{}]*)\}/g)].map((m) => ({ selector: m[1].trim(), body: m[2] }));
}

test("every sticky bar has an opaque background", () => {
  const sticky = sheets.flatMap((file) =>
    rules(fs.readFileSync(path.join(UI, file), "utf8"))
      .filter((r) => /position:\s*sticky/.test(r.body))
      .map((r) => ({ file, ...r })),
  );
  assert.ok(sticky.length >= 2, "the masthead and the recap bar are sticky");
  for (const { file, selector, body } of sticky) {
    const background = /(?:^|;|\s)background:\s*([^;]+);/.exec(body);
    assert.ok(background, `${file} ${selector} sets a background`);
    const value = background[1];
    assert.doesNotMatch(value, /transparent|\/\s*0?\.\d+\s*\)|rgba\(|gradient/, `${file} ${selector}: ${value}`);
  }
});
