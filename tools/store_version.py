"""The Microsoft Store package version for an app version (T-379, D-158).

The Store refuses a package whose first part is 0 and reserves the fourth part
for itself (it must be 0), so the app's `0.2.4` cannot be `0.2.4.0`. The
mapping is `X.Y.Z` -> `(X+1).Y.Z.0` (the Store's major is the app's major
plus one, so it stays monotonic when the app reaches 1.0.0: `2.0.0.0`): the
first part is never 0, the last is always 0, a newer app version is always a newer package version, and two
versions never share one. Each part fits the package format (0 to 65535).
The installer edition keeps its own version.

Usage:
    python tools/store_version.py [VERSION]
        Prints the package version; VERSION defaults to `version` in
        crates/desktop/tauri.conf.json. Exit 1 if it cannot be mapped.
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

CONF = Path(__file__).resolve().parent.parent / "crates" / "desktop" / "tauri.conf.json"
MAX_PART = 65535
PLAIN = re.compile(r"(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)")


class BadVersion(ValueError):
    """The app version has no valid Store package version."""


def store_version(app_version: str) -> str:
    """`X.Y.Z` (plain numbers, no pre-release or build part) as `(X+1).Y.Z.0`."""
    found = PLAIN.fullmatch(app_version)
    if not found:
        raise BadVersion(f"not a plain X.Y.Z version: {app_version!r}")
    major, minor, patch = (int(part) for part in found.groups())
    if major + 1 > MAX_PART or minor > MAX_PART or patch > MAX_PART:
        raise BadVersion(f"a part is over {MAX_PART}: {app_version}")
    return f"{major + 1}.{minor}.{patch}.0"


def app_version() -> str:
    return json.loads(CONF.read_text(encoding="utf-8"))["version"]


def main(argv: list[str]) -> int:
    try:
        print(store_version(argv[1] if len(argv) > 1 else app_version()))
    except BadVersion as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
