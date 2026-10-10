"""Tests for tools/make_fixture.py with synthetic logs.

Every name and id here is invented. The "raw" logs carry fake BattleTags,
fake opponent names and fake account ids in the places real logs put them,
plus lines the parser does not read.
"""

import io
import json
import re
import sys
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout
from datetime import datetime
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / "tools"))
sys.path.insert(0, str(HERE.parent / "tools" / "dev"))
sys.path.insert(0, str(HERE))

import make_fixture  # noqa: E402
import reconnect_marks  # noqa: E402
import parse_bg  # noqa: E402
from bg_log_builder import duo_game, solo_game, solo_shop_game  # noqa: E402

LOCAL = "FakeLocal#4321"
LOCAL_ALIAS = "FakeLocal"
RIVAL_AI = "FakeRivalAI"
RIVAL = "Fake Rival"
HI, LO = "123456789012345678", "987654321"
TS = "D 21:13:07.1234567 "
POWER = TS + "GameState.DebugPrintPower() - "
TASKLIST = TS + "PowerTaskList.DebugPrintPower() - "


def noisy(text: str) -> str:
    """A synthetic game dressed up with personal values and noise lines."""
    text = (text.replace("SyntheticPlayer#0001", LOCAL)
            .replace("PlayerName=The Innkeeper", f"PlayerName={RIVAL_AI}")
            .replace("[hi=1 lo=1]", f"[hi={HI} lo={LO}]")
            .replace("D 12:00:00.0000000 ", TS))
    extra = "\n".join([
        TASKLIST + f"    TAG_CHANGE Entity={LOCAL_ALIAS} tag=RESOURCES value=1 ",
        TS + "GameState.DebugPrintOptions() - id=1",
        POWER + f"    TAG_CHANGE Entity={RIVAL} tag=ZONE value=PLAY ",
        POWER + f"    TAG_CHANGE Entity={LOCAL_ALIAS} tag=NUM_TURNS_IN_PLAY value=1 ",
        POWER + "BLOCK_START BlockType=TRIGGER Entity=[entityName=" + RIVAL
        + " id=3 zone=PLAY zonePos=0 cardId= player=10] EffectCardId="
        "System.Collections.Generic.List`1[System.String] EffectIndex=0 Target=0 SubOption=-1 ",
        POWER + "    META_DATA - Meta=TARGET Data=0 InfoCount=1",
        POWER + f"                Info[0] = {RIVAL}",
        POWER + "BLOCK_END",
        POWER + "    TAG_CHANGE Entity=GameEntity tag=GAME_SEED value=1234567890123 ",
        POWER + "    TAG_CHANGE Entity=[entityName=Bob's Tavern id=11 zone=PLAY zonePos=0 "
        "cardId=TB_BaconShopBob player=10] tag=HEALTH value=30 ",
        POWER + "    TAG_CHANGE Entity=[entityName=Hero Name id=10 zone=PLAY zonePos=0 "
        "cardId=TB_BaconShop_HERO_37 player=2] tag=HEALTH value=30 ",
        POWER + f"    TAG_CHANGE Entity=[entityName={LOCAL} id=2 zone=PLAY zonePos=0 cardId= "
        "player=2] tag=CONTROLLER value=2 ",
        POWER + "    FULL_ENTITY - Creating ID=90 CardID=BG_Enchant_X",
        POWER + "        tag=CARDTYPE value=ENCHANTMENT",
        POWER + "        tag=ZONE value=PLAY",
        POWER + "    TAG_CHANGE Entity=[entityName=Buff id=90 zone=PLAY zonePos=0 cardId=BG_Enchant_X "
        "player=2] tag=ZONE value=GRAVEYARD ",
        POWER + "    HIDE_ENTITY - Entity=90 tag=ZONE value=HAND",
    ])
    marker = "TAG_CHANGE Entity=GameEntity tag=TURN value=1 \n"
    assert marker in text
    head = TS + "LoadingScreen.OnScenePreUnload() - prevMode=HUB\n"
    return head + text.replace(marker, marker + extra + "\n", 1)


