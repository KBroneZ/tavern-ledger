"""Tests for tools/game_build_watch.py with synthetic data (made-up cards, no network)."""

import http.client
import json
import sys
import tempfile
import unittest
import urllib.error
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "tools"))

import game_build_watch as gbw  # noqa: E402


def listing(*builds: int) -> str:
    rows = "".join(f'<a href="/v1/{b}/">{b}</a><a href="/v1/{b}/all/cards.json">x</a>' for b in builds)
    return f'<html><body>{rows}<a href="/v1/latest/">latest</a></body></html>'


def minion(card_id: str, name: str = "Syn", attack: int = 2, health: int = 3, tier: int = 2, text: str = "") -> dict:
    return {"id": card_id, "name": name, "type": "MINION", "set": "BATTLEGROUNDS", "attack": attack,
            "health": health, "techLevel": tier, "text": text}


class Listing(unittest.TestCase):
    def test_newest_build_is_the_highest_number(self):
        self.assertEqual(gbw.newest_build(listing(*range(1000, 1030), 999999, 5000)), 999999)

    def test_a_listing_with_few_builds_is_refused(self):
        with self.assertRaises(gbw.DataError):
            gbw.newest_build(listing(1000, 1001))

    def test_a_page_without_builds_is_refused(self):
        with self.assertRaises(gbw.DataError):
            gbw.newest_build("<html>Service unavailable</html>")


class RepoLists(unittest.TestCase):
    def make_repo(self, parser: str, sim: dict[str, str]) -> Path:
        root = Path(tempfile.mkdtemp())
        (root / "crates/bg-parser/src").mkdir(parents=True)
        (root / "crates/bg-parser/src/lib.rs").write_text(parser, encoding="utf-8")
        (root / "crates/combat-sim/src").mkdir(parents=True)
        for name, text in sim.items():
            (root / "crates/combat-sim/src" / name).write_text(text, encoding="utf-8")
        return root

    def test_parser_tested_builds_are_read(self):
        root = self.make_repo("pub const TESTED_BUILDS: &[i64] = &[253216, 254_477];\n", {})
        self.assertEqual(gbw.parser_builds(root), [253216, 254477])

    def test_missing_parser_list_is_an_error(self):
        root = self.make_repo("fn main() {}\n", {})
        with self.assertRaises(gbw.DataError):
            gbw.parser_builds(root)

    def test_client_builds_map_a_client_build_to_the_build_its_games_write(self):
        root = self.make_repo(
            "pub const TESTED_BUILDS: &[i64] = &[253216, 254477];\n"
            "pub const CLIENT_BUILDS: &[(i64, i64)] = &[(255_130, 254_477)];\n", {})
        self.assertEqual(gbw.client_builds(root), {255130: 254477})

    def test_client_builds_list_is_optional(self):
        root = self.make_repo("pub const TESTED_BUILDS: &[i64] = &[253216];\n", {})
        self.assertEqual(gbw.client_builds(root), {})

    def test_a_client_build_of_an_untested_game_build_is_an_error(self):
        root = self.make_repo(
            "pub const TESTED_BUILDS: &[i64] = &[253216, 254477];\n"
            "pub const CLIENT_BUILDS: &[(i64, i64)] = &[(255130, 254999)];\n", {})
        with self.assertRaises(gbw.DataError):
            gbw.client_builds(root)

    def test_a_client_builds_list_that_does_not_parse_is_an_error(self):
        for body in ("&[(255130_i64, 254477_i64)]", "&[(255130, 254477), /* a; b */]", "&[(255130, 254477), (1,)]"):
            root = self.make_repo(
                "pub const TESTED_BUILDS: &[i64] = &[253216, 254477];\n"
                f"pub const CLIENT_BUILDS: &[(i64, i64)] = {body};\n", {})
            with self.subTest(body=body), self.assertRaises(gbw.DataError):
                gbw.client_builds(root)

    def test_client_builds_may_be_empty_or_span_lines(self):
        head = "pub const TESTED_BUILDS: &[i64] = &[253216, 254477];\n"
        empty = self.make_repo(head + "pub const CLIENT_BUILDS: &[(i64, i64)] = &[];\n", {})
        self.assertEqual(gbw.client_builds(empty), {})
        spread = self.make_repo(
            head + "pub const CLIENT_BUILDS: &[(i64, i64)] = &[\n    (255130, 254477),\n    (255999, 253216),\n];\n", {})
        self.assertEqual(gbw.client_builds(spread), {255130: 254477, 255999: 253216})

    def test_the_repository_lists_parse(self):
        repo = Path(__file__).resolve().parent.parent
        self.assertEqual(gbw.client_builds(repo)[255130], 254477)
        self.assertIn(254477, gbw.parser_builds(repo))

    def test_simulator_builds_list_is_optional(self):
        root = self.make_repo("", {"cards.rs": "pub const X: i32 = 1;\n"})
        self.assertEqual(gbw.simulator_builds(root), [])

    def test_simulator_builds_list_is_read_when_present(self):
        root = self.make_repo("", {"checked.rs": "pub const CHECKED_BUILDS: &[i64] = &[253216,\n 254477];\n"})
        self.assertEqual(gbw.simulator_builds(root), [253216, 254477])

    def test_simulator_card_ids_skip_test_files(self):
        root = self.make_repo("", {
            "cards_round1.rs": '    "BG29_611",\n    "BG29_611_G",\n    "not an id",\n',
            "cards_tests.rs": '    "BG99_999",\n',
        })
        self.assertEqual(gbw.simulator_card_ids(root), {"BG29_611", "BG29_611_G"})


