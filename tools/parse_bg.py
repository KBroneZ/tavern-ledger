"""Prototype: what a Battlegrounds game looks like when rebuilt from Power.log.

Reads a local Power.log (read-only) with hslog and, for each game, reports the
local hero, the teammate in Duos, the lobby, the tribes seen in the shop, the
boards at the start of each combat leg, the opponents faced, health per round
and the final place. Anything the log does not provide is reported as not
available, never guessed. If the log does not match what this prototype
expects (e.g. after a game patch), the game is reported as "unsupported"
instead of showing wrong data.

Privacy: the output never contains player names or account ids, only card ids
and lobby player ids (1-8).

Usage:
    python tools/parse_bg.py PATH_TO_POWER_LOG [--json]

Exit codes: 0 = parsed, 1 = some game unsupported or the log could not be
parsed, 2 = file not found.
"""

from __future__ import annotations

import argparse
import json
import logging
import sys
from collections import Counter
from dataclasses import asdict, dataclass, replace
from pathlib import Path
from typing import Iterator

from hearthstone.enums import BlockType, CardType, GameTag, Race, State, Zone
from hslog import LogParser
from hslog.export import EntityTreeExporter

from check_logs import BUILD_RE, GAME_TYPE_RE

# Builds whose real logs this prototype was checked against.
TESTED_BUILDS = frozenset({253216})
TESTED_GAME_TYPES = frozenset({"GT_BATTLEGROUNDS", "GT_BATTLEGROUNDS_DUO"})
MAX_LOBBY = 8
OWN, OPPONENT = "own", "opponent"
CREATE_GAME = "GameState.DebugPrintPower() - CREATE_GAME"
NOT_IN_POWER_LOG = ("MMR", "available tribes (exact list)")


@dataclass(frozen=True)
class Minion:
    card_id: str | None
    atk: int
    health: int
    position: int
    golden: bool


@dataclass(frozen=True)
class CombatEntry:
    """One side stepping into a combat: its hero and its board at the first attack.

    board is None when that fight never plays out in the local log (in Duos the
    client hides some of the teammate's fights), so the board is unknown.
    """
    side: str
    player_id: int | None
    hero: str | None
    board: tuple[Minion, ...] | None


@dataclass(frozen=True)
class _OpenLeg:
    explicit: bool  # False: the local hero by default, until a hero switch
    hero: str | None
    hint: int | None


@dataclass(frozen=True)
class Round:
    number: int
    entries: tuple[CombatEntry, ...]
    own_health_after: int | None

    @property
    def opponents(self) -> tuple[int | None, ...]:
        return tuple(e.player_id for e in self.entries if e.side == OPPONENT)


@dataclass(frozen=True)
class LobbyPlayer:
    player_id: int
    hero: str | None
    duo_team: int | None
    final_place: int | None
    final_health: int


@dataclass(frozen=True)
class GameReport:
    index: int
    status: str  # ok | incomplete | unsupported | not_battlegrounds
    game_type: str | None
    build: int | None
    local_player_id: int | None = None
    hero: str | None = None
    teammate_player_id: int | None = None
    teammate_hero: str | None = None
    final_place: int | None = None
    final_health: int | None = None
    lobby: tuple[LobbyPlayer, ...] = ()
    shop_tribes: tuple[tuple[str, int], ...] = ()
    rounds: tuple[Round, ...] = ()
    warnings: tuple[str, ...] = ()
    problems: tuple[str, ...] = ()
    not_in_log: tuple[str, ...] = NOT_IN_POWER_LOG


