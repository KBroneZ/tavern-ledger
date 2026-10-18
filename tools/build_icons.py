"""Rebuilds the app icons, website icons and brand images from the hand-drawn logo SVGs (T-522, T-523, D-094).

Sources (docs/brand/logo/):
- app-tile-small.svg  16 to 32 px: drawn on the 16 px grid, so it is sharp at 16 and 32 px
                      (24 px falls between grid lines and is slightly soft, still clearer than the big tile)
- app-tile.svg        48 px and up
- mark.svg            the masthead mark
- tray-20.svg, tray-24.svg   the tray icon on the 20 px grid (T-523); 16 and 32 px come from app-tile-small.svg
- lockup.svg, mark-foam.svg  the social card and the installer images

Writes:
- crates/desktop/icons/icon.ico      16, 24, 32, 48, 64, 128 and 256 px (PNG entries)
- crates/desktop/icons/tray-NN.png   tray icon at 16, 20, 24 and 32 px (the app picks one for the screen's scaling)
- crates/desktop/icons/installer-header.bmp   150 x 57, NSIS installer header (T-105)
- crates/desktop/icons/installer-sidebar.bmp  164 x 314, NSIS welcome and finish pages (T-105)
- web/public/favicon.svg             copy of app-tile-small.svg
- web/public/favicon-32.png          32 px fallback for browsers without SVG icons
- web/public/apple-touch-icon.png    180 px, square (iOS rounds the corners itself)
- web/public/mark.svg                copy of mark.svg (the masthead)
- crates/desktop/ui/mark.svg         copy of mark.svg (the app's masthead, T-531)
- web/public/social-card.png         1200 x 630 link preview (og:image, twitter:image)
- web/public/email/logo.png          112 px, the auth emails' logo shown at 56 px (T-542, tools/build_auth_emails.py)
- docs/brand/logo/avatar-500.png     500 px GitHub avatar (uploaded by hand)
- docs/brand/logo/renders/tray-preview.png    the tray sizes on a light and a dark taskbar, for review

`--store` writes only the Microsoft Store package's logos, crates/desktop/store/Assets/ (T-379): the Store
logo (50 px), the 44 px and 150 px tiles, and the 44 px tile's target sizes for the taskbar and Start list
(plain and `_altform-unplated`, so Windows draws no backplate behind them).

Rendering uses a headless Chrome or Edge already on the PC (no new dependency):
set CHROME to its path if it is not in the usual place. Text other than the lockup's outlines is set
in the site's own fonts (web/public/fonts), embedded into the page so nothing is fetched. Python
standard library only.

Usage:
    python tools/build_icons.py            render and write every file
    python tools/build_icons.py --check    exit 1 if a copied SVG is out of date (no browser)
"""

from __future__ import annotations

