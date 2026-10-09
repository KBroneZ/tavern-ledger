"""Compares tools/parse_bg.py with the Rust port (bg-parse) on a real log.

Read-only. Prints, per game, both statuses and the paths of the fields that
differ. Never prints values from the log, so no player data leaves it.

    cargo build --release -p bg-parser
    python tools/compare_parsers.py PATH_TO_POWER_LOG [...]

Exit code 0 when every game matches.
"""

from __future__ import annotations

import json
import logging
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))

import parse_bg  # noqa: E402

RUST = ROOT / "target" / "release" / ("bg-parse.exe" if sys.platform == "win32" else "bg-parse")
MAX_PATHS = 10


def diff_paths(a, b, path="") -> list[str]:
    if isinstance(a, dict) and isinstance(b, dict):
        out = []
        for key in sorted(set(a) | set(b)):
            out += diff_paths(a.get(key), b.get(key), f"{path}.{key}")
        return out
    if isinstance(a, list) and isinstance(b, list):
        if len(a) != len(b):
            return [f"{path} (length {len(a)} vs {len(b)})"]
        out = []
        for i, (x, y) in enumerate(zip(a, b)):
            out += diff_paths(x, y, f"{path}[{i}]")
        return out
    return [] if a == b else [path or "."]


def compare(path: Path) -> bool:
    logging.getLogger().setLevel(logging.ERROR)
    python = json.loads(json.dumps([parse_bg.to_dict(r) for r in parse_bg.parse_file(path)]))
    proc = subprocess.run([str(RUST), str(path), "--json"], capture_output=True, timeout=1800)
    rust = json.loads(proc.stdout)["games"]
    print(f"{path.parent.name}/{path.name}: {len(python)} games (Python), {len(rust)} (Rust)")
    ok = len(python) == len(rust)
    for py, rs in zip(python, rust):
        paths = diff_paths(py, rs)
        ok = ok and not paths
        mark = "same" if not paths else f"{len(paths)} differences"
        print(f"  game {py['index']}: {py['status']} / {rs['status']} - {mark}")
        for p in paths[:MAX_PATHS]:
            print(f"    {p}")
    return ok


def main(argv: list[str]) -> int:
    results = [compare(Path(p)) for p in argv]
    return 0 if results and all(results) else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
