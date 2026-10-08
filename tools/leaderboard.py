"""Read-only client for the public Battlegrounds leaderboard (prototype, T-006).

Looks up the player's own row and reports one of four states (D-011): the
rating if the name appears once, "below the cut-off" only after every page
was checked, "ambiguous" if the name appears more than once, and "unknown" on
any error, odd response or incomplete scan. It paces and caps its requests
and keeps a small local cache with the player's own state only, never other
players' rows (D-012). Strategy and measurements: docs/research/fuentes-mmr.md.

Usage:
    python tools/leaderboard.py --name NAME --mode solo|duos
                                (--region EU|US|AP | --hearthstone-log PATH) [--json]

Privacy: the output and the cache never contain the name. Exit codes:
0 = rating, below the cut-off or ambiguous; 1 = unknown; 2 = bad arguments.
"""

from __future__ import annotations

import argparse
import hashlib
import http.client
import json
import os
import re
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
import zlib
from dataclasses import asdict, dataclass, replace
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import Callable, Protocol

VERSION = "0.1.0"
USER_AGENT = f"TavernLedger/{VERSION} (+https://github.com/KBroneZ/tavern-ledger)"
BASE_URL = "https://hearthstone.blizzard.com/en-gb/api/community/leaderboardsData"
REGIONS = ("EU", "US", "AP")
BOARDS = {"solo": "battlegrounds", "duos": "battlegroundsduo"}
PAGE_SIZE = 25
MAX_NAME_LENGTH = 64
MAX_WIRE_BYTES = 1_000_000  # a page is ~17 KB gzipped
MAX_BODY_BYTES = 2_000_000  # and ~310 KB once decoded
DEFAULT_CACHE = Path(__file__).resolve().parent.parent / ".local" / "leaderboard-cache.json"
REGION_RE = re.compile(r"^[A-Z] \d{2}:\d{2}:\d{2}\.\d+ Region: ([A-Z]{2})$")
REGION_SCAN_LINES = 500  # the line sits around line 48

RATING = "rating"
BELOW = "below_cutoff"
AMBIGUOUS = "ambiguous"
UNKNOWN = "unknown"


@dataclass(frozen=True)
class Limits:
    timeout: float = 15.0
    pause: float = 1.5  # between requests
    backoff: tuple[float, ...] = (5.0, 15.0)  # one retry per entry
    targeted_pages: int = 5  # last known page and two on each side
    targeted_daily_cap: int = 60  # per region and mode
    max_pages: int = 600  # sanity bound for one full scan
    scan_retry_allowance: int = 10
    min_recheck: timedelta = timedelta(minutes=5)
    cooldown: timedelta = timedelta(hours=1)  # after HTTP 403 or 429
    targeted_trust: timedelta = timedelta(days=7)  # max age of the full scan behind a targeted hit


@dataclass(frozen=True)
class Response:
    status: int
    body: bytes


@dataclass(frozen=True)
class Row:
    rank: int
    name: str
    rating: int


@dataclass(frozen=True)
class Page:
    number: int
    season: int
    total_pages: int
    total_size: int
    rows: tuple[Row, ...]

    def digest(self) -> str:
        data = json.dumps([asdict(r) for r in self.rows], ensure_ascii=False).encode()
        return hashlib.sha256(data).hexdigest()


@dataclass(frozen=True)
class Outcome:
    status: str
    season: int
    total_pages: int
    rating: int | None = None
    rank: int | None = None
    cut: int | None = None


@dataclass(frozen=True)
class Result:
    status: str
    region: str
    mode: str
    season: int | None = None
    rating: int | None = None
    rank: int | None = None
    cut: int | None = None
    checked_at: str | None = None  # when this state was obtained
    full_scan_at: str | None = None
    requests: int = 0
    note: str = ""


class Transport(Protocol):
    def __call__(self, url: str, headers: dict[str, str], timeout: float) -> Response: ...


class TransportError(Exception):
    """Network failure or timeout; worth a retry."""


class LookupFailed(Exception):
    """Stop here: the result is unknown."""


