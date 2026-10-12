"""Tests for tools/build_auth_emails.py and the templates it writes (T-542, D-114)."""

import re
import struct
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "tools"))

import build_auth_emails as ae  # noqa: E402

ROOT = Path(__file__).resolve().parent.parent
# The only Go template actions a template may use: the link, the addresses of an email change, and
# the language switch. Supabase's {{ .Token }}, {{ .TokenHash }} or {{ .SiteURL }} would change the link.
ALLOWED_ACTIONS = {
    "{{ .ConfirmationURL }}",
    "{{ .Email }}",
    "{{ .NewEmail }}",
    ae.IS_SPANISH,
    "{{ else }}",
    "{{ end }}",
}


def committed(name: str) -> str:
    return (ae.TEMPLATES / f"{name}.html").read_text(encoding="utf-8")


def branches(name: str) -> tuple[str, str]:
    """The Spanish and the English email of a committed template."""
    body = committed(name)
    if not body.startswith(ae.IS_SPANISH + "\n"):
        raise AssertionError("the template does not start with the language switch")
    spanish, _, rest = body[len(ae.IS_SPANISH):].partition("{{ else }}")
    english, _, tail = rest.partition("{{ end }}")
    if tail.strip():
        raise AssertionError("text after the language switch")
    return spanish, english


class UpToDate(unittest.TestCase):
    def test_templates_and_config_match_the_generator(self):
        self.assertEqual(ae.stale(), [])

    def test_every_template_is_utf8(self):
        for name in ae.COPY:
            (ae.TEMPLATES / f"{name}.html").read_bytes().decode("utf-8")


class Links(unittest.TestCase):
    def test_each_language_has_the_button_and_the_plain_link(self):
        for name in ae.COPY:
            for lang, email in zip(("es", "en"), branches(name)):
                hrefs = re.findall(r'href="([^"]*)"', email)
                self.assertEqual(hrefs.count(ae.LINK), 2, f"{name} {lang}: button and plain link")
                self.assertIn(f">{ae.LINK}</a>", email, f"{name} {lang}: the link is shown as text")

    def test_the_link_is_never_rewritten(self):
        for name in ae.COPY:
            body = committed(name)
            # Nothing glued to the variable: no extra query, fragment or tracking around it.
            for m in re.finditer(re.escape(ae.LINK), body):
                after = body[m.end():m.end() + 1]
                self.assertIn(after, ('"', "<"), f"{name}: {after!r} after the link")
            self.assertNotIn("utm_", body)

    def test_only_known_template_actions(self):
        for name in ae.COPY:
            actions = set(re.findall(r"\{\{.*?\}\}", committed(name)))
            self.assertLessEqual(actions, ALLOWED_ACTIONS, name)

    def test_email_change_names_both_addresses(self):
        for email in branches("email_change"):
            self.assertIn("{{ .Email }}", email)
            self.assertIn("{{ .NewEmail }}", email)


class Safety(unittest.TestCase):
    def test_no_plain_http_scripts_or_outside_resources(self):
        for name in ae.COPY:
            body = committed(name).lower()
            self.assertNotIn("http://", body, name)
            for banned in ("<script", "<link", "@import", "url(", "data:", "<iframe", "<form", "javascript:"):
                self.assertNotIn(banned, body, f"{name}: {banned}")

    def test_every_address_is_the_site(self):
        for name in ae.COPY:
            for url in re.findall(r'(?:src|href)="(https://[^"]*)"', committed(name)):
                self.assertTrue(url.startswith("https://tavernledger.net/"), f"{name}: {url}")

    def test_one_image_the_logo_with_size_and_alt(self):
        for name in ae.COPY:
            for email in branches(name):
                images = re.findall(r"<img [^>]*>", email)
                self.assertEqual(len(images), 1, name)
                img = images[0]
                self.assertIn(f'src="{ae.LOGO_URL}"', img)
                self.assertIn(f'width="{ae.LOGO_SIZE}"', img)
                self.assertIn(f'height="{ae.LOGO_SIZE}"', img)
                self.assertIn('alt="Tavern Ledger"', img)

    def test_the_logo_file_is_on_the_site_at_twice_its_size(self):
        path = ROOT / "web" / "public" / ae.LOGO_URL.removeprefix("https://tavernledger.net/")
        data = path.read_bytes()
        self.assertEqual(data[:8], b"\x89PNG\r\n\x1a\n")
        self.assertEqual(struct.unpack(">II", data[16:24]), (2 * ae.LOGO_SIZE, 2 * ae.LOGO_SIZE))


class Facts(unittest.TestCase):
    def test_the_expiry_in_the_copy_is_the_configured_one(self):
        config = ae.CONFIG.read_text(encoding="utf-8")
        expiry = int(re.search(r"^otp_expiry = (\d+)$", config, re.M).group(1))
        self.assertEqual(expiry, 3600, "the emails say 1 hour; change their copy with this value")
        for name in ("magic_link", "recovery"):
            self.assertIn("expires in 1 hour", " ".join(ae.COPY[name]["en"]["body"]), name)
            self.assertIn("caduca en 1 hora", " ".join(ae.COPY[name]["es"]["body"]), name)

    def test_no_html_comments(self):
        # Supabase's renderer (Go html/template) drops them, so nothing may rely on one.
        for name in ae.COPY:
            self.assertNotIn("<!--", committed(name), name)


class Languages(unittest.TestCase):
    def test_spanish_first_english_otherwise(self):
        for name in ae.COPY:
            spanish, english = branches(name)
            self.assertIn('<html lang="es">', spanish)
            self.assertIn(str(ae.COPY[name]["es"]["button"]), spanish)
            self.assertIn('<html lang="en">', english)
            self.assertIn(str(ae.COPY[name]["en"]["button"]), english)

    def test_the_fan_notice_is_in_both(self):
        for name in ae.COPY:
            for email in branches(name):
                self.assertIn(ae.NOTICE_EN, email, name)

    def test_both_say_what_to_do_if_you_did_not_ask(self):
        for name in ae.COPY:
            for lang, email in zip(("es", "en"), branches(name)):
                self.assertIn(ae.text(str(ae.COPY[name][lang]["ignore"])), email, f"{name} {lang}")

    def test_subjects_switch_language_the_same_way(self):
        for name in ae.COPY:
            subject = ae.subject(name)
            self.assertTrue(subject.startswith(ae.IS_SPANISH), name)
            self.assertIn(str(ae.COPY[name]["es"]["subject"]), subject)
            self.assertIn(str(ae.COPY[name]["en"]["subject"]), subject)

    def test_copy_has_the_same_keys_in_both_languages(self):
        for name, langs in ae.COPY.items():
            self.assertEqual(set(langs), {"en", "es"})
            self.assertEqual(set(langs["en"]), set(langs["es"]), name)
            self.assertEqual(len(langs["en"]["body"]), len(langs["es"]["body"]), name)


class Escaping(unittest.TestCase):
    def test_text_escapes_html_and_keeps_template_actions(self):
        self.assertEqual(ae.text("a <b> & {{ .Email }}"), "a &lt;b&gt; &amp; {{ .Email }}")

    def test_toml_string_escapes_quotes(self):
        self.assertEqual(ae.toml_string('say "es"'), '"say \\"es\\""')


if __name__ == "__main__":
    unittest.main()
