"""Builds the simulator page's parchment from the user's Gemini panel sheet (T-526, D-112).

Source (docs/brand/art/home/, made by the user's Gemini account for this project):
- panels-sheet.webp    four panels (carved oak, iron corner brackets, burnt parchment) on black

Writes web/public/art/sim/:
- panel-tall.webp      the tall card of the panels sheet (the home page does not use it),
                       nine-sliced by the page (sim.css)

The page's painting behind it is every page's backdrop since T-546 (tools/build_page_art.py).

The tall card is cut like the home page's panels (tools/build_home_art.py, whose helpers it uses):
the black backdrop is flooded in from the crop's edges, the loose embers are dropped, and it is
scaled so its iron corners come out about 80 px, as the other panels.

Needs Pillow (with AVIF) and numpy, local tools only: they are not in requirements.txt and nothing
here ships except the images (PROVENANCE.md).

Usage:
    python tools/build_sim_art.py
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from build_home_art import ART, MIN_AREA, PANEL_DARK, PANEL_QUALITY, ROOT, SHEET_SIZE, silhouette, with_alpha  # noqa: E402

OUT = ROOT / "web" / "public" / "art" / "sim"
# Measured on panels-sheet.webp (2752 x 1536): the tall card on the right.
TALL_BOX = (1770, 66, 2666, 1194)
TALL_SCALE = 0.47


def save_tall_panel() -> list[str]:
    from PIL import Image
    import numpy as np

    sheet = Image.open(ART / "panels-sheet.webp").convert("RGB")
    if sheet.size != SHEET_SIZE:
        raise SystemExit(f"panels-sheet.webp is {sheet.size}, not {SHEET_SIZE}: measure the box again")
    crop = sheet.crop(TALL_BOX)
    mask = silhouette(np.asarray(crop), PANEL_DARK)
    if mask.mean() < MIN_AREA:
        raise SystemExit(f"panel-tall: the cut covers {mask.mean():.0%} of its box; the outline was lost")
    ys, xs = np.nonzero(mask)
    img = with_alpha(crop, mask).crop((int(xs.min()), int(ys.min()), int(xs.max()) + 1, int(ys.max()) + 1))
    size = (round(img.width * TALL_SCALE), round(img.height * TALL_SCALE))
    img.resize(size, Image.Resampling.LANCZOS).save(OUT / "panel-tall.webp", "WEBP", quality=PANEL_QUALITY, method=6, exact=True)
    return ["panel-tall.webp"]


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    for name in save_tall_panel():
        print(f"wrote web/public/art/sim/{name} ({(OUT / name).stat().st_size // 1024} KiB)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