class RateLimited(LookupFailed):
    pass


class CapReached(LookupFailed):
    pass


# --- inputs -----------------------------------------------------------------

def display_name(battletag: str) -> str:
    """The leaderboard shows the BattleTag without its #number."""
    name = battletag.split("#", 1)[0].strip()
    if not name or len(name) > MAX_NAME_LENGTH:
        raise ValueError("invalid player name")
    return name


def read_region(path: Path) -> str | None:
    """Region from Hearthstone.log. Reads only the first lines and keeps only the
    region match: the file also holds BattleTags."""
    with path.open("r", encoding="utf-8", errors="replace") as handle:
        for number, line in enumerate(handle):
            if number >= REGION_SCAN_LINES:
                break
            match = REGION_RE.match(line.rstrip("\r\n"))
            if match:
                return match.group(1) if match.group(1) in REGIONS else None
    return None


# --- transport ----------------------------------------------------------------

def page_url(region: str, board: str, page: int) -> str:
    query = urllib.parse.urlencode({"region": region, "leaderboardId": board, "page": page})
    return f"{BASE_URL}?{query}"


def request_headers() -> dict[str, str]:
    return {"User-Agent": USER_AGENT, "Accept": "application/json", "Accept-Encoding": "gzip"}


def decode_body(raw: bytes, encoding: str | None) -> bytes:
    if encoding in (None, "", "identity"):
        body = raw
    elif encoding == "gzip":
        decoder = zlib.decompressobj(16 + zlib.MAX_WBITS)
        try:
            body = decoder.decompress(raw, MAX_BODY_BYTES + 1)
        except zlib.error as exc:
            raise LookupFailed("corrupt gzip body") from exc
        if len(body) <= MAX_BODY_BYTES and not decoder.eof:
            raise LookupFailed("truncated gzip body")
    else:
        raise LookupFailed("unexpected content encoding")  # never echo server text
    if len(body) > MAX_BODY_BYTES:
        raise LookupFailed("response too large")
    return body


class _NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, *args, **kwargs):  # a redirect comes back as an HTTP error
        return None


def urllib_transport(url: str, headers: dict[str, str], timeout: float) -> Response:
    """Standard-library transport: no cookie handling (cookies are dropped), no redirects."""
    opener = urllib.request.build_opener(_NoRedirect)
    request = urllib.request.Request(url, headers=headers)
    try:
        with opener.open(request, timeout=timeout) as response:
            raw = response.read(MAX_WIRE_BYTES + 1)
            if len(raw) > MAX_WIRE_BYTES:
                raise LookupFailed("response too large")
            return Response(response.status, decode_body(raw, response.headers.get("Content-Encoding")))
    except urllib.error.HTTPError as exc:
        exc.close()
        return Response(exc.code, b"")
    except (urllib.error.URLError, http.client.HTTPException, OSError) as exc:
        raise TransportError(type(exc).__name__) from exc


# --- response validation ----------------------------------------------------------

def _int(value: object, what: str, minimum: int = 0) -> int:
    if type(value) is not int or value < minimum:
        raise LookupFailed(f"invalid {what}")
    return value


def _rows(raw_rows: list, page: int) -> tuple[Row, ...]:
    rows = []
    for i, raw in enumerate(raw_rows):
        if not isinstance(raw, dict):
            raise LookupFailed("invalid row")
        rank = _int(raw.get("rank"), "rank", 1)
        rating = _int(raw.get("rating"), "rating")
        name = raw.get("accountid")
        if not isinstance(name, str) or not name or len(name) > MAX_NAME_LENGTH:
            raise LookupFailed("invalid name field")
        if rank != (page - 1) * PAGE_SIZE + i + 1:
            raise LookupFailed("ranks are not contiguous")
        if rows and rating > rows[-1].rating:
            raise LookupFailed("ratings are not in descending order")
        rows.append(Row(rank, name, rating))
    return tuple(rows)


