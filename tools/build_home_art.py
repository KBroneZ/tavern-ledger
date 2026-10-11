"""Builds the home page's images from the user's Gemini paintings (T-530, D-105).

Sources (docs/brand/art/home/, made by the user's Gemini account for this project):
- tavern-tall.webp     1344 x 3168, the barrel door at the top down to the cellar
- buttons-sheet.webp   four buttons painted on a dark tavern backdrop

Writes web/public/art/home/:
- tavern-<w>.avif, tavern-<w>.webp   the backdrop at 800, 1080 and 1344 px wide
- btn-key.webp, btn-key-lit.webp     gold stone with gems, at rest and lit (hover), each also @2x
- btn-common.webp                    dark wood and bronze, also @2x
- btn-small.webp                     small stone with a thin gold frame, also @2x
- paper-edge.svg                     a torn, uneven sheet edge, the mask for the parchment (our own drawing)
- grain.svg                          fine fibre noise laid over parchment and wood (our own, an SVG filter)

Each button is cut out of the sheet: the dark backdrop is flooded in from the crop's edges, small
gaps in the outline are closed, every hole inside is filled (the recess between frame and plate is
part of the button), and only the piece that holds the crop's centre is kept. The lit button's glow
spills over the backdrop, so it is cut with the plain button's outline after lining the two up
(the page draws the glow around it in CSS).

Needs Pillow (with AVIF) and numpy, local tools only: they are not in requirements.txt and nothing
here ships except the images (PROVENANCE.md).

Usage:
    python tools/build_home_art.py             write every file
    python tools/build_home_art.py --preview   also write the cut buttons on magenta to .local/art-preview/
"""

from __future__ import annotations

import argparse
from collections import deque
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ART = ROOT / "docs" / "brand" / "art" / "home"
OUT = ROOT / "web" / "public" / "art" / "home"
PREVIEW = ROOT / ".local" / "art-preview"

BACKDROP_WIDTHS = (800, 1080, 1344)
AVIF_QUALITY = 46
WEBP_QUALITY = 74
BUTTON_QUALITY = 88


@dataclass(frozen=True)
class Cut:
    name: str
    box: tuple[int, int, int, int]  # left, top, right, bottom in the sheet
    dark: int                       # luminance under which a dull pixel counts as backdrop
    height: int                     # CSS height at 1x; the @2x file is twice as tall
    close: int = 5                  # px: gaps in the outline narrower than this are closed


# Boxes measured on buttons-sheet.webp (2752 x 1536). The two gold buttons share one outline.
KEY = Cut("btn-key", (600, 548, 2160, 965), 60, 100)
KEY_LIT_BOX = (600, 40, 2160, 530)
# The wood is as dark as the backdrop and its frame is broken between the corner plates.
COMMON = Cut("btn-common", (860, 968, 1900, 1275), 60, 56, close=31)
SMALL = Cut("btn-small", (1100, 1285, 1650, 1485), 60, 44)
OPEN = 7  # px: anything joined to the button by a thread thinner than this is dropped
SHEET_SIZE = (2752, 1536)  # the boxes below are measured on this export
MIN_AREA = 0.25  # a cut must cover at least this share of its box, or the outline was lost
MAX_MISMATCH = 1500.0  # mean squared grey difference over the outline: lined up the sheet gives about 1300, 8 px off about 1600
DULL = 90  # max - min channel: anything this grey and this dark is backdrop


def flood(passable, seeds):
    """Marks every passable pixel 4-connected to a seed. passable: 2D bool array."""
    import numpy as np

    h, w = passable.shape
    seen = np.zeros((h, w), dtype=bool)
    queue = deque()
    for y, x in seeds:
        if passable[y, x] and not seen[y, x]:
            seen[y, x] = True
            queue.append((y, x))
    while queue:
        y, x = queue.popleft()
        for ny, nx in ((y + 1, x), (y - 1, x), (y, x + 1), (y, x - 1)):
            if 0 <= ny < h and 0 <= nx < w and passable[ny, nx] and not seen[ny, nx]:
                seen[ny, nx] = True
                queue.append((ny, nx))
    return seen


def edge_seeds(h: int, w: int):
    return [(y, x) for x in range(w) for y in (0, h - 1)] + [(y, x) for y in range(h) for x in (0, w - 1)]


def fill_holes(mask):
    """The mask plus every region it encloses (anything the edges cannot reach)."""
    outside = flood(~mask, edge_seeds(*mask.shape))
    return ~outside


