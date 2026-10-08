"""Builds synthetic Power.log text for the parse_bg tests.

The lines mimic the real Power.log format (GameState.DebugPrintPower and
DebugPrintGame) but every value is invented: no player names, account ids or
real games.
"""

from __future__ import annotations

PREFIX = "D 12:00:00.0000000 GameState.DebugPrintPower() - "
GAME_PREFIX = "D 12:00:00.0000000 GameState.DebugPrintGame() - "


class LogBuilder:
    def __init__(self) -> None:
        self.lines: list[str] = []

    def text(self) -> str:
        return "\n".join(self.lines) + "\n"

    def _power(self, data: str, indent: int = 0) -> "LogBuilder":
        self.lines.append(PREFIX + " " * indent + data)
        return self

    def _tags(self, tags: dict[str, object]) -> None:
        for tag, value in tags.items():
            self._power(f"tag={tag} value={value}", indent=8)

    def create_game(self, local_pid: int = 2, dummy_pid: int = 10,
                    local_hero: int = 10, bob_hero: int = 11,
                    local_tags: dict[str, object] | None = None,
                    build: int = 253216, game_type: str = "GT_BATTLEGROUNDS_DUO") -> "LogBuilder":
        self._power("CREATE_GAME")
        self._power("GameEntity EntityID=1", indent=4)
        self._tags({"CARDTYPE": "GAME", "ZONE": "PLAY", "ENTITY_ID": 1})
        self._power(f"Player EntityID=2 PlayerID={local_pid} GameAccountId=[hi=1 lo=1]", indent=4)
        self._tags({"PLAYER_ID": local_pid, "CARDTYPE": "PLAYER", "CONTROLLER": local_pid,
                    "HERO_ENTITY": local_hero, "ZONE": "PLAY", "ENTITY_ID": 2,
                    **(local_tags or {})})
        self._power(f"Player EntityID=3 PlayerID={dummy_pid} GameAccountId=[hi=0 lo=0]", indent=4)
        self._tags({"PLAYER_ID": dummy_pid, "CARDTYPE": "PLAYER", "CONTROLLER": dummy_pid,
                    "HERO_ENTITY": bob_hero, "BACON_DUMMY_PLAYER": 1, "ZONE": "PLAY",
                    "ENTITY_ID": 3})
        self.lines.append(GAME_PREFIX + f"BuildNumber={build}")
        self.lines.append(GAME_PREFIX + f"GameType={game_type}")
        self.lines.append(GAME_PREFIX + "FormatType=FT_UNKNOWN")
        self.lines.append(GAME_PREFIX + f"PlayerID={local_pid}, PlayerName=SyntheticPlayer#0001")
        self.lines.append(GAME_PREFIX + f"PlayerID={dummy_pid}, PlayerName=The Innkeeper")
        return self

    def full_entity(self, entity_id: int, card_id: str, **tags: object) -> "LogBuilder":
        self._power(f"FULL_ENTITY - Creating ID={entity_id} CardID={card_id}", indent=4)
        self._tags({**tags, "ENTITY_ID": entity_id})
        return self

    def tag(self, entity: int | str, tag: str, value: object) -> "LogBuilder":
        return self._power(f"TAG_CHANGE Entity={entity} tag={tag} value={value} ", indent=4)

    def turn(self, value: int) -> "LogBuilder":
        return self.tag("GameEntity", "TURN", value)

    def attack(self, attacker: int) -> "LogBuilder":
        self._power(
            f"BLOCK_START BlockType=ATTACK Entity={attacker} "
            "EffectCardId=System.Collections.Generic.List`1[System.String] "
            "EffectIndex=0 Target=0 SubOption=-1 "
        )
        self.tag(attacker, "ATTACKING", 1)
        return self._power("BLOCK_END")

    def raw(self, line: str) -> "LogBuilder":
        self.lines.append(line)
        return self