class Diff(unittest.TestCase):
    def test_added_changed_removed(self):
        old = [minion("BG1_001", "Keep"), minion("BG1_002", "Gone"), minion("BG1_003", "Buffed", attack=2)]
        new = [minion("BG1_001", "Keep"), minion("BG1_003", "Buffed", attack=3), minion("BG1_004", "Fresh")]
        diff = gbw.diff_cards(old, new)
        self.assertEqual([c["id"] for c in diff.added], ["BG1_004"])
        self.assertEqual([c["id"] for c in diff.removed], ["BG1_002"])
        self.assertEqual([c["id"] for c in diff.changed], ["BG1_003"])

    def test_same_data_has_no_difference(self):
        cards = [minion("BG1_001"), minion("BG1_002")]
        self.assertTrue(gbw.diff_cards(cards, list(cards)).is_empty())

    def test_other_sets_enchantments_and_skins_are_ignored(self):
        new = [
            {"id": "CS2_001", "name": "Classic", "type": "MINION", "set": "CORE"},
            {"id": "BG1_E", "name": "Ench", "type": "ENCHANTMENT", "set": "BATTLEGROUNDS"},
            {"id": "BG1_HERO_SKIN_A", "name": "Skin", "type": "HERO", "set": "BATTLEGROUNDS"},
        ]
        self.assertTrue(gbw.diff_cards([], new).is_empty())

    def test_the_order_of_a_list_field_is_not_a_change(self):
        a = dict(minion("BG1_001"), mechanics=["TAUNT", "DEATHRATTLE"])
        b = dict(a, mechanics=["DEATHRATTLE", "TAUNT"])
        self.assertTrue(gbw.diff_cards([a], [b]).is_empty())

    def test_a_field_the_game_does_not_show_is_not_a_change(self):
        a = minion("BG1_001")
        b = dict(a, artist="Someone", flavor="Words")
        self.assertTrue(gbw.diff_cards([a], [b]).is_empty())

    def test_data_that_is_not_a_list_of_cards_is_refused(self):
        with self.assertRaises(gbw.DataError):
            gbw.diff_cards([], {"id": "x"})
        with self.assertRaises(gbw.DataError):
            gbw.diff_cards([], ["text"])


