// Which files the workbench may serve. A request path is cut into its
// segments and refused when any of them could leave the folder it is
// served from; what is left must be a plain file inside that folder.
"use strict";

const fs = require("node:fs");
const path = require("node:path");

// The request path (still percent-encoded) as folder-relative segments, or
// null: backslashes, NULs, `.` and `..` segments (also encoded), empty
// segments (so absolute paths and a trailing slash), and `:` (drive letters,
// NTFS streams) are all refused.
function safeSegments(raw) {
  if (typeof raw !== "string" || raw.includes("\\") || raw.includes("\0")) return null;
  let decoded;
  try {
    decoded = decodeURIComponent(raw);
  } catch {
    return null;
  }
  if (decoded.includes("\\") || decoded.includes("\0")) return null;
  const segments = decoded.split("/");
  const bad = (s) => s === "" || s === "." || s === ".." || s.includes(":");
  return segments.some(bad) ? null : segments;
}

// The real path of a file under `root`, or null. Symbolic links that lead
// out of the folder are refused too, and so are folders (no listings).
function resolveInside(root, raw) {
  const segments = safeSegments(raw);
  if (!segments) return null;
  try {
    const base = fs.realpathSync(root);
    const real = fs.realpathSync(path.join(base, ...segments));
    if (!real.startsWith(base + path.sep)) return null;
    return fs.statSync(real).isFile() ? real : null;
  } catch {
    return null;
  }
}

module.exports = { safeSegments, resolveInside };
