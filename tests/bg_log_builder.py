"""Builds synthetic Power.log text for the parse_bg tests.

The lines mimic the real Power.log format (GameState.DebugPrintPower and
DebugPrintGame) but every value is invented: no player names, account ids or
real games.
"""

from __future__ import annotations

START = "12:00:00.0000000"
PREFIX = f"D {START} GameState.DebugPrintPower() - "
GAME_PREFIX = f"D {START} GameState.DebugPrintGame() - "
LIST = "System.Collections.Generic.List`1[System.String]"


class LogBuilder:
    def __init__(self) -> None:
        self.lines: list[str] = []
        self.time = START

    def text(self) -> str:
        return "\n".join(self.lines) + "\n"

    def at(self, seconds: float) -> "LogBuilder":
        """Lines from now on are logged `seconds` after 12:00:00."""
        ticks = round(seconds * 10_000_000)
        whole, fraction = divmod(ticks, 10_000_000)
        minutes, sec = divmod(whole, 60)
        self.time = f"{12 + minutes // 60:02d}:{minutes % 60:02d}:{sec:02d}.{fraction:07d}"
        return self

    def _power(self, data: str, indent: int = 0) -> "LogBuilder":
        self.lines.append(f"D {self.time} GameState.DebugPrintPower() - " + " " * indent + data)
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

    def send_option(self) -> "LogBuilder":
        """The client sending one of the player's options (as real logs print it)."""
        self.lines.append(f"D {self.time} GameState.SendOption() - selectedOption=1 "
                          "selectedSubOption=-1 selectedTarget=0 selectedPosition=0")
        return self

    def send_choice(self, choice_type: str = "GENERAL") -> "LogBuilder":
        self.lines.append(f"D {self.time} GameState.SendChoices() - id=3 ChoiceType={choice_type}")
        return self

    def block(self, block_type: str, entity: int, target: int = 0, *, indent: int = 0,
              inner=None) -> "LogBuilder":
        """A BLOCK_START ... BLOCK_END; `inner(builder)` adds what happens in it."""
        self._power(f"BLOCK_START BlockType={block_type} Entity={entity} EffectCardId={LIST} "
                    f"EffectIndex=0 Target={target} SubOption=-1 ", indent=indent)
        if inner is not None:
            inner(self)
        return self._power("BLOCK_END", indent=indent)

    def offer(self, entity_id: int, card_id: str, bob_pid: int = 10) -> "LogBuilder":
        """A pool minion showing up in the shop (Bob's side of the board)."""
        return self.full_entity(entity_id, card_id, CARDTYPE="MINION", CONTROLLER=bob_pid,
                                ZONE="PLAY", IS_BACON_POOL_MINION=1, ATK=1, HEALTH=1)


# Shop buttons of the local player (player id 2), as real logs create them.
SHOP_BUTTONS = {
    "reroll": (60, "TB_BaconShop_8p_Reroll_Button", "GAME_MODE_BUTTON"),
    "tier_up": (61, "TB_BaconShopTechUp02_Button", "GAME_MODE_BUTTON"),
    "freeze": (62, "TB_BaconShopLockAll_Button", "GAME_MODE_BUTTON"),
    "buy": (63, "TB_BaconShop_DragBuy", "MOVE_MINION_HOVER_TARGET"),
    "sell": (64, "TB_BaconShop_DragSell", "MOVE_MINION_HOVER_TARGET"),
    "hero_power": (65, "TB_BaconShop_HP_101", "HERO_POWER"),
}


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


def _reroll(spent: int, old: tuple[int, ...], new: tuple[tuple[int, str], ...]):
    def inner(b: LogBuilder) -> None:
        b.tag(2, "NUM_RESOURCES_SPENT_THIS_GAME", spent)
        for entity_id in old:
            b.tag(entity_id, "ZONE", "REMOVEDFROMGAME")
        for entity_id, card_id in new:
            b.offer(entity_id, card_id)
    return inner


