"""Tests for tools/build_icons.py: the ICO packing and the PNG checks (no browser needed)."""

import struct
import sys
import unittest
import zlib
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "tools"))

import build_icons as bi  # noqa: E402


def png(w: int, h: int) -> bytes:
    """A real, tiny RGBA PNG of the given size (transparent)."""
    def chunk(kind: bytes, data: bytes) -> bytes:
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))
    raw = b"".join(b"\x00" + b"\x00" * (4 * w) for _ in range(h))
    return (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 6, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(raw)) + chunk(b"IEND", b""))


class PngSize(unittest.TestCase):
    def test_reads_width_and_height(self):
        self.assertEqual(bi.png_size(png(16, 24)), (16, 24))

    def test_rejects_data_that_is_not_png(self):
        with self.assertRaises(ValueError):
            bi.png_size(b"GIF89a" + b"\x00" * 30)

    def test_rejects_a_truncated_png(self):
        with self.assertRaises(ValueError):
            bi.png_size(png(16, 16)[:20])


class Ico(unittest.TestCase):
    def test_packs_every_size_with_a_valid_directory(self):
        images = [png(s, s) for s in (16, 32, 256)]
        ico = bi.ico_bytes(images)
        reserved, kind, count = struct.unpack_from("<HHH", ico, 0)
        self.assertEqual((reserved, kind, count), (0, 1, 3))
        offset = 6 + 16 * 3
        for i, (data, size) in enumerate(zip(images, (16, 32, 256))):
            w, h, colours, res, planes, bits, length, start = struct.unpack_from("<BBBBHHII", ico, 6 + 16 * i)
            self.assertEqual((w, h), (size % 256, size % 256))  # 256 is written as 0
            self.assertEqual((colours, res, planes, bits), (0, 0, 1, 32))
            self.assertEqual((length, start), (len(data), offset))
            self.assertEqual(ico[start:start + length], data)
            offset += length
        self.assertEqual(len(ico), offset)

    def test_rejects_non_square_or_too_big_images(self):
        with self.assertRaises(ValueError):
            bi.ico_bytes([png(16, 20)])
        with self.assertRaises(ValueError):
            bi.ico_bytes([png(512, 512)])

    def test_rejects_an_empty_list(self):
        with self.assertRaises(ValueError):
            bi.ico_bytes([])


class Plan(unittest.TestCase):
    def test_small_sizes_use_the_pixel_grid_tile(self):
        for size in bi.ICO_SIZES:
            source = bi.tile_for(size)
            self.assertEqual(source, "app-tile-small.svg" if size <= 32 else "app-tile.svg", size)

    def test_ico_covers_16_to_256(self):
        self.assertEqual(min(bi.ICO_SIZES), 16)
        self.assertEqual(max(bi.ICO_SIZES), 256)
        self.assertEqual(list(bi.ICO_SIZES), sorted(set(bi.ICO_SIZES)))


class Copies(unittest.TestCase):
    def test_website_copies_match_the_logo_files(self):
        self.assertEqual(bi.stale_copies(), [])

    def test_check_mode_needs_no_browser(self):
        self.assertEqual(bi.main(["--check"]), 0)


def chunk(kind: bytes, data: bytes) -> bytes:
    return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))


def paeth(a: int, b: int, c: int) -> int:
    p = a + b - c
    pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
    return a if pa <= pb and pa <= pc else b if pb <= pc else c