def parse_text(text: str) -> list[dict]:
    with tempfile.TemporaryDirectory() as tmp:
        path = Path(tmp) / "Power.log"
        path.write_text(text, encoding="utf-8")
        return [parse_bg.to_dict(r) for r in parse_bg.parse_file(path)]


class ScrubTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.raw = noisy(duo_game())
        cls.out = make_fixture.make_fixture(cls.raw, game=1)

    def test_no_name_or_account_id_is_left(self):
        for value in (LOCAL, LOCAL_ALIAS, RIVAL_AI, RIVAL, HI, LO, "Fake"):
            self.assertNotIn(value, self.out)
        self.assertIsNone(re.search(r"#\d", self.out))

    def test_the_report_is_the_same(self):
        self.assertEqual(parse_text(self.out), parse_text(self.raw))

    def test_names_become_stable_placeholders(self):
        self.assertIn("PlayerName=Player1", self.out)
        self.assertIn("PlayerName=Opponent1", self.out)
        self.assertIn("TAG_CHANGE Entity=Opponent2 tag=ZONE", self.out)
        self.assertIn("BlockType=ATTACK", self.out)
        self.assertIn("GameAccountId=[hi=1 lo=1]", self.out)
        self.assertIn("GameAccountId=[hi=0 lo=0]", self.out)

    def test_bracketed_entities_become_their_id(self):
        self.assertIn("TAG_CHANGE Entity=11 tag=HEALTH value=30", self.out)
        self.assertIn("TAG_CHANGE Entity=10 tag=HEALTH value=30", self.out)
        self.assertIn("TAG_CHANGE Entity=2 tag=CONTROLLER value=2", self.out)
        self.assertIn("BLOCK_START BlockType=ATTACK Entity=41 ", self.out)
        for text in ("[entityName=", "Bob's Tavern", "Hero Name"):
            self.assertFalse(text in self.out, text)

    def test_an_alias_maps_to_the_same_placeholder(self):
        mapping = make_fixture.placeholders(make_fixture.collect_identities(
            make_fixture.split_games(self.raw.splitlines())[0]))
        self.assertEqual(mapping[LOCAL], "Player1")
        self.assertEqual(mapping[LOCAL_ALIAS], "Player1")

    def test_only_allowed_lines_are_kept(self):
        for line in self.out.splitlines():
            self.assertRegex(line, r"^[DWE] 00:00:00\.0000000 GameState\.DebugPrint(Power|Game)\(\) - ")
        for dropped in ("PowerTaskList", "DebugPrintOptions", "LoadingScreen", "GAME_SEED",
                        "META_DATA", "Info[0]", "NUM_TURNS_IN_PLAY", "FormatType", "21:13:07",
                        "BlockType=TRIGGER", "BG_Enchant_X", "Entity=90 "):
            self.assertFalse(dropped in self.out, dropped)
        # A kept top-level block keeps its end; the TRIGGER's end goes with it.
        self.assertEqual(self.out.count("BLOCK_END"), self.out.count("BLOCK_START BlockType=ATTACK"))

    def test_it_is_smaller(self):
        self.assertLess(len(self.out), len(self.raw))


def named(text: str) -> str:
    """A synthetic game with fake personal values where real logs put them."""
    return (text.replace("SyntheticPlayer#0001", LOCAL)
            .replace("PlayerName=The Innkeeper", f"PlayerName={RIVAL_AI}")
            .replace("[hi=1 lo=1]", f"[hi={HI} lo={LO}]"))