def solo_shop_game(complete: bool = True) -> str:
    """Solo game with a shop record: two shop turns (12:00:10 and 12:01:10),
    a buy, a hero power, a freeze carried into the next turn, a paid and a
    free roll, a tier-up, a minion moved on the board, a sell and a discover
    pick; the hero pick is not an action of the shop.

    Local player 2 (entity 2), Bob 10 (entity 3). Gold: 3 on turn 1, 10 on
    turn 2. Spent (NUM_RESOURCES_SPENT_THIS_GAME): 3 after turn 1, 9 after turn 2.
    """
    b = LogBuilder().create_game(game_type="GT_BATTLEGROUNDS")
    hero = {"CARDTYPE": "HERO", "HEALTH": 30}
    b.full_entity(10, "TB_BaconShop_HERO_37", CONTROLLER=2, ZONE="PLAY", PLAYER_ID=2,
                  PLAYER_LEADERBOARD_PLACE=1, PLAYER_TECH_LEVEL=1, **hero)
    b.full_entity(11, "TB_BaconShopBob", CONTROLLER=10, ZONE="PLAY", **hero)
    b.full_entity(21, "TB_BaconShop_HERO_60", CONTROLLER=10, ZONE="SETASIDE", PLAYER_ID=3,
                  PLAYER_LEADERBOARD_PLACE=2, **hero)
    for entity_id, card_id, cardtype in SHOP_BUTTONS.values():
        b.full_entity(entity_id, card_id, CARDTYPE=cardtype, CONTROLLER=2, ZONE="PLAY")
    button = {name: entity_id for name, (entity_id, _, _) in SHOP_BUTTONS.items()}
    b.send_choice("MULLIGAN")  # the hero pick
    b.tag(2, "HERO_ENTITY", 10)

    # Shop turn 1.
    b.at(10).turn(1)
    b.tag(2, "RESOURCES", 3)
    b.offer(30, "BG20_100").offer(31, "BG28_300").offer(32, "BGS_004")
    b.at(15).send_option().block("PLAY", button["buy"], target=30, inner=lambda x: (
        x.tag(2, "NUM_RESOURCES_SPENT_THIS_GAME", 3), x.tag(30, "CONTROLLER", 2),
        x.tag(30, "ZONE", "HAND")))
    b.at(20).send_option().block("PLAY", button["hero_power"])
    b.at(25).send_option().block("PLAY", button["freeze"], inner=lambda x: (
        x.tag(31, "FROZEN", 1), x.tag(32, "FROZEN", 1)))

    # Combat 1.
    b.at(40).turn(2)
    b.tag(31, "ZONE", "REMOVEDFROMGAME").tag(32, "ZONE", "REMOVEDFROMGAME")
    b.full_entity(42, "TB_BaconShop_HERO_60", CONTROLLER=10, ZONE="PLAY", **hero)
    b.tag(3, "HERO_ENTITY", 42).tag(3, "BACON_CURRENT_COMBAT_PLAYER_ID", 3)
    b.attack(42)
    b.tag(10, "DAMAGE", 4)
    b.tag(3, "HERO_ENTITY", 11)

    # Shop turn 2: the frozen minions come back as new entities, frozen then thawed.
    b.at(70).turn(3)
    b.tag(2, "RESOURCES", 10)
    b.block("TRIGGER", 3, inner=lambda x: (
        x.offer(34, "BG28_300"), x.offer(35, "BGS_004"), x.offer(36, "BG21_009"),
        x.tag(34, "FROZEN", 1), x.tag(35, "FROZEN", 1),
        x.tag(34, "FROZEN", 0), x.tag(35, "FROZEN", 0)))
    b.at(75).send_option().block("PLAY", button["reroll"], inner=_reroll(
        4, (34, 35, 36), ((37, "BG25_011"), (38, "BG26_135"), (39, "BG31_806"))))
    b.at(80).send_option().block("PLAY", button["reroll"], inner=_reroll(
        4, (37, 38, 39), ((71, "BG20_100"), (72, "BG28_300"), (73, "BG20_100"))))
    b.at(85).send_option().block("PLAY", button["tier_up"], inner=lambda x: (
        x.tag(2, "NUM_RESOURCES_SPENT_THIS_GAME", 9), x.tag(10, "PLAYER_TECH_LEVEL", 2)))
    b.at(88).send_option().block("MOVE_MINION", 30)
    b.at(90).send_option().block("PLAY", button["sell"], target=30, inner=lambda x: (
        x.tag(30, "CONTROLLER", 10), x.tag(30, "ZONE", "REMOVEDFROMGAME")))
    b.at(95).send_choice()

    # Combat 2, then the game ends.
    b.at(120).turn(4)
    b.full_entity(44, "TB_BaconShop_HERO_60", CONTROLLER=10, ZONE="PLAY", **hero)
    b.tag(3, "HERO_ENTITY", 44).tag(3, "BACON_CURRENT_COMBAT_PLAYER_ID", 3)
    b.attack(44)
    b.tag(21, "DAMAGE", 30)
    if complete:
        b.at(150).tag(21, "PLAYER_LEADERBOARD_PLACE", 2).tag(10, "PLAYER_LEADERBOARD_PLACE", 1)
        b.tag("GameEntity", "STATE", "COMPLETE")
    return b.text()