def centre_piece(mask):
    """Only the 4-connected piece of the mask that holds the centre."""
    h, w = mask.shape
    return flood(mask, [(h // 2, w // 2)])


def morph(mask, size: int, grow: bool):
    from PIL import Image, ImageFilter
    import numpy as np

    img = Image.fromarray(mask.astype(np.uint8) * 255)
    img = img.filter(ImageFilter.MaxFilter(size) if grow else ImageFilter.MinFilter(size))
    return np.asarray(img) > 127


def silhouette(rgb, dark: int, close: int = 5):
    """The button's outline in a crop of the sheet: True on the button."""
    import numpy as np

    a = rgb.astype(int)
    lum = a[..., 0] * 0.299 + a[..., 1] * 0.587 + a[..., 2] * 0.114
    backdrop = (lum < dark) & (a.max(-1) - a.min(-1) < DULL)
    button = ~flood(backdrop, edge_seeds(*backdrop.shape))
    button = morph(morph(button, close, True), close, False)
    # Opening drops thin strays (a speck of glow, a drip of paint) joined to the button by a thread.
    button = morph(morph(fill_holes(button), OPEN, False), OPEN, True)
    button = centre_piece(button)
    # One pixel in from the edge drops the dark rim the flood stopped on.
    return morph(button, 3, False)


def with_alpha(rgb_image, mask):
    """RGBA image: the mask, softened by one pixel, as alpha."""
    from PIL import Image, ImageFilter
    import numpy as np

    alpha = Image.fromarray(mask.astype(np.uint8) * 255).filter(ImageFilter.GaussianBlur(0.8))
    out = rgb_image.convert("RGBA")
    out.putalpha(alpha)
    return out


def best_offset(plain, lit_area, search: int = 40):
    """Where the plain button's crop sits in the lit area (dx, dy), by the least squared
    difference of the plate and frame (the glow outside the outline is ignored)."""
    import numpy as np

    p = plain.astype(float).mean(-1)
    l = lit_area.astype(float).mean(-1)
    ph, pw = p.shape
    # The middle half of the button: plate, frame and gems, no glow.
    y0, y1, x0, x1 = ph // 4, ph * 3 // 4, pw // 8, pw * 7 // 8
    core = p[y0:y1, x0:x1]
    core = core - core.mean()
    def search_around(ox: int, oy: int, span: int, step: int) -> tuple[int, int]:
        best, where = None, (ox, oy)
        for dy in range(-span, span + 1, step):
            for dx in range(-span, span + 1, step):
                ty, tx = oy + dy + y0, ox + dx + x0
                if ty < 0 or tx < 0 or ty + core.shape[0] > l.shape[0] or tx + core.shape[1] > l.shape[1]:
                    continue
                win = l[ty:ty + core.shape[0], tx:tx + core.shape[1]]
                score = float(((win - win.mean() - core) ** 2).mean())
                if best is None or score < best:
                    best, where = score, (ox + dx, oy + dy)
        return where

    # Coarse, then fine around the coarse answer.
    coarse = search_around((l.shape[1] - pw) // 2, (l.shape[0] - ph) // 2, search, 4)
    return search_around(*coarse, 4, 1)


def cut(sheet, spec: Cut):
    """(RGBA image cropped to the button, the mask's bounding box within spec.box)."""
    import numpy as np

    crop = sheet.crop(spec.box)
    mask = silhouette(np.asarray(crop), spec.dark, spec.close)
    if mask.mean() < MIN_AREA:
        raise SystemExit(f"{spec.name}: the cut covers {mask.mean():.0%} of its box; the outline was lost")
    ys, xs = np.nonzero(mask)
    bbox = (int(xs.min()), int(ys.min()), int(xs.max()) + 1, int(ys.max()) + 1)
    return with_alpha(crop, mask).crop(bbox), mask[bbox[1]:bbox[3], bbox[0]:bbox[2]], bbox


def cut_lit(sheet, plain_box, plain_bbox, mask):
    """The lit button, cut with the plain one's outline once both are lined up."""
    import numpy as np

    left, top = plain_box[0] + plain_bbox[0], plain_box[1] + plain_bbox[1]
    w, h = plain_bbox[2] - plain_bbox[0], plain_bbox[3] - plain_bbox[1]
    plain = np.asarray(sheet.crop((left, top, left + w, top + h)))
    area = sheet.crop(KEY_LIT_BOX)
    dx, dy = best_offset(plain, np.asarray(area))
    lit = area.crop((dx, dy, dx + w, dy + h))
    p, l = plain.astype(float).mean(-1), np.asarray(lit).astype(float).mean(-1)
    mismatch = float((((l - l.mean()) - (p - p.mean()))[mask] ** 2).mean())
    if mismatch > MAX_MISMATCH:
        raise SystemExit(f"btn-key-lit: the lit and plain buttons do not line up (mismatch {mismatch:.0f})")
    return with_alpha(lit, mask)


def scaled(image, height: int):
    from PIL import Image

    width = round(image.width * height / image.height)
    return image.resize((width, height), Image.Resampling.LANCZOS)


def save_button(image, name: str, height: int) -> list[str]:
    written = []
    for suffix, h in (("", height), ("@2x", height * 2)):
        path = OUT / f"{name}{suffix}.webp"
        scaled(image, h).save(path, "WEBP", quality=BUTTON_QUALITY, method=6, exact=True)
        written.append(path.name)
    return written


def save_backdrop(painting) -> list[str]:
    from PIL import Image

    written = []
    for width in BACKDROP_WIDTHS:
        height = round(painting.height * width / painting.width)
        img = painting if width == painting.width else painting.resize((width, height), Image.Resampling.LANCZOS)
        img.save(OUT / f"tavern-{width}.avif", "AVIF", quality=AVIF_QUALITY, speed=4)
        img.save(OUT / f"tavern-{width}.webp", "WEBP", quality=WEBP_QUALITY, method=6)
        written += [f"tavern-{width}.avif", f"tavern-{width}.webp"]
    return written


def torn_edge(seed: int = 92, width: int = 1000, height: int = 400, step: int = 10, depth: float = 9.0) -> str:
    """SVG mask: a sheet with torn edges (deterministic). Stretched to any box
    (preserveAspectRatio none), so the tears scale with the sheet."""
    import random

    rnd = random.Random(seed)

    def jag(n: int) -> list[float]:
        # Small tears on a slow wave, never deeper than `depth`.
        out, drift = [], 0.0
        for _ in range(n + 1):
            drift = max(0.0, min(depth, drift + rnd.uniform(-2.6, 2.6)))
            out.append(round(drift + rnd.uniform(0, 2.2), 1))
        return out

    nx, ny = width // step, height // step
    top, bottom = jag(nx), jag(nx)
    left, right = jag(ny), jag(ny)
    pts = [(i * step, top[i]) for i in range(nx + 1)]
    pts += [(width - right[j], j * step) for j in range(1, ny + 1)]
    pts += [(width - i * step, height - bottom[i]) for i in range(1, nx + 1)]
    pts += [(left[j], height - j * step) for j in range(1, ny)]
    path = "M" + " L".join(f"{x:g} {y:g}" for x, y in pts) + " Z"
    return (f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width} {height}" preserveAspectRatio="none">'
            f'<path d="{path}" fill="#000"/></svg>\n')


GRAIN_SVG = (
    '<svg xmlns="http://www.w3.org/2000/svg" width="220" height="220">'
    '<filter id="g"><feTurbulence type="fractalNoise" baseFrequency="0.9 0.35" numOctaves="3" seed="9" stitchTiles="stitch"/>'
    '<feColorMatrix values="0 0 0 0 0.24  0 0 0 0 0.14  0 0 0 0 0.05  0 0 0 0.55 -0.1"/></filter>'
    '<rect width="100%" height="100%" filter="url(#g)"/></svg>\n'
)


def save_svgs() -> list[str]:
    (OUT / "paper-edge.svg").write_text(torn_edge(), encoding="utf-8", newline="\n")
    (OUT / "grain.svg").write_text(GRAIN_SVG, encoding="utf-8", newline="\n")
    return ["paper-edge.svg", "grain.svg"]


def preview(images: dict) -> None:
    from PIL import Image

    PREVIEW.mkdir(parents=True, exist_ok=True)
    for name, img in images.items():
        board = Image.new("RGB", img.size, (255, 0, 255))
        board.paste(img, mask=img.split()[3])
        board.save(PREVIEW / f"{name}.png")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--preview", action="store_true", help="also write the cut buttons on magenta")
    args = parser.parse_args()
    from PIL import Image

    OUT.mkdir(parents=True, exist_ok=True)
    sheet = Image.open(ART / "buttons-sheet.webp").convert("RGB")
    if sheet.size != SHEET_SIZE:
        raise SystemExit(f"buttons-sheet.webp is {sheet.size}, not {SHEET_SIZE}: measure the boxes again")
    key, key_mask, key_bbox = cut(sheet, KEY)
    lit = cut_lit(sheet, KEY.box, key_bbox, key_mask)
    common, _, _ = cut(sheet, COMMON)
    small, _, _ = cut(sheet, SMALL)
    written = save_button(key, KEY.name, KEY.height) + save_button(lit, "btn-key-lit", KEY.height)
    written += save_button(common, COMMON.name, COMMON.height) + save_button(small, SMALL.name, SMALL.height)
    written += save_backdrop(Image.open(ART / "tavern-tall.webp").convert("RGB"))
    written += save_svgs()
    if args.preview:
        preview({"btn-key": key, "btn-key-lit": lit, "btn-common": common, "btn-small": small})
    for name in written:
        print(f"wrote web/public/art/home/{name} ({(OUT / name).stat().st_size // 1024} KiB)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