import argparse
import base64
import os
import shutil
import struct
import subprocess
import sys
import tempfile
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LOGO = ROOT / "docs" / "brand" / "logo"
RENDERS = LOGO / "renders"
ICONS = ROOT / "crates" / "desktop" / "icons"
ICO = ICONS / "icon.ico"
PUBLIC = ROOT / "web" / "public"
FONTS = PUBLIC / "fonts"
ICO_SIZES = (16, 24, 32, 48, 64, 128, 256)
SMALL_MAX = 32
APPLE_SIZE = 180
AMBER = "#ffb24a"
INK = "#120d0b"
COPIES = {"app-tile-small.svg": "favicon.svg", "mark.svg": "mark.svg"}
APP_MARK = ROOT / "crates" / "desktop" / "ui" / "mark.svg"
# Windows draws the tray icon at 16 px at 100 % scaling, 20 at 125 %, 24 at 150 % and 32 at 200 %.
TRAY_SOURCES = {16: "app-tile-small.svg", 20: "tray-20.svg", 24: "tray-24.svg", 32: "app-tile-small.svg"}
TRAY_SIZES = tuple(TRAY_SOURCES)
STORE_ASSETS = ROOT / "crates" / "desktop" / "store" / "Assets"
STORE_TARGET_SIZES = (16, 24, 32, 48, 256)
# File name -> pixel size. The manifest names the three plain files; the target sizes are found by Windows.
STORE_LOGOS = {
    "StoreLogo.png": 50,
    "Square44x44Logo.png": 44,
    "Square150x150Logo.png": 150,
    **{f"Square44x44Logo.targetsize-{n}.png": n for n in STORE_TARGET_SIZES},
    **{f"Square44x44Logo.targetsize-{n}_altform-unplated.png": n for n in STORE_TARGET_SIZES},
}
SOCIAL_SIZE = (1200, 630)
PREVIEW_SIZE = (900, 380)
AVATAR_SIZE = 500
EMAIL_LOGO_SIZE = 112
INSTALLER = {"installer-header.bmp": (150, 57), "installer-sidebar.bmp": (164, 314)}
# Windows 11 taskbar grounds, light and dark theme.
TASKBARS = ("#f3f3f3", "#202020")
CHROME_PATHS = (
    r"C:\Program Files\Google\Chrome\Application\chrome.exe",
    r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
    r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
    "/usr/bin/google-chrome",
    "/usr/bin/chromium",
)
PNG_SIGNATURE = b"\x89PNG\r\n\x1a\n"
RENDER_TIMEOUT = 60
# The time Chrome is given to load the embedded fonts before the screenshot.
FONT_BUDGET_MS = 3000


def tile_for(size: int) -> str:
    """The source drawing for one icon size."""
    return "app-tile-small.svg" if size <= SMALL_MAX else "app-tile.svg"


def png_size(data: bytes) -> tuple[int, int]:
    """Width and height from a PNG's IHDR chunk."""
    if len(data) < 24 or not data.startswith(PNG_SIGNATURE) or data[12:16] != b"IHDR":
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


def _chunks(data: bytes) -> tuple[bytes, bytes]:
    """The IHDR body and the joined IDAT bodies of a PNG."""
    pos, header, idat = len(PNG_SIGNATURE), b"", []
    while pos + 8 <= len(data):
        length, kind = struct.unpack(">I4s", data[pos:pos + 8])
        body = data[pos + 8:pos + 8 + length]
        if kind == b"IHDR":
            header = body
        elif kind == b"IDAT":
            idat.append(body)
        elif kind == b"IEND":
            break
        pos += 12 + length
    if len(header) < 13 or not idat:
        raise ValueError("PNG has no image data")
    return header, b"".join(idat)


def _paeth(left: int, up: int, corner: int) -> int:
    p = left + up - corner
    pa, pb, pc = abs(p - left), abs(p - up), abs(p - corner)
    return left if pa <= pb and pa <= pc else up if pb <= pc else corner


