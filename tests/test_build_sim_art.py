"""Tests for tools/build_sim_art.py: the simulator page's backdrop sizes and its tall panel (T-526).

The tool needs Pillow and numpy, which are local tools only (not in requirements.txt): without
them these tests are skipped.
"""

import importlib.util
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "tools"))

HAVE_TOOLS = all(importlib.util.find_spec(m) for m in ("numpy", "PIL"))

if HAVE_TOOLS:
    from PIL import Image

    import build_sim_art as art  # noqa: E402


@unittest.skipUnless(HAVE_TOOLS, "Pillow and numpy are local tools only")
class Boxes(unittest.TestCase):
    def test_the_boxes_lie_inside_their_sources(self):
        for box, size in ((art.PHONE_BOX, (2752, 1536)), (art.TALL_BOX, art.SHEET_SIZE)):
            left, top, right, bottom = box
            self.assertTrue(0 <= left < right <= size[0] and 0 <= top < bottom <= size[1], box)

    def test_the_phone_crop_is_portrait(self):
        left, top, right, bottom = art.PHONE_BOX
        self.assertLess(right - left, bottom - top)


@unittest.skipUnless(HAVE_TOOLS, "Pillow and numpy are local tools only")
class Arena(unittest.TestCase):
    def test_every_size_is_written_in_both_formats_at_its_width(self):
        painting = Image.new("RGB", (2752, 1536), (90, 60, 40))
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)
            old = art.OUT
            art.OUT = out
            try:
                written = art.save_arena(painting)
            finally:
                art.OUT = old
            names = {f"arena-{w}.{ext}" for w in art.ARENA_WIDTHS for ext in ("avif", "webp")}
            names |= {"arena-phone.avif", "arena-phone.webp"}
            self.assertEqual(set(written), names)
            for w in art.ARENA_WIDTHS:
                with Image.open(out / f"arena-{w}.webp") as img:
                    self.assertEqual(img.size, (w, round(1536 * w / 2752)))
            with Image.open(out / "arena-phone.webp") as img:
                self.assertEqual(img.width, art.PHONE_WIDTH)
                self.assertGreater(img.height, img.width)


if __name__ == "__main__":
    unittest.main()
