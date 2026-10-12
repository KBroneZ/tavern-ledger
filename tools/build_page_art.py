"""Builds every page's painted backdrop from the user's Gemini paintings (T-546).

Sources (docs/brand/art/pages/, made by the user's Gemini account for this project, 2752 x 1536,
saved from the original JPEGs as WebP):
- sim-duel-table.webp      a duel table between a fireplace (left) and blue magic (right)
- leaderboards-hall.webp   a hall of shields, trophies and banners
- heroes-gallery.webp      a corridor of empty portrait frames
- ledger-archive.webp      shelves of ledgers and an open book on a desk
- account-study.webp       a small study with a chest, a desk and a hearth

Writes web/public/art/pages/, for each scene (the name the pages use, SCENES below):
- <scene>-<w>.avif, <scene>-<w>.webp   the whole painting at 1080, 1344 and 1920 px wide
- <scene>-phone.avif, <scene>-phone.webp   a portrait crop of its middle, 800 px wide, for phones

Needs Pillow (with AVIF), a local tool only: it is not in requirements.txt and nothing here ships
except the images (PROVENANCE.md).

Usage:
    python tools/build_page_art.py
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from build_home_art import AVIF_QUALITY, ROOT, WEBP_QUALITY  # noqa: E402

ART = ROOT / "docs" / "brand" / "art" / "pages"
OUT = ROOT / "web" / "public" / "art" / "pages"
SIZE = (2752, 1536)
WIDTHS = (1080, 1344, 1920)
PHONE_WIDTH = 800
PHONE_CROP = 1100
# Scene name -> (source file, the x of the phone crop's middle in the painting).
SCENES = {
    "duel": ("sim-duel-table.webp", 1400),
    "hall": ("leaderboards-hall.webp", 1376),
    "gallery": ("heroes-gallery.webp", 1376),
    "archive": ("ledger-archive.webp", 1460),
    "study": ("account-study.webp", 1240),
}


def phone_box(centre: int) -> tuple[int, int, int, int]:
    """The portrait crop around `centre`, kept inside the painting."""
    left = min(max(centre - PHONE_CROP // 2, 0), SIZE[0] - PHONE_CROP)
    return (left, 0, left + PHONE_CROP, SIZE[1])


def save_pair(img, name: str) -> list[str]:
    img.save(OUT / f"{name}.avif", "AVIF", quality=AVIF_QUALITY, speed=4)
    img.save(OUT / f"{name}.webp", "WEBP", quality=WEBP_QUALITY, method=6)
    return [f"{name}.avif", f"{name}.webp"]


def save_scene(scene: str, painting, centre: int) -> list[str]:
    from PIL import Image

    written = []
    for width in WIDTHS:
        height = round(painting.height * width / painting.width)
        written += save_pair(painting.resize((width, height), Image.Resampling.LANCZOS), f"{scene}-{width}")
    crop = painting.crop(phone_box(centre))
    height = round(crop.height * PHONE_WIDTH / crop.width)
    written += save_pair(crop.resize((PHONE_WIDTH, height), Image.Resampling.LANCZOS), f"{scene}-phone")
    return written


def main() -> int:
    from PIL import Image

    OUT.mkdir(parents=True, exist_ok=True)
    for scene, (source, centre) in SCENES.items():
        painting = Image.open(ART / source).convert("RGB")
        if painting.size != SIZE:
            raise SystemExit(f"{source} is {painting.size}, not {SIZE}: check the phone crop")
        for name in save_scene(scene, painting, centre):
            print(f"wrote web/public/art/pages/{name} ({(OUT / name).stat().st_size // 1024} KiB)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
