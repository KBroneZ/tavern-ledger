"""Tests for tools/build_home_concepts.py and the home page concepts in docs/brand/home/ (T-525)."""

import html
import re
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "tools"))

import build_home_concepts as bhc  # noqa: E402

HOME = bhc.HOME
CONCEPTS = sorted(p.name for p in HOME.glob("*.html") if bhc.CONCEPT_NAME.match(p.name))
NAMES_TS = bhc.ROOT / "web" / "src" / "lib" / "heroes" / "names.data.ts"


class Build(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.out = bhc.build()
        cls.frames = [html.unescape(m) for m in re.findall(r'<iframe srcdoc="([^"]*)"', cls.out)]

    def test_is_page_content_without_document_tags(self):
        self.assertNotIn("<!doctype", self.out.lower()[:200])
        self.assertIsNone(re.search(r"<html|<head>|<body>", self.out))
        self.assertIn("<title>Tavern Home Concepts</title>", self.out)

    def test_every_frame_is_inlined(self):
        self.assertEqual(len(self.frames), 2 * len(CONCEPTS))
        self.assertNotIn("<iframe src=", self.out)
        self.assertNotIn('href="shared/', self.out)
        self.assertNotIn('src="shared/', self.out)

    def test_frames_draw_silhouettes_and_carry_the_shared_code(self):
        for frame in self.frames:
            self.assertIn('<html lang="es" data-portraits="silhouette"', frame)
            self.assertIn("the shared pieces", frame)  # tavern.css header
            self.assertIn("function silhouette(id)", frame)  # tavern.js
            self.assertNotIn("shared/tavern", frame)

    def test_artifact_note_shown_and_local_links_dropped(self):
        self.assertIn("data-artifact-only>", self.out)
        self.assertNotIn("data-artifact-only hidden", self.out)
        self.assertNotIn("data-local-only", self.out)

    def test_each_concept_has_a_desktop_and_a_phone_frame(self):
        for name in CONCEPTS:
            title = re.search(r"<title>(.*?)</title>", bhc.read(HOME / name)).group(1)
            with self.subTest(name):
                self.assertEqual(sum(f"<title>{title}</title>" in frame for frame in self.frames), 2)

    def test_check_refuses_a_frame_left_with_src(self):
        with self.assertRaises(ValueError):
            bhc.check('<p data-artifact-only></p><iframe loading="lazy" src="a.html"></iframe>')
        with self.assertRaises(ValueError):
            bhc.check('<p data-artifact-only hidden></p>')

    def test_rejects_a_name_outside_the_concepts(self):
        with self.assertRaises(ValueError):
            bhc.concept_for_artifact("../index.html", "", "")
        with self.assertRaises(ValueError):
            bhc.concept_for_artifact("index.html", "", "")

    def test_inline_needs_the_shared_stylesheet(self):
        with self.assertRaises(ValueError):
            bhc.inline_shared("<head></head>", "", "")


class Concepts(unittest.TestCase):
    def test_five_or_more_concepts(self):
        self.assertGreaterEqual(len(CONCEPTS), 5)

    def test_no_pictures_committed(self):
        pictures = [p for p in HOME.rglob("*") if p.suffix.lower() in {".png", ".jpg", ".jpeg", ".webp", ".gif", ".avif"}]
        self.assertEqual(pictures, [])

    def test_each_concept_keeps_the_notice_and_both_languages(self):
        for name in CONCEPTS:
            page = bhc.read(HOME / name)
            with self.subTest(name):
                self.assertIn("Unofficial fan project. Not affiliated with or endorsed by Blizzard Entertainment.", page)
                self.assertIn("Proyecto no oficial de fans.", page)
                self.assertEqual(page.count("<h1"), 1)
                self.assertIn('data-l="es"', page)
                self.assertIn('data-l="en"', page)
                self.assertIn('<meta name="viewport"', page)
                # Portraits come only from tavern.js at view time: no picture address in the page.
                self.assertIsNone(re.search(r'<img[^>]*\ssrc=', page))
                self.assertNotIn("art.hearthstonejson.com", page)

    def test_hero_ids_are_known_and_named_as_on_the_site(self):
        rows = re.findall(r'"([A-Za-z0-9_]+)": \["([^"]*)", "([^"]*)"\]', bhc.read(NAMES_TS))
        site = {hero: (en, es) for hero, en, es in rows}
        js = bhc.read(HOME / "shared" / "tavern.js")
        ours = {m[0]: (m[1], m[2]) for m in re.findall(r'(\w+): \["([^"]*)", "([^"]*)"\]', js)}
        self.assertTrue(ours)
        for hero, names in ours.items():
            with self.subTest(hero):
                self.assertEqual(site.get(hero), names)
        for name in CONCEPTS:
            used = set(re.findall(r'data-hero(?:-name)?="([^"]+)"', bhc.read(HOME / name)))
            with self.subTest(name):
                self.assertLessEqual(used, set(ours))


if __name__ == "__main__":
    unittest.main()
