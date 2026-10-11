"""Tests for tools/gift_words.py and the Dark Gift table (T-528, D-107)."""

import json
import re
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
sys.path.insert(0, str(ROOT / "tools"))

import gift_words  # noqa: E402

GIFT_ID = re.compile(r'"(BG36_MidGameEffect_000t\d{0,3}t?)"')


def ids_in(path: Path, start: str, end: str) -> set[str]:
    """Gift ids quoted between two markers of a Rust source file."""
    text = path.read_text(encoding="utf-8")
    body = text[text.index(start):]
    return set(GIFT_ID.findall(body[:body.index(end)]))


class GiftWordsTest(unittest.TestCase):
    def setUp(self) -> None:
        self.table = json.loads(gift_words.TABLE.read_text(encoding="utf-8"))

    def test_the_desktop_file_is_current(self) -> None:
        self.assertEqual(gift_words.main(["--check"]), 0)

    def test_every_gift_the_parser_and_the_engine_know_has_words(self) -> None:
        parser = ids_in(ROOT / "crates/bg-parser/src/gifts.rs", "pub const GIFT_IDS", "];")
        engine = ids_in(ROOT / "crates/combat-sim/src/gifts.rs", "pub const GIFTS", "\n];")
        self.assertEqual(len(parser), 47)
        self.assertEqual(parser, engine)
        self.assertEqual(set(self.table["gifts"]), parser)

    def test_every_gift_has_a_name_and_a_summary_in_both_languages(self) -> None:
        for gift_id, words in self.table["gifts"].items():
            for lang in ("en", "es"):
                self.assertTrue(words[lang]["name"].strip(), (gift_id, lang))
                self.assertTrue(words[lang]["text"].strip(), (gift_id, lang))
                self.assertNotIn("<", words[lang]["text"], "no card markup")

    def test_a_bad_table_is_refused(self) -> None:
        with self.assertRaises(ValueError):
            gift_words.rows({"gifts": {}})
        with self.assertRaises(ValueError):
            gift_words.rows({"gifts": {"BG36_Other": {}}})
        empty = {"en": {"name": "A", "text": " "}, "es": {"name": "B", "text": "C"}}
        with self.assertRaises(ValueError):
            gift_words.rows({"gifts": {"BG36_MidGameEffect_000t9": empty}})

    def test_rows_are_keyed_by_the_short_gift_id(self) -> None:
        keys = [r[0] for r in gift_words.rows(self.table)]
        self.assertIn("gift.name.000t9", keys)
        self.assertIn("gift.text.000t64t", keys)
        self.assertEqual(len(keys), len(set(keys)))


if __name__ == "__main__":
    unittest.main()
