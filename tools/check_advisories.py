"""Fail on known security advisories in the pinned dependencies.

Runs osv-scanner (path given with --scanner) on the four lockfiles that pin
what we build and ship: Cargo.lock, package-lock.json, web/package-lock.json
and requirements.txt. An advisory fails the check when it has a fixed
version, when its severity is medium or higher (CVSS 4.0 and up), or when
its severity is unknown; only a low advisory with no fix is reported and
let through. An advisory listed in tools/advisory_ignores.toml passes, but
only while its entry is complete (advisory, package, reason, evidence and a
review date not yet past and at most six months away) and, for a crate,
only while the crate is still missing from every build the entry names
(checked with `cargo metadata`). An entry no advisory matches fails too, so
the list stays short.

The scanner runs with an empty config of our own, so an osv-scanner.toml
next to a lockfile cannot hide an advisory; a tracked one fails the check
anyway. A tracked lockfile that is neither scanned nor listed in UNSCANNED
fails it too. A scanner error or a lockfile missing from the results fails
the check: an empty scan never passes.

The Edge Functions (supabase/functions) have no lockfile because they import
nothing from outside this repo. The check fails if that changes (an
`npm:`, `jsr:` or URL import, a deno.json, deno.lock or import map, or an
`import_map` in supabase/config.toml), so a new dependency there cannot go
unscanned.

Usage:
    python tools/check_advisories.py --scanner PATH [--today YYYY-MM-DD]

Exit codes: 0 = nothing blocking, 1 = blocking advisories or problems
(listed), 2 = the scanner, git or cargo could not run or gave unreadable
output.
"""

from __future__ import annotations

import argparse
import datetime
import functools
import json
import re
import subprocess
import sys
import tempfile
import tomllib
from pathlib import Path
from typing import NamedTuple

ROOT = Path(__file__).resolve().parent.parent
LOCKFILES = ("Cargo.lock", "package-lock.json", "web/package-lock.json", "requirements.txt")
# Tracked lockfiles left out on purpose, with why.
UNSCANNED = {
    "spikes/overlay-tauri/Cargo.lock": "throwaway overlay test (T-005); never built in CI or shipped",
}
LOCKFILE_RE = re.compile(
    r"(?:^|/)(?:Cargo\.lock|package-lock\.json|npm-shrinkwrap\.json|yarn\.lock|pnpm-lock\.yaml|bun\.lockb?"
    r"|deno\.lock|requirements[^/]*\.txt|poetry\.lock|uv\.lock|Pipfile\.lock|pdm\.lock|go\.sum|Gemfile\.lock"
    r"|composer\.lock)$"
)
SCANNER_CONFIG_RE = re.compile(r"(?:^|/)\.?osv-scanner\.toml$")
DENO_CONFIG_RE = re.compile(r"(?:^|/)(?:deno\.jsonc?|import_map\.json)$")
IGNORES = ROOT / "tools" / "advisory_ignores.toml"
FUNCTIONS = ROOT / "supabase" / "functions"
SUPABASE_CONFIG = ROOT / "supabase" / "config.toml"
# CVSS v3/v4 base score bands: below 4.0 is "low".
MEDIUM = 4.0
# An ignore is looked at again at least this often.
MAX_REVIEW_DAYS = 183
SCANNER_TIMEOUT_SECONDS = 600
CARGO_TIMEOUT_SECONDS = 300
GIT_TIMEOUT_SECONDS = 60
# osv-scanner exits 0 with no advisory and 1 with some; anything else is an error.
SCANNER_OK = (0, 1)
IGNORE_FIELDS = ("id", "package", "reason", "evidence", "review_by")
IGNORE_KEYS = frozenset({*IGNORE_FIELDS, "absent_from"})
BUILD_KEYS = frozenset({"target", "exclude"})
# Matched over the whole file, so an import split across lines is caught too.
REMOTE_IMPORT_RE = re.compile(r"""\b(?:from|import)\s*\(?\s*["'`](npm:|jsr:|https?://)""", re.I)
# Any dynamic import except of one relative path written out whole: a computed
# or concatenated name could be a URL. Strict on purpose; the functions use
# none today. Like the rules above it is read on the file as it is and without
# comments. These are a tripwire for an honest new dependency, not a parser:
# code built to hide one (eval, new Function) is for review to catch.
DYNAMIC_IMPORT_RE = re.compile(r"""\b(?:import|require)\s*\((?!\s*(["'])\.{1,2}/[^"'`\n]*\1\s*[,)])""")
# The specifier of an import or export statement (one that starts a line or
# follows a ";" or "}", so a "from" in prose or a comment is not one), and of
# import() and require().
# Each must be a relative path inside the functions folder (a JSON file may be
# outside: data imports nothing) or a node: built-in; a bare name could resolve
# through a package.json, so it fails like npm:.
STATEMENT_SPECIFIER_RE = re.compile(r"""(?:^|[;}])[ \t]*(?:import|export)\b(?:[^;]*?\bfrom)?\s*(["'`])([^"'`\n]*)\1""", re.M)
CALL_SPECIFIER_RE = re.compile(r"""\b(?:import|require)\s*\(\s*(["'`])([^"'`\n]*)\1""")
NAME_SEPARATORS_RE = re.compile(r"[-_.]+")
# A package specifier has no other use in this code, so one anywhere counts.
PACKAGE_SPECIFIER_RE = re.compile(r"""["'`](?:npm|jsr):""", re.I)
# Block comments, and line comments not right after a ":" (so "https://" stays).
COMMENT_RE = re.compile(r"/\*[\s\S]*?\*/|(?<!:)//[^\n]*")
SOURCE_SUFFIXES = (".ts", ".mts", ".cts", ".tsx", ".js", ".mjs", ".cjs", ".jsx")
DEPENDENCY_CONFIGS = ("deno.json", "deno.jsonc", "deno.lock", "import_map.json", "package.json", "package-lock.json")


