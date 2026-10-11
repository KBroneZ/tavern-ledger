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


if __name__ == "__main__":
    unittest.main()
