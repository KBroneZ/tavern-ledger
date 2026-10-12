"""Tests for tools/build_sim_art.py: the simulator page's tall panel (T-526).

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
    import build_sim_art as art  # noqa: E402


@unittest.skipUnless(HAVE_TOOLS, "Pillow and numpy are local tools only")
class Boxes(unittest.TestCase):
    def test_the_tall_box_lies_inside_the_sheet(self):
        left, top, right, bottom = art.TALL_BOX
        self.assertTrue(0 <= left < right <= art.SHEET_SIZE[0] and 0 <= top < bottom <= art.SHEET_SIZE[1])


if __name__ == "__main__":
    unittest.main()
