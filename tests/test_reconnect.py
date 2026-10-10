"""Tests for tools/dev/reconnect.py and reconnect_marks.py (T-D01, D-043).

The Windows calls are replaced by a fake; one test reads the real connection
table (read-only, no admin) on Windows. Nothing here drops a connection.
"""

import contextlib
import io
import ipaddress
import json
import os
import socket
import struct
import sys
import tempfile
import unittest
from datetime import datetime, timedelta, timezone
from pathlib import Path
from unittest import mock

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools" / "dev"))

import reconnect  # noqa: E402
import reconnect_marks as marks  # noqa: E402

FIXTURE = ROOT / "crates" / "bg-parser" / "tests" / "data" / "solo_game.log"
SESSION = "Hearthstone_2026_10_09_10_00_00"
UTC = timezone.utc


def raw_addr(text):
    return struct.unpack("<I", ipaddress.IPv4Address(text).packed)[0]


def raw_port(port):
    return socket.htons(port)


def row(remote="203.0.113.5", rport=3724, pid=42, state=reconnect.MIB_TCP_STATE_ESTAB, lport=50000):
    return reconnect.row_from_raw(state, raw_addr("192.168.1.10"), raw_port(lport),
                                  raw_addr(remote), raw_port(rport), pid)


class FakeApi:
    def __init__(self, pids=frozenset({42}), rows=(), ipv6=0, codes=None, data_dir=None):
        self.pids, self.rows, self.ipv6 = set(pids), list(rows), ipv6
        self.codes = codes or {}
        self.deleted = []
        self.data_dir = data_dir
        self.file_lines_at_delete = []

    def find_pids(self, name):
        assert name == "Hearthstone.exe"
        return self.pids

    def tcp_rows(self):
        return self.rows

    def count_ipv6(self, pids):
        return self.ipv6

    def delete(self, r):
        if self.data_dir:
            path = self.data_dir / marks.FILE_NAME
            self.file_lines_at_delete.append(len(path.read_text().splitlines()) if path.exists() else 0)
        self.deleted.append(r)
        return self.codes.get(r.remote_port, 0)