class Advisory(NamedTuple):
    lockfile: str
    package: str
    version: str
    ids: tuple[str, ...]
    severity: float | None
    fixed: bool


class ScanError(Exception):
    """The scanner, git or cargo could not run, or its output is unreadable."""


def _run(command: list[str], what: str, timeout: int, ok: tuple[int, ...] = (0,)) -> str:
    try:
        result = subprocess.run(command, cwd=ROOT, capture_output=True, text=True, encoding="utf-8", timeout=timeout)
    except (OSError, subprocess.TimeoutExpired) as error:
        raise ScanError(f"{what}: {error}") from error
    if result.returncode not in ok:
        raise ScanError(f"{what} exited {result.returncode}: {result.stderr.strip()[-1000:]}")
    return result.stdout


def _json(text: str, what: str) -> dict:
    try:
        return json.loads(text)
    except json.JSONDecodeError as error:
        raise ScanError(f"{what}: unreadable output") from error


def _same_name(a: str, b: str) -> bool:
    """Package names equal the way PyPI (and crates.io) compare them: case,
    "-", "_" and "." do not matter. Stricter ecosystems lose nothing by it."""
    return NAME_SEPARATORS_RE.sub("-", a.lower()) == NAME_SEPARATORS_RE.sub("-", b.lower())


def _has_fix(vulns: list[dict], ids: set[str], package: str) -> bool:
    for vuln in vulns:
        if vuln.get("id") not in ids:
            continue
        for affected in vuln.get("affected") or []:
            if not _same_name((affected.get("package") or {}).get("name") or "", package):
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


def check_tracked(tracked: list[str]) -> list[str]:
    """Problems in the list of tracked files: a lockfile nobody scans, a
    scanner config that could hide advisories, or a Deno config anywhere
    (Deno also reads one from a parent folder of the functions)."""
    problems = []
    for path in tracked:
        if SCANNER_CONFIG_RE.search(path):
            problems.append(f"{path}: osv-scanner config in the repo; ignores go in tools/advisory_ignores.toml")
        elif DENO_CONFIG_RE.search(path):
            problems.append(f"{path}: Deno config in the repo; the scan does not cover what it maps")
        elif LOCKFILE_RE.search(path) and path not in LOCKFILES and path not in UNSCANNED:
            problems.append(f"{path}: lockfile not scanned; add it to LOCKFILES (or UNSCANNED, with why)")
    return problems


def tracked_files() -> list[str]:
    # -z: names as they are, not quoted (git quotes non-ASCII names otherwise).
    return [name for name in _run(["git", "ls-files", "-z"], "git ls-files", GIT_TIMEOUT_SECONDS).split("\0") if name]


