"""Tests for tools/log_inventory.py with synthetic logs (nothing from the real game).

Every fake personal value below (names, BattleTags, account ids, IPs, paths,
e-mails) must never reach the inventory or the printed summary.
"""

import io
import json
import sys
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "tools"))

import log_inventory as inv  # noqa: E402

FAKE_TAG = "Fakeplayer#4321"
FAKE_BOB = "Innkeeperfake"
FAKE_HI = "123456789012345678"
FAKE_LO = "77777777"
FAKE_IP = "203.0.113.45"
FAKE_EMAIL = "someone@example.org"
FAKE_PATH = r"C:\Users\fakeuser\AppData\Local\Blizzard"
FAKE_DECK_CODE = "AAECAZ8FBvoO/RTZrgSTyAS6"+"zgTi5QQMn8gEp8oE+9MEAA=="
SECRETS = [FAKE_DECK_CODE, "Secretdeckname", "Opponentfake", FAKE_TAG, "Fakeplayer", FAKE_BOB, FAKE_HI, FAKE_LO, FAKE_IP, FAKE_EMAIL,
           "fakeuser", "Teammatefake", "4321"]

T = "D 10:00:00.0000000 "


def power(body: str) -> str:
    return f"{T}GameState.DebugPrintPower() - {body}\n"


def game_log(game_type: str = "GT_BATTLEGROUNDS") -> str:
    lines = [
        power("CREATE_GAME"),
        power("    GameEntity EntityID=1"),
        power("        tag=TURN value=0"),
        power("        tag=ZONE value=PLAY"),
        power(f"    Player EntityID=2 PlayerID=1 GameAccountId=[hi={FAKE_HI} lo={FAKE_LO}]"),
        power("        tag=PLAYER_ID value=1"),
        power("    Player EntityID=3 PlayerID=2 GameAccountId=[hi=0 lo=0]"),
        power("        tag=BACON_DUMMY_PLAYER value=1"),
        f"{T}GameState.DebugPrintGame() - GameType={game_type}\n",
        f"{T}GameState.DebugPrintGame() - BuildNumber=253216\n",
        f"{T}GameState.DebugPrintGame() - PlayerID=1, PlayerName={FAKE_TAG}\n",
        f"{T}GameState.DebugPrintGame() - PlayerID=2, PlayerName={FAKE_BOB}\n",
        power("FULL_ENTITY - Creating ID=10 CardID=BG20_HERO_202"),
        power("    tag=CARDTYPE value=HERO"),
        power("    tag=CONTROLLER value=1"),
        power("    tag=ZONE value=PLAY"),
        power("    tag=HEALTH value=30"),
        power(f"TAG_CHANGE Entity={FAKE_TAG} tag=PLAYSTATE value=PLAYING "),
        power("TAG_CHANGE Entity=GameEntity tag=TURN value=1 "),
        power("FULL_ENTITY - Creating ID=20 CardID=BG_CFM_315"),
        power("    tag=CONTROLLER value=2"),
        power("    tag=ZONE value=PLAY"),
        power("    tag=CARDTYPE value=MINION"),
        power("    tag=CARDRACE value=BEAST"),
        power("BLOCK_START BlockType=PLAY Entity=[entityName=Some Card id=20 zone=PLAY "
              "zonePos=1 cardId=BG_CFM_315 player=2] EffectCardId=System.Collections"
              " EffectIndex=0 Target=0 SubOption=-1 "),
        power("    META_DATA - Meta=DAMAGE Data=3 InfoCount=1"),
        power("    TAG_CHANGE Entity=[entityName=Some Card id=20 zone=PLAY zonePos=1 "
              "cardId=BG_CFM_315 player=2] tag=ATK value=4 "),
        power("BLOCK_END"),
        f"{T}GameState.DebugPrintOptions() - id=1\n",
        f"{T}GameState.DebugPrintOptions() -   option 0 type=END_TURN mainEntity= error=INVALID errorParam=\n",
        f"{T}GameState.DebugPrintEntityChoices() - id=2 Player={FAKE_TAG} TaskList=3 "
        "ChoiceType=GENERAL CountMin=1 CountMax=1\n",
        f"{T}GameState.SendOption() - selectedOption=0 selectedSubOption=-1 selectedTarget=0\n",
        f"{T}PowerTaskList.DebugPrintPower() - TAG_CHANGE Entity={FAKE_TAG} tag=ZONE value=PLAY\n",
        power("TAG_CHANGE Entity=GameEntity tag=TURN value=2 "),
        power("TAG_CHANGE Entity=[entityName=Some Card id=20 zone=PLAY zonePos=1 "
              "cardId=BG_CFM_315 player=2] tag=DAMAGE value=2 "),
        power("TAG_CHANGE Entity=Opponentfake tag=CORPSES value=3 "),
        power("TAG_CHANGE Entity=GameEntity tag=STATE value=COMPLETE "),
        power(f"TAG_CHANGE Entity={FAKE_TAG} tag=PLAYER_LEADERBOARD_PLACE value=3 "),
    ]
    return "".join(lines)