def duo_game(complete: bool = True, build: int = 253216) -> str:
    """One Duos game: shop turn, a two-leg combat, then game over.

    Lobby (player id: hero): 2 local, 1 teammate, 3 and 4 opponents.
    """
    b = LogBuilder().create_game(local_tags={"BACON_DUO_TEAMMATE_PLAYER_ID": 1}, build=build)
    hero = {"CARDTYPE": "HERO", "HEALTH": 30}
    b.full_entity(10, "TB_BaconShop_HERO_37", CONTROLLER=2, ZONE="PLAY", PLAYER_ID=2,
                  PLAYER_LEADERBOARD_PLACE=1, PLAYER_TECH_LEVEL=1, **hero)
    b.full_entity(11, "TB_BaconShopBob", CONTROLLER=10, ZONE="PLAY", **hero)
    b.full_entity(20, "TB_BaconShop_HERO_16", CONTROLLER=10, ZONE="SETASIDE", PLAYER_ID=1,
                  BACON_DUO_TEAM_ID=1, PLAYER_LEADERBOARD_PLACE=1, **hero)
    b.full_entity(21, "TB_BaconShop_HERO_60", CONTROLLER=10, ZONE="SETASIDE", PLAYER_ID=3,
                  BACON_DUO_TEAM_ID=2, PLAYER_LEADERBOARD_PLACE=2, **hero)
    b.full_entity(22, "TB_BaconShop_HERO_18", CONTROLLER=10, ZONE="SETASIDE", PLAYER_ID=4,
                  BACON_DUO_TEAM_ID=2, PLAYER_LEADERBOARD_PLACE=2, **hero)

    # Hero pick happens on turn 0: it is not a combat.
    b.tag(2, "HERO_ENTITY", 10)

    # Round 1, shop: Bob offers three pool minions; the player buys one.
    b.turn(1)
    pool = {"CARDTYPE": "MINION", "CONTROLLER": 10, "ZONE": "PLAY", "IS_BACON_POOL_MINION": 1}
    b.full_entity(30, "BG20_100", CARDRACE="QUILBOAR", ATK=2, HEALTH=1, ZONE_POSITION=1, **pool)
    b.full_entity(31, "BG28_300", CARDRACE="PIRATE", ATK=1, HEALTH=1, ZONE_POSITION=2, **pool)
    b.full_entity(32, "BGS_004", ATK=1, HEALTH=3, ZONE_POSITION=3, **pool)
    b.tag(30, "CONTROLLER", 2).tag(30, "ZONE", "HAND").tag(30, "ZONE", "PLAY")

    # Round 1, combat. Leg 1: teammate vs player 3.
    b.turn(2)
    # As in real logs, unsold shop minions leave play when combat starts.
    b.tag(31, "ZONE", "REMOVEDFROMGAME").tag(32, "ZONE", "REMOVEDFROMGAME")
    b.tag(30, "ZONE", "SETASIDE")
    b.tag(3, "BACON_CURRENT_COMBAT_PLAYER_ID", 3)
    b.full_entity(40, "TB_BaconShop_HERO_16", CONTROLLER=2, ZONE="PLAY", **hero)
    b.tag(2, "HERO_ENTITY", 40)
    b.full_entity(41, "BG32_236", CARDTYPE="MINION", CONTROLLER=2, ZONE="PLAY",
                  ATK=2, HEALTH=2, ZONE_POSITION=1)
    b.full_entity(42, "TB_BaconShop_HERO_60", CONTROLLER=10, ZONE="PLAY", **hero)
    b.tag(3, "HERO_ENTITY", 42)
    b.full_entity(43, "BG26_135", CARDTYPE="MINION", CONTROLLER=10, ZONE="PLAY",
                  ATK=3, HEALTH=1, ZONE_POSITION=1, PREMIUM=1)
    b.attack(41)
    # 43 dies and summons 44 mid-fight: 44 was not on the board at combat start.
    b.tag(43, "ZONE", "GRAVEYARD")
    b.full_entity(44, "BGS_119", CARDTYPE="MINION", CONTROLLER=10, ZONE="PLAY",
                  ATK=2, HEALTH=1, ZONE_POSITION=1)
    # The teammate's board dies: the local player steps in against 44.
    b.tag(41, "ZONE", "GRAVEYARD")
    b.tag(2, "HERO_ENTITY", 10)
    b.tag(30, "ZONE", "PLAY")
    b.attack(30)
    # Player 3's board dies: player 4 steps in (its combat id arrives late,
    # as in real logs).
    b.tag(44, "ZONE", "GRAVEYARD")
    b.full_entity(45, "TB_BaconShop_HERO_18", CONTROLLER=10, ZONE="PLAY", **hero)
    b.tag(3, "HERO_ENTITY", 45)
    b.full_entity(46, "BG20_100_G", CARDTYPE="MINION", CONTROLLER=10, ZONE="PLAY",
                  ATK=4, HEALTH=2, ZONE_POSITION=1)
    b.tag(3, "BACON_CURRENT_COMBAT_PLAYER_ID", 4)
    b.attack(46)
    b.tag(10, "DAMAGE", 5).tag(20, "DAMAGE", 5)
    b.tag(3, "HERO_ENTITY", 11)

    b.turn(3)
    if complete:
        b.tag(21, "PLAYER_LEADERBOARD_PLACE", 2).tag(22, "PLAYER_LEADERBOARD_PLACE", 2)
        b.tag("GameEntity", "STATE", "COMPLETE")
    return b.text()


