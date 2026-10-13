"""Check that every GitHub Action a workflow uses is pinned by full commit SHA.

A tag or branch can be moved to other code after we reviewed it; a full
40-character commit SHA cannot. Each `uses:` in `.github/workflows/*.yml` and
`*.yaml` must be `owner/repo@<40 lowercase hex> # <version>` (the comment says
which release the SHA is, so a human can review a bump). Local actions
(`./path`) are in this repo and need no pin; `docker://` images need a
`@sha256:<digest>`. Standard library only; it changes nothing.

Usage:
    python tools/check_action_pins.py [--root DIR]

Exit codes: 0 = everything pinned, 1 = unpinned actions found (listed),
2 = no workflow files found.
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path
from typing import NamedTuple

ROOT = Path(__file__).resolve().parent.parent

# `uses:` as a YAML key, alone or after the `- ` of a step. A line inside a
# `run: |` block does not start this way, so shell text is not read as a key.
USES_RE = re.compile(r"""^\s*(?:-\s+)?uses:\s*(?:"([^"]*)"|'([^']*)'|([^\s#]+))\s*(?:#\s*(.*?))?\s*$""")
ACTION_SHA_RE = re.compile(r"^[^@\s]+@[0-9a-f]{40}$")
DOCKER_DIGEST_RE = re.compile(r"^docker://[^@\s]+@sha256:[0-9a-f]{64}$")
VERSION_COMMENT_RE = re.compile(r"v?\d")


class Problem(NamedTuple):
    path: str
    line: int
    action: str
    reason: str


def _reason(action: str, comment: str | None) -> str | None:
    """Why `action` is not acceptably pinned, or None when it is."""
    if action.startswith("./"):
        return None
    if action.startswith("docker://"):
        if not DOCKER_DIGEST_RE.match(action):
            return "image is not pinned by @sha256:<digest>"
    elif not ACTION_SHA_RE.match(action):
        return "not pinned by a full 40-character lowercase commit SHA"
    if comment is None or not VERSION_COMMENT_RE.match(comment):
        return "pinned, but the version it stands for is not in a trailing comment (# v1.2.3)"
    return None


def find_unpinned(path: str, text: str) -> list[Problem]:
    """Every `uses:` in the workflow text that is not pinned, in file order."""
    found = []
    for number, line in enumerate(text.splitlines(), start=1):
        match = USES_RE.match(line)
        if not match:
            continue
        double, single, bare, comment = match.groups()
        action = next(value for value in (double, single, bare) if value is not None)
        reason = _reason(action, comment)
        if reason:
            found.append(Problem(path, number, action, reason))
    return found


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--root", type=Path, default=ROOT, help="repository root (default: this one)")
    args = parser.parse_args(argv)

    workflows = args.root / ".github" / "workflows"
    files = sorted([*workflows.glob("*.yml"), *workflows.glob("*.yaml")])
    if not files:
        print(f"no workflow files in {workflows}", file=sys.stderr)
        return 2

    problems = []
    for file in files:
        relative = file.relative_to(args.root).as_posix()
        problems += find_unpinned(relative, file.read_text(encoding="utf-8"))
    for p in problems:
        print(f"{p.path}:{p.line}: {p.action}: {p.reason}")
    if problems:
        print(f"{len(problems)} unpinned action(s) in {len(files)} workflow file(s)")
        return 1
    print(f"all actions pinned in {len(files)} workflow file(s)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
