"""Tests de tools/parse_bg.py con logs sintéticos (ver bg_log_builder.py)."""

import io
import json
import sys
import tempfile
import unittest
from contextlib import redirect_stdout
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / "tools"))
sys.path.insert(0, str(HERE))

import parse_bg  # noqa: E402
from bg_log_builder import LogBuilder, duo_game, duo_game_hidden_leg, solo_game  # noqa: E402


def parse_text(text: str) -> list[parse_bg.GameReport]:
    with tempfile.TemporaryDirectory() as tmp:
        path = Path(tmp) / "Power.log"
        path.write_text(text, encoding="utf-8")
        return parse_bg.parse_file(path)


class DuoGameTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        reports = parse_text(duo_game())
        assert len(reports) == 1
        cls.report = reports[0]

    def test_metadata(self):
        self.assertEqual(self.report.status, "ok")
        self.assertEqual(self.report.game_type, "GT_BATTLEGROUNDS_DUO")
        self.assertEqual(self.report.build, 253216)
        self.assertEqual(self.report.warnings, ())

    def test_own_and_teammate_heroes(self):
        self.assertEqual(self.report.local_player_id, 2)
        self.assertEqual(self.report.hero, "TB_BaconShop_HERO_37")
        self.assertEqual(self.report.teammate_player_id, 1)
        self.assertEqual(self.report.teammate_hero, "TB_BaconShop_HERO_16")

    def test_final_place_and_health(self):
        self.assertEqual(self.report.final_place, 1)
        self.assertEqual(self.report.final_health, 25)

    def test_lobby_lists_every_player_once(self):
        lobby = {p.player_id: p for p in self.report.lobby}
        self.assertEqual(sorted(lobby), [1, 2, 3, 4])
        self.assertEqual(lobby[3].hero, "TB_BaconShop_HERO_60")
        self.assertEqual(lobby[3].duo_team, 2)
        self.assertEqual(lobby[3].final_place, 2)

    def test_tribes_are_inferred_from_shop_offers(self):
        self.assertEqual(dict(self.report.shop_tribes), {"QUILBOAR": 1, "PIRATE": 1, "NEUTRAL": 1})

    def test_combat_legs_with_boards_at_combat_start(self):
        self.assertEqual(len(self.report.rounds), 1)
        rnd = self.report.rounds[0]
        self.assertEqual(rnd.number, 1)
        self.assertEqual(rnd.own_health_after, 25)
        legs = [(e.side, e.player_id, e.hero, [m.card_id for m in e.board]) for e in rnd.entries]
        self.assertEqual(legs, [
            ("own", 1, "TB_BaconShop_HERO_16", ["BG32_236"]),
            ("opponent", 3, "TB_BaconShop_HERO_60", ["BG26_135"]),
            ("own", 2, "TB_BaconShop_HERO_37", ["BG20_100"]),
            ("opponent", 4, "TB_BaconShop_HERO_18", ["BG20_100_G"]),
        ])

    def test_minion_stats(self):
        minion = self.report.rounds[0].entries[1].board[0]
        self.assertEqual((minion.atk, minion.health, minion.position), (3, 1, 1))

    def test_is_golden_handles_missing_card_ids(self):
        self.assertIs(parse_bg.is_golden(None), False)
        self.assertIs(parse_bg.is_golden(""), False)
        self.assertIs(parse_bg.is_golden("TB_BaconUps_001"), True)

    def test_golden_comes_from_the_card_id_not_the_premium_tag(self):
        cosmetic = self.report.rounds[0].entries[1].board[0]  # PREMIUM=1, normal card
        tripled = self.report.rounds[0].entries[3].board[0]   # BG20_100_G
        self.assertFalse(cosmetic.golden)
        self.assertTrue(tripled.golden)

    def test_opponents_faced(self):
        self.assertEqual(self.report.rounds[0].opponents, (3, 4))


class HiddenLegTest(unittest.TestCase):
    def test_legs_without_attacks_have_unknown_boards_not_empty_ones(self):
        report = parse_text(duo_game_hidden_leg())[0]
        legs = [(e.side, e.player_id, e.board if e.board is None else len(e.board))
                for e in report.rounds[0].entries]
        self.assertEqual(legs, [
            ("own", 1, None),
            ("opponent", 3, None),
            ("own", 2, 0),
            ("opponent", 4, 1),
            ("opponent", 3, None),
        ])

    def test_text_output_says_not_visible(self):
        report = parse_text(duo_game_hidden_leg())[0]
        self.assertIn("not visible in the log", parse_bg.format_text(report))


