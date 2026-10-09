"""Tests for tools/leaderboard.py with synthetic responses (made-up names, no network)."""

import contextlib
import gzip
import http.server
import io
import json
import socket
import sys
import tempfile
import threading
import unittest
from datetime import datetime, timedelta, timezone
from pathlib import Path
from urllib.parse import parse_qs, urlparse

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "tools"))

import leaderboard as lb  # noqa: E402

NOW = datetime(2026, 10, 9, 12, 0, 0, tzinfo=timezone.utc)
ME = "TestHero"
FAST = lb.Limits(pause=1.0, backoff=(5.0, 15.0))


def board_rows(count, me_at=None, top=9000, step=10, extra_me_at=None):
    """`count` invented rows in descending rating; ME at 1-based rank `me_at`."""
    rows = []
    for i in range(count):
        rank = i + 1
        name = ME if rank in (me_at, extra_me_at) else f"Filler{rank:04d}"
        rows.append((name, max(top - i * step, 8000)))
    return rows


def page_body(rows, page, season=19, region="EU", **overrides):
    total_size = len(rows)
    total_pages = -(-total_size // lb.PAGE_SIZE)
    start = (page - 1) * lb.PAGE_SIZE
    chunk = rows[start:start + lb.PAGE_SIZE] if page <= total_pages else []
    data = {
        "seasonId": season,
        "region": region,
        "displayMetaData": {"ignored": True},
        "leaderboard": {
            "columns": ["rank", "accountid", "rating"],
            "rows": [
                {"rank": start + i + 1, "accountid": n, "rating": r}
                for i, (n, r) in enumerate(chunk)
            ],
            "pagination": {
                "totalPages": total_pages if chunk else 0,
                "totalSize": total_size if chunk else 0,
            },
        },
    }
    data.update(overrides)
    return json.dumps(data).encode()


class FakeServer:
    """Serves a synthetic board; `script` overrides the answer to the n-th call (0-based)."""

    def __init__(self, rows, season=19, script=None):
        self.rows = rows
        self.season = season
        self.script = dict(script or {})
        self.pages = []
        self.headers = []

    def __call__(self, url, headers, timeout):
        query = parse_qs(urlparse(url).query)
        page = int(query["page"][0])
        call = len(self.pages)
        self.pages.append(page)
        self.headers.append((headers, timeout))
        action = self.script.get(call)
        if isinstance(action, Exception):
            raise action
        if callable(action):
            return action(page)
        if isinstance(action, lb.Response):
            return action
        return lb.Response(200, page_body(self.rows, page, self.season))


def run(server, cache=None, now=NOW, name=ME, mode="solo", limits=FAST):
    sleeps = []
    result, new_cache = lb.lookup(
        name, "EU", mode, cache or {}, now, server, sleep=sleeps.append, limits=limits
    )
    return result, new_cache, sleeps


def entry(cache):
    return cache["boards"]["EU/battlegrounds"]


class FullScanTest(unittest.TestCase):
    def test_found_once_gives_rating_rank_and_cut(self):
        rows = board_rows(60, me_at=30)
        server = FakeServer(rows)
        result, _, _ = run(server)
        self.assertEqual(result.status, lb.RATING)
        self.assertEqual(result.rank, 30)
        self.assertEqual(result.rating, rows[29][1])
        self.assertEqual(result.cut, rows[-1][1])
        self.assertEqual(result.season, 19)
        # page 1, then 3, 2 and page 3 again as the consistency check
        self.assertEqual(server.pages, [1, 3, 2, 3])
        self.assertEqual(result.requests, 4)

    def test_not_found_after_every_page_is_below_cutoff(self):
        rows = board_rows(60)
        result, cache, _ = run(FakeServer(rows))
        self.assertEqual(result.status, lb.BELOW)
        self.assertIsNone(result.rating)
        self.assertEqual(result.cut, rows[-1][1])
        self.assertEqual(entry(cache)["full_scan_at"], NOW.isoformat(timespec="seconds"))

    def test_repeated_name_is_ambiguous_without_value(self):
        rows = board_rows(60, me_at=5, extra_me_at=55)
        result, _, _ = run(FakeServer(rows))
        self.assertEqual(result.status, lb.AMBIGUOUS)
        self.assertIsNone(result.rating)
        self.assertIsNone(result.rank)

    def test_single_page_board_needs_no_recheck(self):
        server = FakeServer(board_rows(10, me_at=3))
        result, _, _ = run(server)
        self.assertEqual(result.status, lb.RATING)
        self.assertEqual(server.pages, [1])

    def test_name_comparison_is_exact(self):
        rows = board_rows(30, me_at=3)
        result, _, _ = run(FakeServer(rows), name=ME.lower())
        self.assertEqual(result.status, lb.BELOW)

    def test_battletag_number_is_dropped(self):
        result, _, _ = run(FakeServer(board_rows(30, me_at=3)), name=ME + "#1234")
        self.assertEqual(result.status, lb.RATING)

    def test_duos_uses_its_own_board(self):
        server = FakeServer(board_rows(10, me_at=3))
        captured = []

        def transport(url, headers, timeout):
            captured.append(parse_qs(urlparse(url).query)["leaderboardId"][0])
            return server(url, headers, timeout)

        result, cache, _ = run(transport, mode="duos")
        self.assertEqual(captured, ["battlegroundsduo"])
        self.assertIn("EU/battlegroundsduo", cache["boards"])
        self.assertEqual(result.mode, "duos")

    def test_requests_are_paced_and_identified(self):
        server = FakeServer(board_rows(60, me_at=30))
        _, _, sleeps = run(server)
        self.assertEqual(sleeps, [1.0, 1.0, 1.0])
        headers, timeout = server.headers[0]
        self.assertEqual(headers["User-Agent"], lb.USER_AGENT)
        self.assertRegex(
            lb.USER_AGENT,
            r"^TavernLedger/\d+\.\d+\.\d+ \(\+https://github\.com/KBroneZ/tavern-ledger\)$",
        )
        self.assertEqual(headers["Accept-Encoding"], "gzip")
        self.assertNotIn("Cookie", headers)
        self.assertEqual(timeout, FAST.timeout)


class UnknownTest(unittest.TestCase):
    def assertUnknown(self, result):
        self.assertEqual(result.status, lb.UNKNOWN)
        self.assertIsNone(result.rating)
        self.assertIsNone(result.cut)

    def test_empty_page_inside_the_range_is_unknown_not_below(self):
        rows = board_rows(60)
        empty = lambda page: lb.Response(200, page_body(rows, 99))  # noqa: E731
        result, _, _ = run(FakeServer(rows, script={2: empty}))
        self.assertUnknown(result)

    def test_incomplete_scan_is_unknown(self):
        rows = board_rows(60)
        server = FakeServer(rows, script={2: lb.Response(404, b"")})
        result, _, _ = run(server)
        self.assertUnknown(result)
        self.assertEqual(server.pages, [1, 3, 2])

    def test_timeouts_are_retried_then_unknown(self):
        rows = board_rows(60)
        script = {1: lb.TransportError("timeout"), 2: lb.TransportError("timeout"),
                  3: lb.TransportError("timeout")}
        server = FakeServer(rows, script=script)
        result, _, sleeps = run(server)
        self.assertUnknown(result)
        self.assertEqual(server.pages, [1, 3, 3, 3])
        self.assertEqual(sleeps, [1.0, 5.0, 15.0])
        self.assertIn("timeout", result.note)

    def test_one_timeout_then_success_still_answers(self):
        rows = board_rows(60, me_at=30)
        server = FakeServer(rows, script={1: lb.TransportError("timeout")})
        result, _, _ = run(server)
        self.assertEqual(result.status, lb.RATING)
        self.assertEqual(result.requests, 5)

    def test_server_error_is_retried(self):
        server = FakeServer(board_rows(10, me_at=1), script={0: lb.Response(503, b"")})
        result, _, _ = run(server)
        self.assertEqual(result.status, lb.RATING)
        self.assertEqual(server.pages, [1, 1])

    def test_not_modified_is_unexpected_and_not_retried(self):
        server = FakeServer(board_rows(10), script={0: lb.Response(304, b"")})
        result, _, _ = run(server)
        self.assertUnknown(result)
        self.assertEqual(server.pages, [1])

    def test_rate_limit_stops_and_pauses_later_lookups(self):
        server = FakeServer(board_rows(60), script={1: lb.Response(429, b"")})
        result, cache, _ = run(server)
        self.assertUnknown(result)
        self.assertEqual(server.pages, [1, 3])
        later = FakeServer(board_rows(60))
        result2, _, _ = run(later, cache=cache, now=NOW + timedelta(minutes=30))
        self.assertEqual(later.pages, [])
        self.assertEqual(result2.status, lb.UNKNOWN)
        self.assertIn("rate", result2.note)

    def test_board_changing_during_scan_is_unknown(self):
        rows = board_rows(60)
        changed = board_rows(59)
        script = {3: lambda page: lb.Response(200, page_body(rows[:50] + changed[50:], page))}
        result, _, _ = run(FakeServer(rows, script=script))
        self.assertUnknown(result)
        self.assertIn("changed", result.note)

    def test_total_pages_changing_between_pages_is_unknown(self):
        rows = board_rows(60)
        bigger = board_rows(80)
        script = {2: lambda page: lb.Response(200, page_body(bigger, page))}
        result, _, _ = run(FakeServer(rows, script=script))
        self.assertUnknown(result)

    def test_empty_leaderboard_is_unknown(self):
        result, _, _ = run(FakeServer([]))
        self.assertUnknown(result)

    def test_too_many_pages_is_unknown(self):
        limits = lb.Limits(pause=1.0, max_pages=2)
        server = FakeServer(board_rows(60))
        result, _, _ = run(server, limits=limits)
        self.assertUnknown(result)
        self.assertEqual(server.pages, [1])

    def test_disordered_pages_are_unknown(self):
        rows = board_rows(50)
        rows[25] = (rows[25][0], 9999)  # first row of page 2 above the last row of page 1
        rows = rows[:25] + sorted(rows[25:], key=lambda r: -r[1])
        result, _, _ = run(FakeServer(rows))
        self.assertUnknown(result)


class ParsePageTest(unittest.TestCase):
    def setUp(self):
        self.rows = board_rows(30)

    def body(self, mutate, page=1):
        data = json.loads(page_body(self.rows, page))
        mutate(data)
        return json.dumps(data).encode()

    def assertRejected(self, body, page=1):
        with self.assertRaises(lb.LookupFailed):
            lb.parse_page(body, page, "EU")

    def test_valid_page(self):
        page = lb.parse_page(page_body(self.rows, 2), 2, "EU")
        self.assertEqual((page.total_pages, page.total_size, len(page.rows)), (2, 30, 5))
        self.assertEqual(page.rows[0].rank, 26)

    def test_out_of_range_page_is_empty(self):
        page = lb.parse_page(page_body(self.rows, 9), 9, "EU")
        self.assertEqual((page.rows, page.total_pages), ((), 0))

    def test_not_json(self):
        self.assertRejected(b"<html>maintenance</html>")
        self.assertRejected(b"\xff\xfe")
        self.assertRejected(b"[]")
        self.assertRejected(b"[" * 100_000 + b"]" * 100_000)

    def test_wrong_types(self):
        def rating_str(d):
            d["leaderboard"]["rows"][0]["rating"] = "9000"

        def rating_bool(d):
            d["leaderboard"]["rows"][0]["rating"] = True

        def name_int(d):
            d["leaderboard"]["rows"][0]["accountid"] = 5

        def name_empty(d):
            d["leaderboard"]["rows"][0]["accountid"] = ""

        def season_missing(d):
            del d["seasonId"]

        def rows_dict(d):
            d["leaderboard"]["rows"] = {}

        def no_pagination(d):
            del d["leaderboard"]["pagination"]

        for mutate in (rating_str, rating_bool, name_int, name_empty, season_missing,
                       rows_dict, no_pagination):
            with self.subTest(mutate.__name__):
                self.assertRejected(self.body(mutate))

    def test_ranks_must_be_contiguous(self):
        def skip(d):
            d["leaderboard"]["rows"][3]["rank"] = 5

        self.assertRejected(self.body(skip))

    def test_ratings_must_not_increase(self):
        def bump(d):
            d["leaderboard"]["rows"][3]["rating"] = 9999

        self.assertRejected(self.body(bump))

    def test_total_pages_must_match_total_size(self):
        def wrong(d):
            d["leaderboard"]["pagination"]["totalPages"] = 7

        self.assertRejected(self.body(wrong))

    def test_short_page_before_the_last_is_rejected(self):
        def short(d):
            d["leaderboard"]["rows"].pop()

        self.assertRejected(self.body(short))

    def test_other_region_is_rejected(self):
        self.assertRejected(page_body(self.rows, 1, region="US"))

    def test_empty_page_inside_range_is_rejected(self):
        def empty(d):
            d["leaderboard"]["rows"] = []

        self.assertRejected(self.body(empty))


class CacheStrategyTest(unittest.TestCase):
    def scanned(self, rows):
        _, cache, _ = run(FakeServer(rows))
        return cache

    def test_recent_check_makes_no_request(self):
        cache = self.scanned(board_rows(60, me_at=30))
        server = FakeServer(board_rows(60, me_at=30))
        result, _, _ = run(server, cache=cache, now=NOW + timedelta(minutes=2))
        self.assertEqual(server.pages, [])
        self.assertEqual(result.status, lb.RATING)
        self.assertEqual(result.requests, 0)
        self.assertEqual(result.checked_at, NOW.isoformat(timespec="seconds"))

    def test_known_rank_goes_straight_to_its_page(self):
        cache = self.scanned(board_rows(100, me_at=30))
        server = FakeServer(board_rows(100, me_at=35))
        result, _, _ = run(server, cache=cache, now=NOW + timedelta(hours=1))
        self.assertEqual(server.pages, [2])
        self.assertEqual((result.status, result.rank), (lb.RATING, 35))

    def test_known_rank_tries_neighbour_pages(self):
        cache = self.scanned(board_rows(100, me_at=30))
        server = FakeServer(board_rows(100, me_at=60))
        result, _, _ = run(server, cache=cache, now=NOW + timedelta(hours=1))
        self.assertEqual(server.pages, [2, 1, 3])
        self.assertEqual(result.rank, 60)

    def test_missed_window_falls_back_to_cached_value_when_scan_used_today(self):
        cache = self.scanned(board_rows(200, me_at=30))
        server = FakeServer(board_rows(200, me_at=190))
        result, _, _ = run(server, cache=cache, now=NOW + timedelta(hours=1))
        self.assertEqual(server.pages, [2, 1, 3, 4])
        self.assertEqual(result.status, lb.RATING)
        self.assertEqual(result.rank, 30)  # old value, with its old time
        self.assertEqual(result.checked_at, NOW.isoformat(timespec="seconds"))
        self.assertEqual(result.requests, 4)
        self.assertIn("full scan", result.note)

    def test_missed_window_runs_full_scan_on_a_new_day(self):
        cache = self.scanned(board_rows(200, me_at=30))
        server = FakeServer(board_rows(200, me_at=190))
        result, _, _ = run(server, cache=cache, now=NOW + timedelta(days=1))
        self.assertEqual(server.pages[:4], [2, 1, 3, 4])
        self.assertEqual(server.pages[4], 1)
        self.assertEqual((result.status, result.rank), (lb.RATING, 190))

    def test_below_cutoff_rechecks_only_once_a_day(self):
        cache = self.scanned(board_rows(60))
        server = FakeServer(board_rows(60, me_at=58))
        result, _, _ = run(server, cache=cache, now=NOW + timedelta(hours=3))
        self.assertEqual(server.pages, [])
        self.assertEqual(result.status, lb.BELOW)
        self.assertIn("full scan", result.note)
        result2, _, _ = run(server, cache=cache, now=NOW + timedelta(days=1))
        self.assertEqual((result2.status, result2.rank), (lb.RATING, 58))

    def test_season_change_forces_a_full_scan(self):
        cache = self.scanned(board_rows(100, me_at=30))
        server = FakeServer(board_rows(100, me_at=30), season=20)
        result, _, _ = run(server, cache=cache, now=NOW + timedelta(days=1))
        self.assertEqual(server.pages[0], 2)
        self.assertEqual(server.pages[1], 1)
        self.assertEqual((result.status, result.season), (lb.RATING, 20))

    def test_targeted_requests_have_a_daily_cap(self):
        limits = lb.Limits(pause=1.0, targeted_daily_cap=3)
        _, cache, _ = run(FakeServer(board_rows(200, me_at=30)), limits=limits)
        server = FakeServer(board_rows(200, me_at=190))
        result, cache, _ = run(server, cache=cache, now=NOW + timedelta(hours=1), limits=limits)
        self.assertEqual(server.pages, [2, 1, 3])
        later = FakeServer(board_rows(200, me_at=30))
        run(later, cache=cache, now=NOW + timedelta(hours=2), limits=limits)
        self.assertEqual(later.pages, [])

    def test_two_matches_in_window_are_ambiguous(self):
        cache = self.scanned(board_rows(100, me_at=30))
        server = FakeServer(board_rows(100, me_at=30, extra_me_at=40))
        result, _, _ = run(server, cache=cache, now=NOW + timedelta(hours=1))
        self.assertEqual(result.status, lb.AMBIGUOUS)

    def test_old_full_scan_is_not_trusted_for_targeted_lookups(self):
        cache = self.scanned(board_rows(100, me_at=30))
        server = FakeServer(board_rows(100, me_at=30))
        run(server, cache=cache, now=NOW + timedelta(days=8))
        self.assertEqual(server.pages[0], 1)
        self.assertEqual(len(server.pages), 5)

    def test_other_name_ignores_the_cached_state_but_shares_the_daily_scan(self):
        cache = self.scanned(board_rows(60, me_at=30))
        server = FakeServer(board_rows(60))
        result, _, _ = run(server, cache=cache, name="OtherName", now=NOW + timedelta(minutes=1))
        self.assertEqual(result.status, lb.UNKNOWN)
        self.assertEqual(server.pages, [])
        self.assertIn("full scan", result.note)
        result2, _, _ = run(server, cache=cache, name="OtherName", now=NOW + timedelta(days=1))
        self.assertEqual(result2.status, lb.BELOW)

    def test_failed_lookup_keeps_last_good_state(self):
        cache = self.scanned(board_rows(100, me_at=30))
        server = FakeServer(board_rows(100, me_at=30), script={0: lb.Response(404, b"")})
        result, cache, _ = run(server, cache=cache, now=NOW + timedelta(hours=1))
        self.assertEqual(result.status, lb.UNKNOWN)
        self.assertEqual(entry(cache)["status"], lb.RATING)
        again = FakeServer(board_rows(100, me_at=30))
        result2, _, _ = run(again, cache=cache, now=NOW + timedelta(hours=1, minutes=1))
        self.assertEqual(again.pages, [])
        self.assertEqual(result2.status, lb.RATING)

    def test_malformed_cache_entry_is_ignored(self):
        cache = {"boards": {"EU/battlegrounds": {"name_id": "x", "rank": "thirty"}},
                 "usage": "nonsense", "cooldown_until": 5}
        server = FakeServer(board_rows(10, me_at=1))
        result, _, _ = run(server, cache=cache)
        self.assertEqual(result.status, lb.RATING)

    def test_future_times_in_cache_do_not_block_lookups(self):
        cache = self.scanned(board_rows(60, me_at=30))
        far = (NOW + timedelta(days=30)).isoformat(timespec="seconds")
        cache["boards"]["EU/battlegrounds"]["attempt_at"] = far
        cache["cooldown_until"] = far
        server = FakeServer(board_rows(60, me_at=30))
        result, _, _ = run(server, cache=cache, now=NOW + timedelta(hours=1))
        self.assertEqual(server.pages, [2])
        self.assertEqual(result.status, lb.RATING)

    def test_naive_time_is_rejected(self):
        with self.assertRaises(ValueError):
            lb.lookup(ME, "EU", "solo", {}, NOW.replace(tzinfo=None), FakeServer([]))

    def test_cache_holds_no_other_player_names(self):
        rows = board_rows(60, me_at=30)
        _, cache, _ = run(FakeServer(rows))
        text = json.dumps(cache)
        self.assertNotIn("Filler", text)
        self.assertNotIn(ME, text)


class CacheFileTest(unittest.TestCase):
    def test_round_trip_and_corrupt_file(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "sub" / "cache.json"
            self.assertEqual(lb.load_cache(path), ({}, ""))
            lb.save_cache(path, {"a": 1})
            self.assertEqual(lb.load_cache(path), ({"a": 1}, ""))
            path.write_text("{broken", encoding="utf-8")
            cache, note = lb.load_cache(path)
            self.assertEqual(cache, {})
            self.assertIn("ignored", note)


class TransportHelpersTest(unittest.TestCase):
    def test_gzip_body_is_decoded(self):
        raw = gzip.compress(b'{"a": 1}')
        self.assertEqual(lb.decode_body(raw, "gzip"), b'{"a": 1}')
        self.assertEqual(lb.decode_body(b"plain", None), b"plain")

    def test_oversized_body_is_rejected(self):
        bomb = gzip.compress(b"0" * (lb.MAX_BODY_BYTES + 10))
        with self.assertRaises(lb.LookupFailed):
            lb.decode_body(bomb, "gzip")
        with self.assertRaises(lb.LookupFailed):
            lb.decode_body(b"0" * (lb.MAX_BODY_BYTES + 1), None)

    def test_truncated_gzip_and_unknown_encoding_are_rejected(self):
        with self.assertRaises(lb.LookupFailed):
            lb.decode_body(gzip.compress(b"x" * 1000)[:-10], "gzip")
        with self.assertRaises(lb.LookupFailed):
            lb.decode_body(b"x", "br")

    def test_page_url(self):
        url = lb.page_url("EU", "battlegrounds", 3)
        self.assertEqual(
            url,
            "https://hearthstone.blizzard.com/en-gb/api/community/leaderboardsData"
            "?region=EU&leaderboardId=battlegrounds&page=3",
        )


class InputsTest(unittest.TestCase):
    def test_display_name(self):
        self.assertEqual(lb.display_name("Name#12345"), "Name")
        self.assertEqual(lb.display_name("Name"), "Name")
        for bad in ("", "#123", "x" * 100):
            with self.subTest(bad=bad), self.assertRaises(ValueError):
                lb.display_name(bad)

    def test_bad_region_or_mode(self):
        with self.assertRaises(ValueError):
            lb.lookup(ME, "XX", "solo", {}, NOW, FakeServer([]))
        with self.assertRaises(ValueError):
            lb.lookup(ME, "EU", "arena", {}, NOW, FakeServer([]))

    def test_read_region_uses_only_the_region_line(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "Hearthstone.log"
            path.write_text(
                "I 00:57:47.1 Login as SomeoneElse#1234\nI 00:57:48.2763718 Region: EU\n",
                encoding="utf-8",
            )
            self.assertEqual(lb.read_region(path), "EU")
            path.write_text("I 00:57:48.2 Region: XX\n", encoding="utf-8")
            self.assertIsNone(lb.read_region(path))
            path.write_text("nothing here\n", encoding="utf-8")
            self.assertIsNone(lb.read_region(path))


class OutputTest(unittest.TestCase):
    def test_text_output_never_has_names(self):
        result, _, _ = run(FakeServer(board_rows(60, me_at=30)))
        text = lb.format_result(result)
        self.assertNotIn(ME, text)
        self.assertIn("rating", text.lower())
        self.assertIn("Requests this run: 4", text)
        self.assertNotIn(ME, json.dumps(lb.to_dict(result)))

    def test_below_and_unknown_wording(self):
        below, _, _ = run(FakeServer(board_rows(30)))
        self.assertIn("Below the leaderboard cut-off (< 8710)", lb.format_result(below))
        unknown, _, _ = run(FakeServer([]))
        self.assertIn("Unknown", lb.format_result(unknown))


class MainTest(unittest.TestCase):
    def main(self, argv, transport):
        out = io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(out):
            code = lb.main(argv, transport=transport, sleep=lambda s: None, now=NOW)
        return code, out.getvalue()

    def test_cli_runs_with_injected_transport(self):
        with tempfile.TemporaryDirectory() as tmp:
            cache_path = Path(tmp) / "cache.json"
            server = FakeServer(board_rows(10, me_at=2))
            code, out = self.main(
                ["--name", ME, "--region", "EU", "--mode", "solo", "--cache", str(cache_path)],
                server,
            )
            self.assertEqual(code, 0)
            self.assertIn("Rating 8990 (rank 2)", out)
            self.assertNotIn(ME, out)
            self.assertNotIn(ME, cache_path.read_text(encoding="utf-8"))

    def test_cli_unknown_exit_code(self):
        with tempfile.TemporaryDirectory() as tmp:
            code, _ = self.main(
                ["--name", ME, "--region", "EU", "--mode", "solo",
                 "--cache", str(Path(tmp) / "c.json")],
                FakeServer([]),
            )
            self.assertEqual(code, 1)

    def test_cli_region_from_log_and_json(self):
        with tempfile.TemporaryDirectory() as tmp:
            log = Path(tmp) / "Hearthstone.log"
            log.write_text("I 00:00:01.5 Region: EU\n", encoding="utf-8")
            code, out = self.main(
                ["--name", ME, "--hearthstone-log", str(log), "--mode", "solo", "--json",
                 "--cache", str(Path(tmp) / "c.json")],
                FakeServer(board_rows(10, me_at=2)),
            )
            self.assertEqual(code, 0)
            self.assertEqual(json.loads(out)["region"], "EU")
            code, _ = self.main(
                ["--name", ME, "--hearthstone-log", str(Path(tmp) / "missing.log"),
                 "--mode", "solo", "--cache", str(Path(tmp) / "c.json")],
                FakeServer([]),
            )
            self.assertEqual(code, 2)


class _Handler(http.server.BaseHTTPRequestHandler):
    def do_GET(self):  # noqa: N802 (http.server API)
        self.server.seen.append(dict(self.headers))
        route = self.path.split("?")[0]
        if route == "/gzip":
            body = gzip.compress(b'{"ok": true}')
            self.send_response(200)
            self.send_header("Content-Encoding", "gzip")
            self.send_header("Set-Cookie", "session=abc")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)
        elif route == "/redirect":
            self.send_response(302)
            self.send_header("Location", "/gzip")
            self.end_headers()
        else:
            self.send_response(429)
            self.end_headers()

    def log_message(self, *args):
        pass


class UrllibTransportTest(unittest.TestCase):
    """Real transport against a loopback server: no external network."""

    @classmethod
    def setUpClass(cls):
        cls.server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), _Handler)
        cls.server.seen = []
        threading.Thread(target=cls.server.serve_forever, daemon=True).start()
        cls.base = f"http://127.0.0.1:{cls.server.server_address[1]}"

    @classmethod
    def tearDownClass(cls):
        cls.server.shutdown()
        cls.server.server_close()

    def test_gzip_status_redirect_and_cookies(self):
        ok = lb.urllib_transport(f"{self.base}/gzip", lb.request_headers(), 5)
        self.assertEqual((ok.status, ok.body), (200, b'{"ok": true}'))
        again = lb.urllib_transport(f"{self.base}/gzip", lb.request_headers(), 5)
        self.assertEqual(again.status, 200)
        self.assertNotIn("Cookie", self.server.seen[-1])  # cookies are never sent back
        self.assertEqual(self.server.seen[-1]["User-Agent"], lb.USER_AGENT)
        redirect = lb.urllib_transport(f"{self.base}/redirect", lb.request_headers(), 5)
        self.assertEqual(redirect.status, 302)  # not followed
        limited = lb.urllib_transport(f"{self.base}/limited", lb.request_headers(), 5)
        self.assertEqual(limited.status, 429)

    def test_connection_failure_is_a_transport_error(self):
        with socket.socket() as probe:
            probe.bind(("127.0.0.1", 0))
            port = probe.getsockname()[1]
        with self.assertRaises(lb.TransportError):
            lb.urllib_transport(f"http://127.0.0.1:{port}/", lb.request_headers(), 2)


if __name__ == "__main__":
    unittest.main()
