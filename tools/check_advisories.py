"""Fail on known security advisories in the pinned dependencies.

Runs osv-scanner (path given with --scanner) on the four lockfiles that pin
what we build and ship: Cargo.lock, package-lock.json, web/package-lock.json
and requirements.txt. An advisory fails the check when it has a fixed
version, when its severity is medium or higher (CVSS 4.0 and up), or when
its severity is unknown; only a low advisory with no fix is reported and
let through. An advisory listed in tools/advisory_ignores.toml passes, but
only while its entry is complete (advisory, package, reason, evidence and a
review date not yet past) and, for a crate, only while the crate is still
missing from every build the entry names (checked with `cargo metadata`).
An entry no advisory matches fails too, so the list stays short.

A scanner error, a lockfile missing from the results or a lockfile with no
packages fails the check: an empty scan never passes.

The Edge Functions (supabase/functions) have no lockfile because they import
nothing from outside this repo. The check fails if that changes (an
`npm:`, `jsr:` or URL import, or a deno.json, deno.lock or import map), so a
new dependency there cannot go unscanned.

Usage:
    python tools/check_advisories.py --scanner PATH [--today YYYY-MM-DD]

Exit codes: 0 = nothing blocking, 1 = blocking advisories or problems
(listed), 2 = the scanner or cargo could not run or gave unreadable output.
"""

from __future__ import annotations

import argparse
import datetime
import json
import re
import subprocess
import sys
import tomllib
from pathlib import Path
from typing import NamedTuple

ROOT = Path(__file__).resolve().parent.parent
LOCKFILES = ("Cargo.lock", "package-lock.json", "web/package-lock.json", "requirements.txt")
IGNORES = ROOT / "tools" / "advisory_ignores.toml"
FUNCTIONS = ROOT / "supabase" / "functions"
# CVSS v3/v4 base score bands: below 4.0 is "low".
MEDIUM = 4.0
SCANNER_TIMEOUT_SECONDS = 600
CARGO_TIMEOUT_SECONDS = 300
# osv-scanner exits 0 with no advisory and 1 with some; anything else is an error.
SCANNER_OK = (0, 1)
IGNORE_FIELDS = ("id", "package", "reason", "evidence", "review_by")
REMOTE_IMPORT_RE = re.compile(r"""(?:from|import)\s*\(?\s*["'](npm:|jsr:|https?://)""")
DENO_CONFIGS = ("deno.json", "deno.jsonc", "deno.lock", "import_map.json")


class Advisory(NamedTuple):
    lockfile: str
    package: str
    version: str
    ids: tuple[str, ...]
    severity: float | None
    fixed: bool


class ScanError(Exception):
    """The scanner or cargo could not run, or its output is unreadable."""


def _has_fix(vulns: list[dict], ids: set[str], package: str) -> bool:
    for vuln in vulns:
        if vuln.get("id") not in ids:
            continue
        for affected in vuln.get("affected") or []:
            if (affected.get("package") or {}).get("name") != package:
                continue
            for rng in affected.get("ranges") or []:
                if any("fixed" in event for event in rng.get("events") or []):
                    return True
    return False


def _severity(text: object) -> float | None:
    try:
        return float(text) if text not in (None, "") else None
    except (TypeError, ValueError):
        return None


def read_scan(report: dict, root: Path = ROOT) -> tuple[list[Advisory], list[str]]:
    """The advisories in an osv-scanner JSON report (run with --all-packages),
    and the problems with the scan itself (a lockfile missing or empty)."""
    seen: dict[str, int] = {}
    advisories = []
    for result in report.get("results") or []:
        path = Path((result.get("source") or {}).get("path", ""))
        try:
            name = path.resolve().relative_to(root.resolve()).as_posix()
        except ValueError:
            name = path.as_posix()
        packages = result.get("packages") or []
        seen[name] = seen.get(name, 0) + len(packages)
        for entry in packages:
            pkg = entry.get("package") or {}
            vulns = entry.get("vulnerabilities") or []
            for group in entry.get("groups") or []:
                ids = tuple(sorted(set(group.get("ids") or []) | set(group.get("aliases") or [])))
                advisories.append(
                    Advisory(
                        name,
                        pkg.get("name", "?"),
                        pkg.get("version", "?"),
                        ids,
                        _severity(group.get("max_severity")),
                        _has_fix(vulns, set(ids), pkg.get("name", "")),
                    )
                )
    problems = [f"{lock}: not in the scan results" for lock in LOCKFILES if lock not in seen]
    problems += [f"{lock}: no packages found" for lock in LOCKFILES if seen.get(lock) == 0]
    return advisories, problems


def blocks(advisory: Advisory) -> bool:
    """True unless the advisory is low severity with no fixed version."""
    return advisory.fixed or advisory.severity is None or advisory.severity >= MEDIUM


def check_ignores(entries: list[dict], today: datetime.date) -> list[str]:
    """Problems with the ignore entries themselves."""
    problems = []
    for number, entry in enumerate(entries, start=1):
        label = f"ignore #{number} ({entry.get('id', '?')})"
        missing = [field for field in IGNORE_FIELDS if not entry.get(field)]
        if missing:
            problems.append(f"{label}: missing {', '.join(missing)}")
            continue
        review_by = entry["review_by"]
        if not isinstance(review_by, datetime.date):
            problems.append(f"{label}: review_by must be a date (YYYY-MM-DD)")
        elif review_by < today:
            problems.append(f"{label}: review date {review_by} has passed; check it again and move the date")
    return problems


