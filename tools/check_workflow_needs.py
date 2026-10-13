"""Check that no workflow job is skipped by accident when a job it needs was.

A job's `if:` without a status function (`success()`, `failure()`,
`always()`, `cancelled()`) is read by GitHub as `success() && ...`, and
`success()` is false when any job it needs, directly or through another job,
was skipped. So a job that follows a job that may be skipped (any job with
its own `if:`) must say how it treats that, with a status function in its
`if:` (for example `!cancelled() && needs.x.result == 'success'`); a job with
no `if:` at all is the same trap. The release workflow's draft job was once
skipped this way.

Reads `.github/workflows/*.yml` and `*.yaml` in the layout this repo uses:
`jobs:` at the top level, each job at two spaces, and its `needs:` (a name
or a `[a, b]` list) and `if:` on one line each at four spaces. Anything else
under a job's `needs:` is reported instead of guessed. Standard library only;
it changes nothing.

Usage:
    python tools/check_workflow_needs.py [--root DIR]

Exit codes: 0 = fine, 1 = problems found (listed), 2 = no workflow files.
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path
from typing import NamedTuple

ROOT = Path(__file__).resolve().parent.parent
JOB_RE = re.compile(r"^  ([A-Za-z0-9_-]+):\s*(?:#.*)?$")
NEEDS_RE = re.compile(r"^    needs:\s*(.*?)\s*(?:#.*)?$")
IF_RE = re.compile(r"^    if:\s*(.+?)\s*$")
STATUS_RE = re.compile(r"\b(success|failure|always|cancelled)\(\)")


class Job(NamedTuple):
    needs: tuple[str, ...]
    condition: str | None


def parse(text: str) -> tuple[dict[str, Job], list[str]]:
    """The jobs of one workflow, and the lines this check cannot read."""
    jobs: dict[str, Job] = {}
    unreadable: list[str] = []
    in_jobs = False
    name: str | None = None
    for number, line in enumerate(text.splitlines(), 1):
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        if not line.startswith(" "):
            in_jobs = line.rstrip() == "jobs:"
            name = None
            continue
        if not in_jobs:
            continue
        job = JOB_RE.match(line)
        if job:
            name = job.group(1)
            jobs[name] = Job((), None)
            continue
        if name is None:
            continue
        needs = NEEDS_RE.match(line)
        if needs:
            value = needs.group(1)
            if re.fullmatch(r"[A-Za-z0-9_-]+", value):
                listed: tuple[str, ...] = (value,)
            elif re.fullmatch(r"\[\s*[A-Za-z0-9_-]+(\s*,\s*[A-Za-z0-9_-]+)*\s*\]", value):
                listed = tuple(n.strip() for n in value[1:-1].split(","))
            else:
                unreadable.append(f"line {number}: needs: {value!r}")
                listed = ()
            jobs[name] = jobs[name]._replace(needs=listed)
            continue
        condition = IF_RE.match(line)
        if condition:
            jobs[name] = jobs[name]._replace(condition=condition.group(1))
    return jobs, unreadable


def ancestors(jobs: dict[str, Job], name: str) -> set[str]:
    seen: set[str] = set()
    stack = list(jobs[name].needs)
    while stack:
        n = stack.pop()
        if n in seen or n not in jobs:
            continue
        seen.add(n)
        stack.extend(jobs[n].needs)
    return seen


def problems(text: str) -> list[str]:
    jobs, out = parse(text)
    for name, job in jobs.items():
        for need in job.needs:
            if need not in jobs:
                out.append(f"{name}: needs unknown job {need!r}")
        skippable = sorted(a for a in ancestors(jobs, name) if jobs[a].condition is not None)
        if not skippable:
            continue
        if job.condition is None or not STATUS_RE.search(job.condition):
            out.append(
                f"{name}: follows {', '.join(skippable)}, which can be skipped, but its `if:` has no "
                "status function, so GitHub skips it too; start it with `!cancelled() &&` and check "
                "the results it needs"
            )
    return out


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--root", type=Path, default=ROOT)
    args = parser.parse_args(argv)
    folder = args.root / ".github" / "workflows"
    files = sorted([*folder.glob("*.yml"), *folder.glob("*.yaml")])
    if not files:
        print(f"no workflow files under {folder}")
        return 2
    found = 0
    for path in files:
        for problem in problems(path.read_text(encoding="utf-8")):
            print(f"{path.relative_to(args.root).as_posix()}: {problem}")
            found += 1
    if found:
        return 1
    print(f"job conditions fine in {len(files)} workflow file(s)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
