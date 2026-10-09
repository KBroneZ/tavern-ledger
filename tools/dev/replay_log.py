"""Dev tool, never shipped: replay a saved Power.log into a temporary folder.

Copies the log line by line, at a chosen speed, into
<dest>/Logs/Hearthstone_<date>/Power.log, so `tavern-watch --logs-dir` and the
desktop app can be tested without playing (T-D02). It only reads the source
and only writes under the destination, never in the game's own folder. It
prints counts and paths, never log content or player names.

Usage:
    python tools/dev/replay_log.py SOURCE.log [--dest-dir DIR]
        [--lines-per-second N] [--session-name NAME]

--lines-per-second 0 (the default) copies as fast as possible. A real game
writes a few hundred lines per second in busy moments; 500 is a fair
"game speed" for a live test.
"""

from __future__ import annotations

import argparse
import sys
import tempfile
import time
from datetime import datetime
from pathlib import Path
from typing import Callable

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import check_logs  # noqa: E402

PROGRESS_EVERY = 50_000
BATCHES_PER_SECOND = 20
UNPACED_BATCH = 10_000


def session_folder_name(now: datetime) -> str:
    return now.strftime("Hearthstone_%Y_%m_%d_%H_%M_%S")


def inside_game_folder(dest: Path) -> bool:
    """True when dest is the installed game's folder or below it."""
    install = check_logs.find_install_dir()
    if install is None:
        return False
    return install.resolve() in (dest.resolve(), *dest.resolve().parents)


def replay(
    source: Path,
    logs_dir: Path,
    session_name: str,
    lines_per_second: float = 0,
    *,
    sleep: Callable[[float], None] = time.sleep,
    clock: Callable[[], float] = time.monotonic,
    progress: Callable[[int], None] | None = None,
) -> int:
    """Copy source into logs_dir/session_name/Power.log. Returns the lines copied.

    Lines are written whole and flushed in small batches, the way the game
    flushes, so a follower never sees half a line except by chance.
    """
    session = logs_dir / session_name
    session.mkdir(parents=True, exist_ok=True)
    target = session / "Power.log"
    paced = lines_per_second > 0
    batch = max(1, int(lines_per_second / BATCHES_PER_SECOND)) if paced else UNPACED_BATCH
    copied = 0
    pending = 0
    start = clock()
    with source.open("rb") as src, target.open("ab") as out:
        for line in src:
            out.write(line if line.endswith(b"\n") else line + b"\n")
            copied += 1
            pending += 1
            if pending < batch:
                continue
            out.flush()
            pending = 0
            if paced:
                delay = start + copied / lines_per_second - clock()
                if delay > 0:
                    sleep(delay)
            if progress and copied % PROGRESS_EVERY < batch:
                progress(copied)
        out.flush()
        if paced and pending:
            delay = start + copied / lines_per_second - clock()
            if delay > 0:
                sleep(delay)
    return copied


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("source", type=Path, help="a saved Power.log or Power_old.log")
    parser.add_argument("--dest-dir", type=Path, help="default: a new temporary folder")
    parser.add_argument("--lines-per-second", type=float, default=0)
    parser.add_argument("--session-name", help="default: Hearthstone_<now>")
    args = parser.parse_args(argv)

    if not args.source.is_file():
        print(f"Source not found: {args.source}", file=sys.stderr)
        return 2
    if args.lines_per_second < 0:
        print("--lines-per-second cannot be negative", file=sys.stderr)
        return 2
    dest = args.dest_dir or Path(tempfile.mkdtemp(prefix="tavern-replay-"))
    if inside_game_folder(dest):
        print("Refusing to write inside the game's own folder.", file=sys.stderr)
        return 2
    logs_dir = dest / "Logs"
    name = args.session_name or session_folder_name(datetime.now())
    print(f"Logs folder: {logs_dir}")
    print(f"Follow it with: tavern-watch --logs-dir {logs_dir} --data-dir {dest / 'data'}")
    try:
        copied = replay(
            args.source,
            logs_dir,
            name,
            args.lines_per_second,
            progress=lambda n: print(f"  {n} lines", flush=True),
        )
    except KeyboardInterrupt:
        print("Interrupted.")
        return 1
    print(f"Done: {copied} lines in {logs_dir / name / 'Power.log'}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