class BgCollector(EntityTreeExporter):
    """Replays the packets once and records what the report needs."""

    def __init__(self, packet_tree) -> None:
        super().__init__(packet_tree)
        self.local = None
        self.dummy = None
        self.shop_races: dict[int, str] = {}
        self.entries: dict[int, list[tuple[str, str | None, int | None, tuple | None]]] = {}
        self.health: dict[int, int | None] = {}
        self._open: dict[str, _OpenLeg] = {}

    @property
    def turn(self) -> int:
        return self.game.tags.get(GameTag.TURN, 0) if self.game else 0

    @property
    def in_combat(self) -> bool:
        # Odd turns are shop phases, even turns combats. Turn 0 is the hero pick.
        return self.turn >= 2 and self.turn % 2 == 0

    def handle_player(self, packet):
        player = super().handle_player(packet)
        is_dummy = player.tags.get(GameTag.BACON_DUMMY_PLAYER) or packet.lo == 0
        if is_dummy:
            self.dummy = player
        else:
            self.local = player
        return player

    def handle_full_entity(self, packet):
        entity = super().handle_full_entity(packet)
        self._track_shop(entity)
        return entity

    def handle_block(self, packet):
        if packet.type == BlockType.ATTACK and self.in_combat:
            self._snapshot_open_legs()
        return super().handle_block(packet)

    def handle_tag_change(self, packet):
        entity = super().handle_tag_change(packet)
        if entity is None:
            return None
        if entity is self.game and packet.tag == GameTag.TURN:
            self._on_turn(packet.value)
        elif packet.tag == GameTag.HERO_ENTITY and self.in_combat:
            self._on_hero_switch(entity, packet.value)
        elif packet.tag in (GameTag.ZONE, GameTag.CONTROLLER):
            self._track_shop(entity)
        return entity

    def _on_turn(self, turn: int) -> None:
        # The previous turn's legs that never reached an attack were not visible.
        for side in (OWN, OPPONENT):
            self._close_unseen(side, (turn - 1) // 2)
        if self.in_combat:
            self._open = {OWN: _OpenLeg(False, None, None)}
        else:
            self._open = {}
            if turn > 1:
                own = own_leaderboard_hero(self.game, self.local)
                self.health[(turn - 1) // 2] = hero_health(own) if own else None

    def _on_hero_switch(self, player, hero_id: int) -> None:
        if player is self.local:
            side = OWN
        elif player is self.dummy and hero_id != self.dummy.initial_hero_entity_id:
            side = OPPONENT
        else:
            return
        self._close_unseen(side, self.turn // 2)
        self._open[side] = _OpenLeg(True, *self._current(side))

    def _current(self, side: str) -> tuple[str | None, int | None]:
        player = self.local if side == OWN else self.dummy
        hero = self.game.find_entity_by_id(player.tags.get(GameTag.HERO_ENTITY, 0))
        hint = self.dummy.tags.get(GameTag.BACON_CURRENT_COMBAT_PLAYER_ID) if side == OPPONENT else None
        return (hero.card_id if hero else None), (hint or None)

    def _close_unseen(self, side: str, round_number: int) -> None:
        leg = self._open.pop(side, None)
        if leg is not None and leg.explicit:
            self.entries.setdefault(round_number, []).append((side, leg.hero, leg.hint, None))

    def _snapshot_open_legs(self) -> None:
        for side in (OWN, OPPONENT):
            if side in self._open:
                player = self.local if side == OWN else self.dummy
                hero, hint = self._current(side)
                self.entries.setdefault(self.turn // 2, []).append(
                    (side, hero, hint, board_of(self.game, player.player_id)))
        self._open = {}

    def _track_shop(self, entity) -> None:
        tags = entity.tags
        if (self.turn % 2 == 1 and self.dummy is not None
                and tags.get(GameTag.CARDTYPE) == CardType.MINION
                and tags.get(GameTag.ZONE) == Zone.PLAY
                and tags.get(GameTag.CONTROLLER) == self.dummy.player_id
                and tags.get(GameTag.IS_BACON_POOL_MINION)):
            self.shop_races[entity.id] = race_name(tags.get(GameTag.CARDRACE, 0))


def race_name(value: int) -> str:
    if not value or value == Race.INVALID:
        return "NEUTRAL"
    try:
        return Race(value).name
    except ValueError:
        return f"RACE_{value}"


def is_golden(card_id: str | None) -> bool:
    """Tripled minions have their own card id. The PREMIUM tag is only cosmetic."""
    if not card_id:
        return False
    return card_id.endswith("_G") or card_id.startswith("TB_BaconUps_")


def hero_health(hero) -> int:
    """Health + armor - damage, never below 0: the killing blow can overshoot."""
    tags = hero.tags
    return max(0, tags.get(GameTag.HEALTH, 0) + tags.get(GameTag.ARMOR, 0) - tags.get(GameTag.DAMAGE, 0))


def board_of(game, controller: int) -> tuple[Minion, ...]:
    minions = [
        e for e in game.entities
        if e.tags.get(GameTag.CARDTYPE) == CardType.MINION
        and e.tags.get(GameTag.ZONE) == Zone.PLAY
        and e.tags.get(GameTag.CONTROLLER) == controller
    ]
    minions.sort(key=lambda e: (e.tags.get(GameTag.ZONE_POSITION, 0), e.id))
    return tuple(
        Minion(e.card_id, e.tags.get(GameTag.ATK, 0), e.tags.get(GameTag.HEALTH, 0)
               - e.tags.get(GameTag.DAMAGE, 0), e.tags.get(GameTag.ZONE_POSITION, 0),
               is_golden(e.card_id))
        for e in minions
    )


def leaderboard_heroes(game) -> dict[int, object]:
    """Lobby heroes by player id (the last entity wins if a hero was replaced).

    When the local player is eliminated the game copies its leaderboard hero
    (same PLAYER_ID, COPIED_FROM_ENTITY_ID = the original, stale place, DAMAGE
    back to 0). The original keeps the real damage and gets the final place, so
    a copy of another leaderboard hero is skipped. Other copies stay: the
    opponents' leaderboard heroes are copies of entities that are not.
    """
    candidates = [
        e for e in game.entities
        if e.tags.get(GameTag.CARDTYPE) == CardType.HERO and e.tags.get(GameTag.PLAYER_ID)
        and GameTag.PLAYER_LEADERBOARD_PLACE in e.tags
    ]
    ids = {e.id for e in candidates}
    heroes = {}
    for e in candidates:
        if e.tags.get(GameTag.COPIED_FROM_ENTITY_ID) not in ids:
            heroes[e.tags[GameTag.PLAYER_ID]] = e
    return heroes


def own_leaderboard_hero(game, local):
    return leaderboard_heroes(game).get(local.player_id) if local else None


@dataclass(frozen=True)
class GameSource:
    tree: object | None
    game_type: str | None
    build: int | None
    error: str | None = None


class _GameReader:
    """One hslog parser per game.

    hslog keeps player state for the whole parser, and in Duos the same player
    name can come back with another player id in the next game, which makes it
    raise InconsistentPlayerIdError. A fresh parser per game avoids that and
    keeps memory to one game at a time.
    """

    def __init__(self) -> None:
        self.parser = LogParser()
        self.game_type: str | None = None
        self.build: int | None = None
        self.error: str | None = None

    def feed(self, line: str) -> None:
        if "DebugPrintGame()" in line:
            if m := GAME_TYPE_RE.search(line):
                self.game_type = m.group(1)
            elif m := BUILD_RE.search(line):
                self.build = int(m.group(1))
        if self.error:
            return
        try:
            self.parser.read_line(line)
        except Exception as exc:  # never keep the message: it quotes the log line
            self.error = type(exc).__name__

    def finish(self) -> GameSource:
        if not self.error:
            try:
                self.parser.flush()
            except Exception as exc:
                self.error = type(exc).__name__
        tree = self.parser.games[0] if self.parser.games else None
        return GameSource(tree, self.game_type, self.build, self.error)


def iter_games(path: Path) -> Iterator[GameSource]:
    """Splits the log at each CREATE_GAME and parses every game on its own."""
    reader = None
    with path.open(encoding="utf-8", errors="replace") as fp:
        for line in fp:
            if CREATE_GAME in line:
                if reader is not None:
                    yield reader.finish()
                reader = _GameReader()
            if reader is not None:
                reader.feed(line)
    if reader is not None:
        yield reader.finish()


def build_report(index: int, source: GameSource) -> GameReport:
    game_type, build = source.game_type, source.build
    base = GameReport(index=index, status="unsupported", game_type=game_type, build=build)
    if not game_type or "BATTLEGROUNDS" not in game_type:
        return replace(base, status="not_battlegrounds")
    warnings = []
    if build not in TESTED_BUILDS:
        warnings.append(f"build {build} not tested")
    if game_type not in TESTED_GAME_TYPES:
        warnings.append(f"{game_type} not tested with real logs")
    base = replace(base, warnings=tuple(warnings))
    if source.error or source.tree is None:
        return replace(base, problems=(f"parser error: {source.error or 'no game data'}",))
    try:
        collector = BgCollector(source.tree)
        collector.export()
    except Exception as exc:  # hslog raises many types; never echo log text
        return replace(base, problems=(f"parser error: {type(exc).__name__}",))
    return summarize(base, collector)


def validate(c: BgCollector, heroes: dict) -> tuple[str, ...]:
    """Checks the invariants the report relies on. Any failure = unsupported."""
    problems = []
    if c.local is None or c.dummy is None:
        problems.append("could not identify the local player and the shop player")
    elif c.local.player_id not in heroes:
        problems.append("no leaderboard hero for the local player")
    if not 1 <= len(heroes) <= MAX_LOBBY:
        problems.append(f"unexpected lobby size: {len(heroes)}")
    return tuple(problems)


def lobby_of(c: BgCollector, heroes: dict, complete: bool) -> tuple[LobbyPlayer, ...]:
    local = c.local
    return tuple(
        LobbyPlayer(
            pid, h.card_id,
            # The local hero has no team tag; the local player entity has it.
            h.tags.get(GameTag.BACON_DUO_TEAM_ID)
            or (local.tags.get(GameTag.BACON_DUO_TEAM_ID) if pid == local.player_id else None),
            h.tags.get(GameTag.PLAYER_LEADERBOARD_PLACE) if complete else None,
            hero_health(h))
        for pid, h in sorted(heroes.items())
    )


def rounds_of(c: BgCollector, heroes: dict, last_health: int | None) -> tuple[Round, ...]:
    health = dict(c.health)
    if last_health is not None and c.entries:
        # The game ends right after the last combat: no shop turn records it.
        health.setdefault(max(c.entries), last_health)
    # Combat hero copies share the card id of the lobby hero. Ghosts of
    # eliminated players do not, so their id comes from the combat tag.
    by_card = {h.card_id: pid for pid, h in heroes.items()}
    return tuple(
        Round(n, tuple(CombatEntry(side, by_card.get(card, hint), card, board)
                       for side, card, hint, board in c.entries[n]), health.get(n))
        for n in sorted(c.entries)
    )


def summarize(base: GameReport, c: BgCollector) -> GameReport:
    heroes = leaderboard_heroes(c.game)
    problems = validate(c, heroes)
    if problems:
        return replace(base, problems=problems)
    complete = c.game.tags.get(GameTag.STATE) == State.COMPLETE
    own = heroes[c.local.player_id]
    mate = c.local.tags.get(GameTag.BACON_DUO_TEAMMATE_PLAYER_ID) or None
    return replace(
        base,
        status="ok" if complete else "incomplete",
        local_player_id=c.local.player_id,
        hero=own.card_id,
        teammate_player_id=mate,
        teammate_hero=heroes[mate].card_id if mate in heroes else None,
        final_place=own.tags.get(GameTag.PLAYER_LEADERBOARD_PLACE) if complete else None,
        final_health=hero_health(own),
        lobby=lobby_of(c, heroes, complete),
        shop_tribes=tuple(Counter(c.shop_races.values()).most_common()),
        rounds=rounds_of(c, heroes, hero_health(own) if complete else None),
    )


def parse_file(path: Path) -> list[GameReport]:
    return [build_report(i, source) for i, source in enumerate(iter_games(path), start=1)]


def to_dict(report: GameReport) -> dict:
    data = asdict(report)
    data["shop_tribes"] = dict(report.shop_tribes)
    for rnd, raw in zip(report.rounds, data["rounds"], strict=True):
        raw["opponents"] = list(rnd.opponents)
    return data


def fmt_board(board: tuple[Minion, ...] | None) -> str:
    if board is None:
        return "(not visible in the log)"
    if not board:
        return "(empty)"
    return ", ".join(f"{m.card_id} {m.atk}/{m.health}{' golden' if m.golden else ''}" for m in board)


def format_text(report: GameReport) -> str:
    lines = [f"Game {report.index}: {report.game_type or 'unknown type'}, "
             f"build {report.build or 'unknown'} - {report.status}"]
    lines += [f"  warning: {w}" for w in report.warnings]
    lines += [f"  problem: {p}" for p in report.problems]
    if report.status not in ("ok", "incomplete"):
        return "\n".join(lines)
    mate = (f"; teammate {report.teammate_hero} (player {report.teammate_player_id})"
            if report.teammate_player_id else "")
    place = report.final_place if report.final_place is not None else "not available"
    lines.append(f"  Hero: {report.hero} (player {report.local_player_id}){mate}")
    lines.append(f"  Result: place {place}, health {report.final_health}")
    lines.append("  Lobby: " + "; ".join(
        f"p{p.player_id} {p.hero} team {p.duo_team or '-'} place {p.final_place or '-'}"
        for p in report.lobby))
    tribes = ", ".join(f"{name} {n}" for name, n in report.shop_tribes) or "none seen"
    lines.append(f"  Tribes seen in shop (inferred, not the exact lobby list): {tribes}")
    lines.append("  MMR: not available in Power.log")
    for rnd in report.rounds:
        lines.append(f"  Round {rnd.number}: health after {rnd.own_health_after}, "
                     f"opponents {list(rnd.opponents)}")
        lines += [f"    {e.side} p{e.player_id} {e.hero}: {fmt_board(e.board)}" for e in rnd.entries]
    return "\n".join(lines)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("path", type=Path, help="Power.log or Power_old.log")
    parser.add_argument("--json", action="store_true", help="print JSON instead of text")
    args = parser.parse_args(argv)
    # hslog logs harmless "Broken option nesting" warnings by the thousand.
    logging.getLogger().setLevel(logging.ERROR)

    if not args.path.is_file():
        print(f"File not found: {args.path}")
        return 2
    try:
        reports = parse_file(args.path)
    except Exception as exc:  # never echo log lines: they can hold player names
        print(f"Unsupported log format ({type(exc).__name__}).")
        return 1
    if args.json:
        print(json.dumps({"file": args.path.name, "games": [to_dict(r) for r in reports]}, indent=2))
    else:
        print("\n\n".join(format_text(r) for r in reports) or "No games found.")
    return 1 if any(r.status == "unsupported" for r in reports) else 0


if __name__ == "__main__":
    sys.exit(main())
