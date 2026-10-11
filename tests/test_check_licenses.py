"""Tests for tools/check_licenses.py with synthetic metadata (no cargo, no network)."""

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "tools"))

import check_licenses  # noqa: E402
from check_licenses import OWN_LICENSE, is_allowed  # noqa: E402


class IsAllowedTest(unittest.TestCase):
    def test_single_allowed_license(self):
        self.assertTrue(is_allowed("MIT"))
        self.assertTrue(is_allowed("MPL-2.0"))
        self.assertTrue(is_allowed("LGPL-3.0-or-later"))

    def test_or_needs_one_allowed_side(self):
        self.assertTrue(is_allowed("MIT OR Apache-2.0"))
        self.assertTrue(is_allowed("SSPL-1.0 OR MIT"))
        self.assertFalse(is_allowed("SSPL-1.0 OR BUSL-1.1"))

    def test_and_needs_every_side_allowed(self):
        self.assertTrue(is_allowed("Apache-2.0 AND MIT"))
        self.assertFalse(is_allowed("MIT AND SSPL-1.0"))

    def test_old_slash_form_means_or(self):
        self.assertTrue(is_allowed("MIT/Apache-2.0"))
        self.assertTrue(is_allowed("Apache-2.0 / MIT"))

    def test_parentheses_and_precedence(self):
        self.assertTrue(is_allowed("(MIT OR Apache-2.0) AND Unicode-3.0"))
        # AND binds tighter than OR: MIT alone is enough here.
        self.assertTrue(is_allowed("MIT OR SSPL-1.0 AND BUSL-1.1"))
        self.assertFalse(is_allowed("(MIT OR SSPL-1.0) AND BUSL-1.1"))

    def test_with_exception(self):
        self.assertTrue(is_allowed("Apache-2.0 WITH LLVM-exception"))
        self.assertFalse(is_allowed("Apache-2.0 WITH Made-Up-exception"))

    def test_incompatible_and_unknown_licenses_fail(self):
        for expr in ("GPL-2.0-only", "SSPL-1.0", "Proprietary", "MIT NON-AI"):
            with self.subTest(expr=expr):
                self.assertFalse(is_allowed(expr))

    def test_empty_or_malformed_expression_fails(self):
        for expr in ("", "   ", None, "MIT OR", "(MIT", "MIT)", "AND MIT", "MIT WITH"):
            with self.subTest(expr=expr):
                self.assertFalse(is_allowed(expr))

    def test_own_license_is_not_in_the_third_party_list(self):
        self.assertFalse(is_allowed(OWN_LICENSE))


def cargo_metadata(packages, members):
    return {
        "packages": [{"id": f"{name} 1.0.0", "name": name, "version": "1.0.0", "license": lic} for name, lic in packages],
        "workspace_members": [f"{name} 1.0.0" for name in members],
    }


class CheckCargoTest(unittest.TestCase):
    def test_clean_workspace_has_no_problems(self):
        meta = cargo_metadata([("tracker", OWN_LICENSE), ("serde", "MIT OR Apache-2.0")], ["tracker"])
        problems, counted = check_licenses.check_cargo(meta)
        self.assertEqual(problems, [])
        self.assertEqual(counted, 2)

    def test_own_crate_with_another_license_is_a_problem(self):
        meta = cargo_metadata([("tracker", "MIT")], ["tracker"])
        problems, _ = check_licenses.check_cargo(meta)
        self.assertEqual(len(problems), 1)
        self.assertIn("tracker", problems[0])

    def test_dependency_with_incompatible_or_missing_license_is_a_problem(self):
        meta = cargo_metadata([("tracker", OWN_LICENSE), ("bad", "SSPL-1.0"), ("none", None)], ["tracker"])
        problems, _ = check_licenses.check_cargo(meta)
        self.assertEqual(len(problems), 2)
        self.assertTrue(any("bad" in p for p in problems))
        self.assertTrue(any("none" in p for p in problems))

    def test_no_packages_is_a_problem(self):
        problems, _ = check_licenses.check_cargo({"packages": [], "workspace_members": []})
        self.assertEqual(len(problems), 1)


class CheckNpmLockTest(unittest.TestCase):
    def lock(self, root_license, deps):
        packages = {"": {"name": "tavern-ledger-web", "license": root_license}}
        packages.update({f"node_modules/{name}": {"version": "1.0.0", "license": lic} for name, lic in deps})
        return {"lockfileVersion": 3, "packages": packages}

    def test_clean_lock_has_no_problems(self):
        problems, counted = check_licenses.check_npm_lock(self.lock(OWN_LICENSE, [("astro", "MIT")]), "web")
        self.assertEqual(problems, [])
        self.assertEqual(counted, 1)

    def test_root_with_another_license_is_a_problem(self):
        problems, _ = check_licenses.check_npm_lock(self.lock("MIT", []), "web")
        self.assertEqual(len(problems), 1)

    def test_dependency_with_incompatible_or_missing_license_is_a_problem(self):
        problems, _ = check_licenses.check_npm_lock(self.lock(OWN_LICENSE, [("bad", "SSPL-1.0"), ("none", None)]), "web")
        self.assertEqual(len(problems), 2)

    def test_lock_without_root_package_is_a_problem(self):
        problems, _ = check_licenses.check_npm_lock({"packages": {}}, "web")
        self.assertEqual(len(problems), 1)


if __name__ == "__main__":
    unittest.main()