def solo_game(local_pid: int = 2) -> str:
    """Minimal Solo game: no teammate, one combat leg, finishes 3rd."""
    b = LogBuilder().create_game(local_pid=local_pid, game_type="GT_BATTLEGROUNDS")
    hero = {"CARDTYPE": "HERO", "HEALTH": 30}
    b.full_entity(10, "TB_BaconShop_HERO_37", CONTROLLER=local_pid, ZONE="PLAY", PLAYER_ID=local_pid,
                  PLAYER_LEADERBOARD_PLACE=1, ARMOR=5, **hero)
    b.full_entity(11, "TB_BaconShopBob", CONTROLLER=10, ZONE="PLAY", **hero)
    b.full_entity(21, "TB_BaconShop_HERO_60", CONTROLLER=10, ZONE="SETASIDE", PLAYER_ID=3,
                  PLAYER_LEADERBOARD_PLACE=2, **hero)
    b.turn(1).turn(2)
    b.full_entity(42, "TB_BaconShop_HERO_60", CONTROLLER=10, ZONE="PLAY", **hero)
    b.tag(3, "HERO_ENTITY", 42)
    b.tag(3, "BACON_CURRENT_COMBAT_PLAYER_ID", 3)
    b.attack(42)
    b.tag(10, "DAMAGE", 12)
    b.turn(3)
    b.tag(10, "PLAYER_LEADERBOARD_PLACE", 3)
    b.tag("GameEntity", "STATE", "COMPLETE")
    return b.text()


def solo_game_local_eliminated(opponent_dies_first: bool = False) -> str:
    """Solo game, 4-player lobby, where the local player (2) dies in round 2.

    Mirrors the real sequence (Solo and Duos logs, build 253216): the killing
    blow leaves DAMAGE above HEALTH on the local leaderboard hero; the game
    then creates a copy of it with the same PLAYER_ID, a stale
    PLAYER_LEADERBOARD_PLACE, COPIED_FROM_ENTITY_ID and DAMAGE reset to 0. The
    original goes to the GRAVEYARD and gets the final place right before
    STATE=COMPLETE. Players still alive get their standing at that moment.

    With opponent_dies_first, player 5 is eliminated in round 1 (place 4), so
    the local player finishes 3rd. Opponent deaths create no copy.

    As in real logs, the opponents' leaderboard heroes are themselves copies
    (COPIED_FROM_ENTITY_ID) of entities that are not leaderboard heroes.
    """
    b = LogBuilder().create_game(game_type="GT_BATTLEGROUNDS")
    hero = {"CARDTYPE": "HERO", "HEALTH": 30}
    b.full_entity(10, "TB_BaconShop_HERO_37", CONTROLLER=2, ZONE="PLAY", PLAYER_ID=2,
                  PLAYER_LEADERBOARD_PLACE=1, **hero)
    b.full_entity(11, "TB_BaconShopBob", CONTROLLER=10, ZONE="PLAY", **hero)
    for entity_id, card, pid in ((21, "TB_BaconShop_HERO_60", 3), (22, "TB_BaconShop_HERO_18", 4),
                                 (23, "TB_BaconShop_HERO_16", 5)):
        b.full_entity(entity_id, card, CONTROLLER=10, ZONE="SETASIDE", PLAYER_ID=pid,
                      PLAYER_LEADERBOARD_PLACE=1, COPIED_FROM_ENTITY_ID=entity_id + 70, **hero)

    b.turn(1).turn(2)
    b.full_entity(42, "TB_BaconShop_HERO_60", CONTROLLER=10, ZONE="PLAY", **hero)
    b.tag(3, "HERO_ENTITY", 42).tag(3, "BACON_CURRENT_COMBAT_PLAYER_ID", 3)
    b.attack(42)
    b.tag(10, "DAMAGE", 12)
    if opponent_dies_first:
        b.tag(23, "DAMAGE", 31).tag(23, "PLAYER_LEADERBOARD_PLACE", 4)
    b.tag(3, "HERO_ENTITY", 11)

    b.turn(3).turn(4)
    b.full_entity(44, "TB_BaconShop_HERO_18", CONTROLLER=10, ZONE="PLAY", **hero)
    b.tag(3, "HERO_ENTITY", 44).tag(3, "BACON_CURRENT_COMBAT_PLAYER_ID", 4)
    b.attack(44)
    b.tag(10, "DAMAGE", 33)  # overkill: 30 health, 33 damage
    stale_place, final_place = (2, 3) if opponent_dies_first else (3, 4)
    b.full_entity(50, "TB_BaconShop_HERO_37", CARDTYPE="HERO", HEALTH=30, CONTROLLER=2,
                  ZONE="SETASIDE")
    b.tag(50, "PLAYER_LEADERBOARD_PLACE", stale_place).tag(50, "PLAYER_ID", 2)
    b.tag(50, "DAMAGE", 33).tag(50, "COPIED_FROM_ENTITY_ID", 10).tag(50, "DAMAGE", 0)
    b.tag(10, "ZONE", "GRAVEYARD").tag(2, "PLAYSTATE", "LOSING")
    b.tag(2, "PLAYSTATE", "LOST")
    b.tag(21, "PLAYER_LEADERBOARD_PLACE", 1).tag(22, "PLAYER_LEADERBOARD_PLACE", 2)
    if not opponent_dies_first:
        b.tag(23, "PLAYER_LEADERBOARD_PLACE", 3)
    b.tag(10, "PLAYER_LEADERBOARD_PLACE", final_place)
    b.tag("GameEntity", "STATE", "COMPLETE")
    return b.text()