NET = (
    f"{T}Network.Connect() - address={FAKE_IP} port=1119 account={FAKE_LO}\n"
    f"{T}Network.OnLogin() - BattleTag={FAKE_TAG} email={FAKE_EMAIL}\n"
)
DECKS = (
    f"{T}Deck.Print() - ### Secretdeckname\n"
    f"{T}Deck.Print() - # Deck ID: 1234567\n"
    f"{T}Deck.Print() - {FAKE_DECK_CODE}\n"
    f"{T}{FAKE_DECK_CODE}\n"
)
HEARTHSTONE = f"Some unstructured line about {FAKE_TAG} at {FAKE_PATH}\n"
LOADING = (
    f"{T}LoadingScreen.OnSceneUnloaded() - prevMode=HUB nextMode=GAMEPLAY\n"
    f"{T}LoadingScreen.OnSceneLoaded() - Fakeplayer joined from {FAKE_IP} code=SECRETCODE\n"
)


def write_folder(root: Path, files: dict[str, str], name: str = "Hearthstone_2026_01_01_00_00_00") -> Path:
    folder = root / name
    folder.mkdir()
    for file_name, text in files.items():
        (folder / file_name).write_text(text, encoding="utf-8")
    return root


class InventoryTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)

    def tearDown(self):
        self.tmp.cleanup()

    def run_inventory(self, files):
        return inv.inventory(write_folder(self.root, files))

    def assert_no_secret(self, text):
        for secret in SECRETS:
            self.assertNotIn(secret.casefold(), text.casefold(), secret)

    def test_power_tags_are_counted_per_mode_entity_zone_and_phase(self):
        result = self.run_inventory({"Power_old.log": game_log() + game_log("GT_BATTLEGROUNDS_DUO")})
        power = result["power"]
        self.assertEqual(power["games_by_mode"], {"duos": 1, "solo": 1})
        tags = power["by_mode"]["solo"]["tags"]
        self.assertEqual(tags["CARDRACE"]["values"]["enums"], {"BEAST": 1})
        self.assertEqual(tags["CARDRACE"]["entity_types"], {"MINION": 1})
        self.assertEqual(tags["CARDRACE"]["sides"], {"bob": 1})
        # A block's tags count on its own type and zone, whatever their order.
        self.assertEqual(tags["CONTROLLER"]["entity_types"], {"HERO": 1, "MINION": 1})
        self.assertEqual(tags["CONTROLLER"]["zones"], {"PLAY": 2})
        self.assertEqual(tags["ATK"]["phases"], {"shop": 1})
        self.assertEqual(tags["ATK"]["zones"], {"PLAY": 1})
        self.assertEqual(tags["DAMAGE"]["phases"], {"combat": 1})
        self.assertEqual(tags["HEALTH"]["values"]["min"], 30)
        self.assertEqual(tags["TURN"]["entity_types"], {"GAME": 3})
        self.assertEqual(tags["PLAYSTATE"]["entity_types"], {"PLAYER": 1})
        self.assertEqual(tags["PLAYER_LEADERBOARD_PLACE"]["phases"], {"end": 1})
        # In combat Bob's player carries the opponent's name.
        self.assertEqual(tags["CORPSES"]["entity_types"], {"PLAYER_BOB": 1})
        self.assertEqual(tags["BACON_DUMMY_PLAYER"]["entity_types"], {"PLAYER_BOB": 1})
        self.assertEqual(tags["BACON_DUMMY_PLAYER"]["phases"], {"setup": 1})
        self.assertEqual(power["all"]["tags"]["CARDRACE"]["count"], 2)
        self.assertEqual(power["all"]["builds"], {"253216": 2})

    def test_blocks_meta_options_choices_and_card_types(self):
        section = self.run_inventory({"Power.log": game_log()})["power"]["all"]
        self.assertEqual(section["block_types"]["PLAY"]["card_ids"], {"BG_CFM_315": 1})
        self.assertEqual(section["meta_data"], {"DAMAGE": 1})
        self.assertEqual(section["option_types"], {"END_TURN": 1})
        self.assertEqual(section["option_errors"], {"INVALID": 1})
        self.assertEqual(section["choice_types"], {"DebugPrintEntityChoices:GENERAL": 1})
        self.assertEqual(section["card_types"]["MINION"]["card_ids"], {"BG_CFM_315": 1})

    def test_other_game_types_are_kept_apart(self):
        result = self.run_inventory({"Power.log": game_log("GT_RANKED")})
        self.assertEqual(result["power"]["games_by_mode"], {"other:GT_RANKED": 1})
        tags = result["power"]["by_mode"]["other:GT_RANKED"]["tags"]
        self.assertEqual(set(tags["ATK"]["phases"]), {"other_mode"})

    def test_generic_files_give_kinds_keys_enum_values_and_shapes(self):
        result = self.run_inventory({"LoadingScreen.log": LOADING})
        kind = result["file_kinds"]["LoadingScreen.log"]["line_kinds"]["LoadingScreen.OnSceneUnloaded"]
        self.assertEqual(kind["keys"], {"prevMode": 1, "nextMode": 1})
        self.assertEqual(kind["values"]["nextMode"]["enums"], {"GAMEPLAY": 1})
        self.assertEqual(list(kind["shapes"]), ["prevMode=HUB nextMode=GAMEPLAY"])
        loaded = result["file_kinds"]["LoadingScreen.log"]["line_kinds"]["LoadingScreen.OnSceneLoaded"]
        self.assertNotIn("code", loaded.get("values", {}))

    def test_sensitive_files_give_keys_and_counts_only(self):
        result = self.run_inventory({"Net.log": NET, "Hearthstone.log": HEARTHSTONE})
        net = result["file_kinds"]["Net.log"]
        self.assertTrue(net["sensitive"])
        connect = net["line_kinds"]["Network.Connect"]
        self.assertEqual(connect, {"lines": 1, "keys": {"address": 1, "port": 1, "account": 1}})
        self.assertEqual(result["file_kinds"]["Hearthstone.log"]["line_kinds"], {"<unstructured>": {"lines": 1}})

    def test_no_personal_value_reaches_the_json_or_the_summary(self):
        files = {"Power.log": game_log(), "Net.log": NET, "Hearthstone.log": HEARTHSTONE,
                 "Decks.log": DECKS,
                 "LoadingScreen.log": LOADING, "GameNetLogger.log": NET, "Login.log": NET}
        result = self.run_inventory(files)
        self.assert_no_secret(json.dumps(result))
        self.assert_no_secret(inv.summary(result))

    def test_main_writes_json_and_prints_summary_without_secrets(self):
        write_folder(self.root, {"Power.log": game_log(), "Net.log": NET})
        out_json = self.root / "out.json"
        stdout = io.StringIO()
        with redirect_stdout(stdout):
            code = inv.main(["--logs-dir", str(self.root), "--json", str(out_json)])
        self.assertEqual(code, 0)
        self.assert_no_secret(stdout.getvalue() + out_json.read_text(encoding="utf-8"))

    def test_missing_folder_is_an_error(self):
        stderr = io.StringIO()
        with redirect_stderr(stderr):
            self.assertEqual(inv.main(["--logs-dir", str(self.root / "nope")]), 2)