def filtered_png(rows: list[list[tuple[int, ...]]], colour: int, filters: list[int]) -> bytes:
    """A PNG of the given pixels, each row stored with the given filter type (0 to 4)."""
    w, h, n = len(rows[0]), len(rows), len(rows[0][0])
    raw, prev = b"", bytes(w * n)
    for row, kind in zip(rows, filters):
        line = bytes(v for px in row for v in px)
        out = bytearray()
        for i, v in enumerate(line):
            a = line[i - n] if i >= n else 0
            b = prev[i]
            c = prev[i - n] if i >= n else 0
            out.append((v - (0, a, b, (a + b) // 2, paeth(a, b, c))[kind]) & 0xFF)
        raw += bytes([kind]) + bytes(out)
        prev = line
    return (bi.PNG_SIGNATURE + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, colour, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(raw)) + chunk(b"IEND", b""))


ROWS = [[(10 * x + y, 200 - x, 7 * y, 255) for x in range(5)] for y in range(5)]


class Pixels(unittest.TestCase):
    def test_decodes_every_filter_type(self):
        w, h, pixels = bi.png_pixels(filtered_png(ROWS, 6, [0, 1, 2, 3, 4]))
        self.assertEqual((w, h), (5, 5))
        self.assertEqual(pixels, [px for row in ROWS for px in row])

    def test_decodes_rgb_as_opaque(self):
        rgb = [[px[:3] for px in row] for row in ROWS]
        _, _, pixels = bi.png_pixels(filtered_png(rgb, 2, [4, 3, 2, 1, 0]))
        self.assertEqual(pixels, [px for row in ROWS for px in row])

    def test_rejects_other_colour_types(self):
        grey = [[(v[0],) for v in row] for row in ROWS]
        with self.assertRaises(ValueError):
            bi.png_pixels(filtered_png(grey, 0, [0] * 5))

    def test_rejects_an_unknown_filter(self):
        data = bytearray(filtered_png(ROWS, 6, [0] * 5))
        start = data.index(b"IDAT") + 4
        length = struct.unpack(">I", data[start - 8:start - 4])[0]
        raw = bytearray(zlib.decompress(bytes(data[start:start + length])))
        raw[0] = 9
        body = zlib.compress(bytes(raw))
        broken = bytes(data[:start - 8]) + chunk(b"IDAT", body) + chunk(b"IEND", b"")
        with self.assertRaises(ValueError):
            bi.png_pixels(broken)


class Bmp(unittest.TestCase):
    def test_writes_a_bottom_up_24_bit_bitmap_with_padded_rows(self):
        pixels = [(1, 2, 3, 255), (4, 5, 6, 255), (7, 8, 9, 255), (10, 11, 12, 255)]
        bmp = bi.bmp_bytes(1, 4, pixels)
        magic, size, _, _, offset = struct.unpack_from("<2sIHHI", bmp, 0)
        _, w, h, planes, bits, compression = struct.unpack_from("<IiiHHI", bmp, 14)
        self.assertEqual((magic, size, offset), (b"BM", len(bmp), 54))
        self.assertEqual((w, h, planes, bits, compression), (1, 4, 1, 24, 0))
        # Each row is 3 bytes padded to 4, stored bottom row first, in BGR order.
        self.assertEqual(bmp[54:], b"\x0c\x0b\x0a\x00\x09\x08\x07\x00\x06\x05\x04\x00\x03\x02\x01\x00")

    def test_rejects_transparent_pixels_or_a_wrong_count(self):
        with self.assertRaises(ValueError):
            bi.bmp_bytes(1, 1, [(0, 0, 0, 128)])
        with self.assertRaises(ValueError):
            bi.bmp_bytes(2, 2, [(0, 0, 0, 255)])


class Tray(unittest.TestCase):
    def test_every_tray_size_has_a_pixel_grid_drawing(self):
        self.assertEqual(bi.TRAY_SIZES, (16, 20, 24, 32))
        for size, name in bi.TRAY_SOURCES.items():
            svg = (bi.LOGO / name).read_text(encoding="utf-8")
            self.assertIn('viewBox="0 0 ', svg, name)
            grid = int(svg.split('viewBox="0 0 ')[1].split()[0])
            # Drawn at that size, or at twice it in half-pixel units (app-tile-small.svg): whole pixels.
            self.assertIn(grid, (size, 2 * size), name)

    def test_inline_svg_drops_the_comment(self):
        self.assertNotIn("<!--", bi.inline_svg("tray-20.svg"))
        self.assertIn("<path", bi.inline_svg("tray-20.svg"))


class Committed(unittest.TestCase):
    """The committed binaries are whole and have the planned sizes (no browser needed)."""

    def test_app_icon_has_every_size(self):
        ico = bi.ICO.read_bytes()
        reserved, kind, count = struct.unpack_from("<HHH", ico, 0)
        self.assertEqual((reserved, kind, count), (0, 1, len(bi.ICO_SIZES)))
        for i, size in enumerate(bi.ICO_SIZES):
            w, h, _, _, _, _, length, start = struct.unpack_from("<BBBBHHII", ico, 6 + 16 * i)
            self.assertEqual((w, h), (size % 256, size % 256))
            self.assertEqual(bi.png_size(ico[start:start + length]), (size, size))

    def test_website_pngs_have_their_sizes(self):
        self.assertEqual(bi.png_size((bi.PUBLIC / "favicon-32.png").read_bytes()), (32, 32))
        size = bi.APPLE_SIZE
        self.assertEqual(bi.png_size((bi.PUBLIC / "apple-touch-icon.png").read_bytes()), (size, size))
        self.assertEqual(bi.png_size((bi.PUBLIC / "social-card.png").read_bytes()), bi.SOCIAL_SIZE)

    def test_tray_images_and_avatar_have_their_sizes(self):
        for size in bi.TRAY_SIZES:
            self.assertEqual(bi.png_size((bi.ICONS / f"tray-{size}.png").read_bytes()), (size, size))
        avatar = bi.png_size((bi.LOGO / "avatar-500.png").read_bytes())
        self.assertEqual(avatar, (bi.AVATAR_SIZE, bi.AVATAR_SIZE))

    def test_installer_bitmaps_are_24_bit_at_their_sizes(self):
        for name, (width, height) in bi.INSTALLER.items():
            bmp = (bi.ICONS / name).read_bytes()
            self.assertEqual(bmp[:2], b"BM", name)
            _, w, h, _, bits = struct.unpack_from("<IiiHH", bmp, 14)
            self.assertEqual((w, h, bits), (width, height, 24), name)
            row = (width * 3 + 3) // 4 * 4
            self.assertEqual(len(bmp), 54 + row * height, name)


if __name__ == "__main__":
    unittest.main()