class Report(unittest.TestCase):
    def diff(self):
        old = [minion("BG1_003", "Buffed", attack=2), minion("BG1_002", "Gone")]
        new = [minion("BG1_003", "Buffed", attack=3), minion("BG1_004", "Fresh"), minion("BG1_005", "Known")]
        return gbw.diff_cards(old, new)

    def test_report_names_the_build_and_flags_simulator_gaps(self):
        text = gbw.render_report(255000, 254477, self.diff(), {"BG1_003", "BG1_005"}, [254477], [])
        self.assertIn("255000", text)
        self.assertIn("254477", text)
        self.assertIn("`BG1_004` Fresh", text)
        self.assertRegex(text, r"(?s)not in the simulator.*`BG1_004`")
        self.assertRegex(text, r"(?s)to review again.*`BG1_003`")
        self.assertIn("- [ ] ", text)
        for word in ("TESTED_BUILDS", "memory reader", "card data"):
            self.assertIn(word.lower(), text.lower())

    def test_report_never_mentions_private_things(self):
        text = gbw.render_report(255000, 254477, self.diff(), set(), [254477], [])
        for word in ("hub", "session", "D-1", "notes"):
            self.assertNotIn(word, text)

    def test_names_cannot_inject_markup_or_mentions(self):
        bad = minion("BG1_009", "@everyone `x`\n# Title [a](http://e)")
        text = gbw.render_report(255000, 254477, gbw.diff_cards([], [bad]), set(), [254477], [])
        self.assertNotIn("\n# Title", text)
        self.assertNotIn("](http", text)
        self.assertNotIn("`x`", text)

    def test_links_and_references_in_names_are_neutralised(self):
        bad = minion("BG1_009", "see #123 or owner/repo#1 www.evil.example *bold* ~~x~~ :smile:")
        text = gbw.render_report(255000, 254477, gbw.diff_cards([], [bad]), set(), [254477], [])
        for part in ("#123", "repo#1", "www.", "*bold*", "~~", ":smile:"):
            self.assertNotIn(part, text)

    def test_long_lists_are_cut_and_the_count_stays(self):
        many = [minion(f"BG1_{i:04d}", f"Card {i}") for i in range(gbw.MAX_LISTED + 25)]
        text = gbw.render_report(255000, 254477, gbw.diff_cards([], many), set(), [254477], [])
        self.assertIn(f"{gbw.MAX_LISTED + 25} cards", text)
        self.assertIn("25 more", text)
        self.assertLess(len(text), 60000)

    def test_report_says_when_the_simulator_list_is_behind(self):
        text = gbw.render_report(255000, 254477, gbw.Diff([], [], []), set(), [254477], [253216])
        self.assertIn("simulator", text)
        self.assertIn("253216", text)


class Fake:
    """A stand-in for the network and the `gh` command."""

    def __init__(self, listing_html: str, cards: dict[int, object], issues: list[str] | None = None):
        self.listing_html, self.cards, self.issues = listing_html, cards, issues or []
        self.urls: list[str] = []
        self.created: list[tuple[str, str]] = []

    def get(self, url: str, max_bytes: int) -> bytes:
        self.urls.append(url)
        if url == gbw.LISTING_URL:
            return self.listing_html.encode()
        build = int(url.split("/v1/")[1].split("/")[0])
        return json.dumps(self.cards[build]).encode()

    def gh(self, args: list[str], body: str | None = None) -> str:
        if args[:2] == ["issue", "list"]:
            return json.dumps([{"title": t} for t in self.issues])
        if args[:2] == ["issue", "create"]:
            self.created.append((args[args.index("--title") + 1], body or ""))
            return "https://example.invalid/issues/1"
        raise AssertionError(args)