def reachable(metadata: dict, exclude: set[str]) -> set[str]:
    """Names of the packages the workspace (minus `exclude`) builds, from
    `cargo metadata --filter-platform` output."""
    names = {pkg["id"]: pkg["name"] for pkg in metadata.get("packages") or []}
    nodes = {node["id"]: node for node in (metadata.get("resolve") or {}).get("nodes") or []}
    todo = [member for member in metadata.get("workspace_members") or [] if names.get(member) not in exclude]
    found: set[str] = set()
    while todo:
        current = todo.pop()
        if current in found:
            continue
        found.add(current)
        todo += [dep["pkg"] for dep in (nodes.get(current) or {}).get("deps") or []]
    return {names[pkg] for pkg in found if pkg in names}


def _cargo_metadata(target: str) -> dict:
    command = ["cargo", "metadata", "--format-version", "1", "--locked", "--filter-platform", target]
    try:
        result = subprocess.run(
            command, cwd=ROOT, capture_output=True, text=True, encoding="utf-8", timeout=CARGO_TIMEOUT_SECONDS
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        raise ScanError(f"cargo metadata for {target}: {error}") from error
    if result.returncode != 0:
        raise ScanError(f"cargo metadata for {target} failed: {result.stderr.strip()[-500:]}")
    try:
        return json.loads(result.stdout)
    except json.JSONDecodeError as error:
        raise ScanError(f"cargo metadata for {target}: unreadable output") from error


def check_absent(entry: dict, metadata_for=_cargo_metadata) -> list[str]:
    """Problems when a crate an entry says is not built is built after all."""
    problems = []
    for build in entry.get("absent_from") or []:
        target, exclude = build.get("target"), set(build.get("exclude") or [])
        if not target:
            problems.append(f"{entry['id']}: an absent_from entry has no target")
            continue
        if entry["package"] in reachable(metadata_for(target), exclude):
            but = f" without {', '.join(sorted(exclude))}" if exclude else ""
            problems.append(
                f"{entry['id']}: {entry['package']} is now built for {target}{but}; the ignore no longer holds"
            )
    return problems


def check_functions(folder: Path = FUNCTIONS) -> list[str]:
    """Problems when the Edge Functions start using third-party code."""
    if not folder.is_dir():
        return [f"{folder}: not found"]
    configs = [path for name in DENO_CONFIGS for path in folder.rglob(name)]
    problems = [f"{path.relative_to(folder).as_posix()}: Deno config or lockfile" for path in configs]
    for path in sorted([*folder.rglob("*.ts"), *folder.rglob("*.js")]):
        for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
            if REMOTE_IMPORT_RE.search(line):
                problems.append(f"{path.relative_to(folder).as_posix()}:{number}: third-party import")
    if problems:
        problems.append("supabase/functions: the scan does not cover third-party Edge Function code; cover it first")
    return problems


def run_scanner(scanner: str) -> dict:
    command = [scanner, "scan", "source", "--format", "json", "--all-packages"]
    for lock in LOCKFILES:
        command += ["-L", str(ROOT / lock)]
    try:
        result = subprocess.run(
            command, cwd=ROOT, capture_output=True, text=True, encoding="utf-8", timeout=SCANNER_TIMEOUT_SECONDS
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        raise ScanError(f"osv-scanner: {error}") from error
    if result.returncode not in SCANNER_OK:
        raise ScanError(f"osv-scanner exited {result.returncode}: {result.stderr.strip()[-1000:]}")
    try:
        return json.loads(result.stdout)
    except json.JSONDecodeError as error:
        raise ScanError("osv-scanner: unreadable JSON output") from error


def evaluate(advisories: list[Advisory], entries: list[dict]) -> tuple[list[str], list[str], list[dict]]:
    """Blocking advisories, let-through ones, and the ignore entries used."""
    blocking, passed, used = [], [], []
    for adv in advisories:
        text = f"{adv.lockfile}: {adv.package} {adv.version}: {', '.join(adv.ids)}"
        severity = "unknown" if adv.severity is None else f"{adv.severity:g}"
        text += f" (severity {severity}, {'fix available' if adv.fixed else 'no fix'})"
        entry = next((e for e in entries if e.get("id") in adv.ids and e.get("package") == adv.package), None)
        if entry is not None:
            used.append(entry)
            passed.append(f"{text}: ignored until {entry.get('review_by')} ({entry.get('reason')})")
        elif blocks(adv):
            blocking.append(text)
        else:
            passed.append(f"{text}: low with no fix, let through")
    return blocking, passed, used


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--scanner", required=True, help="path to the osv-scanner binary")
    parser.add_argument("--today", type=datetime.date.fromisoformat, default=datetime.date.today())
    args = parser.parse_args(argv)

    try:
        entries = tomllib.loads(IGNORES.read_text(encoding="utf-8")).get("ignore") or []
        advisories, problems = read_scan(run_scanner(args.scanner))
        blocking, passed, used = evaluate(advisories, entries)
        problems += check_ignores(entries, args.today)
        for entry in used:
            problems += check_absent(entry)
    except (OSError, tomllib.TOMLDecodeError, ScanError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 2
    unused = [e for e in entries if e not in used]
    problems += [f"ignore {e.get('id')} ({e.get('package')}): no advisory matches it; remove it" for e in unused]
    problems += check_functions()

    for line in passed:
        print(f"ok: {line}")
    for line in blocking + problems:
        print(f"FAIL: {line}")
    if blocking or problems:
        return 1
    print(f"no blocking advisory in {', '.join(LOCKFILES)}; Edge Functions import nothing third-party")
    return 0


if __name__ == "__main__":
    sys.exit(main())
