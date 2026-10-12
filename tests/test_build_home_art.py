"""Tests for tools/build_home_art.py: cutting a button out of a painted backdrop (T-530).

The tool needs Pillow and numpy, which are local tools only (not in requirements.txt): without
them these tests are skipped.
"""

import importlib.util
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "tools"))

HAVE_TOOLS = all(importlib.util.find_spec(m) for m in ("numpy", "PIL"))

if HAVE_TOOLS:
    import numpy as np

    import build_home_art as art  # noqa: E402

BACKDROP = (40, 30, 25)  # dark, dull brown, like the sheet's tavern wall
FRAME = (200, 160, 60)   # gold
RECESS = (35, 28, 22)    # the dark channel between frame and plate
PLATE = (150, 150, 150)  # grey stone


def button_sheet(h: int = 60, w: int = 120):
    """A gold frame with a dark recess and a stone plate, on the dark backdrop."""
    img = np.zeros((h, w, 3), dtype=np.uint8)
    img[:] = BACKDROP
    img[10:50, 10:110] = FRAME
    img[14:46, 14:106] = RECESS
    img[18:42, 18:102] = PLATE
    return img


@unittest.skipUnless(HAVE_TOOLS, "Pillow and numpy are local tools only")
class Masks(unittest.TestCase):
    def test_fill_holes_keeps_the_ring_and_what_it_encloses(self):
        ring = np.zeros((9, 9), dtype=bool)
        ring[2:7, 2:7] = True
        ring[3:6, 3:6] = False
        filled = art.fill_holes(ring)
        self.assertTrue(filled[4, 4])
        self.assertEqual(int(filled.sum()), 25)

    def test_an_open_ring_is_not_filled(self):
        ring = np.zeros((9, 9), dtype=bool)
        ring[2:7, 2:7] = True
        ring[3:6, 3:6] = False
        ring[4, 2] = False  # a gap to the outside
        self.assertFalse(art.fill_holes(ring)[4, 4])

    def test_centre_piece_drops_a_separate_speck(self):
        mask = np.zeros((9, 9), dtype=bool)
        mask[3:6, 3:6] = True
        mask[0, 0] = True
        piece = art.centre_piece(mask)
        self.assertFalse(piece[0, 0])
        self.assertEqual(int(piece.sum()), 9)


@unittest.skipUnless(HAVE_TOOLS, "Pillow and numpy are local tools only")
class Silhouette(unittest.TestCase):
    def test_the_dark_recess_inside_the_frame_belongs_to_the_button(self):
        mask = art.silhouette(button_sheet(), dark=60)
        self.assertTrue(mask[16, 60], "recess")
        self.assertTrue(mask[30, 60], "plate")
        self.assertTrue(mask[11, 60], "frame")

    def test_the_backdrop_around_it_does_not(self):
        mask = art.silhouette(button_sheet(), dark=60)
        self.assertFalse(mask[2, 2])
        self.assertFalse(mask[30, 4])
        self.assertFalse(mask[55, 60])

    def test_a_speck_of_glow_joined_by_a_thread_is_dropped(self):
        img = button_sheet()
        img[2:5, 58:62] = FRAME  # a speck above the frame
        img[5:10, 60] = FRAME    # joined to it by a one-pixel thread
        mask = art.silhouette(img, dark=60)
        self.assertFalse(mask[3, 60])
        self.assertTrue(mask[30, 60])


@unittest.skipUnless(HAVE_TOOLS, "Pillow and numpy are local tools only")
class LiningUp(unittest.TestCase):
    def test_best_offset_finds_the_same_button_moved(self):
        # Smooth shapes, like paint (the coarse pass steps 4 px at a time).
        yy, xx = np.mgrid[0:70, 0:120]
        area = (127 + 60 * np.sin(xx / 7.0) + 60 * np.cos(yy / 5.0)).astype(np.uint8)
        area = np.repeat(area[..., None], 3, axis=2)
        plain = area[17:57, 23:103].copy()  # the same pixels, 23 right and 17 down
        area[:17] = 30  # the area around it differs
        area[57:] = 30
        self.assertEqual(art.best_offset(plain, area, search=20), (23, 17))


if __name__ == "__main__":
    unittest.main()
