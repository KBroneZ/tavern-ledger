"""Tests for tools/store_version.py: the Microsoft Store package version."""

import subprocess
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))

import store_version  # noqa: E402


def key(version: str) -> tuple[int, ...]:
    return tuple(int(part) for part in version.split("."))


class StoreVersionTest(unittest.TestCase):
    def test_the_first_part_is_never_zero_and_the_last_is_always_zero(self) -> None:
        self.assertEqual(store_version.store_version("0.2.4"), "1.2.4.0")
        self.assertEqual(store_version.store_version("0.0.1"), "1.0.1.0")
        self.assertEqual(store_version.store_version("1.0.0"), "2.0.0.0")
        self.assertEqual(store_version.store_version("12.34.56"), "13.34.56.0")

    def test_a_newer_app_version_is_always_a_newer_store_version(self) -> None:
        versions = ["0.0.9", "0.1.0", "0.2.4", "0.2.10", "0.10.0", "1.0.0", "1.0.1", "1.2.0", "2.0.0"]
        stores = [key(store_version.store_version(v)) for v in versions]
        self.assertEqual(stores, sorted(stores))
        self.assertEqual(len(set(stores)), len(stores), "two versions never share a Store version")

    def test_the_parts_must_fit_the_package_format(self) -> None:
        self.assertEqual(store_version.store_version("65534.65535.65535"), "65535.65535.65535.0")
        for bad in ["65535.0.0", "0.65536.0", "0.0.65536"]:
            with self.assertRaises(store_version.BadVersion, msg=bad):
                store_version.store_version(bad)

    def test_only_plain_three_part_versions_are_accepted(self) -> None:
        for bad in ["", "1", "1.2", "1.2.3.4", "1.2.3-beta.1", "1.2.3+build", "v1.2.3", "1.2.x", "01.2.3", "-1.2.3", " 1.2.3", "1.2.3\n"]:
            with self.assertRaises(store_version.BadVersion, msg=repr(bad)):
                store_version.store_version(bad)

    def test_the_command_line_prints_the_version(self) -> None:
        run = subprocess.run(
            [sys.executable, str(ROOT / "tools" / "store_version.py"), "0.2.4"],
            capture_output=True, text=True, check=False,
        )
        self.assertEqual((run.returncode, run.stdout.strip()), (0, "1.2.4.0"))
        bad = subprocess.run(
            [sys.executable, str(ROOT / "tools" / "store_version.py"), "nope"],
            capture_output=True, text=True, check=False,
        )
        self.assertEqual(bad.returncode, 1)
        self.assertEqual(bad.stdout, "")

    def test_the_app_version_in_the_config_maps(self) -> None:
        version = store_version.app_version()
        self.assertRegex(store_version.store_version(version), r"^[1-9]\d*\.\d+\.\d+\.0$")


if __name__ == "__main__":
    unittest.main()
