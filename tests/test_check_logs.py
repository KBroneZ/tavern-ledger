"""Tests de tools/check_logs.py con datos sintéticos (nada del juego real)."""

import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "tools"))

import check_logs  # noqa: E402

POWER_OK = """[Power]
LogLevel=1
FilePrinting=true
ConsolePrinting=false
ScreenPrinting=false
Verbose=true
[Decks]
LogLevel=1
FilePrinting=True
"""


class ParseLogConfigTest(unittest.TestCase):
    def test_parses_sections_and_keys(self):
        cfg = check_logs.parse_log_config(POWER_OK)
        self.assertEqual(cfg["Power"]["LogLevel"], "1")
        self.assertEqual(cfg["Decks"]["FilePrinting"], "True")

    def test_ignores_blank_lines_comments_and_spaces(self):
        cfg = check_logs.parse_log_config("\n; comment\n[Power]\n  Verbose = true  \n")
        self.assertEqual(cfg["Power"]["Verbose"], "true")

    def test_keys_outside_a_section_are_ignored(self):
        cfg = check_logs.parse_log_config("LogLevel=1\n[Power]\nLogLevel=1\n")
        self.assertEqual(list(cfg), ["Power"])


class PowerProblemsTest(unittest.TestCase):
    def test_no_problems_when_power_is_enabled(self):
        cfg = check_logs.parse_log_config(POWER_OK)
        self.assertEqual(check_logs.power_problems(cfg), [])

    def test_missing_power_section(self):
        cfg = check_logs.parse_log_config("[Decks]\nLogLevel=1\n")
        self.assertEqual(check_logs.power_problems(cfg), ["Missing [Power] section"])

    def test_values_are_case_insensitive(self):
        cfg = check_logs.parse_log_config(POWER_OK.replace("true", "TRUE"))
        self.assertEqual(check_logs.power_problems(cfg), [])

    def test_reports_each_wrong_or_missing_key(self):
        cfg = check_logs.parse_log_config("[Power]\nLogLevel=0\nFilePrinting=false\n")
        self.assertEqual(
            check_logs.power_problems(cfg),
            [
                "[Power] LogLevel=0 (expected 1)",
                "[Power] FilePrinting=false (expected true)",
                "[Power] Verbose missing (expected true)",
            ],
        )


class SessionStartTest(unittest.TestCase):
    def test_parses_session_folder_name(self):
        self.assertEqual(
            check_logs.session_start("Hearthstone_2026_10_08_00_05_26"),
            "2026-10-08 00:05:26",
        )

    def test_unknown_folder_name_returns_none(self):
        self.assertIsNone(check_logs.session_start("Something_else"))


class ScanPowerLogTest(unittest.TestCase):
    def _write(self, text):
        tmp = tempfile.NamedTemporaryFile("w", suffix=".log", delete=False, encoding="utf-8")
        tmp.write(text)
        tmp.close()
        self.addCleanup(Path(tmp.name).unlink)
        return Path(tmp.name)

    def test_counts_game_types_and_builds(self):
        path = self._write(
            "D 00:09:50.85 GameState.DebugPrintGame() - BuildNumber=253216\n"
            "D 00:09:50.85 GameState.DebugPrintGame() - GameType=GT_BATTLEGROUNDS_DUO\n"
            "D 00:09:50.85 GameState.DebugPrintGame() - PlayerID=1, PlayerName=Someone#1234\n"
            "D 00:30:00.00 GameState.DebugPrintGame() - BuildNumber=253216\n"
            "D 00:30:00.00 GameState.DebugPrintGame() - GameType=GT_BATTLEGROUNDS\n"
            "D 00:31:00.00 PowerTaskList.DebugPrintPower() - GameType=GT_RANKED\n"
        )
        result = check_logs.scan_power_log(path)
        self.assertEqual(result.game_types, {"GT_BATTLEGROUNDS_DUO": 1, "GT_BATTLEGROUNDS": 1})
        self.assertEqual(result.builds, {"253216"})

    def test_empty_file_has_no_games(self):
        result = check_logs.scan_power_log(self._write(""))
        self.assertEqual(result.game_types, {})
        self.assertEqual(result.builds, set())


class InventoryTest(unittest.TestCase):
    def test_lists_power_logs_per_session_sorted(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            old = root / "Hearthstone_2026_10_02_15_44_55"
            new = root / "Hearthstone_2026_10_08_00_05_26"
            empty = root / "Hearthstone_2026_10_05_03_54_53"
            for folder in (old, new, empty):
                folder.mkdir()
            (old / "Power_old.log").write_text(
                "D 1 GameState.DebugPrintGame() - GameType=GT_BATTLEGROUNDS_DUO\n", encoding="utf-8"
            )
            (new / "Power.log").write_text("", encoding="utf-8")
            (new / "Decks.log").write_text("ignored", encoding="utf-8")

            sessions = check_logs.inventory(root)

        self.assertEqual([s.name for s in sessions], [old.name, empty.name, new.name])
        self.assertEqual([f.name for f in sessions[0].power_logs], ["Power_old.log"])
        self.assertEqual(sessions[0].power_logs[0].scan.game_types, {"GT_BATTLEGROUNDS_DUO": 1})
        self.assertEqual(sessions[1].power_logs, ())
        self.assertEqual([f.name for f in sessions[2].power_logs], ["Power.log"])

    def test_unreadable_log_is_reported_not_raised(self):
        def boom(path):
            raise PermissionError("locked")

        with tempfile.TemporaryDirectory() as tmp:
            session = Path(tmp) / "Hearthstone_2026_10_08_00_05_26"
            session.mkdir()
            (session / "Power.log").write_text("x", encoding="utf-8")
            original = check_logs.scan_power_log
            check_logs.scan_power_log = boom
            self.addCleanup(setattr, check_logs, "scan_power_log", original)

            sessions = check_logs.inventory(Path(tmp))

        log = sessions[0].power_logs[0]
        self.assertIsNone(log.scan)
        self.assertIn("locked", log.error)
        report = check_logs.format_report(None, None, Path(tmp), sessions)
        self.assertIn("Power.log: unreadable (locked)", report)

    def test_missing_logs_dir_returns_empty(self):
        self.assertEqual(check_logs.inventory(Path("does/not/exist")), [])


if __name__ == "__main__":
    unittest.main()