def check_ignores(entries: list[dict], today: datetime.date) -> list[str]:
    """Problems with the ignore entries themselves."""
    problems = []
    latest = today + datetime.timedelta(days=MAX_REVIEW_DAYS)
    for number, entry in enumerate(entries, start=1):
        label = f"ignore #{number} ({entry.get('id', '?')})"
        unknown = sorted(set(entry) - IGNORE_KEYS)
        if unknown:
            problems.append(f"{label}: unknown key {', '.join(unknown)}")
        missing = [field for field in IGNORE_FIELDS if not entry.get(field)]
        if missing:
            problems.append(f"{label}: missing {', '.join(missing)}")
            continue
        review_by = entry["review_by"]
        # A TOML date-time is a datetime, which is also a date; only a plain date is meant.
        if type(review_by) is not datetime.date:
            problems.append(f"{label}: review_by must be a date (YYYY-MM-DD)")
        elif review_by < today:
            problems.append(f"{label}: review date {review_by} has passed; check it again and move the date")
        elif review_by > latest:
            problems.append(f"{label}: review date {review_by} is more than {MAX_REVIEW_DAYS} days away")
        for build in entry.get("absent_from") or []:
            exclude = build.get("exclude", []) if isinstance(build, dict) else None
            if (
                not isinstance(build, dict)
                or set(build) - BUILD_KEYS
                or not isinstance(build.get("target"), str)
                or not build["target"]
                or not isinstance(exclude, list)
                or not all(isinstance(name, str) for name in exclude)
            ):
                problems.append(f"{label}: each absent_from item is {{ target = \"…\", exclude = [\"…\"] }}")
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


@functools.cache
def _cargo_metadata(target: str) -> dict:
    command = ["cargo", "metadata", "--format-version", "1", "--locked", "--filter-platform", target]
    return _json(_run(command, f"cargo metadata for {target}", CARGO_TIMEOUT_SECONDS), f"cargo metadata for {target}")


def check_absent(entry: dict, metadata_for=_cargo_metadata) -> list[str]:
    """Problems when a crate an entry says is not built is built after all."""
    problems = []
    for build in entry.get("absent_from") or []:
        if not isinstance(build, dict) or not build.get("target") or not isinstance(build.get("exclude", []), list):
            continue  # reported by check_ignores
        target, exclude = build["target"], set(build.get("exclude") or [])
        metadata = metadata_for(target)
        member_ids = set(metadata.get("workspace_members") or [])
        members = {pkg["name"] for pkg in metadata.get("packages") or [] if pkg["id"] in member_ids}
        if exclude - members:
            problems.append(f"{entry['id']}: exclude names no workspace crate: {', '.join(sorted(exclude - members))}")
        elif members and members <= exclude:
            problems.append(f"{entry['id']}: exclude leaves no workspace crate for {target}")
        elif entry["package"] in reachable(metadata, exclude):
            but = f" without {', '.join(sorted(exclude))}" if exclude else ""
            problems.append(
                f"{entry['id']}: {entry['package']} is now built for {target}{but}; the ignore no longer holds"
            )
    return problems


def _line(text: str, at: int) -> int:
    return text.count("\n", 0, at) + 1


def _without_comments(text: str) -> str:
    """The text with comments blanked out, line breaks kept so line numbers hold."""
    return COMMENT_RE.sub(lambda m: "\n" * m.group(0).count("\n") or " ", text)


def _function_configs(config: Path, folder: Path) -> list[str]:
    """An `import_map`, or an `entrypoint` outside the folder, on any function in supabase/config.toml."""
    if not config.is_file():
        return []
    try:
        functions = tomllib.loads(config.read_text(encoding="utf-8")).get("functions") or {}
    except tomllib.TOMLDecodeError as error:
        return [f"{config.name}: unreadable ({error})"]
    problems = []
    for name, settings in functions.items():
        if not isinstance(settings, dict):
            continue
        if "import_map" in settings:
            problems.append(f"{config.name}: function {name} has an import_map")
        entrypoint = settings.get("entrypoint")
        if entrypoint is not None and not (
            isinstance(entrypoint, str) and (config.parent / entrypoint).resolve().is_relative_to(folder.resolve())
        ):
            problems.append(f"{config.name}: function {name} has an entrypoint outside the functions folder")
    return problems


def _specifier_problem(path: Path, folder: Path, specifier: str) -> str | None:
    if specifier.startswith(("./", "../")):
        target = (path.parent / specifier).resolve()
        if target.is_relative_to(folder.resolve()) or target.suffix.lower() == ".json":
            return None
        return "import from outside the functions folder"
    return None if specifier.startswith("node:") else "third-party import"


