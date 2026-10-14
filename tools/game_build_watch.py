"""Warn when Hearthstone ships a game build the app has not been checked against.

HearthstoneJSON lists every game build it has card data for. Once a day this
reads that listing (one request) and compares the newest build with the
builds the log parser was tested on (`TESTED_BUILDS` in
`crates/bg-parser/src/lib.rs`). When the newest build is newer, it reads the
card data of both builds, lists the Battlegrounds cards added, changed and
removed, marks the minions the simulator does not know or has to review
again, and opens one issue, `New game build <N>`, with a checklist. An issue
with that title, open or closed, is never opened twice.

Only the listing and, on a new build, two card data files are read; nothing
from Blizzard's own site. Standard library only. `gh` (GitHub's command line,
with `GITHUB_TOKEN` in the environment) opens the issue.

Usage:
    python tools/game_build_watch.py              print what it would do
    python tools/game_build_watch.py --open-issue open the issue when a build is new

Exit codes: 0 = nothing to do or issue opened, 1 = unreadable data,
2 = network or `gh` error (the workflow fails and GitHub emails the owner).
"""

from __future__ import annotations

import argparse
import http.client
import json
import re
import subprocess
import sys
import urllib.error
import urllib.request
from pathlib import Path
from typing import Callable, NamedTuple

ROOT = Path(__file__).resolve().parent.parent
USER_AGENT = "TavernLedger-tools (+https://tavernledger.net)"
LISTING_URL = "https://api.hearthstonejson.com/v1/"
CARDS_URL = "https://api.hearthstonejson.com/v1/{build}/enUS/cards.json"
NEWS_URL = "https://hearthstone.blizzard.com/en-us/news"
TIMEOUT = 60.0
MAX_LISTING_BYTES = 8 * 1024 * 1024
MAX_CARDS_BYTES = 64 * 1024 * 1024
# The real listing has hundreds of builds; far fewer means a bad answer.
MIN_BUILDS = 20
# Per section of the report; an issue body may not pass 65536 characters.
MAX_LISTED = 40
MAX_NAME = 60

BG_TYPES = frozenset({
    "MINION", "HERO", "HERO_POWER", "BATTLEGROUND_SPELL", "BATTLEGROUND_TRINKET",
    "BATTLEGROUND_ANOMALY", "BATTLEGROUND_QUEST_REWARD",
})
# What the player can see on a card; art credits and flavour text are not a change.
SHOWN_FIELDS = ("name", "text", "type", "cost", "attack", "health", "armor", "techLevel", "race", "races", "mechanics")

BUILD_LINK = re.compile(r'href="/v1/(\d+)/"')
TESTED_LIST = re.compile(r"TESTED_BUILDS\s*:\s*&\[i64\]\s*=\s*&\[([^\]]*)\]")
CHECKED_LIST = re.compile(r"CHECKED_BUILDS\s*:\s*&\[i64\]\s*=\s*&\[([^\]]*)\]")
CARD_ID_LITERAL = re.compile(r'"((?:BG|TB)[A-Za-z0-9_]+)"')
CARD_ID = re.compile(r"[A-Za-z0-9_]{1,64}")
URL_LIKE = re.compile(r"(?i)\b(?:www\.|https?\b)\S*")
UNSAFE_TEXT = re.compile(r"[\x00-\x1f\x7f`@\[\]<>|\\#:*_~!()/]")

Get = Callable[[str, int], bytes]
Gh = Callable[..., str]


class DataError(Exception):
    """The data is not what the check needs."""


class Diff(NamedTuple):
    added: list[dict]
    changed: list[dict]
    removed: list[dict]

    def is_empty(self) -> bool:
        return not (self.added or self.changed or self.removed)


def http_get(url: str, max_bytes: int) -> bytes:
    request = urllib.request.Request(url, headers={"User-Agent": USER_AGENT, "Accept": "*/*"})
    with urllib.request.urlopen(request, timeout=TIMEOUT) as response:
        if response.status != 200:
            raise OSError(f"{url}: HTTP {response.status}")
        body = response.read(max_bytes + 1)
    if len(body) > max_bytes:
        raise DataError(f"{url}: answer too large")
    return body