def duo_game_hidden_leg() -> str:
    """Duos combat where the teammate's fight never plays out in the local log.

    Real logs do this: the opponent's minions are hidden (HIDE_ENTITY, moved to
    HAND) and no ATTACK block happens, so that leg's boards are not visible.
    """
    b = LogBuilder().create_game(local_tags={"BACON_DUO_TEAMMATE_PLAYER_ID": 1})
    hero = {"CARDTYPE": "HERO", "HEALTH": 30}
    b.full_entity(10, "TB_BaconShop_HERO_37", CONTROLLER=2, ZONE="PLAY", PLAYER_ID=2,
                  PLAYER_LEADERBOARD_PLACE=1, **hero)
    b.full_entity(11, "TB_BaconShopBob", CONTROLLER=10, ZONE="PLAY", **hero)
    for entity_id, card, pid in ((20, "TB_BaconShop_HERO_16", 1), (21, "TB_BaconShop_HERO_60", 3),
                                 (22, "TB_BaconShop_HERO_18", 4)):
        b.full_entity(entity_id, card, CONTROLLER=10, ZONE="SETASIDE", PLAYER_ID=pid,
                      PLAYER_LEADERBOARD_PLACE=1, **hero)
    b.turn(1).turn(2)
    b.full_entity(40, "TB_BaconShop_HERO_16", CONTROLLER=2, ZONE="PLAY", **hero)
    b.tag(2, "HERO_ENTITY", 40)
    b.full_entity(42, "TB_BaconShop_HERO_60", CONTROLLER=10, ZONE="PLAY", **hero)
    b.tag(3, "HERO_ENTITY", 42)
    b.full_entity(43, "BG26_135", CARDTYPE="MINION", CONTROLLER=10, ZONE="HAND",
                  ATK=3, HEALTH=1)
    # Next leg starts with no attack in between.
    b.tag(2, "HERO_ENTITY", 10)
    b.full_entity(45, "TB_BaconShop_HERO_18", CONTROLLER=10, ZONE="PLAY", **hero)
    b.tag(3, "HERO_ENTITY", 45)
    b.full_entity(46, "BG20_100", CARDTYPE="MINION", CONTROLLER=10, ZONE="PLAY",
                  ATK=2, HEALTH=1, ZONE_POSITION=1)
    b.attack(46)
    # A last leg that never fights before the turn ends is not visible either.
    b.full_entity(47, "TB_BaconShop_HERO_60", CONTROLLER=10, ZONE="PLAY", **hero)
    b.tag(3, "HERO_ENTITY", 47)
    b.turn(3)
    b.tag("GameEntity", "STATE", "COMPLETE")
    return b.text()