def parse_page(body: bytes, page: int, region: str) -> Page:
    """Strict check of one page. Out-of-range pages come back as 200 with no rows
    and totalPages 0: they parse as an empty page and the caller decides."""
    try:
        data = json.loads(body)
    except (ValueError, RecursionError) as exc:  # ValueError includes UnicodeDecodeError
        raise LookupFailed("response is not JSON") from exc
    if not isinstance(data, dict) or data.get("region") != region:
        raise LookupFailed("unexpected response shape or region")
    season = _int(data.get("seasonId"), "season", 1)
    board = data.get("leaderboard")
    pagination = board.get("pagination") if isinstance(board, dict) else None
    raw_rows = board.get("rows") if isinstance(board, dict) else None
    if not isinstance(pagination, dict) or not isinstance(raw_rows, list):
        raise LookupFailed("unexpected leaderboard shape")
    total_pages = _int(pagination.get("totalPages"), "totalPages")
    total_size = _int(pagination.get("totalSize"), "totalSize")
    if total_pages != -(-total_size // PAGE_SIZE):
        raise LookupFailed("totalPages does not match totalSize")
    if not raw_rows:
        if total_pages and page <= total_pages:
            raise LookupFailed("empty page inside the leaderboard")
        return Page(page, season, total_pages, total_size, ())
    if page > total_pages:
        raise LookupFailed("rows on a page beyond totalPages")
    expected = PAGE_SIZE if page < total_pages else total_size - PAGE_SIZE * (total_pages - 1)
    if len(raw_rows) != expected:
        raise LookupFailed("unexpected number of rows")
    return Page(page, season, total_pages, total_size, _rows(raw_rows, page))


# --- fetching ---------------------------------------------------------------------

class Fetcher:
    """Paces requests, retries transient failures and counts every request."""

    def __init__(self, transport: Transport, region: str, board: str,
                 limits: Limits, sleep: Callable[[float], None]) -> None:
        self.transport = transport
        self.region = region
        self.board = board
        self.limits = limits
        self.sleep = sleep
        self.requests = 0
        self.limit = 0  # set by the caller before each phase

    def get(self, number: int) -> Page:
        url = page_url(self.region, self.board, number)
        delays = (self.limits.pause, *self.limits.backoff)
        last = "no attempt"
        for delay in delays:
            if self.requests >= self.limit:
                raise CapReached("request cap reached")
            if self.requests:
                self.sleep(delay)
            self.requests += 1
            try:
                response = self.transport(url, request_headers(), self.limits.timeout)
            except TransportError as exc:
                last = f"network error ({exc})"
                continue
            if response.status == 200:
                return parse_page(response.body, number, self.region)
            if response.status in (403, 429):
                raise RateLimited(f"HTTP {response.status}")
            if 500 <= response.status < 600:
                last = f"HTTP {response.status}"
                continue
            raise LookupFailed(f"unexpected HTTP {response.status}")
        raise LookupFailed(f"{last} after {len(delays)} attempts")


def _match(page: Page, name: str) -> list[Row]:
    # Exact comparison, no case folding or Unicode normalisation (D-011).
    return [row for row in page.rows if row.name == name]


def _outcome(matches: list[Row], season: int, total_pages: int, cut: int | None) -> Outcome:
    if len(matches) == 1:
        row = matches[0]
        return Outcome(RATING, season, total_pages, row.rating, row.rank, cut)
    if matches:
        return Outcome(AMBIGUOUS, season, total_pages, cut=cut)
    return Outcome(BELOW, season, total_pages, cut=cut)


def full_scan(fetcher: Fetcher, name: str) -> Outcome:
    """Every page: page 1 for the size, then from the last page up, then the last
    page again to check that the board did not change during the scan. Only the
    own rows and each page's rating bounds are kept."""
    first = fetcher.get(1)
    total = first.total_pages
    if total == 0:
        raise LookupFailed("leaderboard is empty")
    if total > fetcher.limits.max_pages:
        raise LookupFailed("leaderboard has more pages than allowed")
    matches = _match(first, name)
    bounds = {1: (first.rows[0].rating, first.rows[-1].rating)}
    last_digest = first.digest()
    for number in range(total, 1, -1):
        page = fetcher.get(number)
        _check_same_board(first, page)
        matches += _match(page, name)
        bounds[number] = (page.rows[0].rating, page.rows[-1].rating)
        if number == total:
            last_digest = page.digest()
    if total > 1:
        again = fetcher.get(total)
        _check_same_board(first, again)
        if again.digest() != last_digest:
            raise LookupFailed("leaderboard changed during the scan")
    for number in range(1, total):
        if bounds[number][1] < bounds[number + 1][0]:
            raise LookupFailed("pages are not in descending rating order")
    return _outcome(matches, first.season, total, cut=bounds[total][1])


def _check_same_board(first: Page, page: Page) -> None:
    same = (page.season, page.total_pages, page.total_size) == (
        first.season, first.total_pages, first.total_size)
    if not same or not page.rows:
        raise LookupFailed("leaderboard changed during the scan")


def _window(start: int, total: int, size: int) -> list[int]:
    radius = size // 2
    order = [start]
    for offset in range(1, radius + 1):
        order += [start - offset, start + offset]
    return [p for p in order if 1 <= p <= total]


def targeted(fetcher: Fetcher, name: str, state: dict) -> Outcome | None:
    """Pages around the last known rank. None means "not settled here": not found,
    season changed or cap reached. Uniqueness rests on the last full scan."""
    start = -(-state["rank"] // PAGE_SIZE)
    for number in _window(start, state["total_pages"], fetcher.limits.targeted_pages):
        try:
            page = fetcher.get(number)
        except CapReached:
            return None
        if page.season != state["season"] or not page.rows:
            return None
        matches = _match(page, name)
        if matches:
            return _outcome(matches, page.season, page.total_pages, state.get("cut"))
    return None


# --- cache --------------------------------------------------------------------

def load_cache(path: Path) -> tuple[dict, str]:
    if not path.exists():
        return {}, ""
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return {}, "cache file unreadable, ignored"
    return (data, "") if isinstance(data, dict) else ({}, "cache file unreadable, ignored")


def save_cache(path: Path, cache: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.with_suffix(".tmp")
    tmp.write_text(json.dumps(cache, indent=2, sort_keys=True), encoding="utf-8")
    os.replace(tmp, path)


def name_id(name: str) -> str:
    """Tells whether the cached state belongs to this name without storing it."""
    return hashlib.sha256(("tavern-ledger:" + name).encode()).hexdigest()[:16]


def _stamp(moment: datetime) -> str:
    return moment.astimezone(timezone.utc).isoformat(timespec="seconds")


def _time(value: object) -> datetime | None:
    if not isinstance(value, str):
        return None
    try:
        moment = datetime.fromisoformat(value)
    except ValueError:
        return None
    return moment if moment.tzinfo else None


STATE_TYPES = {"season": int, "total_pages": int, "rank": int, "rating": int, "cut": int}


def _state(raw: object, wanted_id: str) -> dict | None:
    """The cached entry for this name, or None if missing, foreign or malformed."""
    if not isinstance(raw, dict) or raw.get("name_id") != wanted_id:
        return None
    if _time(raw.get("attempt_at")) is None:
        return None
    if raw.get("status") not in (None, RATING, BELOW, AMBIGUOUS):
        return None
    for key, kind in STATE_TYPES.items():
        if raw.get(key) is not None and type(raw.get(key)) is not kind:
            return None
    if raw.get("status") is not None and (
            _time(raw.get("checked_at")) is None or raw.get("season") is None):
        return None
    if raw.get("status") == RATING and (raw.get("rank") is None or raw.get("total_pages") is None):
        return None
    return dict(raw)


def _usage(cache: dict, key: str, day: str) -> dict:
    usage = cache.get("usage")
    if not isinstance(usage, dict) or usage.get("day") != day:
        return {"targeted": 0, "full_scan": False}
    raw = usage.get("boards", {}).get(key) if isinstance(usage.get("boards"), dict) else None
    if not isinstance(raw, dict) or type(raw.get("targeted")) is not int:
        return {"targeted": 0, "full_scan": False}
    return {"targeted": raw["targeted"], "full_scan": raw.get("full_scan") is True}


def _with_usage(cache: dict, key: str, day: str, usage: dict) -> dict:
    old = cache.get("usage")
    boards = dict(old["boards"]) if (isinstance(old, dict) and old.get("day") == day
                                     and isinstance(old.get("boards"), dict)) else {}
    boards[key] = usage
    return {**cache, "usage": {"day": day, "boards": boards}}


def _with_state(cache: dict, key: str, state: dict) -> dict:
    boards = dict(cache["boards"]) if isinstance(cache.get("boards"), dict) else {}
    boards[key] = state
    return {**cache, "boards": boards}


# --- lookup -------------------------------------------------------------------

def _from_state(state: dict | None, region: str, mode: str, note: str, requests: int = 0) -> Result:
    if state is None or state.get("status") is None:
        return Result(UNKNOWN, region, mode, requests=requests, note=note)
    status = state["status"]
    return Result(
        status, region, mode, season=state["season"],
        rating=state.get("rating") if status == RATING else None,
        rank=state.get("rank") if status == RATING else None,
        cut=state.get("cut"), checked_at=state["checked_at"],
        full_scan_at=state.get("full_scan_at"), requests=requests, note=note,
    )


def _can_target(state: dict | None, now: datetime, limits: Limits) -> bool:
    if state is None or state.get("status") != RATING:
        return False
    scanned = _time(state.get("full_scan_at"))
    return scanned is not None and now - scanned <= limits.targeted_trust


def _recent(stamp: object, now: datetime, window: timedelta, tolerance: timedelta) -> bool:
    """True if `stamp` is less than `window` ago. A time further in the future
    than `tolerance` (clock change or edited cache) does not count."""
    moment = _time(stamp)
    return moment is not None and now - window < moment <= now + tolerance


@dataclass(frozen=True)
class _Query:
    outcome: Outcome | None
    usage: dict
    note: str = ""
    failed: bool = False
    rate_limited: bool = False
    full_scanned: bool = False


def _query(fetcher: Fetcher, name: str, state: dict | None, usage: dict,
           now: datetime) -> _Query:
    """Targeted pages first when the cached state allows it, then at most one
    full scan per day. Every request is counted in the returned usage."""
    limits = fetcher.limits
    new_usage, outcome = dict(usage), None
    try:
        if _can_target(state, now, limits) and usage["targeted"] < limits.targeted_daily_cap:
            fetcher.limit = limits.targeted_daily_cap - usage["targeted"]
            try:
                outcome = targeted(fetcher, name, state)
            finally:
                new_usage["targeted"] += fetcher.requests
        if outcome is not None:
            return _Query(outcome, new_usage)
        if usage["full_scan"]:
            return _Query(None, new_usage, "not refreshed: the daily full scan was already used")
        new_usage["full_scan"] = True
        fetcher.limit = fetcher.requests + limits.max_pages + 1 + limits.scan_retry_allowance
        return _Query(full_scan(fetcher, name), new_usage, full_scanned=True)
    except RateLimited as exc:
        return _Query(None, new_usage, f"stopped: {exc}", failed=True, rate_limited=True)
    except LookupFailed as exc:
        return _Query(None, new_usage, f"stopped: {exc}", failed=True)


def lookup(name: str, region: str, mode: str, cache: dict, now: datetime,
           transport: Transport, sleep: Callable[[float], None] = time.sleep,
           limits: Limits = Limits()) -> tuple[Result, dict]:
    """One lookup. Returns the result and the new cache; `cache` is not modified."""
    name = display_name(name)
    if region not in REGIONS or mode not in BOARDS:
        raise ValueError("invalid region or mode")
    if now.tzinfo is None:
        raise ValueError("now must be timezone-aware")
    key, day, me = f"{region}/{BOARDS[mode]}", now.date().isoformat(), name_id(name)
    boards = cache.get("boards") if isinstance(cache.get("boards"), dict) else {}
    state = _state(boards.get(key), me)

    cooldown = _time(cache.get("cooldown_until"))
    if cooldown and now < cooldown <= now + limits.cooldown:  # a later time is not trusted
        return _from_state(state, region, mode, "paused after a rate-limit response"), cache
    if state and _recent(state.get("attempt_at"), now, limits.min_recheck, timedelta(minutes=1)):
        return _from_state(state, region, mode, "checked moments ago; not refreshed"), cache

    fetcher = Fetcher(transport, region, BOARDS[mode], limits, sleep)
    query = _query(fetcher, name, state, _usage(cache, key, day), now)
    new_cache = _with_usage(cache, key, day, query.usage)
    if query.rate_limited:
        new_cache = {**new_cache, "cooldown_until": _stamp(now + limits.cooldown)}
    base = {**(state or {}), "name_id": me, "attempt_at": _stamp(now)}
    if query.full_scanned:
        base["full_scan_at"] = _stamp(now)
    if query.outcome is not None:
        fresh = {**base, **_outcome_fields(query.outcome), "checked_at": _stamp(now)}
        return (_from_state(fresh, region, mode, query.note, fetcher.requests),
                _with_state(new_cache, key, fresh))
    new_cache = _with_state(new_cache, key, base)
    if query.failed:
        return Result(UNKNOWN, region, mode, requests=fetcher.requests, note=query.note), new_cache
    return _from_state(state, region, mode, query.note, fetcher.requests), new_cache


def _outcome_fields(outcome: Outcome) -> dict:
    return {
        "status": outcome.status, "season": outcome.season,
        "total_pages": outcome.total_pages, "rating": outcome.rating,
        "rank": outcome.rank, "cut": outcome.cut,
    }


# --- output and CLI -------------------------------------------------------------

def to_dict(result: Result) -> dict:
    return asdict(result)


def format_result(result: Result) -> str:
    if result.status == RATING:
        head = f"Rating {result.rating} (rank {result.rank})"
    elif result.status == BELOW:
        head = f"Below the leaderboard cut-off (< {result.cut})"
    elif result.status == AMBIGUOUS:
        head = "Ambiguous: the name appears more than once"
    else:
        head = "Unknown"
    lines = [head, f"Board: {result.region} {result.mode}, season {result.season or '?'}"]
    if result.checked_at:
        scan = f", last full scan {result.full_scan_at}" if result.full_scan_at else ""
        lines.append(f"Checked at {result.checked_at}{scan}")
    if result.cut is not None and result.status != BELOW:
        lines.append(f"Cut-off at the last full scan: {result.cut}")
    lines.append(f"Requests this run: {result.requests}")
    if result.note:
        lines.append(f"Note: {result.note}")
    return "\n".join(lines)


def main(argv: list[str] | None = None, transport: Transport = urllib_transport,
         sleep: Callable[[float], None] = time.sleep, now: datetime | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--name", required=True, help="your BattleTag (the #number is dropped)")
    parser.add_argument("--mode", required=True, choices=sorted(BOARDS))
    where = parser.add_mutually_exclusive_group(required=True)
    where.add_argument("--region", choices=REGIONS)
    where.add_argument("--hearthstone-log", type=Path, help="Hearthstone.log to read the region from")
    parser.add_argument("--cache", type=Path, default=DEFAULT_CACHE)
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args(argv)

    region = args.region
    if region is None:
        try:
            region = read_region(args.hearthstone_log)
        except OSError:
            region = None
        if region is None:
            print("Region not found in that log.", file=sys.stderr)
            return 2
    try:
        name = display_name(args.name)
    except ValueError as exc:
        print(str(exc), file=sys.stderr)
        return 2

    cache, cache_note = load_cache(args.cache)
    result, new_cache = lookup(name, region, args.mode, cache, now or datetime.now(timezone.utc),
                               transport, sleep=sleep)
    if cache_note:
        result = replace(result, note="; ".join(n for n in (result.note, cache_note) if n))
    save_cache(args.cache, new_cache)
    print(json.dumps(to_dict(result), indent=2) if args.json else format_result(result))
    return 1 if result.status == UNKNOWN else 0


if __name__ == "__main__":
    sys.exit(main())
