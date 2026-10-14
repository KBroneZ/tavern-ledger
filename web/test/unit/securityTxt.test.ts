// /.well-known/security.txt (RFC 9116): where to report a problem, in step
// with SECURITY.md, and not expired.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const ROOT = join(import.meta.dirname, "..", "..");
const TXT = readFileSync(join(ROOT, "public", ".well-known", "security.txt"), "utf8");
const POLICY = readFileSync(join(ROOT, "..", "SECURITY.md"), "utf8");

function field(name: string): string[] {
  return TXT.split(/\r?\n/).flatMap((line) => (line.startsWith(`${name}: `) ? [line.slice(name.length + 2).trim()] : []));
}

test("security.txt names the contact address SECURITY.md already uses", () => {
  const [contact, ...rest] = field("Contact");
  assert.equal(rest.length, 0);
  assert.match(contact, /^mailto:[^\s@]+@[^\s@]+$/);
  assert.ok(POLICY.includes(contact.slice("mailto:".length)), "the same address as SECURITY.md");
});

test("security.txt expires within a year and has not expired", () => {
  const [expires, ...rest] = field("Expires");
  assert.equal(rest.length, 0);
  assert.match(expires, /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?Z$/);
  const ms = Date.parse(expires) - Date.now();
  const DAY = 24 * 60 * 60 * 1000;
  assert.ok(ms > 0, "expired: set a new date a year out (and review SECURITY.md)");
  assert.ok(ms <= 366 * DAY, "RFC 9116 asks for less than a year ahead");
});

test("security.txt asks for English or Spanish and points at its own address and the policy", () => {
  assert.deepEqual(field("Preferred-Languages"), ["en, es"]);
  assert.deepEqual(field("Canonical"), ["https://tavernledger.net/.well-known/security.txt"]);
  assert.deepEqual(field("Policy"), ["https://github.com/KBroneZ/tavern-ledger/blob/main/SECURITY.md"]);
  assert.match(TXT, /\r?\n$/, "ends with a line break");
});