def _unfilter(kind: int, line: bytes, previous: bytes, channels: int) -> bytes:
    """Undoes one row's PNG filter (types 0 to 4)."""
    if kind not in range(5):
        raise ValueError(f"unknown PNG filter {kind}")
    out = bytearray(line)
    for i in range(len(out)):
        left = out[i - channels] if i >= channels else 0
        up = previous[i]
        corner = previous[i - channels] if i >= channels else 0
        add = (0, left, up, (left + up) // 2, _paeth(left, up, corner))[kind]
        out[i] = (out[i] + add) & 0xFF
    return bytes(out)


def png_pixels(data: bytes) -> tuple[int, int, list[tuple[int, int, int, int]]]:
    """Decodes an 8-bit RGB or RGBA, non-interlaced PNG into (width, height, RGBA pixels row by row)."""
    width, height = png_size(data)
    header, idat = _chunks(data)
    depth, colour, _, _, interlace = header[8:13]
    if depth != 8 or colour not in (2, 6) or interlace != 0:
        raise ValueError(f"unsupported PNG (depth {depth}, colour type {colour}, interlace {interlace})")
    channels = 4 if colour == 6 else 3
    stride = width * channels
    raw = zlib.decompress(idat)
    if len(raw) != height * (stride + 1):
        raise ValueError("PNG image data has the wrong length")
    pixels, previous = [], bytes(stride)
    for y in range(height):
        start = y * (stride + 1)
        previous = _unfilter(raw[start], raw[start + 1:start + 1 + stride], previous, channels)
        for x in range(0, stride, channels):
            px = previous[x:x + channels]
            pixels.append((px[0], px[1], px[2], px[3] if channels == 4 else 255))
    return width, height, pixels


def bmp_bytes(width: int, height: int, pixels: list[tuple[int, int, int, int]]) -> bytes:
    """A 24-bit, bottom-up BMP (what the NSIS installer pages load) from opaque RGBA pixels."""
    if width < 1 or height < 1 or len(pixels) != width * height:
        raise ValueError("pixel count does not match the size")
    if any(p[3] != 255 for p in pixels):
        raise ValueError("installer images must be opaque")
    pad = b"\x00" * ((4 - (width * 3) % 4) % 4)
    rows = [b"".join(bytes((b, g, r)) for r, g, b, _ in pixels[y * width:(y + 1) * width]) + pad
            for y in range(height - 1, -1, -1)]
    image = b"".join(rows)
    header = struct.pack("<2sIHHI", b"BM", 54 + len(image), 0, 0, 54)
    info = struct.pack("<IiiHHIIiiII", 40, width, height, 1, 24, 0, len(image), 2835, 2835, 0, 0)
    return header + info + image


def find_chrome() -> str:
    for path in (os.environ.get("CHROME"), *CHROME_PATHS):
        if path and Path(path).is_file():
            return path
    raise SystemExit("No Chrome or Edge found: set CHROME to the browser's path.")


def render_page(chrome: str, body: str, width: int, height: int, css: str = "") -> bytes:
    """Renders an HTML body at width x height px with headless Chrome and returns the PNG bytes."""
    page = ("<!doctype html><html><head><meta charset='utf-8'><style>html,body{margin:0;overflow:hidden;"
            "width:%dpx;height:%dpx}%s</style></head><body>%s</body></html>" % (width, height, css, body))
    # Chrome's helper processes can hold the profile a moment after exit: do not fail on cleanup.
    with tempfile.TemporaryDirectory(ignore_cleanup_errors=True) as tmp:
        html = Path(tmp) / "page.html"
        out = Path(tmp) / "page.png"
        html.write_text(page, encoding="utf-8")
        try:
            subprocess.run(
                [chrome, "--headless=new", "--disable-gpu", "--hide-scrollbars", "--force-device-scale-factor=1",
                 f"--user-data-dir={Path(tmp) / 'profile'}", f"--window-size={width},{height}",
                 f"--virtual-time-budget={FONT_BUDGET_MS}", "--default-background-color=00000000",
                 f"--screenshot={out}", html.as_uri()],
                check=True, capture_output=True, timeout=RENDER_TIMEOUT)
        except (subprocess.CalledProcessError, subprocess.TimeoutExpired) as exc:
            detail = (getattr(exc, "stderr", None) or b"").decode("utf-8", errors="replace")[-500:]
            raise SystemExit(f"Chrome could not render the {width}x{height} image: {exc}\n{detail}") from exc
        data = out.read_bytes()
    if png_size(data) != (width, height):
        raise SystemExit(f"Chrome rendered {png_size(data)} instead of {width}x{height}.")
    return data


def render(chrome: str, svg: str, size: int, background: str = "transparent") -> bytes:
    """Renders an SVG at size x size px and returns the PNG bytes."""
    css = "html,body{background:%s}svg{display:block;width:%dpx;height:%dpx}" % (background, size, size)
    return render_page(chrome, svg, size, size, css)


def font_css() -> str:
    """The site's fonts as @font-face rules with the files embedded (headless Chrome blocks file:// fonts)."""
    faces = (("Bricolage Grotesque", "bricolage-grotesque-latin-wdth-normal.woff2", "200 800"),
             ("Figtree", "figtree-latin-wght-normal.woff2", "300 900"))
    return "".join(
        "@font-face{font-family:'%s';font-weight:%s;font-display:block;"
        "src:url(data:font/woff2;base64,%s) format('woff2')}"
        % (name, weights, base64.b64encode((FONTS / file).read_bytes()).decode("ascii"))
        for name, file, weights in faces)


def inline_svg(name: str) -> str:
    """A logo SVG without its comment, ready to drop into a page."""
    svg = (LOGO / name).read_text(encoding="utf-8")
    start, end = svg.find("<!--"), svg.find("-->")
    return svg[:start] + svg[end + 3:] if 0 <= start < end else svg


# The website's dark ground with its warm glow (web/src/styles/global.css).
GROUND = ("background:%s;background-image:radial-gradient(ellipse at 18%% 0%%,rgb(255 178 74/0.16),transparent 60%%);"
          "color:#f7efe6;font-family:'Figtree',sans-serif" % INK)
SOCIAL_CSS = (
    "body{%s;position:relative;box-sizing:border-box;padding:0 96px;display:flex;flex-direction:column;"
    "justify-content:center}"
    ".lockup svg{display:block;width:820px;height:auto}"
    ".line{margin:44px 0 0;font:600 54px/1.1 'Bricolage Grotesque',sans-serif;color:#ffd08a;letter-spacing:-0.01em}"
    ".notice{position:absolute;left:96px;bottom:44px;margin:0;font:400 22px/1.3 'Figtree',sans-serif;color:#cbbdaf}"
    % GROUND)
SIDEBAR_CSS = (
    "body{%s;display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center}"
    "svg{display:block;width:96px;height:auto}"
    ".name{margin:22px 0 0;font:700 26px/1.05 'Bricolage Grotesque',sans-serif;color:#f7efe6}" % GROUND)
HEADER_CSS = ("body{background:%s;display:flex;align-items:center;justify-content:center}"
              "svg{display:block;width:128px;height:auto}" % INK)
PREVIEW_CSS = (
    ".row{display:flex;gap:28px;align-items:flex-end;padding:20px 24px;height:190px;box-sizing:border-box}"
    "figure{margin:0;display:flex;gap:12px;align-items:flex-end}"
    "img{image-rendering:pixelated}figcaption{font:500 14px 'Figtree',sans-serif;align-self:flex-start;white-space:nowrap}")


def social_card(chrome: str) -> bytes:
    """1200 x 630 link preview: the lockup, the line, and the fan-project notice (rule 5)."""
    body = ("<div class='lockup'>%s</div><p class='line'>Hearthstone Battlegrounds tracker</p>"
            "<p class='notice'>Unofficial fan project. Not affiliated with or endorsed by Blizzard Entertainment.</p>"
            % inline_svg("lockup.svg"))
    return render_page(chrome, body, *SOCIAL_SIZE, font_css() + SOCIAL_CSS)


def installer_header(chrome: str) -> bytes:
    """150 x 57 NSIS header: the lockup on the dark ground."""
    return render_page(chrome, inline_svg("lockup.svg"), *INSTALLER["installer-header.bmp"], HEADER_CSS)


def installer_sidebar(chrome: str) -> bytes:
    """164 x 314 NSIS welcome and finish pages: the foam mark over the name, on the dark ground."""
    body = "%s<p class='name'>Tavern<br>Ledger</p>" % inline_svg("mark-foam.svg")
    return render_page(chrome, body, *INSTALLER["installer-sidebar.bmp"], font_css() + SIDEBAR_CSS)


def tray_preview(chrome: str) -> bytes:
    """The four tray images at their real size and at 4x, on a light and a dark taskbar."""
    images = {s: base64.b64encode((ICONS / f"tray-{s}.png").read_bytes()).decode("ascii") for s in TRAY_SIZES}
    rows = []
    for ground, text in zip(TASKBARS, reversed(TASKBARS)):
        cells = "".join(
            "<figure><img src='data:image/png;base64,%s' width='%d' height='%d'>"
            "<img src='data:image/png;base64,%s' width='%d' height='%d'><figcaption>%d px</figcaption></figure>"
            % (images[s], s, s, images[s], 4 * s, 4 * s, s) for s in TRAY_SIZES)
        rows.append("<div class='row' style='background:%s;color:%s'>%s</div>" % (ground, text, cells))
    return render_page(chrome, "".join(rows), *PREVIEW_SIZE, font_css() + PREVIEW_CSS)


def stale_copies() -> list[str]:
    return [dst for src, dst in COPIES.items()
            if not (PUBLIC / dst).is_file() or (PUBLIC / dst).read_bytes() != (LOGO / src).read_bytes()]


def write(path: Path, data: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(data)
    print(f"wrote {path.relative_to(ROOT).as_posix()}")


def build() -> None:
    chrome = find_chrome()
    svgs = {name: (LOGO / name).read_text(encoding="utf-8")
            for name in ("app-tile.svg", "app-tile-small.svg", "tray-20.svg", "tray-24.svg")}
    write(ICO, ico_bytes([render(chrome, svgs[tile_for(s)], s) for s in ICO_SIZES]))
    write(PUBLIC / "favicon-32.png", render(chrome, svgs[tile_for(32)], 32))
    # iOS masks the icon itself and fills transparent corners with black: draw it square.
    write(PUBLIC / "apple-touch-icon.png", render(chrome, svgs["app-tile.svg"], APPLE_SIZE, AMBER))
    for src, dst in COPIES.items():
        shutil.copyfile(LOGO / src, PUBLIC / dst)
    print("wrote web/public/favicon.svg and mark.svg")
    shutil.copyfile(LOGO / "mark.svg", APP_MARK)
    print(f"wrote {APP_MARK.relative_to(ROOT).as_posix()}")
    for size in TRAY_SIZES:
        write(ICONS / f"tray-{size}.png", render(chrome, svgs[TRAY_SOURCES[size]], size))
    write(PUBLIC / "social-card.png", social_card(chrome))
    # Emails cannot use SVG: a PNG at twice the size it is shown, transparent around the rounded tile.
    write(PUBLIC / "email" / "logo.png", render(chrome, svgs["app-tile.svg"], EMAIL_LOGO_SIZE))
    # GitHub crops avatars to a circle: a square amber ground keeps the corners clean.
    write(LOGO / "avatar-500.png", render(chrome, svgs["app-tile.svg"], AVATAR_SIZE, AMBER))
    write(ICONS / "installer-header.bmp", bmp_bytes(*png_pixels(installer_header(chrome))))
    write(ICONS / "installer-sidebar.bmp", bmp_bytes(*png_pixels(installer_sidebar(chrome))))
    write(RENDERS / "tray-preview.png", tray_preview(chrome))


def build_store() -> None:
    chrome = find_chrome()
    svgs = {name: (LOGO / name).read_text(encoding="utf-8") for name in ("app-tile.svg", "app-tile-small.svg")}
    for name, size in STORE_LOGOS.items():
        write(STORE_ASSETS / name, render(chrome, svgs[tile_for(size)], size))


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--check", action="store_true", help="only check that the copied SVGs are up to date")
    parser.add_argument("--store", action="store_true", help="only write the Microsoft Store package's logos")
    args = parser.parse_args(argv)
    if args.store and args.check:
        parser.error("--store and --check cannot be combined")
    if args.store:
        build_store()
        return 0
    if args.check:
        stale = stale_copies()
        for name in stale:
            print(f"out of date: web/public/{name}", file=sys.stderr)
        return 1 if stale else 0
    build()
    return 0


if __name__ == "__main__":
    sys.exit(main())
