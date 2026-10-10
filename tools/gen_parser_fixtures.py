"""Writes the synthetic logs and expected reports for the Rust parser tests.

The logs come from tests/bg_log_builder.py (invented data, no real games) and
the expected JSON from tools/parse_bg.py, so the Rust port is checked against
the Python prototype. Rerun after changing either:

    python tools/gen_parser_fixtures.py
"""

from __future__ import annotations

import json
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
sys.path.insert(0, str(ROOT / "tests"))

import parse_bg  # noqa: E402
from bg_log_builder import (  # noqa: E402
    LogBuilder, duo_data_game, duo_game, duo_game_hidden_leg, solo_data_game, solo_game,
    solo_game_local_eliminated, solo_shop_game)

OUT = ROOT / "crates" / "bg-parser" / "tests" / "data"

CASES = {
    "duo_game": duo_game(),
    "duo_game_incomplete": duo_game(complete=False),
    "duo_game_untested_build": duo_game(build=999999),
    "duo_game_hidden_leg": duo_game_hidden_leg(),
    "solo_game": solo_game(),
    "solo_game_friendly": solo_game().replace(
        "GameType=GT_BATTLEGROUNDS", "GameType=GT_BATTLEGROUNDS_FRIENDLY"),
    "solo_local_eliminated": solo_game_local_eliminated(),
    "solo_local_eliminated_after_opponent": solo_game_local_eliminated(opponent_dies_first=True),
    "solo_shop_game": solo_shop_game(),
    "solo_shop_game_cut": solo_shop_game(complete=False),
    "solo_data_game": solo_data_game(),
    "duo_data_game": duo_data_game(),
    "not_battlegrounds": LogBuilder().create_game(game_type="GT_RANKED").turn(1).text(),
    "missing_own_hero": LogBuilder().create_game(game_type="GT_BATTLEGROUNDS").turn(1).turn(2).text(),
    "missing_entity": duo_game().replace(
        "TAG_CHANGE Entity=46 tag=ATTACKING", "TAG_CHANGE Entity=9999 tag=ATTACKING"),
    "two_games_new_player_id": duo_game() + solo_game(local_pid=5),
    "broken_then_ok": duo_game().replace(
        "TAG_CHANGE Entity=GameEntity tag=TURN value=3", "garbage line without timestamp") + solo_game(),
}


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    for name, text in CASES.items():
        (OUT / f"{name}.log").write_text(text, encoding="utf-8", newline="\n")
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "Power.log"
            path.write_text(text, encoding="utf-8")
            games = [parse_bg.to_dict(r) for r in parse_bg.parse_file(path)]
        (OUT / f"{name}.json").write_text(json.dumps(games, indent=2) + "\n", encoding="utf-8", newline="\n")
    print(f"{len(CASES)} cases written to {OUT.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
