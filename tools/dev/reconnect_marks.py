"""Dev tool, never shipped: the record of reconnect dev tool uses (D-043).

`tools/dev/reconnect.py` appends one line per use to
%APPDATA%\\TavernLedger\\dev-reconnects.jsonl: {"utc": "2026-10-10T19:15:03Z"},
nothing else. This module writes and reads that file and tells whether a
saved Power.log holds a game played while the tool was used, the same way the
tracker does (crates/tracker/src/game_clock.rs and dev_reconnect.rs): each
game runs from its CREATE_GAME line to its last line, on the log's own clock,
plus a 30-second margin.

Fixture tools must call `refuse_marked_log` before using a log (D-022: a game
played with the tool never becomes a fixture). From the command line:

    python tools/dev/reconnect_marks.py LOG [LOG ...] [--session NAME]

exits 1 if any log has a marked game, or a game whose time cannot be told
while uses are recorded; 0 otherwise. It prints counts and file names only.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import sys
from dataclasses import dataclass
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import Callable, Iterable

FILE_NAME = "dev-reconnects.jsonl"
MARGIN = timedelta(seconds=30)
MAX_FILE_BYTES = 4 << 20
CREATE_GAME = "GameState.DebugPrintPower() - CREATE_GAME"
SESSION_RE = re.compile(r"^Hearthstone_(\d{4})_(\d{2})_(\d{2})_(\d{2})_(\d{2})_(\d{2})$")
LINE_TIME_RE = re.compile(r"^[DWE] (\d{2}):(\d{2}):(\d{2})(?:\.\d+)? ")
UTC_RE = re.compile(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$")
HALF_DAY = 12 * 3600


def default_data_dir() -> Path:
    """%APPDATA%\\TavernLedger, as the tracker (D-015); .local elsewhere."""
    appdata = os.environ.get("APPDATA")
    return Path(appdata) / "TavernLedger" if appdata else Path(".local")


def format_utc(moment: datetime) -> str:
    return moment.astimezone(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def parse_utc(text: str) -> datetime | None:
    if not isinstance(text, str) or not UTC_RE.match(text):
        return None
    try:
        return datetime.strptime(text, "%Y-%m-%dT%H:%M:%SZ").replace(tzinfo=timezone.utc)
    except ValueError:
        return None


def append_use(data_dir: Path, moment: datetime) -> Path:
    """Appends one use (UTC time only) and returns the file's path."""
    data_dir.mkdir(parents=True, exist_ok=True)
    path = data_dir / FILE_NAME
    line = json.dumps({"utc": format_utc(moment)}) + "\n"
    # One write per line, so the tracker never reads half of one.
    with path.open("a", encoding="utf-8", newline="\n") as out:
        out.write(line)
        out.flush()
        os.fsync(out.fileno())
    return path


@dataclass
class Uses:
    times: list[datetime]
    warning: str | None = None


def read_uses(data_dir: Path) -> Uses:
    """Every recorded use. No file: none, no warning (the tool was never
    used). A file that cannot be read, or bad lines: what can be read, and
    a warning."""
    path = data_dir / FILE_NAME
    try:
        if path.stat().st_size > MAX_FILE_BYTES:
            return Uses([], f"{FILE_NAME} is too large to be the tool's file; not read.")
        raw = path.read_bytes()
    except FileNotFoundError:
        return Uses([])
    except OSError as e:
        return Uses([], f"Could not read {FILE_NAME} ({type(e).__name__}).")
    times, bad = [], 0
    for line in raw.decode("utf-8", errors="replace").splitlines():
        if not line.strip():
            continue
        try:
            value = json.loads(line)
        except ValueError:
            value = None
        moment = parse_utc(value.get("utc")) if isinstance(value, dict) else None
        if moment is None:
            bad += 1
        else:
            times.append(moment)
    times.sort()
    warning = f"{bad} line(s) of {FILE_NAME} could not be read and were skipped." if bad else None
    return Uses(times, warning)


@dataclass
class GameSpan:
    index: int
    start: datetime | None  # aware UTC; None when the log gives no time
    end: datetime | None


