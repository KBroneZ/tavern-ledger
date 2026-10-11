"""Check that every dependency's license is compatible with AGPL-3.0-only.

Reads the Rust workspace through `cargo metadata --locked` (every crate in
Cargo.lock, all platforms) and the two npm lockfiles (package-lock.json and
web/package-lock.json). Our own crates and packages must say AGPL-3.0-only;
every third-party one must have an SPDX expression that ALLOWED satisfies
(OR needs one allowed side, AND needs all of them). A missing or unreadable
license counts as a problem. Standard library only; it changes nothing.
The throwaway spikes (`spikes/`, own lockfiles, never shipped) are not checked.

Usage:
    python tools/check_licenses.py

Exit codes: 0 = all compatible, 1 = problems found (listed), 2 = cargo
metadata or a lockfile could not be read.
"""

from __future__ import annotations

import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OWN_LICENSE = "AGPL-3.0-only"
NPM_LOCKS = (("root", ROOT / "package-lock.json"), ("web", ROOT / "web" / "package-lock.json"))
CARGO_TIMEOUT_SECONDS = 300

# Licenses whose code can be combined with AGPL-3.0 (FSF list of
# GPLv3-compatible licenses; MPL-2.0 through its secondary-license clause,
# LGPL as the "or later" versions). Add one only after checking it there.
ALLOWED = frozenset(
    {
        "0BSD",
        "Apache-2.0",
        "BlueOak-1.0.0",
        "BSD-2-Clause",
        "BSD-3-Clause",
        "CC0-1.0",
        "ISC",
        "LGPL-2.1-or-later",
        "LGPL-3.0-or-later",
        "MIT",
        "MIT-0",
        "MPL-2.0",
        "Python-2.0",
        "Unicode-3.0",
        "Unlicense",
        "Zlib",
    }
)
ALLOWED_EXCEPTIONS = frozenset({"LLVM-exception"})

TOKEN_RE = re.compile(r"\(|\)|[^\s()]+")


def _tokens(expr: str) -> list[str]:
    # Old Cargo manifests write "MIT/Apache-2.0" for "MIT OR Apache-2.0".
    return TOKEN_RE.findall(expr.replace("/", " OR "))


def _parse_or(tokens: list[str], pos: int) -> tuple[bool, int]:
    value, pos = _parse_and(tokens, pos)
    while pos < len(tokens) and tokens[pos] == "OR":
        right, pos = _parse_and(tokens, pos + 1)
        value = value or right
    return value, pos


def _parse_and(tokens: list[str], pos: int) -> tuple[bool, int]:
    value, pos = _parse_atom(tokens, pos)
    while pos < len(tokens) and tokens[pos] == "AND":
        right, pos = _parse_atom(tokens, pos + 1)
        value = value and right
    return value, pos


def _parse_atom(tokens: list[str], pos: int) -> tuple[bool, int]:
    if pos >= len(tokens):
        raise ValueError("expression ends early")
    token = tokens[pos]
    if token == "(":
        value, pos = _parse_or(tokens, pos + 1)
        if pos >= len(tokens) or tokens[pos] != ")":
            raise ValueError("missing )")
        return value, pos + 1
    if token in ("AND", "OR", "WITH", ")"):
        raise ValueError(f"unexpected {token}")
    value = token in ALLOWED
    pos += 1
    if pos < len(tokens) and tokens[pos] == "WITH":
        if pos + 1 >= len(tokens):
            raise ValueError("WITH without an exception")
        value = value and tokens[pos + 1] in ALLOWED_EXCEPTIONS
        pos += 2
    return value, pos


def is_allowed(expr: str | None) -> bool:
    """True if the SPDX expression is satisfied by ALLOWED; malformed is False."""
    if not isinstance(expr, str) or not expr.strip():
        return False
    tokens = _tokens(expr)
    try:
        value, pos = _parse_or(tokens, 0)
    except (ValueError, RecursionError):
        return False
    return value and pos == len(tokens)


def check_cargo(metadata: dict) -> tuple[list[str], int]:
    """Problems in `cargo metadata` output, and how many packages were checked."""
    packages = metadata.get("packages") or []
    if not packages:
        return ["cargo: no packages in the metadata"], 0
    members = set(metadata.get("workspace_members") or [])
    problems = []
    for pkg in packages:
        name = f"cargo: {pkg.get('name')} {pkg.get('version')}"
        license_expr = pkg.get("license")
        if pkg.get("id") in members:
            if license_expr != OWN_LICENSE:
                problems.append(f"{name}: own crate must be {OWN_LICENSE}, is {license_expr!r}")
        elif not is_allowed(license_expr):
            problems.append(f"{name}: license {license_expr!r} not in the allow list")
    return problems, len(packages)


def check_npm_lock(lock: dict, label: str) -> tuple[list[str], int]:
    """Problems in an npm lockfile (v2/v3), and how many dependencies were checked."""
    packages = lock.get("packages") or {}
    root = packages.get("")
    if root is None:
        return [f"npm {label}: no root package in the lockfile"], 0
    problems = []
    if root.get("license") != OWN_LICENSE:
        problems.append(f"npm {label}: own package must be {OWN_LICENSE}, is {root.get('license')!r}")
    deps = {path: pkg for path, pkg in packages.items() if path}
    for path, pkg in sorted(deps.items()):
        license_expr = pkg.get("license")
        if not is_allowed(license_expr):
            problems.append(f"npm {label}: {path} {pkg.get('version')}: license {license_expr!r} not in the allow list")
    return problems, len(deps)


def _cargo_metadata() -> dict:
    result = subprocess.run(
        ["cargo", "metadata", "--locked", "--format-version", "1"],
        cwd=ROOT,
        capture_output=True,
        check=True,
        timeout=CARGO_TIMEOUT_SECONDS,
    )
    return json.loads(result.stdout.decode("utf-8"))


def main() -> int:
    try:
        metadata = _cargo_metadata()
        locks = [(label, json.loads(path.read_text(encoding="utf-8"))) for label, path in NPM_LOCKS]
    except subprocess.CalledProcessError as exc:
        stderr = exc.stderr.decode("utf-8", "replace").strip() if exc.stderr else ""
        print(f"error: cargo metadata failed: {stderr or exc}", file=sys.stderr)
        return 2
    except (OSError, subprocess.SubprocessError, ValueError) as exc:
        print(f"error: could not read the dependency metadata: {exc}", file=sys.stderr)
        return 2

    problems, counted = check_cargo(metadata)
    print(f"cargo: {counted} packages checked")
    for label, lock in locks:
        lock_problems, lock_counted = check_npm_lock(lock, label)
        print(f"npm {label}: {lock_counted} packages checked")
        problems.extend(lock_problems)

    for problem in problems:
        print(problem)
    if problems:
        print(f"{len(problems)} license problem(s)")
        return 1
    print(f"all licenses compatible with {OWN_LICENSE}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
