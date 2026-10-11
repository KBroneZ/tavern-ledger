"""Tests for tools/hero_names.py with synthetic card data (made-up heroes, no network)."""

import json
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "tools"))

import hero_names as hn  # noqa: E402


def cards(lang: str, bases: int = hn.MIN_BASES, skins_per_base: int = 3) -> list[dict]:
    """Enough made-up heroes and skins to pass the size check, plus the odd cases."""
    suffix = " (es)" if lang == "es" else ""
    out: list[dict] = []
    for i in range(bases):
        out.append({"id": f"SYN_HERO_{i:03d}", "dbfId": 1000 + i, "name": f"Hero {i}{suffix}",
                    "type": "HERO", "set": "BATTLEGROUNDS", "battlegroundsHero": True})
        for j in range(skins_per_base):
            out.append({"id": f"SYN_HERO_{i:03d}_SKIN_{'ABC'[j]}", "dbfId": 5000 + i * 10 + j,
                        "name": f"Skin {i}{j}", "type": "HERO", "set": "BATTLEGROUNDS",
                        "battlegroundsSkinParentId": 1000 + i})
    out += [
        # A skin whose id names another card: listed.
        {"id": "OLD_HERO_7_SKIN_A", "dbfId": 9001, "name": "Odd skin", "type": "HERO",
         "set": "BATTLEGROUNDS", "battlegroundsSkinParentId": 1007},
        # A retired hero (no battlegroundsHero flag) and its skin.
        {"id": "SYN_RETIRED", "dbfId": 9100, "name": f"Retired{suffix}", "type": "HERO", "set": "BATTLEGROUNDS"},
        {"id": "SYN_RETIRED_SKIN_A", "dbfId": 9101, "name": "Retired skin", "type": "HERO",
         "set": "BATTLEGROUNDS", "battlegroundsSkinParentId": 9100},
        # Not heroes a player plays.
        {"id": "TB_BaconShopBob", "dbfId": 9200, "name": "Bob", "type": "HERO", "set": "BATTLEGROUNDS"},
        {"id": "TB_BaconShopBob_SKIN_A", "dbfId": 9201, "name": "Bob skin", "type": "HERO",
         "set": "BATTLEGROUNDS", "battlegroundsSkinParentId": 9200},
        {"id": "SYN_HERO_000_Tutorial", "dbfId": 9300, "name": "Tutorial", "type": "HERO", "set": "BATTLEGROUNDS"},
        {"id": "SYN_MINION", "dbfId": 9400, "name": "Minion", "type": "MINION", "set": "BATTLEGROUNDS"},
        {"id": "bad id", "name": "x", "battlegroundsHero": True},
    ]
    return out


class BuildTable(unittest.TestCase):
    def test_bases_in_both_languages_and_only_odd_skins_listed(self):
        names, skins = hn.build_table(cards("en"), cards("es"))
        self.assertEqual(names["SYN_HERO_001"], ("Hero 1", "Hero 1 (es)"))
        self.assertEqual(names["SYN_RETIRED"], ("Retired", "Retired (es)"))
        self.assertEqual(skins, {"OLD_HERO_7_SKIN_A": "SYN_HERO_007"})
        self.assertNotIn("TB_BaconShopBob", names)
        self.assertNotIn("SYN_HERO_000_Tutorial", names)
        self.assertNotIn("SYN_MINION", names)
        self.assertNotIn("bad id", names)
        # Skins whose id gives their base by the rule are not listed.
        self.assertEqual(hn.skin_rule("SYN_RETIRED_SKIN_A"), "SYN_RETIRED")

    def test_a_hero_without_a_spanish_card_is_refused(self):
        spanish = [c for c in cards("es") if c["id"] != "SYN_HERO_003"]
        with self.assertRaises(hn.DataError):
            hn.build_table(cards("en"), spanish)

    def test_too_little_data_is_refused(self):
        with self.assertRaises(hn.DataError):
            hn.build_table(cards("en", bases=10), cards("es", bases=10))
        with self.assertRaises(hn.DataError):
            hn.build_table({"not": "a list"}, [])

    def test_an_empty_or_huge_name_is_refused(self):
        english = cards("en")
        english[0] = {**english[0], "name": " "}
        with self.assertRaises(hn.DataError):
            hn.build_table(english, cards("es"))

    def test_render_is_sorted_and_escaped(self):
        text = hn.render({"B": ('Say "hi"', "Señor"), "A": ("A", "A")}, {"B_SKIN_X": "A"})
        self.assertLess(text.index('"A": ["A", "A"]'), text.index('"B": '))
        self.assertIn('["Say \\"hi\\"", "Señor"]', text)
        self.assertIn('"B_SKIN_X": "A",', text)
        self.assertTrue(text.endswith("};\n"))


class Main(unittest.TestCase):
    def test_from_folder_writes_and_check_agrees(self):
        with tempfile.TemporaryDirectory() as tmp:
            src = Path(tmp)
            (src / "enUS.json").write_text(json.dumps(cards("en")), encoding="utf-8")
            (src / "esES.json").write_text(json.dumps(cards("es")), encoding="utf-8")
            out = src / "names.data.ts"
            with mock.patch.object(hn, "OUT", out):
                self.assertEqual(hn.main(["--from", str(src), "--check"]), 1, "nothing written yet")
                self.assertEqual(hn.main(["--from", str(src)]), 0)
                self.assertIn('"SYN_HERO_001": ["Hero 1", "Hero 1 (es)"]', out.read_text(encoding="utf-8"))
                self.assertEqual(hn.main(["--from", str(src), "--check"]), 0)

    def test_bad_data_exits_1_and_writes_nothing(self):
        with tempfile.TemporaryDirectory() as tmp:
            src = Path(tmp)
            (src / "enUS.json").write_text("[]", encoding="utf-8")
            (src / "esES.json").write_text("[]", encoding="utf-8")
            out = src / "names.data.ts"
            with mock.patch.object(hn, "OUT", out):
                self.assertEqual(hn.main(["--from", str(src)]), 1)
            self.assertFalse(out.exists())


if __name__ == "__main__":
    unittest.main()
