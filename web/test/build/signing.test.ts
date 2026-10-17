// The code signing policy page SignPath's terms ask for (T-378 area, session
// 193), checked on the built files: its required words and roles, the privacy
// link in the page's language, and the term "Code signing policy" linked from
// the home page and the download page.
import { test } from "node:test";
import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";

const DIST = join(import.meta.dirname, "..", "..", "dist");
const read = (file: string) => readFileSync(join(DIST, file), "utf8");

test("the policy page names SignPath as the terms ask, in both languages", () => {
  for (const [file, privacy] of [["code-signing/index.html", "/privacy/"], ["es/code-signing/index.html", "/es/privacy/"]] as const) {
    assert.ok(existsSync(join(DIST, file)), "run `npm run build` first");
    const html = read(file);
    assert.match(
      html,
      /Free code signing provided by <a href="https:\/\/signpath\.io">SignPath\.io<\/a>, certificate by <a href="https:\/\/signpath\.org">SignPath Foundation<\/a>/,
      file,
    );
    assert.match(html, /href="https:\/\/github\.com\/KBroneZ"/, `${file}: team members`);
    assert.ok(html.includes(`href="${privacy}"`), `${file}: privacy link in the page's language`);
    assert.doesNotMatch(html, /<meta name="robots" content="noindex">/, file);
  }
});

test("the home and download pages link the policy with the words 'Code signing policy'", () => {
  for (const [file, href, words] of [
    ["index.html", "/code-signing/", "Code signing policy"],
    ["download/index.html", "/code-signing/", "Code signing policy"],
    ["es/index.html", "/es/code-signing/", "Política de firma de código"],
    ["es/download/index.html", "/es/code-signing/", "Política de firma de código"],
  ] as const) {
    assert.match(read(file), new RegExp(`<a href="${href}">${words}</a>`), file);
  }
});
