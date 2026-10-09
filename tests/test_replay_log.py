"""Tests for tools/dev/replay_log.py with synthetic logs (T-D02)."""

import contextlib
import io
import re
import sys
import tempfile
import unittest
from datetime import datetime
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools" / "dev"))

import replay_log  # noqa: E402

FIXTURE = ROOT / "crates" / "bg-parser" / "tests" / "data" / "duo_game.log"
SESSION_RE = re.compile(r"^Hearthstone_\d{4}_\d{2}_\d{2}_\d{2}_\d{2}_\d{2}$")


class ReplayTest(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.dir = Path(tmp.name)

    def test_session_folder_name_matches_what_the_tracker_follows(self):
        name = replay_log.session_folder_name(datetime(2026, 10, 9, 5, 6, 7))
        self.assertEqual(name, "Hearthstone_2026_10_09_05_06_07")
        self.assertRegex(name, SESSION_RE)

    def test_copy_is_identical_to_the_source(self):
        logs = self.dir / "Logs"
        copied = replay_log.replay(FIXTURE, logs, "Hearthstone_2026_10_09_05_06_07")
        target = logs / "Hearthstone_2026_10_09_05_06_07" / "Power.log"
        self.assertEqual(target.read_bytes(), FIXTURE.read_bytes())
        self.assertEqual(copied, len(FIXTURE.read_bytes().splitlines()))

    def test_last_line_without_newline_is_completed(self):
        source = self.dir / "src.log"
        source.write_bytes(b"one\ntwo")
        replay_log.replay(source, self.dir / "Logs", "S")
        self.assertEqual((self.dir / "Logs" / "S" / "Power.log").read_bytes(), b"one\ntwo\n")

    def test_speed_is_paced_by_lines_per_second(self):
        source = self.dir / "src.log"
        source.write_bytes(b"x\n" * 100)
        now = [0.0]
        sleeps = []

        def sleep(seconds):
            sleeps.append(seconds)
            now[0] += seconds

        # 100 lines at 100 lines/s must take about one second, in several steps.
        replay_log.replay(
            source, self.dir / "Logs", "S", 100, sleep=sleep, clock=lambda: now[0]
        )
        self.assertAlmostEqual(sum(sleeps), 1.0, places=6)
        self.assertGreater(len(sleeps), 1)

    def test_no_pacing_by_default(self):
        source = self.dir / "src.log"
        source.write_bytes(b"x\n" * 100)
        replay_log.replay(
            source, self.dir / "Logs", "S", sleep=lambda s: self.fail("slept")
        )

    def test_main_prints_paths_and_counts_but_never_the_log_content(self):
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            code = replay_log.main([str(FIXTURE), "--dest-dir", str(self.dir)])
        self.assertEqual(code, 0)
        text = out.getvalue()
        self.assertIn("Done:", text)
        for line in FIXTURE.read_text(encoding="utf-8").splitlines()[:50]:
            self.assertNotIn(line, text)
        sessions = [p.name for p in (self.dir / "Logs").iterdir()]
        self.assertEqual(len(sessions), 1)
        self.assertRegex(sessions[0], SESSION_RE)

    def test_missing_source_is_an_error(self):
        err = io.StringIO()
        with contextlib.redirect_stderr(err):
            code = replay_log.main([str(self.dir / "nope.log")])
        self.assertEqual(code, 2)

    def test_session_name_cannot_leave_the_destination(self):
        err = io.StringIO()
        with contextlib.redirect_stderr(err):
            code = replay_log.main(
                [str(FIXTURE), "--dest-dir", str(self.dir), "--session-name", "../x"]
            )
        self.assertEqual(code, 2)

    def test_refuses_to_write_inside_the_game_folder(self):
        original = replay_log.check_logs.find_install_dir
        replay_log.check_logs.find_install_dir = lambda: self.dir / "Hearthstone"
        self.addCleanup(setattr, replay_log.check_logs, "find_install_dir", original)
        (self.dir / "Hearthstone").mkdir()
        err = io.StringIO()
        with contextlib.redirect_stderr(err):
            code = replay_log.main(
                [str(FIXTURE), "--dest-dir", str(self.dir / "Hearthstone" / "x")]
            )
        self.assertEqual(code, 2)
        self.assertFalse((self.dir / "Hearthstone" / "x").exists())


if __name__ == "__main__":
    unittest.main()
