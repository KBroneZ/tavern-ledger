"""Names the cards of the accuracy export for the website (T-607, D-091).

    python tools/accuracy_card_names.py --export web/src/data/accuracy.json \
        --cards-en cards.json --cards-es cards.es.json \
        --out web/src/data/accuracy-cards.json

Reads HearthstoneJSON's cards.json in English and Spanish (fetched by hand
into a temporary folder, never committed) and writes, for each card id in the
export's blocking_cards only, its name in both languages and whether it is
the golden card. Nothing else from the card data is kept: no text, art or
other cards. A card missing from either file stops the script with no output,
so the page never shows half a list.
"""

import argparse
import json
import re
import sys
from pathlib import Path

CARD_ID = re.compile(r"^[A-Za-z0-9_]{1,64}$")
FORMAT = 1


def load_cards(path):
    """Card id to card, from a HearthstoneJSON cards.json (a list of cards)."""
    data = json.loads(Path(path).read_text(encoding="utf-8"))
    if not isinstance(data, list):
        raise ValueError(f"{path}: expected a list of cards")
    return {c["id"]: c for c in data if isinstance(c, dict) and isinstance(c.get("id"), str)}


def card_ids(export):
    """The blocking card ids of an accuracy export, checked."""
    if export.get("format") != 1:
        raise ValueError("export: format is not 1")
    ids = []
    for card in export.get("blocking_cards", []):
        card_id = card.get("card_id") if isinstance(card, dict) else None
        if not isinstance(card_id, str) or not CARD_ID.match(card_id):
            raise ValueError(f"export: not a card id: {card_id!r}")
        ids.append(card_id)
    return ids


def is_golden(card):
    """A golden Battlegrounds card links to its normal card; older ones are premium."""
    return "battlegroundsNormalDbfId" in card or bool(card.get("premium"))


def names(ids, english, spanish):
    """{id: {en, es, golden}} for each id; raises when a card or a name is missing."""
    out = {}
    for card_id in ids:
        en = english.get(card_id)
        es = spanish.get(card_id)
        if en is None or es is None:
            raise ValueError(f"{card_id}: not in the card data")
        en_name, es_name = en.get("name"), es.get("name")
        if not isinstance(en_name, str) or not en_name.strip() or not isinstance(es_name, str) or not es_name.strip():
            raise ValueError(f"{card_id}: no name")
        out[card_id] = {"en": en_name, "es": es_name, "golden": is_golden(en)}
    return out


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--export", required=True)
    parser.add_argument("--cards-en", required=True)
    parser.add_argument("--cards-es", required=True)
    parser.add_argument("--out", required=True)
    args = parser.parse_args(argv)
    try:
        export = json.loads(Path(args.export).read_text(encoding="utf-8"))
        result = names(card_ids(export), load_cards(args.cards_en), load_cards(args.cards_es))
    except (OSError, ValueError, KeyError) as error:
        print(f"accuracy_card_names: {error}", file=sys.stderr)
        return 1
    text = json.dumps({"format": FORMAT, "cards": result}, ensure_ascii=False, indent=2, sort_keys=True) + "\n"
    Path(args.out).write_text(text, encoding="utf-8", newline="\n")
    print(f"{len(result)} card names written to {args.out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