class ShopLinesTest(unittest.TestCase):
    """What the shop record and the logged actions need (T-204, T-205, D-046)."""

    @classmethod
    def setUpClass(cls):
        chosen = ("D 12:01:35.0000000 GameState.SendChoices() -   m_chosenEntities[0]="
                  f"[entityName={LOCAL} id=2 zone=PLAY zonePos=0 cardId= player=2]")
        options = "D 12:01:35.0000000 GameState.DebugPrintOptions() - id=7"
        nested = ("D 12:01:35.0000000 GameState.DebugPrintPower() -     BLOCK_START BlockType=PLAY "
                  "Entity=65 EffectCardId=X EffectIndex=0 Target=0 SubOption=-1 ")
        marker = "ChoiceType=GENERAL\n"
        cls.raw = named(solo_shop_game()).replace(
            marker, marker + "\n".join([chosen, options, nested]) + "\n")
        cls.out = make_fixture.make_fixture(cls.raw, game=1)

    def test_times_run_from_the_game_start(self):
        lines = self.out.splitlines()
        self.assertTrue(lines[0].startswith("D 00:00:00.0000000 "))
        self.assertIn("D 00:00:10.0000000 GameState.DebugPrintPower() -     TAG_CHANGE "
                      "Entity=GameEntity tag=TURN value=1 ", self.out)
        self.assertIn("D 00:02:30.0000000 ", self.out)
        self.assertNotIn("12:0", self.out)

    def test_options_and_choice_headers_are_kept(self):
        self.assertEqual(self.out.count("GameState.SendOption() - selectedOption="), 8)
        self.assertIn("GameState.SendChoices() - id=3 ChoiceType=GENERAL", self.out)
        self.assertIn("GameState.SendChoices() - id=3 ChoiceType=MULLIGAN", self.out)
        for dropped in ("m_chosenEntities", "DebugPrintOptions", LOCAL_ALIAS):
            self.assertNotIn(dropped, self.out)

    def test_top_level_action_blocks_and_their_entities_are_kept(self):
        for kept in ("BLOCK_START BlockType=PLAY Entity=63 ", "Target=30 ",
                     "BLOCK_START BlockType=MOVE_MINION Entity=30 ",
                     "FULL_ENTITY - Creating ID=60 CardID=TB_BaconShop_8p_Reroll_Button",
                     "tag=CARDTYPE value=GAME_MODE_BUTTON", "tag=CARDTYPE value=HERO_POWER",
                     "tag=CARDTYPE value=MOVE_MINION_HOVER_TARGET",
                     "tag=FROZEN value=1", "tag=RESOURCES value=10",
                     "tag=NUM_RESOURCES_SPENT_THIS_GAME value=9", "tag=PLAYER_TECH_LEVEL value=2"):
            self.assertIn(kept, self.out)
        # The turn-start TRIGGER goes, the offers inside it stay.
        self.assertNotIn("BlockType=TRIGGER", self.out)
        self.assertIn("FULL_ENTITY - Creating ID=34 CardID=BG28_300", self.out)
        # A nested PLAY block is not an action of the player.
        self.assertEqual(self.out.count("BlockType=PLAY"), 7)
        self.assertEqual(self.out.count("() - BLOCK_END"), self.out.count("() - BLOCK_START"))

    def test_the_report_is_the_same(self):
        self.assertEqual(parse_text(self.out), parse_text(self.raw))


class RelativeTimeTest(unittest.TestCase):
    def test_past_midnight_keeps_counting(self):
        clock = make_fixture.Clock()
        self.assertEqual(clock.relative("23:59:59.5000000"), "00:00:00.0000000")
        self.assertEqual(clock.relative("00:00:01.2500000"), "00:00:01.7500000")
        self.assertEqual(clock.relative("01:00:01.2500000"), "01:00:01.7500000")


class GameSelectionTest(unittest.TestCase):
    def test_picks_the_requested_game(self):
        raw = noisy(duo_game()) + noisy(solo_game(local_pid=5))
        out = make_fixture.make_fixture(raw, game=2)
        self.assertIn("GameType=GT_BATTLEGROUNDS\n", out)
        self.assertEqual(out.count("CREATE_GAME"), 1)
        self.assertEqual(parse_text(out), [{**parse_text(raw)[1], "index": 1}])

    def test_a_missing_game_is_an_error(self):
        with self.assertRaises(make_fixture.FixtureError):
            make_fixture.make_fixture(noisy(duo_game()), game=2)


