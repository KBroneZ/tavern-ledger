"""Rebuilds the app and website icons from the hand-drawn logo SVGs (T-522, D-094).

Sources (docs/brand/logo/):
- app-tile-small.svg  16 to 32 px: drawn on the 16 px grid, so it stays sharp
- app-tile.svg        48 px and up
- mark.svg            the masthead mark

Writes:
- crates/desktop/icons/icon.ico      16, 24, 32, 48, 64, 128 and 256 px (PNG entries)
- web/public/favicon.svg             copy of app-tile-small.svg
- web/public/favicon-32.png          32 px fallback for browsers without SVG icons
- web/public/apple-touch-icon.png    180 px, square (iOS rounds the corners itself)
- web/public/mark.svg                copy of mark.svg (the masthead)

Rendering uses a headless Chrome or Edge already on the PC (no new dependency):
set CHROME to its path if it is not in the usual place. Python standard library only.

Usage:
    python tools/build_icons.py            render and write every file
    python tools/build_icons.py --check    exit 1 if a copied SVG is out of date (no browser)
"""

from __future__ import annotations

import argparse
import os
import shutil
import struct
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LOGO = ROOT / "docs" / "brand" / "logo"
ICO = ROOT / "crates" / "desktop" / "icons" / "icon.ico"
PUBLIC = ROOT / "web" / "public"
ICO_SIZES = (16, 24, 32, 48, 64, 128, 256)
SMALL_MAX = 32
APPLE_SIZE = 180
AMBER = "#ffb24a"
COPIES = {"app-tile-small.svg": "favicon.svg", "mark.svg": "mark.svg"}
CHROME_PATHS = (
    r"C:\Program Files\Google\Chrome\Application\chrome.exe",
    r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
    r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
    "/usr/bin/google-chrome",
    "/usr/bin/chromium",
)
PNG_SIGNATURE = b"\x89PNG\r\n\x1a\n"
RENDER_TIMEOUT = 60


def tile_for(size: int) -> str:
    """The source drawing for one icon size."""
    return "app-tile-small.svg" if size <= SMALL_MAX else "app-tile.svg"


def png_size(data: bytes) -> tuple[int, int]:
    """Width and height from a PNG's IHDR chunk."""
    if not data.startswith(PNG_SIGNATURE) or data[12:16] != b"IHDR":
        raise ValueError("not a PNG file")
    return struct.unpack(">II", data[16:24])


def ico_bytes(pngs: list[bytes]) -> bytes:
    """Packs square PNG images (16 to 256 px) into one .ico file, in the given order."""
    if not pngs:
        raise ValueError("no images")
    header = struct.pack("<HHH", 0, 1, len(pngs))
    offset = len(header) + 16 * len(pngs)
    entries, blobs = [], []
    for data in pngs:
        w, h = png_size(data)
        if w != h or not 1 <= w <= 256:
            raise ValueError(f"icon images must be square and at most 256 px, got {w}x{h}")
        entries.append(struct.pack("<BBBBHHII", w % 256, h % 256, 0, 0, 1, 32, len(data), offset))
        blobs.append(data)
        offset += len(data)
    return header + b"".join(entries) + b"".join(blobs)


def find_chrome() -> str:
    for path in (os.environ.get("CHROME"), *CHROME_PATHS):
        if path and Path(path).is_file():
            return path
    raise SystemExit("No Chrome or Edge found: set CHROME to the browser's path.")


def render(chrome: str, svg: str, size: int, background: str = "transparent") -> bytes:
    """Renders an SVG at size x size px with headless Chrome and returns the PNG bytes."""
    page = ("<!doctype html><html><head><style>html,body{margin:0;background:%s}"
            "svg{display:block;width:%dpx;height:%dpx}</style></head><body>%s</body></html>"
            % (background, size, size, svg))
    with tempfile.TemporaryDirectory() as tmp:
        html = Path(tmp) / "icon.html"
        out = Path(tmp) / "icon.png"
        html.write_text(page, encoding="utf-8")
        subprocess.run(
            [chrome, "--headless=new", "--disable-gpu", "--hide-scrollbars", "--force-device-scale-factor=1",
             f"--user-data-dir={Path(tmp) / 'profile'}", f"--window-size={size},{size}",
             "--default-background-color=00000000", f"--screenshot={out}", html.as_uri()],
            check=True, capture_output=True, timeout=RENDER_TIMEOUT)
        data = out.read_bytes()
    if png_size(data) != (size, size):
        raise SystemExit(f"Chrome rendered {png_size(data)} instead of {size}x{size}.")
    return data


def stale_copies() -> list[str]:
    return [dst for src, dst in COPIES.items()
            if not (PUBLIC / dst).is_file() or (PUBLIC / dst).read_bytes() != (LOGO / src).read_bytes()]


def build() -> None:
    chrome = find_chrome()
    svgs = {name: (LOGO / name).read_text(encoding="utf-8") for name in ("app-tile.svg", "app-tile-small.svg")}
    ICO.write_bytes(ico_bytes([render(chrome, svgs[tile_for(s)], s) for s in ICO_SIZES]))
    print(f"wrote {ICO.relative_to(ROOT)} ({', '.join(map(str, ICO_SIZES))} px)")
    (PUBLIC / "favicon-32.png").write_bytes(render(chrome, svgs[tile_for(32)], 32))
    # iOS masks the icon itself and fills transparent corners with black: draw it square.
    (PUBLIC / "apple-touch-icon.png").write_bytes(render(chrome, svgs["app-tile.svg"], APPLE_SIZE, AMBER))
    for src, dst in COPIES.items():
        shutil.copyfile(LOGO / src, PUBLIC / dst)
    print("wrote web/public/favicon.svg, favicon-32.png, apple-touch-icon.png and mark.svg")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--check", action="store_true", help="only check that the copied SVGs are up to date")
    args = parser.parse_args(argv)
    if args.check:
        stale = stale_copies()
        for name in stale:
            print(f"out of date: web/public/{name}", file=sys.stderr)
        return 1 if stale else 0
    build()
    return 0


if __name__ == "__main__":
    sys.exit(main())