def _file_problems(path: Path, folder: Path) -> list[str]:
    text = path.read_text(encoding="utf-8-sig", errors="replace")
    # Both the text as it is and without comments: a comment can sit inside
    # an import, and a "/*" inside a string must not hide one either.
    versions = (text, _without_comments(text))
    third_party = {_line(t, m.start(1)) for t in versions for m in REMOTE_IMPORT_RE.finditer(t)}
    third_party |= {_line(text, m.start()) for m in PACKAGE_SPECIFIER_RE.finditer(text)}
    third_party |= {_line(t, m.start()) for t in versions for m in DYNAMIC_IMPORT_RE.finditer(t)}
    found = dict.fromkeys(third_party, "third-party import")
    for t in versions:
        for m in (*STATEMENT_SPECIFIER_RE.finditer(t), *CALL_SPECIFIER_RE.finditer(t)):
            problem = _specifier_problem(path, folder, m.group(2))
            if problem:
                found.setdefault(_line(t, m.start(2)), problem)
    where = path.relative_to(folder).as_posix()
    return [f"{where}:{n}: {found[n]}" for n in sorted(found)]


def check_functions(folder: Path = FUNCTIONS, config: Path = SUPABASE_CONFIG) -> list[str]:
    """Problems when the Edge Functions start using third-party code."""
    if not folder.is_dir():
        return [f"{folder}: not found"]
    configs = sorted(path for name in DEPENDENCY_CONFIGS for path in folder.rglob(name))
    problems = [f"{path.relative_to(folder).as_posix()}: dependency config or lockfile" for path in configs]
    problems += _function_configs(config, folder)
    for path in sorted(p for p in folder.rglob("*") if p.suffix.lower() in SOURCE_SUFFIXES):
        problems += _file_problems(path, folder)
    if problems:
        problems.append("supabase/functions: the scan does not cover third-party Edge Function code; cover it first")
    return problems


def scanner_command(scanner: str, config: Path) -> list[str]:
    # --config replaces the osv-scanner.toml the scanner would read next to each lockfile.
    command = [scanner, "scan", "source", "--format", "json", "--all-packages", "--config", str(config)]
    for lock in LOCKFILES:
        command += ["-L", str(ROOT / lock)]
    return command


def run_scanner(scanner: str) -> dict:
    with tempfile.TemporaryDirectory() as folder:
        config = Path(folder) / "osv-scanner.toml"
        config.write_text("", encoding="utf-8")
        output = _run(scanner_command(scanner, config), "osv-scanner", SCANNER_TIMEOUT_SECONDS, SCANNER_OK)
    return _json(output, "osv-scanner")


def evaluate(advisories: list[Advisory], entries: list[dict]) -> tuple[list[str], list[str], list[dict], list[str]]:
    """Blocking advisories, let-through ones, the ignore entries used, and
    problems with how they were used."""
    blocking, passed, used, problems = [], [], [], []
    for adv in advisories:
        text = f"{adv.lockfile}: {adv.package} {adv.version}: {', '.join(adv.ids)}"
        severity = "unknown" if adv.severity is None else f"{adv.severity:g}"
        text += f" (severity {severity}, {'fix available' if adv.fixed else 'no fix'})"
        entry = next((e for e in entries if e.get("id") in adv.ids and e.get("package") == adv.package), None)
        if entry is None and blocks(adv):
            blocking.append(text)
            continue
        if entry is None:
            passed.append(f"{text}: low with no fix, let through")
            continue
        if not any(entry is e for e in used):
            used.append(entry)
        if adv.lockfile == "Cargo.lock" and not entry.get("absent_from"):
            problems.append(f"{text}: a crate's ignore needs absent_from, the builds it stays out of")
        passed.append(f"{text}: ignored until {entry.get('review_by')} ({entry.get('reason')})")
    return blocking, passed, used, problems


def load_ignores(path: Path = IGNORES) -> list[dict]:
    entries = tomllib.loads(path.read_text(encoding="utf-8")).get("ignore") or []
    if not isinstance(entries, list) or not all(isinstance(e, dict) for e in entries):
        raise ScanError(f"{path.name}: expected [[ignore]] tables")
    return entries


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--scanner", required=True, help="path to the osv-scanner binary")
    parser.add_argument("--today", type=datetime.date.fromisoformat, default=datetime.date.today())
    args = parser.parse_args(argv)

    try:
        entries = load_ignores()
        problems = check_tracked(tracked_files())
        advisories, scan_problems = read_scan(run_scanner(args.scanner))
        blocking, passed, used, use_problems = evaluate(advisories, entries)
        problems += scan_problems + use_problems + check_ignores(entries, args.today)
        for entry in used:
            problems += check_absent(entry)
    except (OSError, tomllib.TOMLDecodeError, ScanError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 2
    unused = [e for e in entries if not any(e is u for u in used)]
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