class DenyListTest(unittest.TestCase):
    def setUp(self):
        self.ids = make_fixture.Identities(names=[LOCAL, LOCAL_ALIAS, RIVAL], accounts={HI, LO})

    def hits(self, body: str) -> list[str]:
        line = "D 00:00:00.0000000 GameState.DebugPrintPower() - " + body
        return make_fixture.deny_hits(line + "\n", self.ids)

    def test_clean_text_passes(self):
        self.assertEqual(self.hits("TAG_CHANGE Entity=Player1 tag=ZONE value=PLAY "), [])

    def test_each_kind_of_leak_is_caught(self):
        cases = {
            "battletag": "TAG_CHANGE Entity=Someone#1234 tag=ZONE value=PLAY ",
            "name": "FULL_ENTITY - Creating ID=5 CardID=X [entityName=fakelocal's Pet]",
            "account id": f"tag=ZONE value={LO}",
            "account": "Player EntityID=2 PlayerID=2 GameAccountId=[hi=7 lo=88]",
            "long number": "tag=ZONE value=123456789012",
            "bracket": "TAG_CHANGE Entity=[entityName=Card id=5 zone=PLAY] tag=ZONE value=PLAY ",
        }
        for kind, body in cases.items():
            with self.subTest(kind):
                hits = self.hits(body)
                self.assertTrue(hits)
                for value in (LOCAL_ALIAS.lower(), LO, "Someone", "123456789012"):
                    self.assertNotIn(value, " ".join(hits), "a hit must not print the value")

    def test_a_line_outside_the_allow_list_is_caught(self):
        hits = make_fixture.deny_hits("D 00:00:00.0000000 PowerTaskList.DebugPrintPower() - x\n",
                                      self.ids)
        self.assertTrue(hits)

    def test_only_the_number_shapes_of_options_and_choices_pass(self):
        ok = ("D 00:00:01.0000000 GameState.SendOption() - selectedOption=1 selectedSubOption=-1 "
              "selectedTarget=336 selectedPosition=0\n"
              "D 00:00:01.0000000 GameState.SendChoices() - id=3 ChoiceType=GENERAL\n")
        self.assertEqual(make_fixture.deny_hits(ok, self.ids), [])
        for bad in ("D 00:00:01.0000000 GameState.SendChoices() -   m_chosenEntities[0]=5\n",
                    "D 00:00:01.0000000 GameState.SendOption() - selectedOption=1 text\n"):
            with self.subTest(bad):
                self.assertTrue(make_fixture.deny_hits(bad, self.ids))