class OnlyRecurringValuesTest(unittest.TestCase):
    """Outside Power.log, what shows up in one session only is folded away."""

    def setUp(self):
        self.root = Path(self.enterContext(tempfile.TemporaryDirectory()))

    def two_sessions(self, kind, first, second):
        write_folder(self.root, {kind: first}, "Hearthstone_a")
        write_folder(self.root, {kind: second}, "Hearthstone_b")
        return inv.inventory(self.root)

    def test_shapes_hide_sensitive_values_and_one_off_capitals(self):
        common = f"{T}LoadingScreen.OnSceneLoaded() - prevMode=HUB nextMode=GAMEPLAY code=SECRETCODE\n"
        once = f"{T}LoadingScreen.OnSceneLoaded() - state=ZORRO playerName=ANDY\n"
        result = self.two_sessions("LoadingScreen.log", common + once, common)
        text = json.dumps(result)
        for secret in ("SECRETCODE", "ZORRO", "ANDY"):
            self.assertNotIn(secret, text)
        shapes = result["file_kinds"]["LoadingScreen.log"]["line_kinds"]["LoadingScreen.OnSceneLoaded"]["shapes"]
        self.assertEqual(shapes, {"prevMode=HUB nextMode=GAMEPLAY code=<w>": 2, inv.RARE: 1})

    def test_one_off_kinds_and_keys_of_sensitive_files_are_folded(self):
        once = f"alice.smith said hello\n[Alice] hi\n{T}Net.Chat() - Bobby_1=5 Carol=7\n"
        common = f"{T}Network.Connect() - port=1\n"
        result = self.two_sessions("Net.log", once + common, common)
        text = json.dumps(result)
        for secret in ("alice", "Alice", "Bobby", "Carol", "Chat"):
            self.assertNotIn(secret, text)
        kinds = result["file_kinds"]["Net.log"]["line_kinds"]
        self.assertEqual(kinds["Network.Connect"], {"lines": 2, "keys": {"port": 2}})
        self.assertEqual(kinds[inv.RARE], {"lines": 3})

    def test_card_ids_need_a_set_code(self):
        for card in ("BG36_345", "TB_BaconShop_HERO_18", "Bacon_TagTransferPlayerE", "PET_3_4", "BGS_034"):
            self.assertTrue(inv.CARD_ID.match(card), card)
        for name in ("xX_Slayer_99", "Fake_Player1", "alice_1"):
            self.assertIsNone(inv.CARD_ID.match(name), name)


