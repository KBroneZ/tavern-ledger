"""Tests for tools/accuracy_card_names.py with synthetic cards (made-up ids and names)."""

import io
import json
import sys
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / "tools"))

import accuracy_card_names as acn  # noqa: E402

EXPORT = {
    "format": 2,
    "blocking_cards": [{"card_id": "BGX_1", "combats": 4}, {"card_id": "BGX_1_G", "combats": 1}],
}
ENGLISH = [
    {"id": "BGX_1", "name": "Made-up Minion", "text": "Some text"},
    {"id": "BGX_1_G", "name": "Made-up Minion", "battlegroundsNormalDbfId": 1},
    {"id": "BGX_2", "name": "Not Blocking"},
]
SPANISH = [
    {"id": "BGX_1", "name": "Esbirro inventado"},
    {"id": "BGX_1_G", "name": "Esbirro inventado", "battlegroundsNormalDbfId": 1},
    {"id": "BGX_2", "name": "No bloquea"},
]


class AccuracyCardNamesTest(unittest.TestCase):
    def run_tool(self, export=EXPORT, english=ENGLISH, spanish=SPANISH):
        with tempfile.TemporaryDirectory() as tmp:
            d = Path(tmp)
            (d / "export.json").write_text(json.dumps(export), encoding="utf-8")
            (d / "en.json").write_text(json.dumps(english), encoding="utf-8")
            (d / "es.json").write_text(json.dumps(spanish), encoding="utf-8")
            out = d / "names.json"
            err = io.StringIO()
            with redirect_stdout(io.StringIO()), redirect_stderr(err):
                code = acn.main(["--export", str(d / "export.json"), "--cards-en", str(d / "en.json"),
                                 "--cards-es", str(d / "es.json"), "--out", str(out)])
            raw = out.read_bytes() if out.exists() else None
            return code, raw, err.getvalue()

    def test_names_only_the_blocking_cards_in_both_languages(self):
        code, raw, _ = self.run_tool()
        self.assertEqual(code, 0)
        data = json.loads(raw)
        self.assertEqual(data, {
            "format": 1,
            "cards": {
                "BGX_1": {"en": "Made-up Minion", "es": "Esbirro inventado", "golden": False},
                "BGX_1_G": {"en": "Made-up Minion", "es": "Esbirro inventado", "golden": True},
            },
        })
        self.assertNotIn(b"text", raw)
        self.assertNotIn(b"\r\n", raw)

    def test_a_missing_card_writes_nothing(self):
        code, raw, err = self.run_tool(spanish=SPANISH[1:])
        self.assertEqual(code, 1)
        self.assertIsNone(raw)
        self.assertIn("BGX_1", err)

    def test_another_format_or_a_bad_id_writes_nothing(self):
        code, raw, _ = self.run_tool(export={**EXPORT, "format": 1})
        self.assertEqual((code, raw), (1, None))
        code, raw, _ = self.run_tool(export={"format": 2, "blocking_cards": [{"card_id": "../x", "combats": 1}]})
        self.assertEqual((code, raw), (1, None))

    def test_card_data_that_is_not_a_list_writes_nothing(self):
        code, raw, _ = self.run_tool(english={"cards": []})
        self.assertEqual((code, raw), (1, None))


if __name__ == "__main__":
    unittest.main()
