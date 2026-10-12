"""Tests for tools/build_page_art.py: every page's painted backdrop (T-546).

The tool needs Pillow (with AVIF), a local tool only (not in requirements.txt): without it these
tests are skipped.
"""

import importlib.util
import re
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))

HAVE_TOOLS = all(importlib.util.find_spec(m) for m in ("numpy", "PIL"))

if HAVE_TOOLS:
    from PIL import Image

    import build_page_art as art  # noqa: E402


@unittest.skipUnless(HAVE_TOOLS, "Pillow is a local tool only")
class PhoneCrop(unittest.TestCase):
    def test_the_crop_is_portrait_and_inside_the_painting_for_any_centre(self):
        for centre in (0, 300, 1376, 2500, art.SIZE[0]):
            left, top, right, bottom = art.phone_box(centre)
            self.assertTrue(0 <= left < right <= art.SIZE[0] and top == 0 and bottom == art.SIZE[1], centre)
            self.assertLess(right - left, bottom - top)

    def test_every_scene_has_its_source_and_a_centre_inside_the_painting(self):
        for scene, (source, centre) in art.SCENES.items():
            self.assertTrue((art.ART / source).is_file(), source)
            self.assertTrue(0 < centre < art.SIZE[0], scene)


@unittest.skipUnless(HAVE_TOOLS, "Pillow is a local tool only")
class Scene(unittest.TestCase):
    def test_every_size_is_written_in_both_formats_at_its_width(self):
        painting = Image.new("RGB", art.SIZE, (90, 60, 40))
        with tempfile.TemporaryDirectory() as tmp:
            old = art.OUT
            art.OUT = Path(tmp)
            try:
                written = art.save_scene("test", painting, 1376)
            finally:
                art.OUT = old
            names = {f"test-{w}.{ext}" for w in art.WIDTHS for ext in ("avif", "webp")}
            names |= {"test-phone.avif", "test-phone.webp"}
            self.assertEqual(set(written), names)
            for w in art.WIDTHS:
                with Image.open(Path(tmp) / f"test-{w}.webp") as img:
                    self.assertEqual(img.size, (w, round(art.SIZE[1] * w / art.SIZE[0])))
            with Image.open(Path(tmp) / "test-phone.webp") as img:
                self.assertEqual(img.width, art.PHONE_WIDTH)
                self.assertGreater(img.height, img.width)


class Pages(unittest.TestCase):
    """The scenes the site names are the ones the tool writes (no Pillow needed)."""

    def test_the_component_names_the_tool_s_scenes_and_widths(self):
        component = (ROOT / "web" / "src" / "components" / "PageBackdrop.astro").read_text(encoding="utf-8")
        scenes = set(re.search(r'type Scene = ([^;]+);', component).group(1).replace('"', "").replace(" ", "").split("|"))
        tool = (ROOT / "tools" / "build_page_art.py").read_text(encoding="utf-8")
        self.assertEqual(scenes, set(re.findall(r'^    "([a-z]+)": \(', tool, re.M)))
        widths = set(re.findall(r"-(\d+)\.\$\{ext\}", component))
        self.assertEqual(widths, set(re.search(r"WIDTHS = \(([^)]+)\)", tool).group(1).replace(" ", "").split(",")))
        for scene in scenes:
            for name in [f"{scene}-{w}" for w in widths] + [f"{scene}-phone"]:
                for ext in ("avif", "webp"):
                    self.assertTrue((ROOT / "web" / "public" / "art" / "pages" / f"{name}.{ext}").is_file(), f"{name}.{ext}")


if __name__ == "__main__":
    unittest.main()