def newest_build(listing_html: str) -> int:
    builds = {int(m) for m in BUILD_LINK.findall(listing_html)}
    if len(builds) < MIN_BUILDS:
        raise DataError(f"the listing shows {len(builds)} builds, expected at least {MIN_BUILDS}")
    return max(builds)


def _numbers(list_body: str) -> list[int]:
    return [int(n.replace("_", "")) for n in re.findall(r"\d[\d_]*", list_body)]


def parser_builds(root: Path) -> list[int]:
    text = (root / "crates" / "bg-parser" / "src" / "lib.rs").read_text(encoding="utf-8")
    match = TESTED_LIST.search(text)
    if not match:
        raise DataError("TESTED_BUILDS not found in crates/bg-parser/src/lib.rs")
    builds = _numbers(match.group(1))
    if not builds:
        raise DataError("TESTED_BUILDS is empty")
    return sorted(builds)


def _sim_sources(root: Path) -> list[Path]:
    return sorted((root / "crates" / "combat-sim" / "src").glob("*.rs"))


def simulator_builds(root: Path) -> list[int]:
    """The simulator's list of checked builds, empty when it has none (yet)."""
    found: set[int] = set()
    for path in _sim_sources(root):
        for match in CHECKED_LIST.finditer(path.read_text(encoding="utf-8")):
            found.update(_numbers(match.group(1)))
    return sorted(found)


def simulator_card_ids(root: Path) -> set[str]:
    """Every Battlegrounds card id the simulator's rule files name (tests left out)."""
    ids: set[str] = set()
    for path in _sim_sources(root):
        if path.name.startswith("cards") and not path.stem.endswith("_tests"):
            ids.update(CARD_ID_LITERAL.findall(path.read_text(encoding="utf-8")))
    return ids


def _bg_cards(data: object) -> dict[str, dict]:
    if not isinstance(data, list) or not all(isinstance(c, dict) for c in data):
        raise DataError("card data is not a list of cards")
    return {
        c["id"]: c for c in data
        if isinstance(c.get("id"), str) and c.get("set") == "BATTLEGROUNDS"
        and c.get("type") in BG_TYPES and "_SKIN_" not in c["id"]
    }


def _shown(card: dict) -> tuple:
    values = (card.get(f) for f in SHOWN_FIELDS)
    return tuple(json.dumps(sorted(v) if isinstance(v, list) else v, sort_keys=True, default=str) for v in values)


def diff_cards(old: object, new: object) -> Diff:
    before, after = _bg_cards(old), _bg_cards(new)
    return Diff(
        added=[after[i] for i in sorted(after.keys() - before.keys())],
        changed=[after[i] for i in sorted(after.keys() & before.keys()) if _shown(before[i]) != _shown(after[i])],
        removed=[before[i] for i in sorted(before.keys() - after.keys())],
    )


def _clean(text: object) -> str:
    return " ".join(UNSAFE_TEXT.sub(" ", URL_LIKE.sub(" ", str(text))).split())[:MAX_NAME]


def _line(card: dict) -> str:
    card_id = str(card.get("id", ""))
    safe_id = card_id if CARD_ID.fullmatch(card_id) else "(odd id)"
    return f"- `{safe_id}` {_clean(card.get('name', ''))}"


def _section(title: str, cards: list[dict]) -> list[str]:
    if not cards:
        return []
    lines = [f"### {title}: {len(cards)} card{'' if len(cards) == 1 else 's'}", ""]
    lines += [_line(c) for c in cards[:MAX_LISTED]]
    if len(cards) > MAX_LISTED:
        lines.append(f"- … and {len(cards) - MAX_LISTED} more")
    return lines + [""]