def session_start(name: str) -> datetime | None:
    """Local (naive) start time from a session folder name."""
    m = SESSION_RE.match(name)
    if not m:
        return None
    try:
        return datetime(*(int(g) for g in m.groups()))
    except ValueError:
        return None


def local_to_utc(naive: datetime) -> datetime:
    """The system's time zone rules for that date (daylight saving included)."""
    return naive.astimezone(timezone.utc)


def game_spans(
    lines: Iterable[str],
    session: str,
    to_utc: Callable[[datetime], datetime] | None = None,
) -> list[GameSpan]:
    """When each game of a session's log was played; games numbered like the
    parser (the n-th CREATE_GAME is game n)."""
    to_utc = to_utc or local_to_utc
    start = session_start(session)
    spans: list[GameSpan] = []
    local: dict[int, list[datetime]] = {}
    day = 0
    last = start.hour * 3600 + start.minute * 60 + start.second if start else 0
    for line in lines:
        is_new = CREATE_GAME in line
        if is_new:
            spans.append(GameSpan(len(spans) + 1, None, None))
        m = LINE_TIME_RE.match(line)
        if start is None or m is None:
            continue
        h, mi, s = (int(g) for g in m.groups())
        if h > 23 or mi > 59 or s > 59:
            continue
        tod = h * 3600 + mi * 60 + s
        if tod + HALF_DAY < last:
            day += 1
        last = tod
        moment = datetime(start.year, start.month, start.day) + timedelta(days=day, seconds=tod)
        if is_new:
            local[len(spans)] = [moment, moment]
        elif spans and len(spans) in local:
            local[len(spans)][1] = moment
    for span in spans:
        if span.index in local:
            first, last_seen = local[span.index]
            span.start, span.end = to_utc(first), to_utc(last_seen)
    return spans


def is_marked(span: GameSpan, uses: list[datetime]) -> bool:
    if span.start is None or span.end is None:
        return False
    return any(span.start - MARGIN <= t <= span.end + MARGIN for t in uses)


class MarkedLogError(Exception):
    """A log that must not become a fixture."""


def refuse_marked_log(
    log: Path,
    session: str | None = None,
    data_dir: Path | None = None,
    to_utc: Callable[[datetime], datetime] | None = None,
) -> list[str]:
    """Raises MarkedLogError if `log` holds a game played with the reconnect
    dev tool, or a game whose time cannot be told while uses are recorded.
    `session` is the session folder name (default: the log's folder). Returns
    warnings about the uses file (empty when it was read whole or is missing)."""
    uses = read_uses(data_dir or default_data_dir())
    warnings = [uses.warning] if uses.warning else []
    if not uses.times:
        return warnings
    name = session or log.parent.name
    with log.open(encoding="utf-8", errors="replace") as f:
        spans = game_spans(f, name, to_utc)
    marked = [s.index for s in spans if is_marked(s, uses.times)]
    if marked:
        raise MarkedLogError(
            f"{log.name}: game(s) {marked} were played with the reconnect dev tool (D-022); not a fixture.")
    unknown = [s.index for s in spans if s.start is None]
    if unknown:
        raise MarkedLogError(
            f"{log.name}: the time of game(s) {unknown} cannot be told (pass --session with the "
            "log's Hearthstone_<date> folder name), and reconnect dev tool uses are recorded.")
    return warnings


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("logs", nargs="+", type=Path)
    parser.add_argument("--session", help="the logs' Hearthstone_<date> folder name")
    parser.add_argument("--data-dir", type=Path, help="default: %%APPDATA%%\\TavernLedger")
    args = parser.parse_args(argv)
    refused = 0
    for log in args.logs:
        if not log.is_file():
            print(f"Not found: {log}", file=sys.stderr)
            return 2
        try:
            for warning in refuse_marked_log(log, args.session, args.data_dir):
                print(f"Warning: {warning}", file=sys.stderr)
            print(f"{log.name}: no game played with the reconnect dev tool")
        except MarkedLogError as e:
            print(e, file=sys.stderr)
            refused += 1
    return 1 if refused else 0


if __name__ == "__main__":
    sys.exit(main())
