"""Read-only check of the local Hearthstone logging setup.

Reports whether log.config enables the Power log, where the game writes its
logs on this machine, and an inventory of the Power logs found (date, size,
game types). It never writes, copies or modifies anything, and never prints
player names.

Usage:
    python tools/check_logs.py [--config PATH] [--logs-dir PATH]

Exit codes: 0 = Power log enabled and all logs readable, 1 = problems found,
2 = log.config or the logs folder could not be located.
"""

from __future__ import annotations

import argparse
import os
import re
import sys
from collections import Counter
from dataclasses import dataclass, field
from pathlib import Path

# Values the Power section needs so the game writes Power.log with full detail.
REQUIRED_POWER = {"LogLevel": "1", "FilePrinting": "true", "Verbose": "true"}

SESSION_RE = re.compile(r"^Hearthstone_(\d{4})_(\d{2})_(\d{2})_(\d{2})_(\d{2})_(\d{2})$")
GAME_TYPE_RE = re.compile(r"GameState\.DebugPrintGame\(\) - GameType=(\w+)")
BUILD_RE = re.compile(r"GameState\.DebugPrintGame\(\) - BuildNumber=(\d+)")

UNINSTALL_KEYS = (
    r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\Hearthstone",
    r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\Hearthstone",
)
FALLBACK_INSTALL_DIRS = (
    Path(r"C:\Program Files (x86)\Hearthstone"),
    Path(r"C:\Program Files\Hearthstone"),
)

BYTES_PER_MB = 1024 * 1024


@dataclass(frozen=True)
class PowerScan:
    game_types: dict[str, int]
    builds: frozenset[str] = field(default_factory=frozenset)


@dataclass(frozen=True)
class PowerLog:
    name: str
    size_bytes: int
    scan: PowerScan | None
    error: str | None = None


@dataclass(frozen=True)
class Session:
    name: str
    started: str | None
    power_logs: tuple[PowerLog, ...]


def parse_log_config(text: str) -> dict[str, dict[str, str]]:
    """Parse the INI-like log.config. Keys before any section are ignored."""
    sections: dict[str, dict[str, str]] = {}
    current: dict[str, str] | None = None
    for raw in text.splitlines():
        line = raw.strip()
        if not line or line.startswith((";", "#")):
            continue
        if line.startswith("[") and line.endswith("]"):
            current = sections.setdefault(line[1:-1].strip(), {})
            continue
        if current is not None and "=" in line:
            key, value = line.split("=", 1)
            current[key.strip()] = value.strip()
    return sections


def power_problems(config: dict[str, dict[str, str]]) -> list[str]:
    """Return what is missing or wrong in the [Power] section; empty if fine."""
    power = config.get("Power")
    if power is None:
        return ["Missing [Power] section"]
    problems = []
    for key, expected in REQUIRED_POWER.items():
        value = power.get(key)
        if value is None:
            problems.append(f"[Power] {key} missing (expected {expected})")
        elif value.lower() != expected:
            problems.append(f"[Power] {key}={value} (expected {expected})")
    return problems


def session_start(folder_name: str) -> str | None:
    match = SESSION_RE.match(folder_name)
    if not match:
        return None
    y, mo, d, h, mi, s = match.groups()
    return f"{y}-{mo}-{d} {h}:{mi}:{s}"


def scan_power_log(path: Path) -> PowerScan:
    """Stream the file (it can be hundreds of MB) and count game starts by type."""
    game_types: Counter[str] = Counter()
    builds: set[str] = set()
    with path.open("r", encoding="utf-8", errors="replace") as handle:
        for line in handle:
            if "DebugPrintGame" not in line:
                continue
            if match := GAME_TYPE_RE.search(line):
                game_types[match.group(1)] += 1
            elif match := BUILD_RE.search(line):
                builds.add(match.group(1))
    return PowerScan(game_types=dict(game_types), builds=frozenset(builds))