class PowerEdgeCaseTest(unittest.TestCase):
    def setUp(self):
        self.root = Path(self.enterContext(tempfile.TemporaryDirectory()))

    def test_rotated_log_is_read_before_the_current_one(self):
        write_folder(self.root, {"Power.log": "", "Power_old.log": "", "Net.log": ""})
        self.assertEqual([p.name for p in inv.log_files(self.root)], ["Net.log", "Power_old.log", "Power.log"])

    def test_names_are_not_guessed_without_the_players_list(self):
        text = "".join(line for line in game_log().splitlines(True) if "PlayerName=" not in line)
        tags = inv.inventory(write_folder(self.root, {"Power.log": text}))["power"]["all"]["tags"]
        self.assertEqual(tags["CORPSES"]["entity_types"], {"UNRESOLVED": 1})

    def test_unparsed_entity_headers_are_counted(self):
        text = game_log() + power("FULL_ENTITY - Something new ID=99")
        section = inv.inventory(write_folder(self.root, {"Power.log": text}))["power"]["all"]
        self.assertEqual(section["line_sub_kinds"]["<unparsed entity header>"], 1)

    def test_large_tag_values_are_only_counted(self):
        values = inv.Values()
        values.add("12345678901")
        self.assertEqual(values.to_json(), {"count": 1, "large_numbers": 1})


class ErrorTest(unittest.TestCase):
    def setUp(self):
        self.root = Path(self.enterContext(tempfile.TemporaryDirectory()))
        write_folder(self.root, {"Power.log": game_log()})

    def run_main(self, *args):
        stdout, stderr = io.StringIO(), io.StringIO()
        with redirect_stdout(stdout), redirect_stderr(stderr):
            code = inv.main(["--logs-dir", str(self.root), *args])
        return code, stdout.getvalue(), stderr.getvalue()

    def test_unwritable_output_is_exit_4_without_the_path(self):
        code, stdout, stderr = self.run_main("--json", str(self.root))
        self.assertEqual((code, stdout), (4, ""))
        self.assertNotIn(str(self.root), stderr)

    def test_unreadable_log_is_exit_4_without_the_path(self):
        original = inv.inventory

        def fail(*_):
            raise PermissionError(13, "denied", FAKE_PATH)
        inv.inventory = fail
        try:
            code, stdout, stderr = self.run_main()
        finally:
            inv.inventory = original
        self.assertEqual((code, stdout), (4, ""))
        self.assertNotIn("fakeuser", stderr)