class Run(unittest.TestCase):
    def setUp(self):
        self.root = Path(tempfile.mkdtemp())
        (self.root / "crates/bg-parser/src").mkdir(parents=True)
        (self.root / "crates/bg-parser/src/lib.rs").write_text(
            "pub const TESTED_BUILDS: &[i64] = &[253216, 254477];\n", encoding="utf-8")
        (self.root / "crates/combat-sim/src").mkdir(parents=True)
        (self.root / "crates/combat-sim/src/cards.rs").write_text('"BG1_003",\n', encoding="utf-8")
        self.builds = list(range(254000, 254030)) + [254477]

    def test_same_build_does_nothing_and_asks_once(self):
        fake = Fake(listing(*self.builds), {})
        self.assertEqual(gbw.run(self.root, fake.get, fake.gh), 0)
        self.assertEqual(fake.urls, [gbw.LISTING_URL])
        self.assertEqual(fake.created, [])

    def write_parser(self, extra: str) -> None:
        (self.root / "crates/bg-parser/src/lib.rs").write_text(
            "pub const TESTED_BUILDS: &[i64] = &[253216, 254477];\n" + extra, encoding="utf-8")

    def test_a_client_build_whose_games_write_a_tested_build_does_nothing(self):
        self.write_parser("pub const CLIENT_BUILDS: &[(i64, i64)] = &[(255130, 254477)];\n")
        fake = Fake(listing(*self.builds, 255130), {})
        self.assertEqual(gbw.run(self.root, fake.get, fake.gh), 0)
        self.assertEqual(fake.urls, [gbw.LISTING_URL])
        self.assertEqual(fake.created, [])

    def test_the_same_client_build_without_the_mapping_opens_an_issue(self):
        cards = {254477: [minion("BG1_003")], 255130: [minion("BG1_003")]}
        fake = Fake(listing(*self.builds, 255130), cards)
        self.assertEqual(gbw.run(self.root, fake.get, fake.gh), 0)
        self.assertEqual([title for title, _ in fake.created], ["New game build 255130"])

    def test_a_build_newer_than_every_mapped_client_build_opens_an_issue(self):
        self.write_parser("pub const CLIENT_BUILDS: &[(i64, i64)] = &[(255130, 254477)];\n")
        cards = {254477: [minion("BG1_003")], 255200: [minion("BG1_003"), minion("BG1_004")]}
        fake = Fake(listing(*self.builds, 255130, 255200), cards)
        self.assertEqual(gbw.run(self.root, fake.get, fake.gh), 0)
        self.assertEqual([title for title, _ in fake.created], ["New game build 255200"])

    def test_a_mapping_to_an_untested_game_build_is_bad_data(self):
        self.write_parser("pub const CLIENT_BUILDS: &[(i64, i64)] = &[(255130, 254999)];\n")
        fake = Fake(listing(*self.builds, 255130), {})
        self.assertEqual(gbw.run(self.root, fake.get, fake.gh), 1)
        self.assertEqual(fake.created, [])

    def test_older_build_does_nothing(self):
        fake = Fake(listing(*self.builds[:-1]), {})
        self.assertEqual(gbw.run(self.root, fake.get, fake.gh), 0)
        self.assertEqual(fake.created, [])

    def test_newer_build_opens_one_issue(self):
        cards = {254477: [minion("BG1_003", attack=2)], 255000: [minion("BG1_003", attack=3), minion("BG1_004")]}
        fake = Fake(listing(*self.builds, 255000), cards)
        self.assertEqual(gbw.run(self.root, fake.get, fake.gh), 0)
        self.assertEqual(len(fake.created), 1)
        title, body = fake.created[0]
        self.assertEqual(title, "New game build 255000")
        self.assertIn("BG1_004", body)

    def test_an_issue_that_exists_open_or_closed_is_not_repeated(self):
        fake = Fake(listing(*self.builds, 255000), {254477: [minion("BG1_003")], 255000: [minion("BG1_003")]}, issues=["New game build 255000"])
        self.assertEqual(gbw.run(self.root, fake.get, fake.gh), 0)
        self.assertEqual(fake.created, [])
        self.assertEqual(fake.urls, [gbw.LISTING_URL])

    def test_an_issue_for_another_build_does_not_block(self):
        fake = Fake(listing(*self.builds, 255000), {254477: [minion("BG1_003")], 255000: [minion("BG1_003")]},
                    issues=["New game build 2550001", "New game build 254999"])
        self.assertEqual(gbw.run(self.root, fake.get, fake.gh), 0)
        self.assertEqual(len(fake.created), 1)

    def test_dry_run_creates_nothing(self):
        fake = Fake(listing(*self.builds, 255000), {254477: [minion("BG1_003")], 255000: [minion("BG1_003")]})
        self.assertEqual(gbw.run(self.root, fake.get, None), 0)
        self.assertEqual(fake.created, [])

    def test_bad_card_data_is_an_error_and_opens_nothing(self):
        fake = Fake(listing(*self.builds, 255000), {254477: [], 255000: {"not": "a list"}})
        self.assertEqual(gbw.run(self.root, fake.get, fake.gh), 1)
        self.assertEqual(fake.created, [])

    def test_no_battlegrounds_cards_at_all_is_bad_data(self):
        fake = Fake(listing(*self.builds, 255000), {254477: [], 255000: []})
        self.assertEqual(gbw.run(self.root, fake.get, fake.gh), 1)
        self.assertEqual(fake.created, [])

    def test_card_data_not_published_yet_is_not_an_error(self):
        def get(url: str, max_bytes: int) -> bytes:
            if url == gbw.LISTING_URL:
                return listing(*self.builds, 255000).encode()
            if "/254477/" in url:
                return json.dumps([minion("BG1_003")]).encode()
            raise urllib.error.HTTPError(url, 404, "Not Found", None, None)

        fake = Fake("", {})
        self.assertEqual(gbw.run(self.root, get, fake.gh), 0)
        self.assertEqual(fake.created, [])

    def test_a_cut_answer_is_a_network_error(self):
        def get(url: str, max_bytes: int) -> bytes:
            raise http.client.IncompleteRead(b"x")

        self.assertEqual(gbw.run(self.root, get, None), 2)

    def test_network_error_is_exit_2(self):
        def broken(url: str, max_bytes: int) -> bytes:
            raise OSError("down")

        self.assertEqual(gbw.run(self.root, broken, None), 2)


if __name__ == "__main__":
    unittest.main()
