"""Tests for tools/winget_manifest.py with synthetic release records (no network)."""

import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "tools"))

import winget_manifest as wm  # noqa: E402
from winget_manifest import Refused  # noqa: E402

SHA = "104b2d4abf6e164d1509bf6cb8acb8a48119453e512a8c319491fceb47b771ce"


def record(version: str = "0.2.7", **changes) -> dict:
    r = {
        "version": version,
        "tag": f"v{version}",
        "file": f"TavernLedger_{version}_x64-setup.exe",
        "bytes": 8_373_699,
        "sha256": SHA,
        "date": "2026-10-24",
        "sums": True,
    }
    r.update(changes)
    return r


class NewestTest(unittest.TestCase):
    def test_takes_the_first_record(self):
        self.assertEqual(wm.newest([record("0.2.8"), record("0.2.7")])["version"], "0.2.8")

    def test_refuses_records_it_cannot_trust(self):
        for bad in (
            [],
            {},
            [record(version="0.2")],
            [record(tag="v0.2.6")],
            [record(file="setup.exe")],
            [record(sha256="ABC")],
            [record(sha256=SHA.upper())],
            [record(date="24 Oct 2026")],
        ):
            with self.subTest(bad=bad), self.assertRaises(Refused):
                wm.newest(bad)


class ManifestsTest(unittest.TestCase):
    def setUp(self):
        self.files = wm.manifests(record())

    def test_four_files_named_as_winget_pkgs_expects(self):
        self.assertEqual(
            sorted(self.files),
            [
                "KBroneZ.TavernLedger.installer.yaml",
                "KBroneZ.TavernLedger.locale.en-US.yaml",
                "KBroneZ.TavernLedger.locale.es-ES.yaml",
                "KBroneZ.TavernLedger.yaml",
            ],
        )
        for name, text in self.files.items():
            self.assertTrue(text.startswith("# yaml-language-server: $schema=https://aka.ms/winget-manifest."), name)
            self.assertIn('ManifestVersion: "1.12.0"', text, name)
            self.assertIn('PackageVersion: "0.2.7"', text, name)

    def test_installer_points_at_the_versioned_release_file_with_its_hash(self):
        text = self.files["KBroneZ.TavernLedger.installer.yaml"]
        url = "https://github.com/KBroneZ/tavern-ledger/releases/download/v0.2.7/TavernLedger_0.2.7_x64-setup.exe"
        self.assertIn(f'InstallerUrl: "{url}"', text)
        self.assertIn(f'InstallerSha256: "{SHA.upper()}"', text)
        for line in ('InstallerType: "nullsoft"', 'Scope: "user"', "RequireExplicitUpgrade: true", 'UpgradeBehavior: "install"'):
            self.assertIn(line, text)
        self.assertIn('  DisplayVersion: "0.2.7"', text)

    def test_both_locales_carry_the_fixed_fan_notice_and_the_memory_read(self):
        en = self.files["KBroneZ.TavernLedger.locale.en-US.yaml"]
        es = self.files["KBroneZ.TavernLedger.locale.es-ES.yaml"]
        self.assertIn("Unofficial fan project. Not affiliated with or endorsed by Blizzard Entertainment.", en)
        self.assertIn("Proyecto no oficial de fans. Sin afiliación con Blizzard Entertainment ni respaldo de esta.", es)
        self.assertIn("the account at risk is yours", en)
        self.assertIn("la cuenta en riesgo es la tuya", es)
        self.assertIn('ManifestType: "defaultLocale"', en)
        self.assertIn('ManifestType: "locale"', es)
        self.assertIn('LicenseUrl: "https://github.com/KBroneZ/tavern-ledger/blob/v0.2.7/LICENSE"', en)
        # Spanish prose uses the game's own terms (docs/brand/voice.md).
        description = json.loads(next(line for line in es.splitlines() if line.startswith("Description: "))[len("Description: ") :])
        self.assertNotIn("Battlegrounds", description)
        self.assertNotIn(" log ", description)

    def test_every_scalar_is_a_quoted_string_yaml_cannot_misread(self):
        for name, text in self.files.items():
            for line in text.splitlines():
                if not line or line.startswith("#") or line.endswith(":"):
                    continue
                item = line[2:] if line.startswith("- ") else line
                value = item.split(": ", 1)[1] if ": " in item else item
                if value in ("true", "false"):
                    continue
                self.assertTrue(value.startswith('"') and value.endswith('"'), f"{name}: {line}")
                json.loads(value)

    def test_short_descriptions_fit_winget_limits(self):
        for locale, text in wm.LOCALES.items():
            self.assertLessEqual(len(text["ShortDescription"]), 256, locale)
            self.assertLessEqual(len(text["Description"]), 10000, locale)
            self.assertLessEqual(len(text["Tags"]), 16, locale)


class FolderTest(unittest.TestCase):
    def test_write_then_check_is_clean_and_old_versions_go(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)
            wm.write(out, record("0.2.6"))
            wm.write(out, record("0.2.7"))
            self.assertEqual(wm.stale(out, record("0.2.7")), [])
            self.assertEqual([p.name for p in out.joinpath(*wm.PACKAGE_DIR).iterdir()], ["0.2.7"])

    def test_check_names_each_problem(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)
            target = wm.write(out, record("0.2.7"))
            (target / "KBroneZ.TavernLedger.yaml").unlink()
            (target / "KBroneZ.TavernLedger.installer.yaml").write_text("edited\n", encoding="utf-8")
            (target / "notes.txt").write_text("x", encoding="utf-8")
            out.joinpath(*wm.PACKAGE_DIR, "0.2.6").mkdir()
            problems = wm.stale(out, record("0.2.7"))
            self.assertEqual(
                problems,
                [
                    "0.2.6: only the newest release's folder is kept",
                    "KBroneZ.TavernLedger.yaml: missing",
                    "KBroneZ.TavernLedger.installer.yaml: differs from the generator's output",
                    "notes.txt: not made by the generator",
                ],
            )

    def test_a_new_release_makes_the_committed_folder_stale(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)
            wm.write(out, record("0.2.7"))
            self.assertIn("0.2.7: only the newest release's folder is kept", wm.stale(out, record("0.2.8")))

    def test_the_committed_folder_is_current(self):
        release = wm.newest(json.loads(wm.RELEASES.read_text(encoding="utf-8")))
        self.assertEqual(wm.stale(wm.OUT, release), [], "run `python tools/winget_manifest.py`")


if __name__ == "__main__":
    unittest.main()