def render_report(build: int, previous: int, diff: Diff, sim_ids: set[str], parser_list: list[int],
                  sim_list: list[int]) -> str:
    minions = [c for c in diff.added + diff.changed if c.get("type") == "MINION"]
    unknown = [c for c in diff.added if c.get("type") == "MINION" and c["id"] not in sim_ids]
    review = [c for c in diff.changed if c.get("type") == "MINION" and c["id"] in sim_ids]
    lines = [
        f"Hearthstone build {build} is out. The newest build the log parser was tested on is {previous}.",
        "",
        f"HearthstoneJSON lists the Battlegrounds card data of both builds; {len(minions)} minions were added or changed.",
        f"Official patch news: {NEWS_URL}",
        "",
        "## Battlegrounds cards, build "
        f"{previous} to {build}",
        "",
    ]
    if diff.is_empty():
        lines += ["No Battlegrounds card was added, changed or removed.", ""]
    lines += _section("Added", diff.added) + _section("Changed", diff.changed) + _section("Removed", diff.removed)
    lines += _section("Minions that are not in the simulator", unknown)
    lines += _section("Minions the simulator plays that changed, to review again", review)
    if sim_list and max(sim_list) < previous:
        lines += [f"The simulator's checked builds end at {max(sim_list)}.", ""]
    lines += [
        "## Checklist",
        "",
        f"- [ ] Log parser: play a game on build {build}, make a fixture, add {build} to `TESTED_BUILDS`",
        "- [ ] Simulator: review the minions listed above and the checked builds list",
        f"- [ ] Memory reader: check that it still reads build {build}",
        "- [ ] Card data: refresh the committed card data and run the data checks",
        "",
        f"Tested builds now: {', '.join(map(str, parser_list))}.",
    ]
    return "\n".join(lines) + "\n"


def run_gh(args: list[str], body: str | None = None) -> str:
    done = subprocess.run(["gh", *args], input=body, capture_output=True, text=True, timeout=60)
    if done.returncode != 0:
        raise OSError(f"gh {' '.join(args[:2])} failed: {done.stderr.strip()}")
    return done.stdout


def issue_title(build: int) -> str:
    return f"New game build {build}"


def issue_exists(title: str, gh: Gh) -> bool:
    out = gh(["issue", "list", "--state", "all", "--search", f'"{title}" in:title', "--limit", "100",
              "--json", "title"])
    return any(item.get("title") == title for item in json.loads(out))


def run(root: Path, get: Get, gh: Gh | None) -> int:
    try:
        newest = newest_build(get(LISTING_URL, MAX_LISTING_BYTES).decode("utf-8"))
        tested = parser_builds(root)
        previous = max(tested)
        if newest <= previous:
            print(f"Build {newest}: nothing new (parser tested on {previous}).")
            return 0
        title = issue_title(newest)
        if gh is not None and issue_exists(title, gh):
            print(f"Build {newest} is new, and the issue '{title}' already exists.")
            return 0
        old = json.loads(get(CARDS_URL.format(build=previous), MAX_CARDS_BYTES))
        try:
            new = json.loads(get(CARDS_URL.format(build=newest), MAX_CARDS_BYTES))
        except urllib.error.HTTPError as err:
            if err.code != 404:
                raise
            print(f"Build {newest} is listed but its card data is not published yet; will look again.")
            return 0
        before, after = _bg_cards(old), _bg_cards(new)
        if not before or not after or len(after) * 2 < len(before):
            raise DataError(f"build {newest} has {len(after)} Battlegrounds cards, build {previous} has {len(before)}")
        report = render_report(newest, previous, diff_cards(old, new), simulator_card_ids(root), tested,
                               simulator_builds(root))
        if gh is None:
            print(f"Build {newest} is new. Would open '{title}':\n\n{report}")
            return 0
        print(gh(["issue", "create", "--title", title, "--body-file", "-"], report).strip())
        return 0
    except (DataError, ValueError, UnicodeDecodeError) as err:
        print(f"bad data: {err}", file=sys.stderr)
        return 1
    except (OSError, http.client.HTTPException, subprocess.SubprocessError) as err:
        print(f"could not finish: {err}", file=sys.stderr)
        return 2


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--open-issue", action="store_true", help="open the issue when the build is new")
    args = parser.parse_args(argv)
    return run(ROOT, http_get, run_gh if args.open_issue else None)


if __name__ == "__main__":
    sys.exit(main())