class GuardTest(unittest.TestCase):
    def setUp(self):
        self.guard = inv.Guard()
        self.guard.add_name(FAKE_TAG)
        self.guard.add_name(FAKE_BOB)
        self.guard.add_number(FAKE_LO)

    def refuses(self, obj):
        with self.assertRaises(inv.PrivacyRefusal):
            self.guard.check(obj)

    def test_refuses_names_read_from_the_logs(self):
        self.refuses({"shapes": {"<w> Fakeplayer <w>": 1}})
        self.refuses({"FAKEPLAYER": 1})
        self.refuses(["innkeeperfake"])

    def test_refuses_battletags_ips_emails_paths_and_account_ids(self):
        self.refuses({"x": "Other#12345"})
        self.refuses({"x": FAKE_IP})
        self.refuses({"x": "2001:db8:0:1::5"})
        self.refuses({"x": FAKE_EMAIL})
        self.refuses({"x": FAKE_PATH})
        self.refuses({"x": "GameAccountId=[hi=1 lo=2]"})
        self.refuses({"x": "id 123456789"})
        self.refuses({"max": int(FAKE_LO)})
        self.refuses({"x": "fe80::1"})

    def test_refuses_deck_codes(self):
        self.refuses({"x": FAKE_DECK_CODE})

    def test_allows_tags_enums_card_ids_and_counts(self):
        self.guard.check({"tags": {"PLAYER_LEADERBOARD_PLACE": {"count": 12, "min": -1, "max": 253216,
                                                                "card_ids": {"BG20_HERO_202_SKIN_C": 3},
                                                                "enums": {"BEAST": 2}}},
                          "shape": "prevMode=HUB nextMode=<w…> <n>"})

    def test_scrub_withholds_unsafe_keys_texts_and_numbers(self):
        clean = self.guard.scrub({"keys": {"fakeplayer": 2, "prevMode": 1},
                                  "values": [FAKE_IP, "BEAST", int(FAKE_LO), 7]})
        self.assertEqual(clean, {"keys": {"<withheld>0": 2, "prevMode": 1},
                                 "values": ["<withheld>", "BEAST", "<withheld>", 7]})
        self.assertEqual(self.guard.withheld, {"player name": 1, "ipv4": 1, "account number": 1})
        self.guard.check(clean)

    def test_value_equal_to_an_account_id_is_withheld(self):
        root = Path(self.enterContext(tempfile.TemporaryDirectory()))
        short_lo = FAKE_LO[:7]
        text = game_log().replace("tag=HEALTH value=30", f"tag=HEALTH value={short_lo}")
        write_folder(root, {"Power.log": text.replace(f"lo={FAKE_LO}", f"lo={short_lo}")})
        result = inv.inventory(root)
        self.assertGreaterEqual(result["withheld"].get("account number", 0), 1)
        self.assertNotIn(short_lo, json.dumps(result))

    def test_main_refuses_and_prints_nothing_if_the_final_check_fails(self):
        root = Path(self.enterContext(tempfile.TemporaryDirectory()))
        short_lo = FAKE_LO[:7]
        text = game_log().replace("tag=HEALTH value=30", f"tag=HEALTH value={short_lo}")
        write_folder(root, {"Power.log": text.replace(f"lo={FAKE_LO}", f"lo={short_lo}")})
        original = inv.Guard.scrub
        inv.Guard.scrub = lambda self, obj: obj  # a scrub that misses everything
        try:
            stdout, stderr = io.StringIO(), io.StringIO()
            with redirect_stdout(stdout), redirect_stderr(stderr):
                code = inv.main(["--logs-dir", str(root)])
        finally:
            inv.Guard.scrub = original
        self.assertEqual(code, 3)
        self.assertEqual(stdout.getvalue(), "")
        self.assertIn("Refused", stderr.getvalue())
        self.assert_no_secret_in(stderr.getvalue())

    def assert_no_secret_in(self, text):
        for secret in SECRETS:
            self.assertNotIn(secret.casefold(), text.casefold(), secret)


class ShapeTest(unittest.TestCase):
    def test_values_become_placeholders(self):
        self.assertEqual(inv.shape("Fakeplayer joined at 12 from host"), "<w…> <n> <w…>")
        self.assertEqual(inv.shape("state=LOADED id=3"), "state=LOADED id=<n>")

    def test_entity_references_are_hidden(self):
        ref = "[entityName=Some Card id=5 zone=PLAY zonePos=1 cardId=BG_X_1 player=1]"
        self.assertEqual(inv.shape(f"target {ref}"), "<w> <entity>")

    def test_line_kind(self):
        self.assertEqual(inv.line_kind("Net.Connect() - x=1"), ("Net.Connect", "x=1"))
        self.assertEqual(inv.line_kind("[Login] hello"), ("[Login]", " hello"))
        self.assertEqual(inv.line_kind("Fakeplayer said hi")[0], "<unstructured>")

    def test_file_kind_merges_old_files(self):
        self.assertEqual(inv.file_kind("Power_old.log"), "Power.log")
        self.assertEqual(inv.file_kind("Achievements.log"), "Achievements.log")


if __name__ == "__main__":
    unittest.main()
