"""The installer's license page: the fan notice on top, then LICENSE unchanged."""

import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
INSTALLER_LICENSE = ROOT / "crates" / "desktop" / "installer" / "license.txt"
FAN_NOTICE = (
    "Unofficial fan project. Not affiliated with or endorsed by Blizzard Entertainment."
)
SEPARATOR = "-" * 70


class InstallerLicenseTest(unittest.TestCase):
    def setUp(self):
        self.text = INSTALLER_LICENSE.read_text(encoding="utf-8")

    def test_starts_with_the_fan_notice(self):
        self.assertEqual(self.text.splitlines()[0], FAN_NOTICE)

    def test_links_the_privacy_policy(self):
        head = self.text.split(SEPARATOR, 1)[0]
        self.assertIn("https://tavernledger.net/privacy/", head)

    def test_ends_with_the_license_unchanged(self):
        # After a LICENSE change, copy it again below the separator line.
        license_text = (ROOT / "LICENSE").read_text(encoding="utf-8")
        self.assertIn(SEPARATOR, self.text)
        self.assertEqual(self.text.split(SEPARATOR + "\n\n", 1)[1], license_text)


if __name__ == "__main__":
    unittest.main()
