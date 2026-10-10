"""Turns one game of a real Power.log into a small, scrubbed parser fixture.

    python tools/make_fixture.py RAW_LOG --game N --out FIXTURE.log \
        [--parser target/release/bg-parse]

Allow-list first: only the GameState lines the parser reads are kept: the
packets in KEEP_OPCODES, ATTACK blocks, the player's top-level action blocks
(PLAY, MOVE_MINION, DECK_ACTION) with their BLOCK_END, entities of the card
types in KEEP_CARDTYPES, the tags in KEEP_TAGS, and the numbers-only
SendOption lines and SendChoices headers (the actions the client sent, T-205).
Everything else is dropped, including every PowerTaskList, Options and
chosen-entity line. Timestamps count from the game's first line (00:00:00),
so durations stay and the time of day goes (D-046). A bracketed entity ("[entityName=... id=N ...]") becomes its id N,
so no card text or name in brackets is kept; hero names are then missing from
the fixture's report (card_names), which the raw game would have. Every player name (BattleTags, the plain names opponents get during
combat, the AI slot's name) becomes Player1 (the local player) or OpponentN
in first-seen order, and the account ids become hi=1 lo=N (hi=0 lo=0 stays).

Then a deny-list check on the result: a "#" followed by digits, any name or
account id read from the raw game, a GameAccountId that is not a
placeholder, a number of 9 or more digits, or a line outside the allow-list
makes the script refuse to write anything. With --parser (the Rust
`bg-parse` binary, or any command that prints its --json output), the raw
game and the fixture must give the same report, or nothing is written; that
report is saved next to the fixture (FIXTURE.json) for the parser tests.

A log with a game played with the reconnect dev tool (D-022, D-043, see
tools/dev/reconnect_marks.py) is refused before anything else; pass --session
with the Hearthstone_<date> folder name when the log was copied out of it.

Never prints a log line, a name or an id: only line numbers and kinds.
Raw logs belong in .local/ (ignored by git). Standard library only.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
import tempfile
from dataclasses import dataclass, field
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent / "dev"))
from reconnect_marks import MarkedLogError, default_data_dir, refuse_marked_log  # noqa: E402

POWER = "GameState.DebugPrintPower() - "
GAME = "GameState.DebugPrintGame() - "
CREATE_GAME = POWER + "CREATE_GAME"
LOCAL_PLACEHOLDER = "Player1"

# Tags the Rust parser reads (crates/bg-parser/src). A tag it starts using
# later must be added here before new fixtures are made.
KEEP_TAGS = frozenset({
    "ARMOR", "ATK", "BACON_CURRENT_COMBAT_PLAYER_ID", "BACON_DUMMY_PLAYER",
    "BACON_DUO_TEAMMATE_PLAYER_ID", "BACON_DUO_TEAM_ID", "CARDRACE", "CARDTYPE",
    "CONTROLLER", "COPIED_FROM_ENTITY_ID", "DAMAGE", "FROZEN", "HEALTH", "HERO_ENTITY",
    "IS_BACON_POOL_MINION", "NUM_RESOURCES_SPENT_THIS_GAME", "PLAYER_ID",
    "PLAYER_LEADERBOARD_PLACE", "PLAYER_TECH_LEVEL", "RESOURCES", "STATE",
    "TURN", "ZONE", "ZONE_POSITION",
})
# Power packets kept (their tag= lines are filtered by KEEP_TAGS). Of the
# blocks, ATTACK is kept at any depth (the parser takes the combat boards
# there) and the player's actions at the top level only, with their end.
KEEP_OPCODES = frozenset({
    "CREATE_GAME", "GameEntity", "Player", "FULL_ENTITY", "SHOW_ENTITY",
    "CHANGE_ENTITY", "HIDE_ENTITY",
})
TOP_BLOCKS = frozenset({"PLAY", "MOVE_MINION", "DECK_ACTION"})
# Entities created with any other card type (enchantments, spells,
# trinkets...) are dropped with every line about them. Hidden cards (no type
# yet) stay. Shop buttons and hero powers tell what an action was.
KEEP_CARDTYPES = frozenset({"GAME", "PLAYER", "HERO", "MINION", "INVALID",
                            "GAME_MODE_BUTTON", "MOVE_MINION_HOVER_TARGET", "HERO_POWER"})
KEEP_GAME_KEYS = ("GameType=", "BuildNumber=", "PlayerID=")
# Names the game itself uses for hidden players and cards: not personal.
GAME_NAMES = ("UNKNOWN HUMAN PLAYER", "UNKNOWN ENTITY")

SEND_OPTION = "GameState.SendOption() - "
SEND_CHOICES = "GameState.SendChoices() - "
LINE = re.compile(r"^([DWE]) ([\d:.]+) (GameState\.(?:DebugPrint(?:Power|Game)|SendOption|SendChoices)"
                  r"\(\) - )(.*)$")
STAMP = re.compile(r"^[DWE] \d{1,2}:\d{2}:\d{2}(?:\.\d{1,7})? ")
OPTION_BODY = re.compile(r"^selectedOption=-?\d+ selectedSubOption=-?\d+ selectedTarget=-?\d+ "
                         r"selectedPosition=-?\d+$")
CHOICE_BODY = re.compile(r"^id=\d+ ChoiceType=[A-Z_]+$")
ALLOWED = re.compile(r"^[DWE] \d{2}:\d{2}:\d{2}\.\d{7} (?:GameState\.DebugPrint(?:Power|Game)\(\) - "
                     r"|GameState\.SendOption\(\) - selectedOption=-?\d+ selectedSubOption=-?\d+ "
                     r"selectedTarget=-?\d+ selectedPosition=-?\d+$"
                     r"|GameState\.SendChoices\(\) - id=\d+ ChoiceType=[A-Z_]+$)")
TICKS_PER_SECOND = 10_000_000
DAY_TICKS = 24 * 3600 * TICKS_PER_SECOND
ACCOUNT = re.compile(r"GameAccountId=\[hi=(\d+) lo=(\d+)\]")
PLACEHOLDER_ACCOUNT = re.compile(r"^GameAccountId=\[hi=(0 lo=0|1 lo=[1-9])\]$")
ENTITY_REF = re.compile(r"(?:FULL_ENTITY - Creating ID=|(?:SHOW|CHANGE)_ENTITY - Updating Entity="
                        r"|HIDE_ENTITY - Entity=|TAG_CHANGE Entity=)(.+?)(?: CardID=| tag=|$)")
BRACKET = re.compile(r"\[entityName=(?:(?!\[entityName=).)*? id=(\d+) zone=\w* zonePos=\d+ "
                     r"(?:cardId=\S* )?player=\d+\]")
LONG_NUMBER = re.compile(r"\d{9,}")
BATTLETAG = re.compile(r"#\d")
# Where real logs put player names (checked on the user's logs, build 253216).
NAME_SPOTS = (
    re.compile(r"PlayerName=(.+)$"),
    re.compile(r"(?:Entity|Target|Player)=(?![\[\d])(.+?)"
               r"(?= (?:tag|CardID|EffectCardId|EffectIndex|SubOption|Target|BlockType|TaskList"
               r"|ChoiceType)=|$)"),
    re.compile(r"(?:Info\[\d+\]|Targets\[\d+\]|Source) = (?![\[\d])(.+)$"),
    # A bracketed entity without a card id is a player.
    re.compile(r"\[entityName=(.+?) id=\d+ zone=\w* zonePos=\d+ (?:cardId= )?player=\d+\]"),
)
# Short names are only looked for where names go, not anywhere in the text.
MIN_FREE_TEXT_NAME = 4


class FixtureError(Exception):
    """The fixture cannot be made safely; the message never holds log text."""


@dataclass
class Identities:
    """Personal values of one raw game, in first-seen order."""
    names: list[str] = field(default_factory=list)
    accounts: set[str] = field(default_factory=set)
    local_names: set[str] = field(default_factory=set)


def split_games(lines: list[str]) -> list[list[str]]:
    """Each game runs from its CREATE_GAME to the next one."""
    games: list[list[str]] = []
    for line in lines:
        if CREATE_GAME in line:
            games.append([])
        if games:
            games[-1].append(line)
    return games


def _add(names: list[str], name: str) -> None:
    name = name.strip()
    if name and name != "GameEntity" and not name.startswith(GAME_NAMES) and name not in names:
        names.append(name)


def collect_identities(lines: list[str]) -> Identities:
    """Every player name and account id in a game, from any kind of line."""
    ids = Identities()
    local_pids: set[str] = set()
    for line in lines:
        for hi, lo in ACCOUNT.findall(line):
            if lo != "0":
                ids.accounts.update(v for v in (hi, lo) if v != "0")
                pid = re.search(r"PlayerID=(\d+)", line)
                if pid:
                    local_pids.add(pid.group(1))
    for line in lines:
        local = re.search(r"PlayerID=(\d+), PlayerName=(.+)$", line)
        if local and local.group(1) in local_pids:
            ids.local_names.add(local.group(2).strip())
        for spot in NAME_SPOTS:
            for match in spot.finditer(line):
                _add(ids.names, match.group(1))
    # "Name#1234" is also written as just "Name".
    for name in list(ids.names):
        base = name.split("#", 1)[0]
        if base != name:
            _add(ids.names, base)
            if name in ids.local_names:
                ids.local_names.add(base)
    return ids


def placeholders(ids: Identities) -> dict[str, str]:
    """Name -> Player1 (local) or OpponentN; a "Name#1234" and its "Name" share one."""
    mapping: dict[str, str] = {}
    count = 0
    for name in ids.names:
        base = name.split("#", 1)[0]
        if base in mapping:
            mapping[name] = mapping[base]
        elif name in ids.local_names or base in ids.local_names:
            mapping[name] = LOCAL_PLACEHOLDER
        else:
            count += 1
            mapping[name] = f"Opponent{count}"
        mapping.setdefault(base, mapping[name])
    return mapping


def _entity_id(body: str) -> int | None:
    """Entity id of a FULL/SHOW/CHANGE/HIDE_ENTITY or TAG_CHANGE line, if it names one."""
    match = ENTITY_REF.match(body)
    if not match:
        return None
    ref = match.group(1)
    if ref.isdigit():
        return int(ref)
    ids = re.findall(r"id=(\d+)", ref) if ref.startswith("[") else []
    return int(ids[-1]) if ids else None


def dropped_entities(lines: list[str]) -> set[int]:
    """Entities created with a card type the parser never reads (KEEP_CARDTYPES)."""
    dropped: set[int] = set()
    current = None
    for line in lines:
        match = LINE.match(line.rstrip("\r\n"))
        if not match or match.group(3) != POWER:
            continue
        body = match.group(4).strip()
        created = re.match(r"FULL_ENTITY - Creating ID=(\d+) ", body)
        if created:
            current = int(created.group(1))
            continue
        cardtype = re.match(r"tag=CARDTYPE value=(\w+)$", body)
        if cardtype and current is not None:
            if cardtype.group(1) not in KEEP_CARDTYPES:
                dropped.add(current)
            continue
        if not body.startswith("tag="):
            current = None
    return dropped


class Clock:
    """A line's time as time since the game's first line, past midnight too."""

    def __init__(self) -> None:
        self.first: int | None = None
        self.last = 0
        self.days = 0

    def relative(self, stamp: str) -> str:
        hms, _, fraction = stamp.partition(".")
        h, m, s = (int(x) for x in hms.split(":"))
        ticks = ((h * 60 + m) * 60 + s) * TICKS_PER_SECOND + int(fraction.ljust(7, "0")[:7])
        if self.first is None:
            self.first = ticks
        if ticks < self.last - DAY_TICKS // 2:
            self.days += 1
        self.last = ticks
        since = max(0, ticks + self.days * DAY_TICKS - self.first)
        whole, fraction_ticks = divmod(since, TICKS_PER_SECOND)
        minutes, seconds = divmod(whole, 60)
        return f"{minutes // 60:02d}:{minutes % 60:02d}:{seconds:02d}.{fraction_ticks:07d}"


class _AllowList:
    """Line kinds, block types, entities and tags the parser reads."""

    def __init__(self, dropped: set[int]) -> None:
        self.dropped = dropped
        self.in_dropped_entity = False  # its tag= lines go too
        self.top_block_kept = False  # so its BLOCK_END stays too

    def keep(self, prefix: str, body: str) -> bool:
        if prefix == GAME:
            return body.startswith(KEEP_GAME_KEYS)
        if prefix == SEND_OPTION:
            return bool(OPTION_BODY.match(body))
        if prefix == SEND_CHOICES:
            return bool(CHOICE_BODY.match(body))
        stripped = body.strip()
        top = not body[:1].isspace()
        opcode = stripped.split(" ", 1)[0]
        if opcode.startswith("tag="):
            return not self.in_dropped_entity and opcode[len("tag="):] in KEEP_TAGS
        self.in_dropped_entity = False
        if opcode == "BLOCK_START":
            block = re.match(r"BLOCK_START BlockType=(\w+) ", stripped)
            kind = block.group(1) if block else ""
            if not top:
                return kind == "ATTACK"
            self.top_block_kept = kind in TOP_BLOCKS or kind == "ATTACK"
            return self.top_block_kept
        if opcode == "BLOCK_END":
            if not top:
                return False
            kept, self.top_block_kept = self.top_block_kept, False
            return kept
        if opcode not in KEEP_OPCODES and opcode != "TAG_CHANGE":
            return False
        if _entity_id(stripped) in self.dropped:
            self.in_dropped_entity = True
            return False
        if opcode == "TAG_CHANGE":
            tag = re.search(r" tag=(\w+) value=", stripped)
            return bool(tag) and tag.group(1) in KEEP_TAGS
        return True


def _scrub(body: str, names: re.Pattern | None, mapping: dict[str, str],
           accounts: dict[tuple[str, str], str]) -> str:
    # The parser reads only the id in a bracketed entity. The rest is a
    # player's name or card text (game text, kept out of the repo: D-017).
    body = BRACKET.sub(lambda m: m.group(1), body)
    if names is not None:
        body = names.sub(lambda m: mapping[m.group(0)], body)

    def account(m: re.Match) -> str:
        key = (m.group(1), m.group(2))
        if key not in accounts:
            humans = sum(1 for v in accounts.values() if v != "hi=0 lo=0")
            accounts[key] = "hi=0 lo=0" if m.group(2) == "0" else f"hi=1 lo={humans + 1}"
        return f"GameAccountId=[{accounts[key]}]"
    return ACCOUNT.sub(account, body)


def _name_pattern(mapping: dict[str, str]) -> re.Pattern | None:
    if not mapping:
        return None
    # Longest first, so "Foo Bar" wins over "Foo"; only where a name goes:
    # after "=" or "= ", up to a space, "]" or the end.
    alternatives = "|".join(re.escape(n) for n in sorted(mapping, key=len, reverse=True))
    return re.compile(r"(?<==)(?:" + alternatives + r")(?=[ \]]|$)|(?<== )(?:"
                      + alternatives + r")$")


def scrub_game(lines: list[str], ids: Identities) -> str:
    mapping = placeholders(ids)
    names = _name_pattern(mapping)
    accounts: dict[tuple[str, str], str] = {}
    allow = _AllowList(dropped_entities(lines))
    clock = Clock()
    out = []
    for line in lines:
        # Every line moves the clock, kept or not, so midnight is noticed.
        time = clock.relative(line.split(" ", 2)[1]) if STAMP.match(line) else None
        match = LINE.match(line.rstrip("\r\n"))
        if not match or time is None or not allow.keep(match.group(3), match.group(4)):
            continue
        level, _, prefix, body = match.groups()
        out.append(f"{level} {time} {prefix}{_scrub(body, names, mapping, accounts)}")
    return "\n".join(out) + "\n"


def deny_hits(text: str, ids: Identities) -> list[str]:
    """Reasons to refuse, as "line N: kind". Never the value itself."""
    free = [n for n in ids.names if len(n) >= MIN_FREE_TEXT_NAME]
    free_rx = (re.compile(r"(?<![^\W_])(?:" + "|".join(re.escape(n) for n in free)
                          + r")(?![^\W_])", re.IGNORECASE) if free else None)
    spot_rx = _name_pattern({n: n for n in ids.names})
    account_rx = (re.compile(r"(?<!\d)(?:" + "|".join(sorted(ids.accounts)) + r")(?!\d)")
                  if ids.accounts else None)
    hits = []
    for number, line in enumerate(text.splitlines(), 1):
        checks = (
            ("line outside the allow-list", not ALLOWED.match(line)),
            ("BattleTag-like '#' and digits", BATTLETAG.search(line)),
            ("bracketed entity text", "[entityName=" in line),
            ("player name", (free_rx and free_rx.search(line)) or (spot_rx and spot_rx.search(line))),
            ("account id", account_rx and account_rx.search(line)),
            ("GameAccountId not a placeholder",
             any(not PLACEHOLDER_ACCOUNT.match(m.group(0)) for m in ACCOUNT.finditer(line))),
            ("long number", LONG_NUMBER.search(line)),
        )
        hits.extend(f"line {number}: {kind}" for kind, hit in checks if hit)
    return hits


def make_fixture(raw: str, game: int) -> str:
    """The scrubbed text of game number `game` (1-based); raises FixtureError."""
    games = split_games(raw.splitlines())
    if not 1 <= game <= len(games):
        raise FixtureError(f"game {game} not found: the log has {len(games)} game(s)")
    lines = games[game - 1]
    ids = collect_identities(lines)
    out = scrub_game(lines, ids)
    hits = deny_hits(out, ids)
    if hits:
        shown = "; ".join(hits[:20])
        raise FixtureError(f"deny-list check failed ({len(hits)} hit(s)): {shown}")
    return out


def _report(command: list[str], text: str, folder: Path) -> object:
    fd, name = tempfile.mkstemp(suffix=".log", dir=folder)
    try:
        with os.fdopen(fd, "w", encoding="utf-8", newline="\n") as f:
            f.write(text)
        done = subprocess.run([*command, name, "--json"], capture_output=True, text=True,
                              encoding="utf-8", timeout=300, check=False)
        try:
            return json.loads(done.stdout)["games"]
        except (ValueError, KeyError, TypeError) as exc:
            raise FixtureError(f"the parser gave no report (exit {done.returncode})") from exc
    finally:
        os.unlink(name)


def _without_card_names(games: object) -> object:
    if isinstance(games, list):
        return [{k: v for k, v in g.items() if k != "card_names"} if isinstance(g, dict) else g
                for g in games]
    return games


def check_same_report(command: list[str], raw_game: str, fixture: str, folder: Path) -> object:
    """The fixture must read exactly like the raw game, but for hero names
    (card text, never kept); returns the fixture's report."""
    report = _report(command, fixture, folder)
    if _without_card_names(_report(command, raw_game, folder)) != _without_card_names(report):
        raise FixtureError("the fixture's report differs from the raw game's report")
    return report


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("raw", type=Path, help="raw Power.log (keep it under .local/)")
    ap.add_argument("--game", type=int, required=True, help="game number in the log, from 1")
    ap.add_argument("--out", type=Path, required=True, help="fixture file to write")
    ap.add_argument("--session", help="the log's Hearthstone_<date> folder name, when the "
                    "log was copied out of it (to tell when its games were played)")
    ap.add_argument("--reconnect-data-dir", type=Path, default=None,
                    help="folder of the reconnect dev tool's record (default: the app's data folder)")
    ap.add_argument("--parser", nargs="+", metavar="CMD",
                    help="bg-parse command; the raw game and the fixture must report the same")
    args = ap.parse_args(argv)
    expected_path = args.out.with_suffix(".json")
    for path in (args.out, expected_path):
        if path.exists():
            print(f"{path} exists; not overwriting", file=sys.stderr)
            return 2
    report = None
    try:
        # D-022, D-043: a log with a game played with the reconnect dev tool is never used.
        for warning in refuse_marked_log(args.raw, args.session,
                                         args.reconnect_data_dir or default_data_dir()):
            print(f"warning: {warning}", file=sys.stderr)
        raw = args.raw.read_text(encoding="utf-8", errors="replace")
        fixture = make_fixture(raw, args.game)
        if args.parser:
            raw_game = "\n".join(split_games(raw.splitlines())[args.game - 1]) + "\n"
            report = check_same_report(args.parser, raw_game, fixture, args.raw.resolve().parent)
    except (OSError, FixtureError, MarkedLogError, subprocess.SubprocessError) as exc:
        print(f"refused: {exc}", file=sys.stderr)
        return 1
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(fixture, encoding="utf-8", newline="\n")
    if report is not None:
        # The expected report for the parser tests: check it by hand.
        expected_path.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8", newline="\n")
    print(f"wrote {args.out}: {fixture.count(chr(10))} lines, {len(fixture.encode())} bytes")
    return 0


if __name__ == "__main__":
    sys.exit(main())
