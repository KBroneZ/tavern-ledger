"""Writes the desktop app's Dark Gift words from the website's table.

    python tools/gift_words.py          # write crates/desktop/ui/strings/gifts.js
    python tools/gift_words.py --check  # exit 1 when it is not up to date

`web/src/data/gifts.json` (T-528, D-107) is the one table of the season's
Dark Gifts: each gift's name in English and Spanish (from HearthstoneJSON's
card data) and a short summary written for this project. The website reads
it directly; the desktop app reads `strings/gifts.js`, made from it here so
the two never drift. Standard library only.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
TABLE = ROOT / "web" / "src" / "data" / "gifts.json"
OUT = ROOT / "crates" / "desktop" / "ui" / "strings" / "gifts.js"
PREFIX = "BG36_MidGameEffect_"
SHORT = re.compile(r"^000t\d{0,3}t?$")

# Words around the gift names, [key, English, Spanish].
FIXED = [
    ["gift.mark", "Dark Gift", "Don siniestro"],
    ["gift.tip", "{name}: {text}", "{name}: {text}"],
    ["gift.unknown", "Dark Gift (unknown)", "Don siniestro (desconocido)"],
    ["gift.unknown_text", "The game marks a Dark Gift, but its log does not say which.",
     "El juego marca un don siniestro, pero su registro no dice cuál."],
]

HEAD = """// Dark Gift words (T-528, D-107): written by tools/gift_words.py from
// web/src/data/gifts.json; do not edit by hand. Each gift is keyed by its
// gift spell's card id after BG36_MidGameEffect_ (gift.name.000t9,
// gift.text.000t9). Each row is [key, English, Spanish]. Prefix: gift.
"use strict";

(() => {
  const rows = [
"""

TAIL = """  ];

  const table = { en: {}, es: {} };
  rows.forEach(([key, en, es]) => {
    table.en[key] = en;
    table.es[key] = es;
  });

  const i18n = typeof module !== "undefined" && module.exports ? require("../i18n.js") : window.TLI18n;
  i18n.define(table);
  if (typeof module !== "undefined" && module.exports) module.exports = table;
})();
"""


def rows(table: dict) -> list[list[str]]:
    """The i18n rows, refusing a table that is not the expected shape."""
    gifts = table.get("gifts")
    if not isinstance(gifts, dict) or not gifts:
        raise ValueError("gifts.json has no gifts")
    out = [list(r) for r in FIXED]
    for gift_id, words in gifts.items():
        short = gift_id.removeprefix(PREFIX)
        if short == gift_id or not SHORT.match(short):
            raise ValueError(f"not a gift id: {gift_id!r}")
        for part in ("name", "text"):
            en, es = words["en"][part], words["es"][part]
            if not (isinstance(en, str) and en.strip() and isinstance(es, str) and es.strip()):
                raise ValueError(f"{gift_id}: empty {part}")
            out.append([f"gift.{part}.{short}", en, es])
    return out


def render(table: dict) -> str:
    lines = [f"    {json.dumps(r, ensure_ascii=False)},\n" for r in rows(table)]
    return HEAD + "".join(lines) + TAIL


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--check", action="store_true", help="only check that the file is current")
    args = ap.parse_args(argv)
    text = render(json.loads(TABLE.read_text(encoding="utf-8")))
    if args.check:
        current = OUT.read_text(encoding="utf-8") if OUT.exists() else ""
        if current != text:
            print(f"{OUT.relative_to(ROOT)} is out of date: run python tools/gift_words.py",
                  file=sys.stderr)
            return 1
        return 0
    OUT.write_text(text, encoding="utf-8", newline="\n")
    print(f"wrote {OUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