class SoloGameTest(unittest.TestCase):
    def test_solo_has_no_teammate_and_counts_armor(self):
        report = parse_text(solo_game())[0]
        self.assertEqual(report.status, "ok")
        self.assertIsNone(report.teammate_player_id)
        self.assertIsNone(report.teammate_hero)
        self.assertEqual(report.final_place, 3)
        self.assertEqual(report.final_health, 23)  # 30 + 5 armor - 12 damage
        self.assertEqual(report.rounds[0].opponents, (3,))


    def test_last_round_health_is_the_final_health(self):
        # The game ends right after the last combat, with no shop turn after it.
        text = solo_game().replace(
            "TAG_CHANGE Entity=GameEntity tag=TURN value=3", "TAG_CHANGE Entity=10 tag=4999 value=0")
        report = parse_text(text)[0]
        self.assertEqual(report.rounds[-1].own_health_after, 23)


class StatusTest(unittest.TestCase):
    def test_incomplete_game_has_unknown_place(self):
        report = parse_text(duo_game(complete=False))[0]
        self.assertEqual(report.status, "incomplete")
        self.assertIsNone(report.final_place)

    def test_untested_build_is_flagged(self):
        report = parse_text(duo_game(build=999999))[0]
        self.assertEqual(report.status, "ok")
        self.assertIn("build 999999 not tested", report.warnings)

    def test_non_battlegrounds_game_is_skipped(self):
        text = LogBuilder().create_game(game_type="GT_RANKED").turn(1).text()
        report = parse_text(text)[0]
        self.assertEqual(report.status, "not_battlegrounds")
        self.assertEqual(report.rounds, ())

    def test_missing_own_leaderboard_hero_is_unsupported_not_guessed(self):
        text = LogBuilder().create_game(game_type="GT_BATTLEGROUNDS").turn(1).turn(2).text()
        report = parse_text(text)[0]
        self.assertEqual(report.status, "unsupported")
        self.assertIsNone(report.hero)
        self.assertTrue(any("leaderboard" in r for r in report.problems))

    def test_export_error_does_not_leak_log_text(self):
        text = duo_game().replace("TAG_CHANGE Entity=46 tag=ATTACKING",
                                   "TAG_CHANGE Entity=9999 tag=ATTACKING")
        report = parse_text(text)[0]
        self.assertEqual(report.status, "unsupported")
        self.assertNotIn("SyntheticPlayer", json.dumps(parse_bg.to_dict(report)))


class MultiGameLogTest(unittest.TestCase):
    def test_same_name_with_a_new_player_id_in_the_next_game(self):
        # hslog keeps player state per parser; one parser per game avoids
        # InconsistentPlayerIdError when the local player id changes.
        reports = parse_text(duo_game() + solo_game(local_pid=5))
        self.assertEqual([r.status for r in reports], ["ok", "ok"])
        self.assertEqual(reports[1].local_player_id, 5)
        self.assertEqual([r.index for r in reports], [1, 2])

    def test_a_broken_game_does_not_hide_the_next_one(self):
        broken = duo_game().replace("TAG_CHANGE Entity=GameEntity tag=TURN value=3",
                                    "garbage line without timestamp")
        reports = parse_text(broken + solo_game())
        self.assertEqual([r.status for r in reports], ["unsupported", "ok"])
        self.assertEqual(len(reports[0].problems), 1)
        self.assertTrue(reports[0].problems[0].startswith("parser error: "))
        self.assertNotIn("garbage", reports[0].problems[0])


class OutputTest(unittest.TestCase):
    def test_json_output_never_contains_player_names(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "Power.log"
            path.write_text(duo_game() + solo_game(), encoding="utf-8")
            out = io.StringIO()
            with redirect_stdout(out):
                code = parse_bg.main([str(path), "--json"])
        self.assertEqual(code, 0)
        data = json.loads(out.getvalue())
        self.assertEqual(len(data["games"]), 2)
        self.assertNotIn("SyntheticPlayer", out.getvalue())
        self.assertNotIn("Innkeeper", out.getvalue())

    def test_text_summary_marks_unavailable_data(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "Power.log"
            path.write_text(duo_game(), encoding="utf-8")
            out = io.StringIO()
            with redirect_stdout(out):
                code = parse_bg.main([str(path)])
        self.assertEqual(code, 0)
        self.assertIn("MMR: not available", out.getvalue())
        self.assertIn("place 1", out.getvalue())

    def test_missing_file_exit_code(self):
        out = io.StringIO()
        with redirect_stdout(out):
            code = parse_bg.main(["does-not-exist.log"])
        self.assertEqual(code, 2)


if __name__ == "__main__":
    unittest.main()