class CliTest(unittest.TestCase):
    def tmpdir(self) -> Path:
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        return Path(tmp.name)

    def run_cli(self, raw: str, *args: str, uses_dir: Path | None = None) -> tuple[int, Path, str]:
        tmp = self.tmpdir()
        src, out = tmp / "Power.log", tmp / "fixture.log"
        src.write_text(raw, encoding="utf-8")
        uses_dir = uses_dir or tmp / "no-uses"  # never the real %APPDATA% record
        err = io.StringIO()
        with redirect_stdout(io.StringIO()), redirect_stderr(err):
            code = make_fixture.main([str(src), "--game", "1", "--out", str(out),
                                      "--reconnect-data-dir", str(uses_dir), *args])
        return code, out, err.getvalue()

    def test_refuses_a_log_with_a_game_played_with_the_reconnect_tool(self):
        # D-022, D-043: the dev tool's record says it was used during this game.
        uses = self.tmpdir()
        played = reconnect_marks.local_to_utc(datetime(2026, 10, 10, 21, 13, 7))
        reconnect_marks.append_use(uses, played)
        code, out, err = self.run_cli(noisy(duo_game()), "--session",
                                      "Hearthstone_2026_10_10_21_00_00", uses_dir=uses)
        self.assertNotEqual(code, 0)
        self.assertFalse(out.exists())
        self.assertIn("reconnect", err)

    def test_a_reconnect_use_on_another_day_does_not_block(self):
        uses = self.tmpdir()
        reconnect_marks.append_use(uses, reconnect_marks.local_to_utc(datetime(2026, 10, 1, 9, 0, 0)))
        code, _, _ = self.run_cli(noisy(duo_game()), "--session",
                                  "Hearthstone_2026_10_10_21_00_00", uses_dir=uses)
        self.assertEqual(code, 0)

    def test_writes_a_clean_fixture(self):
        code, out, _ = self.run_cli(noisy(duo_game()))
        self.assertEqual(code, 0)
        self.assertNotIn(LOCAL_ALIAS, out.read_text(encoding="utf-8"))

    def test_refuses_and_writes_nothing_when_a_name_survives(self):
        # A bracket of a shape the scrubber does not know (no zonePos) keeps
        # its text, here holding the player's name: the deny-list must stop it.
        raw = noisy(duo_game()).replace(
            "FULL_ENTITY - Creating ID=41 CardID=BG32_236",
            "FULL_ENTITY - Creating ID=41 CardID=BG32_236\n" + POWER
            + f"    TAG_CHANGE Entity=[entityName={LOCAL_ALIAS}'s Hero id=10 zone=PLAY "
            "cardId=TB_BaconShop_HERO_37 player=2] tag=ZONE value=PLAY ")
        code, out, err = self.run_cli(raw)
        self.assertNotEqual(code, 0)
        self.assertFalse(out.exists())
        self.assertNotIn(LOCAL_ALIAS, err)

    def test_refuses_when_the_report_would_change(self):
        tmp = self.tmpdir()
        fake = tmp / "fake_parser.py"
        fake.write_text("import sys, json\n"
                        "text = open(sys.argv[1], encoding='utf-8').read()\n"
                        "print(json.dumps({'games': [{'status': 'ok' if 'Fake' in text else 'x'}]}))\n",
                        encoding="utf-8")
        code, out, err = self.run_cli(noisy(duo_game()), "--parser", sys.executable, str(fake))
        self.assertNotEqual(code, 0)
        self.assertFalse(out.exists())
        self.assertIn("report", err)

    def test_hero_names_may_differ_since_card_text_is_dropped(self):
        tmp = self.tmpdir()
        fake = tmp / "fake_parser.py"
        fake.write_text("import sys, json\n"
                        "names = {'X': 'Hero'} if 'Fake' in open(sys.argv[1]).read() else {}\n"
                        "print(json.dumps({'games': [{'status': 'ok', 'card_names': names}]}))\n",
                        encoding="utf-8")
        code, out, _ = self.run_cli(noisy(duo_game()), "--parser", sys.executable, str(fake))
        self.assertEqual(code, 0)
        expected = json.loads(out.with_suffix(".json").read_text(encoding="utf-8"))
        self.assertEqual(expected, [{"status": "ok", "card_names": {}}])

    def test_writes_the_expected_report_next_to_the_fixture(self):
        tmp = self.tmpdir()
        fake = tmp / "fake_parser.py"
        fake.write_text("import json\nprint(json.dumps({'file': 'x', 'games': [{'status': 'ok'}]}))\n",
                        encoding="utf-8")
        code, out, _ = self.run_cli(noisy(duo_game()), "--parser", sys.executable, str(fake))
        self.assertEqual(code, 0)
        expected = out.with_suffix(".json").read_text(encoding="utf-8")
        self.assertEqual(json.loads(expected), [{"status": "ok"}])

    def test_does_not_overwrite(self):
        tmp = self.tmpdir()
        src, out = tmp / "Power.log", tmp / "fixture.log"
        src.write_text(noisy(duo_game()), encoding="utf-8")
        out.write_text("keep", encoding="utf-8")
        with redirect_stdout(io.StringIO()), redirect_stderr(io.StringIO()):
            code = make_fixture.main([str(src), "--game", "1", "--out", str(out)])
        self.assertNotEqual(code, 0)
        self.assertEqual(out.read_text(encoding="utf-8"), "keep")


if __name__ == "__main__":
    unittest.main()