class TempDirTest(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.dir = Path(tmp.name)


class HotkeyTest(unittest.TestCase):
    def test_default_and_custom_hotkeys(self):
        self.assertEqual(reconnect.parse_hotkey("ctrl+alt+f9"),
                         (reconnect.MOD_CONTROL | reconnect.MOD_ALT, 0x78))
        self.assertEqual(reconnect.parse_hotkey(" Ctrl + Shift + R "),
                         (reconnect.MOD_CONTROL | reconnect.MOD_SHIFT, ord("R")))
        self.assertEqual(reconnect.parse_hotkey("alt+0"), (reconnect.MOD_ALT, ord("0")))

    def test_hotkeys_that_could_fire_by_accident_or_are_reserved_are_refused(self):
        for bad in ["f9", "shift+f9", "ctrl+alt+f12", "win+f9", "ctrl+alt+space", "ctrl+", ""]:
            with self.assertRaises(reconnect.UsageError, msg=bad):
                reconnect.parse_hotkey(bad)


class RowTest(unittest.TestCase):
    def test_addresses_and_ports_are_read_from_the_tables_byte_order(self):
        r = row(remote="203.0.113.5", rport=3724, lport=50001)
        self.assertEqual(str(r.remote), "203.0.113.5")
        self.assertEqual(r.remote_port, 3724)
        self.assertEqual(r.local_port, 50001)

    def test_junk_in_the_high_bits_of_a_port_is_ignored(self):
        r = reconnect.row_from_raw(5, 0, raw_port(80) | 0xABCD0000, 0, raw_port(443), 1)
        self.assertEqual((r.local_port, r.remote_port), (80, 443))

    def test_only_the_games_open_remote_connections_are_targets(self):
        rows = [
            row(),
            row(pid=7),  # another program
            row(remote="127.0.0.1"),  # local helper
            row(state=2),  # listening
            row(remote="0.0.0.0"),
            row(rport=1119),
        ]
        targets = reconnect.game_targets(rows, {42}, None)
        self.assertEqual([t.remote_port for t in targets], [3724, 1119])
        only = reconnect.game_targets(rows, {42}, {1119})
        self.assertEqual([t.remote_port for t in only], [1119])


class DropTest(TempDirTest):
    NOW = datetime(2026, 10, 9, 9, 0, 10, tzinfo=UTC)

    def drop(self, api, ports=None):
        return reconnect.drop_once(api, self.dir, ports, now=lambda: self.NOW)

    def lines(self):
        path = self.dir / marks.FILE_NAME
        return path.read_text().splitlines() if path.exists() else []

    def test_a_press_drops_the_games_connections_and_records_utc_time_only(self):
        api = FakeApi(rows=[row(), row(pid=7), row(rport=1119)], data_dir=self.dir)
        result = self.drop(api)
        self.assertEqual([r.remote_port for r in api.deleted], [3724, 1119])
        self.assertEqual(self.lines(), ['{"utc": "2026-10-09T09:00:10Z"}'])
        self.assertEqual(api.file_lines_at_delete, [1, 1], "recorded before any reset")
        self.assertEqual(result.message(), "Dropped 2 connection(s).")

    def test_without_the_game_running_nothing_is_dropped_or_recorded(self):
        api = FakeApi(pids=set(), rows=[row()])
        result = self.drop(api)
        self.assertEqual(api.deleted, [])
        self.assertEqual(self.lines(), [])
        self.assertIn("not running", result.message())

    def test_with_no_open_connection_nothing_is_recorded(self):
        api = FakeApi(rows=[row(remote="127.0.0.1")], ipv6=1)
        result = self.drop(api)
        self.assertEqual(self.lines(), [])
        self.assertIn("No open connection", result.message())

    def test_failures_and_ipv6_connections_are_reported(self):
        api = FakeApi(rows=[row(), row(rport=1119)], ipv6=2, codes={1119: 5})
        text = self.drop(api).message()
        self.assertIn("Dropped 1", text)
        self.assertIn("1 could not be dropped (error 5)", text)
        self.assertIn("2 IPv6", text)
        self.assertEqual(len(self.lines()), 1, "a failed reset is still recorded: the game may be affected")


class CooldownTest(unittest.TestCase):
    def test_presses_inside_the_cooldown_wait(self):
        now = [100.0]
        c = reconnect.Cooldown(10, clock=lambda: now[0])
        self.assertEqual(c.left(), 0)
        c.use()
        now[0] = 104
        self.assertAlmostEqual(c.left(), 6)
        now[0] = 110
        self.assertEqual(c.left(), 0)


class MainTest(TempDirTest):
    def run_main(self, argv):
        out, err = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            code = reconnect.main(argv)
        return code, out.getvalue(), err.getvalue()

    def test_off_windows_it_refuses(self):
        with mock.patch.object(reconnect.sys, "platform", "linux"):
            code, _, err = self.run_main([])
        self.assertEqual(code, 2)
        self.assertIn("only runs on Windows", err)

    def test_without_admin_rights_it_refuses_and_does_nothing(self):
        fake = mock.Mock()
        fake.is_admin.return_value = False
        with mock.patch.object(reconnect.sys, "platform", "win32"), \
                mock.patch.object(reconnect, "Win32", return_value=fake):
            code, out, err = self.run_main(["--data-dir", str(self.dir)])
        self.assertEqual(code, 2)
        self.assertIn("Run as administrator", err)
        fake.register_hotkey.assert_not_called()
        fake.delete.assert_not_called()
        self.assertNotIn("DEV TOOL", out)
        self.assertEqual(list(self.dir.iterdir()), [])

    def test_bad_options_are_refused_before_any_windows_call(self):
        with mock.patch.object(reconnect.sys, "platform", "win32"), \
                mock.patch.object(reconnect, "Win32") as win:
            for argv in (["--hotkey", "f9"], ["--cooldown", "0.5"], ["--remote-port", "0"]):
                self.assertEqual(self.run_main(argv)[0], 2, argv)
        win.assert_not_called()

    def test_with_admin_it_shows_the_risk_banner_then_waits_for_the_hotkey(self):
        fake = mock.Mock()
        fake.is_admin.return_value = True
        fake.hotkey_pressed.side_effect = KeyboardInterrupt
        with mock.patch.object(reconnect.sys, "platform", "win32"), \
                mock.patch.object(reconnect, "Win32", return_value=fake):
            code, out, _ = self.run_main(["--data-dir", str(self.dir)])
        self.assertEqual(code, 0)
        self.assertIn("RISK FOR YOUR BATTLE.NET ACCOUNT", out)
        fake.register_hotkey.assert_called_once_with(
            reconnect.MOD_CONTROL | reconnect.MOD_ALT, 0x78)
        fake.unregister_hotkey.assert_called_once()
        fake.delete.assert_not_called()


@unittest.skipUnless(sys.platform == "win32", "Windows only")
class RealTableTest(unittest.TestCase):
    def test_the_real_table_shows_a_local_connection_of_this_process(self):
        api = reconnect.Win32()
        with socket.socket() as server:
            server.bind(("127.0.0.1", 0))
            server.listen(1)
            port = server.getsockname()[1]
            with socket.create_connection(("127.0.0.1", port)) as client, server.accept()[0]:
                local_port = client.getsockname()[1]
                mine = [r for r in api.tcp_rows()
                        if r.pid == os.getpid() and r.local_port == local_port]
        self.assertEqual(len(mine), 1)
        self.assertEqual((str(mine[0].remote), mine[0].remote_port), ("127.0.0.1", port))
        self.assertEqual(reconnect.game_targets(mine, {os.getpid()}, None), [], "loopback is never a target")
        self.assertIn(os.getpid(), api.find_pids(Path(sys.executable).name))


def at(h, m, s=0):
    return datetime(2026, 10, 9, h, m, s, tzinfo=UTC)


def utc_plus_2(naive):
    return (naive - timedelta(hours=2)).replace(tzinfo=UTC)


def two_games_log():
    text = FIXTURE.read_text(encoding="utf-8")
    return text.replace("D 12:00:00.", "D 11:00:00.") + text.replace("D 12:00:00.", "D 12:30:00.")


class SpansTest(unittest.TestCase):
    def test_games_span_from_create_game_to_their_last_line_like_the_tracker(self):
        spans = marks.game_spans(two_games_log().splitlines(), SESSION, utc_plus_2)
        self.assertEqual([(s.index, s.start, s.end) for s in spans],
                         [(1, at(9, 0), at(9, 0)), (2, at(10, 30), at(10, 30))])

    def test_midnight_and_a_session_without_a_date(self):
        lines = [f"D 23:55:00.0 {marks.CREATE_GAME}", "D 23:59:59.0 x", "D 00:10:00.0 x"]
        span = marks.game_spans(lines, "Hearthstone_2026_10_10_23_50_00", lambda n: n.replace(tzinfo=UTC))[0]
        self.assertEqual(span.end, datetime(2026, 10, 11, 0, 10, tzinfo=UTC))
        unknown = marks.game_spans(lines, "replay", utc_plus_2)[0]
        self.assertIsNone(unknown.start)

    def test_marked_within_the_margin_only(self):
        span = marks.GameSpan(1, at(9, 0), at(9, 20))
        self.assertTrue(marks.is_marked(span, [at(9, 10)]))
        self.assertTrue(marks.is_marked(span, [at(9, 20, 25)]))
        self.assertFalse(marks.is_marked(span, [at(9, 21)]))
        self.assertFalse(marks.is_marked(marks.GameSpan(1, None, None), [at(9, 10)]))


class UsesFileTest(TempDirTest):
    def test_appended_uses_are_read_back(self):
        marks.append_use(self.dir, at(9, 0, 10))
        marks.append_use(self.dir, datetime(2026, 10, 9, 11, 0, 10, tzinfo=timezone(timedelta(hours=2))))
        uses = marks.read_uses(self.dir)
        self.assertEqual(uses.times, [at(9, 0, 10), at(9, 0, 10)])
        self.assertIsNone(uses.warning)
        for line in (self.dir / marks.FILE_NAME).read_text().splitlines():
            self.assertEqual(list(json.loads(line)), ["utc"], "UTC time and nothing else")

    def test_missing_file_is_no_uses_and_no_warning(self):
        self.assertEqual(marks.read_uses(self.dir), marks.Uses([]))

    def test_bad_lines_are_skipped_with_a_warning(self):
        (self.dir / marks.FILE_NAME).write_text(
            'nope\n{"utc": "2026-02-30T00:00:00Z"}\n[1]\n{"utc": "2026-10-09T09:00:10Z"}\n')
        uses = marks.read_uses(self.dir)
        self.assertEqual(uses.times, [at(9, 0, 10)])
        self.assertTrue(uses.warning.startswith("3 line(s)"))

    def test_unreadable_file_is_no_uses_with_a_warning(self):
        (self.dir / marks.FILE_NAME).mkdir()
        uses = marks.read_uses(self.dir)
        self.assertEqual(uses.times, [])
        self.assertIn("Could not read", uses.warning)


class RefuseTest(TempDirTest):
    def setUp(self):
        super().setUp()
        self.data = self.dir / "data"
        session_dir = self.dir / SESSION
        session_dir.mkdir()
        self.log = session_dir / "Power.log"
        self.log.write_text(two_games_log(), encoding="utf-8")

    def refuse(self, **kw):
        return marks.refuse_marked_log(self.log, data_dir=self.data, to_utc=utc_plus_2, **kw)

    def test_a_log_with_a_marked_game_is_refused(self):
        marks.append_use(self.data, at(10, 30, 5))
        with self.assertRaisesRegex(marks.MarkedLogError, r"game\(s\) \[2\]"):
            self.refuse()

    def test_a_log_with_no_marked_game_passes(self):
        marks.append_use(self.data, at(10, 0))
        self.assertEqual(self.refuse(), [])

    def test_no_uses_file_means_every_log_passes(self):
        self.assertEqual(self.refuse(session="not-a-session"), [])

    def test_a_log_whose_time_cannot_be_told_is_refused_while_uses_exist(self):
        marks.append_use(self.data, at(10, 0))
        with self.assertRaisesRegex(marks.MarkedLogError, "cannot be told"):
            self.refuse(session="copied-log")

    def test_command_line_exit_codes(self):
        marks.append_use(self.data, at(9, 0, 5))
        out, err = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err), \
                mock.patch.object(marks, "local_to_utc", utc_plus_2):
            code = marks.main([str(self.log), "--data-dir", str(self.data)])
        self.assertEqual(code, 1)
        self.assertIn("reconnect dev tool", err.getvalue())


if __name__ == "__main__":
    unittest.main()