def read_power_log(path: Path) -> PowerLog:
    """Scan one log; an unreadable file is reported, never silently skipped."""
    try:
        return PowerLog(path.name, path.stat().st_size, scan_power_log(path))
    except OSError as exc:
        return PowerLog(path.name, 0, None, error=str(exc))


def inventory(logs_dir: Path) -> list[Session]:
    if not logs_dir.is_dir():
        return []
    sessions = []
    for folder in sorted(p for p in logs_dir.iterdir() if p.is_dir()):
        power_logs = tuple(
            read_power_log(f) for f in sorted(folder.glob("Power*.log")) if f.is_file()
        )
        sessions.append(Session(folder.name, session_start(folder.name), power_logs))
    return sessions


def default_config_path() -> Path | None:
    local = os.environ.get("LOCALAPPDATA")
    return Path(local) / "Blizzard" / "Hearthstone" / "log.config" if local else None


def find_install_dir() -> Path | None:
    """Look up the install folder in the Windows uninstall registry (read-only)."""
    try:
        import winreg
    except ImportError:
        winreg = None
    if winreg is not None:
        for key_path in UNINSTALL_KEYS:
            try:
                with winreg.OpenKey(winreg.HKEY_LOCAL_MACHINE, key_path) as key:
                    location, _ = winreg.QueryValueEx(key, "InstallLocation")
            except OSError:
                continue
            if location and Path(location).is_dir():
                return Path(location)
    return next((p for p in FALLBACK_INSTALL_DIRS if p.is_dir()), None)


def format_report(config_path: Path | None, problems: list[str] | None,
                  logs_dir: Path | None, sessions: list[Session]) -> str:
    lines = [f"log.config: {config_path or 'not found'}"]
    if problems is None:
        lines.append("  Power log: unknown (log.config not found)")
    elif problems:
        lines.append("  Power log: NOT fully enabled")
        lines.extend(f"    - {p}" for p in problems)
    else:
        lines.append("  Power log: enabled (LogLevel=1, FilePrinting=true, Verbose=true)")

    lines.append(f"Logs folder: {logs_dir or 'not found'}")
    totals: Counter[str] = Counter()
    for session in sessions:
        lines.append(f"  {session.name} (started {session.started or 'unknown'})")
        if not session.power_logs:
            lines.append("    no Power log")
        for log in session.power_logs:
            if log.scan is None:
                lines.append(f"    {log.name}: unreadable ({log.error})")
                continue
            totals.update(log.scan.game_types)
            types = ", ".join(f"{k} x{v}" for k, v in sorted(log.scan.game_types.items())) or "none"
            builds = ", ".join(sorted(log.scan.builds)) or "unknown"
            lines.append(
                f"    {log.name}: {log.size_bytes / BYTES_PER_MB:.1f} MB, "
                f"game starts: {types}, build: {builds}"
            )
    summary = ", ".join(f"{k} x{v}" for k, v in sorted(totals.items())) or "none"
    lines.append(f"Total game starts by type: {summary}")
    return "\n".join(lines)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--config", type=Path, help="path to log.config")
    parser.add_argument("--logs-dir", type=Path, help="path to the Hearthstone Logs folder")
    args = parser.parse_args(argv)

    config_path = args.config or default_config_path()
    problems = None
    if config_path and config_path.is_file():
        text = config_path.read_text(encoding="utf-8-sig", errors="replace")
        problems = power_problems(parse_log_config(text))
    else:
        config_path = None

    logs_dir = args.logs_dir
    if logs_dir is None:
        install_dir = find_install_dir()
        logs_dir = install_dir / "Logs" if install_dir else None
    if logs_dir is not None and not logs_dir.is_dir():
        logs_dir = None

    sessions = inventory(logs_dir) if logs_dir else []
    print(format_report(config_path, problems, logs_dir, sessions))

    if config_path is None or logs_dir is None:
        return 2
    logs = [log for s in sessions for log in s.power_logs]
    all_readable = all(log.scan is not None for log in logs)
    return 0 if not problems and logs and all_readable else 1


if __name__ == "__main__":
    sys.exit(main())
