"""Inventory of what Hearthstone's log files hold (T-111).

Read-only, Python standard library only. For a logs folder (one session folder
or the folder that holds the `Hearthstone_<date>` session folders) it lists,
per file kind, the line kinds (logger and method) and the keys they carry, and
for `Power.log` every `tag=` name with its counts, the entity types, zones and
sides it appears on, its value range or value set, the game phases and turns it
appears in, and the game modes (Solo, Duos, others).

Privacy (T-111): the output only holds line kinds, key and tag names, enum
values, entity types, card ids, numbers, counts and line shapes with every
other word and number replaced. Files that may hold network or account values
(`SENSITIVE_KINDS`) give kinds, keys and counts only. Outside `Power.log`, a
kind, key, shape or enum value is shown only if it appears in two or more
session folders. `log_inventory_guard.Guard` then withholds anything personal
and refuses the whole output if anything is left.

    python tools/log_inventory.py [--logs-dir DIR] [--json OUT.json]
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from collections import Counter
from dataclasses import dataclass, field
from pathlib import Path

from log_inventory_guard import INT, WITHHELD, Guard, PrivacyRefusal  # noqa: F401 (re-exported)

DEFAULT_LOGS_DIR = Path(r"C:\Battle.net\Battle.net\Hearthstone\Logs")

# Files that may hold network or account values: kinds, keys and counts only.
# Decks.log holds deck names the user typed and deck codes.
SENSITIVE_KINDS = {"Net.log", "GameNetLogger.log", "Login.log", "Hearthstone.log", "Decks.log"}

# Keys whose values are never kept, in any file (not even in a shape).
SENSITIVE_KEY = re.compile(
    r"(?i)^(hi|lo|name|.*(account|battletag|player_?name|email|address|host|\bip\b|"
    r"ipaddr|token|ticket|session|password|secret|region|user|friend|whisper|chat|"
    r"message|path|url|guid|uuid|serial|key|code|deck|owner)).*$"
)

# `[Section]` prefixes the game writes into Hearthstone.log (log.config names).
KNOWN_SECTIONS = {
    "Achievements", "Arena", "Asset", "Bob", "Decks", "Downloader", "ExceptionReporter",
    "FullScreenFX", "GameNetLogger", "Gameplay", "Kyle", "LoadingScreen", "Login",
    "LuckyDraw", "Net", "Packets", "Power", "Rachelle", "Sound", "Spells", "Startup",
    "Store", "Zone",
}

LINE = re.compile(r"^[A-Z] \d\d:\d\d:\d\d\.\d+ (.*)$")
# Logger.Method: every part starts with a capital, as the game's class names do.
METHOD = re.compile(r"^([A-Z][A-Za-z0-9_]{0,40}(?:\.[A-Z][A-Za-z0-9_`]{0,40})+)(?:\(\))?:?$")
BRACKET_KIND = re.compile(r"^\[([A-Za-z]{1,30})\]")
IDENT = re.compile(r"^[A-Za-z_][A-Za-z0-9_]{0,60}$")
TAG_NAME = re.compile(r"^[A-Z0-9_]{1,80}$")
ENUM = re.compile(r"^[A-Z][A-Z0-9_]{1,39}$")
HEX_LIKE = re.compile(r"^[A-F0-9]{8,}$")
# Card ids start with an all-capitals set code (BG36_, TB_, BGS_, PET_) or Bacon_.
CARD_ID = re.compile(r"^(?:[A-Z][A-Z0-9]{1,9}|Bacon)_[A-Za-z0-9_]{1,60}$")
# A key starts a word (never inside a base64 text such as a deck code) and is
# followed by one `=` only.
KEY_VALUE = re.compile(r"(?<![\w+/=])([A-Za-z_][A-Za-z0-9_]{0,40})=(?!=)([^\s,\]]*)")
SHAPE_TOKEN = re.compile(r"([A-Za-z_][A-Za-z0-9_]{0,60})=([^\s,\]]*)|([^\s=,;:()\[\]{}|<>\"'\x00]+)")
ENTITY_REF = re.compile(r"\[entityName=.*? id=(\d+) zone=(\w+) zonePos=\d+ cardId=(\S*) player=(\d+)\]")

# Tag values this large are only counted, never printed.
LARGE = 10_000_000
MAX_VALUES = 40
MAX_CARDS = 12
MAX_SHAPES = 15
MAX_SHAPE_LEN = 160
RARE = "<seen in one session only>"
PLACEHOLDER = chr(0)  # stands for an entity reference while a line is shaped

FINAL_PLAYSTATES = {"WON", "LOST", "TIED", "CONCEDED"}
PHASES = ("setup", "hero_select", "shop", "combat", "end", "other_mode")


def is_enum(token: str) -> bool:
    return bool(ENUM.match(token)) and not HEX_LIKE.match(token)


# ---------------------------------------------------------------- values


@dataclass
class Values:
    """Value range (numbers) or value set (enum names, card ids) of one tag."""

    count: int = 0
    num_min: int | None = None
    num_max: int | None = None
    large: int = 0
    enums: Counter = field(default_factory=Counter)
    cards: Counter = field(default_factory=Counter)
    other: int = 0

    def add(self, raw: str, cards: bool = True) -> None:
        self.count += 1
        if INT.match(raw):
            n = int(raw)
            if abs(n) >= LARGE:
                self.large += 1
            else:
                self.widen(n, n)
        elif is_enum(raw):
            self.enums[raw] += 1
        elif cards and CARD_ID.match(raw):
            self.cards[raw] += 1
        else:
            self.other += 1

    def widen(self, low: int | None, high: int | None) -> None:
        if low is not None:
            self.num_min = low if self.num_min is None else min(self.num_min, low)
            self.num_max = high if self.num_max is None else max(self.num_max, high)

    def merge(self, other: Values) -> None:
        self.count += other.count
        self.widen(other.num_min, other.num_max)
        self.large += other.large
        self.enums.update(other.enums)
        self.cards.update(other.cards)
        self.other += other.other

    def to_json(self) -> dict:
        out: dict = {"count": self.count}
        if self.num_min is not None:
            out["min"], out["max"] = self.num_min, self.num_max
        if self.large:
            out["large_numbers"] = self.large
        if self.enums:
            out["enums"] = _top(self.enums, MAX_VALUES)
        if self.cards:
            out["card_ids"] = _top(self.cards, MAX_CARDS)
        if self.other:
            out["other_text"] = self.other
        return out


def _top(counter: Counter, n: int) -> dict:
    top = dict(counter.most_common(n))
    if len(counter) > n:
        top["<more>"] = len(counter) - n
    return top


# ---------------------------------------------------------------- generic files


@dataclass
class FileKind:
    files: int = 0
    bytes: int = 0
    lines: int = 0
    kinds: Counter = field(default_factory=Counter)
    keys: dict = field(default_factory=dict)  # kind -> Counter of keys
    values: dict = field(default_factory=dict)  # (kind, key) -> Values (no card ids)
    shapes: dict = field(default_factory=dict)  # kind -> Counter of shapes
    folders: dict = field(default_factory=dict)  # item -> session folders it was seen in

    def seen(self, item: tuple, folder: str) -> None:
        self.folders.setdefault(item, set()).add(folder)

    def to_json(self, sensitive: bool, min_folders: int) -> dict:
        def shown(item: tuple) -> bool:
            return len(self.folders.get(item, ())) >= min_folders

        out = {"files": self.files, "bytes": self.bytes, "lines": self.lines,
               "sensitive": sensitive, "line_kinds": {}}
        for kind, n in self.kinds.most_common():
            name = kind if shown(("kind", kind)) else RARE
            entry = out["line_kinds"].setdefault(name, {"lines": 0})
            entry["lines"] += n
            if name == RARE:
                continue
            keys = self.keys.get(kind, Counter())
            entry.update(_fold({k: n for k, n in keys.items() if shown(("key", kind, k))}, keys, "keys"))
            if sensitive:
                continue
            vals = {key: self._values(kind, key, shown) for (k, key) in self.values
                    if k == kind and shown(("key", kind, key))}
            if vals:
                entry["values"] = vals
            shapes = self.shapes.get(kind, Counter())
            kept = Counter({s: n for s, n in shapes.items() if shown(("shape", kind, s))})
            if shapes:
                entry["shapes"] = _top(kept, MAX_SHAPES)
                if sum(kept.values()) < sum(shapes.values()):
                    entry["shapes"][RARE] = sum(shapes.values()) - sum(kept.values())
        return out

    def _values(self, kind: str, key: str, shown) -> dict:
        values = self.values[(kind, key)]
        rare = sum(n for v, n in values.enums.items() if not shown(("enum", kind, key, v)))
        out = values.to_json()
        if values.enums:
            kept = Counter({v: n for v, n in values.enums.items() if shown(("enum", kind, key, v))})
            out["enums"] = _top(kept, MAX_VALUES)
            if rare:
                out["enums"][RARE] = rare
        return out


def _fold(kept: dict, everything: Counter, name: str) -> dict:
    if not everything:
        return {}
    out = dict(sorted(kept.items(), key=lambda kv: -kv[1]))
    if sum(kept.values()) < sum(everything.values()):
        out[RARE] = sum(everything.values()) - sum(kept.values())
    return {name: out}


def file_kind(name: str) -> str:
    return re.sub(r"_old(?=\.log$)", "", name)


def line_kind(rest: str) -> tuple[str, str]:
    """('Logger.Method', message), ('[Section]', message) or ('<unstructured>', line)."""
    head, sep, message = rest.partition(" - ")
    match = METHOD.match(head.strip()) if sep else None
    if match:
        return match.group(1), message
    match = BRACKET_KIND.match(rest)
    if match:
        section = match.group(1) if match.group(1) in KNOWN_SECTIONS else "<other>"
        return f"[{section}]", rest[match.end():]
    first = rest.split(" ", 1)[0]
    match = METHOD.match(first)
    if match:
        return match.group(1), rest[len(first):]
    return "<unstructured>", rest


def shape(message: str) -> str:
    """The line with every value replaced: keys, enums and card ids stay."""
    message = ENTITY_REF.sub(PLACEHOLDER, message)

    def word(token: str) -> str:
        if INT.match(token):
            return "<n>"
        return token if is_enum(token) or CARD_ID.match(token) else "<w>"

    def repl(m: re.Match) -> str:
        if m.group(1):
            value = m.group(2)
            if SENSITIVE_KEY.match(m.group(1)) or not value:
                return f"{m.group(1)}=" + ("<w>" if value else "")
            return f"{m.group(1)}=" + SHAPE_TOKEN.sub(lambda v: word(v.group(0)), value)
        return word(m.group(3))

    out = SHAPE_TOKEN.sub(repl, message).replace(PLACEHOLDER, "<entity>")
    out = re.sub(r"<w>(?:\s+<w>)+", "<w…>", out)
    return re.sub(r"\s+", " ", out).strip()[:MAX_SHAPE_LEN]


def scan_generic(path: Path, stats: FileKind, sensitive: bool) -> None:
    folder = path.parent.name
    stats.files += 1
    stats.bytes += path.stat().st_size
    with open(path, encoding="utf-8", errors="replace", newline="") as handle:
        for raw in handle:
            stats.lines += 1
            match = LINE.match(raw.rstrip("\r\n"))
            kind, message = line_kind(match.group(1) if match else raw.strip())
            stats.kinds[kind] += 1
            stats.seen(("kind", kind), folder)
            keys = stats.keys.setdefault(kind, Counter())
            for key, value in KEY_VALUE.findall(message):
                keys[key] += 1
                stats.seen(("key", kind, key), folder)
                if not sensitive and not SENSITIVE_KEY.match(key):
                    stats.values.setdefault((kind, key), Values()).add(value, cards=False)
                    stats.seen(("enum", kind, key, value), folder)
            if not sensitive:
                text = shape(message)
                stats.shapes.setdefault(kind, Counter())[text] += 1
                stats.seen(("shape", kind, text), folder)


# ---------------------------------------------------------------- Power.log


@dataclass
class TagStats:
    count: int = 0
    opcodes: Counter = field(default_factory=Counter)
    entity_types: Counter = field(default_factory=Counter)
    zones: Counter = field(default_factory=Counter)
    sides: Counter = field(default_factory=Counter)
    phases: Counter = field(default_factory=Counter)
    turns: Values = field(default_factory=Values)  # only its range is used
    games: int = 0
    values: Values = field(default_factory=Values)
    cards: Counter = field(default_factory=Counter)

    def merge(self, other: TagStats) -> None:
        self.count += other.count
        for name in ("opcodes", "entity_types", "zones", "sides", "phases", "cards"):
            getattr(self, name).update(getattr(other, name))
        self.turns.widen(other.turns.num_min, other.turns.num_max)
        self.games += other.games
        self.values.merge(other.values)

    def to_json(self) -> dict:
        out = {
            "count": self.count,
            "games": self.games,
            "opcodes": dict(self.opcodes.most_common()),
            "entity_types": dict(self.entity_types.most_common()),
            "zones": dict(self.zones.most_common()),
            "sides": dict(self.sides.most_common()),
            "phases": {p: self.phases[p] for p in PHASES if self.phases[p]},
            "values": self.values.to_json(),
        }
        if self.turns.num_min is not None:
            out["turns"] = [self.turns.num_min, self.turns.num_max]
        if self.cards:
            out["card_ids"] = _top(self.cards, 8)
        return out


@dataclass
class Section:
    """Everything counted for one game mode (or for all games)."""

    games: int = 0
    builds: Counter = field(default_factory=Counter)
    tags: dict = field(default_factory=dict)
    blocks: Counter = field(default_factory=Counter)
    block_cards: dict = field(default_factory=dict)
    meta: Counter = field(default_factory=Counter)
    options: Counter = field(default_factory=Counter)
    option_errors: Counter = field(default_factory=Counter)
    choices: Counter = field(default_factory=Counter)
    card_types: dict = field(default_factory=dict)
    sub_kinds: Counter = field(default_factory=Counter)

    def merge(self, other: Section) -> None:
        self.games += other.games
        self.builds.update(other.builds)
        for tag, stats in other.tags.items():
            self.tags.setdefault(tag, TagStats()).merge(stats)
        for name in ("blocks", "meta", "options", "option_errors", "choices", "sub_kinds"):
            getattr(self, name).update(getattr(other, name))
        for table in ("block_cards", "card_types"):
            mine = getattr(self, table)
            for key, counter in getattr(other, table).items():
                mine.setdefault(key, Counter()).update(counter)

    def to_json(self) -> dict:
        return {
            "games": self.games,
            "builds": dict(self.builds),
            "line_sub_kinds": dict(self.sub_kinds.most_common()),
            "block_types": {b: {"count": n, "card_ids": _top(self.block_cards.get(b, Counter()), 10)}
                            for b, n in self.blocks.most_common()},
            "meta_data": dict(self.meta.most_common()),
            "option_types": dict(self.options.most_common()),
            "option_errors": dict(self.option_errors.most_common()),
            "choice_types": dict(self.choices.most_common()),
            "card_types": {t: {"entities": sum(c.values()), "distinct_cards": len(c),
                               "card_ids": _top(c, MAX_CARDS)}
                           for t, c in sorted(self.card_types.items())},
            "tags": {t: self.tags[t].to_json()
                     for t in sorted(self.tags, key=lambda t: -self.tags[t].count)},
        }


def mode_of(game_type: str | None) -> str:
    if game_type == "GT_BATTLEGROUNDS":
        return "solo"
    if game_type == "GT_BATTLEGROUNDS_DUO":
        return "duos"
    if game_type and is_enum(game_type):
        return f"other:{game_type}"
    return "other:unknown"


class Game:
    """State of one game while its lines are read. Names stay in here."""

    def __init__(self) -> None:
        self.section = Section(games=1)
        self.game_type: str | None = None
        self.entities: dict[int, dict] = {}
        self.player_ids: dict[int, int] = {}  # entity id -> PlayerID
        self.bob_player_ids: set[int] = set()
        self.names: dict[str, int] = {}  # player name -> entity id (never output)
        self.game_entity = 1
        self.turn: int | None = None
        self.setup = True
        self.ended = False
        self.current: int | None = None  # entity the next `tag=` lines belong to
        self.current_op = "CREATE_GAME"
        self.pending: list[tuple[str, str]] = []
        self.seen_tags: set[str] = set()

    @property
    def is_bg(self) -> bool:
        return self.game_type in ("GT_BATTLEGROUNDS", "GT_BATTLEGROUNDS_DUO")

    def phase(self) -> str:
        if self.game_type is not None and not self.is_bg:
            return "other_mode"
        if self.ended:
            return "end"
        if self.setup:
            return "setup"
        if not self.turn:
            return "hero_select"
        return "shop" if self.turn % 2 else "combat"

    def entity_label(self, eid: int) -> str:
        if eid == self.game_entity:
            return "GAME"
        if eid in self.player_ids:
            return "PLAYER_BOB" if self.player_ids[eid] in self.bob_player_ids else "PLAYER"
        card_type = self.entities.get(eid, {}).get("CARDTYPE")
        return card_type if card_type and is_enum(card_type) else "UNKNOWN"

    def side(self, eid: int) -> str:
        controller = self.entities.get(eid, {}).get("CONTROLLER")
        if eid in self.player_ids:
            controller = str(self.player_ids[eid])
        if controller is None or not INT.match(controller):
            return "none"
        pid = int(controller)
        if pid in self.bob_player_ids:
            return "bob"
        return "local" if pid in self.player_ids.values() else "other"

    def resolve(self, ref: str) -> int | None:
        ref = ref.strip()
        if INT.match(ref):
            return int(ref)
        if ref == "GameEntity":
            return self.game_entity
        match = ENTITY_REF.search(ref)
        if match:
            return int(match.group(1))
        return self.names.get(ref, self.names.get(ref.split("#", 1)[0]))

    def record(self, opcode: str, eid: int | None, tag: str, value: str) -> None:
        """Count one tag value. For TAG_CHANGE the zone is the one before the change."""
        tag = tag if TAG_NAME.match(tag) else "<other>"
        stats = self.section.tags.setdefault(tag, TagStats())
        stats.count += 1
        stats.opcodes[opcode] += 1
        if tag not in self.seen_tags:
            self.seen_tags.add(tag)
            stats.games += 1
        stats.phases[self.phase()] += 1
        if self.turn is not None:
            stats.turns.widen(self.turn, self.turn)
        if eid is None:
            stats.entity_types["UNRESOLVED"] += 1
        else:
            ent = self.entities.setdefault(eid, {})
            zone = ent.get("ZONE", "none")
            stats.entity_types[self.entity_label(eid)] += 1
            stats.zones[zone if zone == "none" or is_enum(zone) else "<other>"] += 1
            stats.sides[self.side(eid)] += 1
            if ent.get("card"):
                stats.cards[ent["card"]] += 1
            ent[tag] = value
        stats.values.add(value)
        self.after_tag(eid, tag, value)

    def after_tag(self, eid: int | None, tag: str, value: str) -> None:
        if eid == self.game_entity and tag == "TURN" and INT.match(value):
            self.turn = int(value)
        if eid == self.game_entity and tag == "STATE" and value == "COMPLETE":
            self.ended = True
        if tag == "PLAYSTATE" and value in FINAL_PLAYSTATES and eid in self.player_ids:
            if self.player_ids[eid] not in self.bob_player_ids:
                self.ended = True
        if tag == "BACON_DUMMY_PLAYER" and value == "1" and eid in self.player_ids:
            self.bob_player_ids.add(self.player_ids[eid])

    def set_card(self, eid: int, card: str) -> None:
        if CARD_ID.match(card.strip()):
            self.entities.setdefault(eid, {})["card"] = card.strip()

    def flush(self) -> None:
        """Record the `tag=` lines of the entity block that just ended.

        A block lists CONTROLLER and ZONE before CARDTYPE, so the block's own
        type, zone and controller are set first and every tag is counted on them.
        """
        if self.current is not None and self.pending:
            ent = self.entities.setdefault(self.current, {})
            for tag, value in self.pending:
                if tag in ("CARDTYPE", "ZONE", "CONTROLLER"):
                    ent[tag] = value
            for tag, value in self.pending:
                self.record(self.current_op, self.current, tag, value)
        self.current, self.pending = None, []

    def finish(self) -> None:
        self.flush()
        for eid, ent in self.entities.items():
            card_type = ent.get("CARDTYPE")
            if card_type and is_enum(card_type) and ent.get("card") and eid not in self.player_ids:
                self.section.card_types.setdefault(card_type, Counter())[ent["card"]] += 1


POWER_LINE = re.compile(r"^[A-Z] \d\d:\d\d:\d\d\.\d+ ([A-Za-z]+)\.([A-Za-z]+)\(\) - ?(.*)$")
TAG_LINE = re.compile(r"^tag=(\S+) value=(\S*)")


class PowerScanner:
    def __init__(self, guard: Guard) -> None:
        self.guard = guard
        self.modes: dict[str, Section] = {}
        self.game: Game | None = None
        self.kinds = FileKind()

    def scan(self, path: Path) -> None:
        self.kinds.files += 1
        self.kinds.bytes += path.stat().st_size
        with open(path, encoding="utf-8", errors="replace", newline="") as handle:
            for raw in handle:
                self.line(raw.rstrip("\r\n"))
        self.end_game()

    def end_game(self) -> None:
        if self.game is None:
            return
        self.game.finish()
        self.modes.setdefault(mode_of(self.game.game_type), Section()).merge(self.game.section)
        self.game = None

    def line(self, raw: str) -> None:
        self.kinds.lines += 1
        match = POWER_LINE.match(raw)
        if not match:
            rest = LINE.match(raw)
            self.kinds.kinds[line_kind(rest.group(1) if rest else raw)[0]] += 1
            return
        logger, method, body = match.groups()
        kind = f"{logger}.{method}"
        self.kinds.kinds[kind] += 1
        stripped = body.strip()
        first = re.split(r"[\s=]", stripped, maxsplit=1)[0] if stripped else ""
        sub = re.sub(r"\[\d+\]", "[]", first)
        sub = sub if IDENT.match(sub.replace("[]", "")) else "<other>"
        self.kinds.keys.setdefault(kind, Counter())[sub] += 1
        if logger != "GameState":
            return
        if method == "DebugPrintPower":
            self.power(stripped, sub)
        elif method == "DebugPrintGame":
            self.print_game(stripped)
        elif self.game is not None:
            self.other_game_state(method, stripped)

    def print_game(self, body: str) -> None:
        if self.game is None:
            return
        key, _, value = body.partition("=")
        if key == "GameType":
            self.game.game_type = value.strip()
        elif key == "BuildNumber" and INT.match(value.strip()):
            self.game.section.builds[value.strip()] += 1
        elif key == "PlayerID":
            # "PlayerID=1, PlayerName=<name>": the name only feeds the guard.
            pid = re.match(r"PlayerID=(\d+)", body)
            name = body.split("PlayerName=", 1)[1].strip() if "PlayerName=" in body else ""
            if name and pid:
                self.guard.add_name(name)
                for eid, player_id in self.game.player_ids.items():
                    if player_id == int(pid.group(1)):
                        self.game.names[name] = eid
                        self.game.names[name.split("#", 1)[0]] = eid

    def other_game_state(self, method: str, body: str) -> None:
        sec = self.game.section
        sec.sub_kinds[method] += 1
        if method == "DebugPrintOptions":
            m = re.match(r"option \d+ type=(\S+)", body)
            if m and is_enum(m.group(1)):
                sec.options[m.group(1)] += 1
            err = re.search(r"\berror=(\S+)", body)
            if err and is_enum(err.group(1)):
                sec.option_errors[err.group(1)] += 1
        elif method in ("DebugPrintEntityChoices", "SendChoices"):
            player = re.search(r"\bPlayer=(.*?) TaskList=", body)
            if player:
                self.guard.add_name(player.group(1))
            m = re.search(r"\bChoiceType=(\S+)", body)
            if m and is_enum(m.group(1)):
                sec.choices[f"{method}:{m.group(1)}"] += 1

    def power(self, body: str, sub: str) -> None:
        if body == "CREATE_GAME":
            self.end_game()
            self.game = Game()
            return
        game = self.game
        if game is None:
            return
        game.section.sub_kinds[sub] += 1
        tag = TAG_LINE.match(body)
        if tag:
            if game.current is not None:
                game.pending.append(tag.groups())
            return
        game.flush()
        if not body.startswith(("GameEntity", "Player ")):
            # The CREATE_GAME packet ends at the first packet not its own.
            game.setup = False
        if body.startswith("GameEntity EntityID="):
            game.game_entity = int(re.search(r"EntityID=(\d+)", body).group(1))
            game.current, game.current_op = game.game_entity, "CREATE_GAME"
        elif body.startswith("Player EntityID="):
            self.player(body)
        elif body.startswith(("FULL_ENTITY", "SHOW_ENTITY", "CHANGE_ENTITY")):
            self.entity(body)
        elif body.startswith("TAG_CHANGE Entity="):
            self.tag_change(body)
        elif body.startswith("HIDE_ENTITY"):
            m = re.match(r"HIDE_ENTITY - Entity=(.*) tag=(\S+) value=(\S*)", body)
            if m:
                game.record("HIDE_ENTITY", self.ref(m.group(1)), m.group(2), m.group(3))
        elif body.startswith("BLOCK_START BlockType="):
            self.block(body)
        elif body.startswith("META_DATA - Meta="):
            m = re.match(r"META_DATA - Meta=(\S+)", body)
            if m and is_enum(m.group(1)):
                game.section.meta[m.group(1)] += 1

    def ref(self, ref: str) -> int | None:
        """The entity a reference names; a player name feeds the guard."""
        eid = self.game.resolve(ref)
        if eid is None and not ENTITY_REF.search(ref) and not INT.match(ref.strip()):
            self.guard.add_name(ref)
        return eid

    def player(self, body: str) -> None:
        game = self.game
        m = re.search(r"EntityID=(\d+) PlayerID=(\d+)", body)
        if not m:
            game.section.sub_kinds["<unparsed player>"] += 1
            return
        eid, pid = int(m.group(1)), int(m.group(2))
        game.player_ids[eid] = pid
        account = re.search(r"hi=(\d+) lo=(\d+)", body)
        if account:
            self.guard.add_number(account.group(1))
            self.guard.add_number(account.group(2))
            if account.group(2) == "0":
                game.bob_player_ids.add(pid)
        game.current, game.current_op = eid, "CREATE_GAME"

    def entity(self, body: str) -> None:
        game = self.game
        created = re.match(r"FULL_ENTITY - Creating ID=(\d+) CardID=(\S*)", body)
        updated = re.match(r"\w+ - Updating Entity=(.*) CardID=(\S*)$", body)
        if created:
            eid = int(created.group(1))
            game.entities[eid] = {}
            game.set_card(eid, created.group(2))
        elif updated and self.ref(updated.group(1)) is not None:
            eid = game.resolve(updated.group(1))
            game.set_card(eid, updated.group(2))
        else:
            # Counted so a format change shows instead of tags going missing.
            game.section.sub_kinds["<unparsed entity header>"] += 1
            return
        game.current, game.current_op = eid, body.split(" ", 1)[0]

    def tag_change(self, body: str) -> None:
        game = self.game
        rest = body[len("TAG_CHANGE Entity="):]
        split = rest.rfind(" tag=")
        m = TAG_LINE.match(rest[split + 1:]) if split >= 0 else None
        if not m:
            game.section.sub_kinds["<unparsed tag change>"] += 1
            return
        eid = self.ref(rest[:split])
        if eid is None and game.is_bg and game.names and not ENTITY_REF.search(rest[:split]):
            # The log has two player entities and the local one is named (by
            # DebugPrintGame), so this is Bob's player under the name of the
            # opponent it stands for in combat.
            bob = [e for e, pid in game.player_ids.items() if pid in game.bob_player_ids]
            eid = bob[0] if len(bob) == 1 else None
        game.record("TAG_CHANGE", eid, m.group(1), m.group(2))

    def block(self, body: str) -> None:
        sec = self.game.section
        m = re.match(r"BLOCK_START BlockType=(\S+) Entity=(.*?) EffectCardId=", body)
        if not m or not is_enum(m.group(1)):
            sec.sub_kinds["<unparsed block>"] += 1
            return
        sec.blocks[m.group(1)] += 1
        eid = self.ref(m.group(2))
        card = self.game.entities.get(eid, {}).get("card") if eid is not None else None
        if card:
            sec.block_cards.setdefault(m.group(1), Counter())[card] += 1


# ---------------------------------------------------------------- driver


def log_files(logs_dir: Path) -> list[Path]:
    """Every .log file, per folder, a rotated `_old` file before the current one."""
    folders = [logs_dir] + sorted(p for p in logs_dir.iterdir() if p.is_dir())
    order = lambda p: (file_kind(p.name), "_old" not in p.name)  # noqa: E731
    return [f for d in folders for f in sorted(d.glob("*.log"), key=order) if f.is_file()]


def inventory(logs_dir: Path, guard: Guard | None = None) -> dict:
    guard = guard or Guard()
    power = PowerScanner(guard)
    generic: dict[str, FileKind] = {}
    folders = set()
    for path in log_files(logs_dir):
        folders.add(path.parent)
        kind = file_kind(path.name)
        if kind == "Power.log":
            power.scan(path)
        else:
            scan_generic(path, generic.setdefault(kind, FileKind()), kind in SENSITIVE_KINDS)
    all_games = Section()
    for section in power.modes.values():
        all_games.merge(section)
    min_folders = min(2, len(folders))
    result = {
        "tool": "log_inventory",
        "folders": len(folders),
        "file_kinds": {k: generic[k].to_json(k in SENSITIVE_KINDS, min_folders) for k in sorted(generic)},
        "power": {
            "files": power.kinds.files,
            "bytes": power.kinds.bytes,
            "lines": power.kinds.lines,
            "line_kinds": {k: {"lines": n, "sub_kinds": dict(power.kinds.keys.get(k, Counter()).most_common(40))}
                           for k, n in power.kinds.kinds.most_common()},
            "games_by_mode": {m: s.games for m, s in sorted(power.modes.items())},
            "all": all_games.to_json(),
            "by_mode": {m: s.to_json() for m, s in sorted(power.modes.items())},
        },
    }
    result = guard.scrub(result)
    guard.check(result)
    # Added after the check: its keys are this script's own reason labels.
    result["withheld"] = dict(guard.withheld)
    return result


def summary(result: dict) -> str:
    lines = [f"Folders: {result['folders']}",
             f"Withheld by the privacy guard: {result['withheld'] or 'nothing'}", "", "File kinds:"]
    for kind, info in result["file_kinds"].items():
        mark = " (keys and counts only)" if info["sensitive"] else ""
        lines.append(f"  {kind}: {info['files']} files, {info['lines']} lines, "
                     f"{len(info['line_kinds'])} line kinds{mark}")
    power, tags = result["power"], result["power"]["all"]["tags"]
    lines += [f"  Power.log: {power['files']} files, {power['lines']} lines", "",
              "Power.log games: " + ", ".join(f"{m} {n}" for m, n in power["games_by_mode"].items()),
              f"Distinct tags: {len(tags)}"]
    lines += [f"  {mode}: {len(section['tags'])} tags" for mode, section in power["by_mode"].items()]
    lines += ["", "Top 25 tags:"] + [f"  {tag}: {info['count']}" for tag, info in list(tags.items())[:25]]
    return "\n".join(lines)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--logs-dir", type=Path, default=DEFAULT_LOGS_DIR)
    parser.add_argument("--json", type=Path, help="write the full inventory here")
    args = parser.parse_args(argv)
    if not args.logs_dir.is_dir():
        print("Logs folder not found.", file=sys.stderr)
        return 2
    try:
        result = inventory(args.logs_dir)
        if args.json:
            args.json.write_text(json.dumps(result, indent=1, ensure_ascii=True) + "\n", encoding="utf-8")
    except PrivacyRefusal as refusal:
        print(f"Refused: the inventory holds a value outside the allow-list ({refusal}).",
              file=sys.stderr)
        return 3
    except OSError as error:
        # The error type only: its text holds a path, which may hold the user name.
        print(f"Could not read the logs or write the output ({type(error).__name__}).", file=sys.stderr)
        return 4
    print(summary(result))
    return 0


if __name__ == "__main__":
    sys.exit(main())
