"""Tests for tools/check_advisories.py with synthetic scanner and cargo output (no network)."""

import datetime
import io
import sys
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "tools"))

import check_advisories as tool  # noqa: E402
from check_advisories import Advisory  # noqa: E402

ROOT = tool.ROOT
TODAY = datetime.date(2026, 10, 23)


def package(name, version, groups=(), vulns=()):
    return {
        "package": {"name": name, "version": version},
        "groups": list(groups),
        "vulnerabilities": list(vulns),
    }


def vuln(vid, name, fixed):
    events = [{"introduced": "0"}] + ([{"fixed": "9.9.9"}] if fixed else [])
    return {"id": vid, "affected": [{"package": {"name": name}, "ranges": [{"type": "SEMVER", "events": events}]}]}


def report(**by_lock):
    """A scan report with every lockfile, one harmless package each, plus extras."""
    results = []
    for lock in tool.LOCKFILES:
        packages = [package("ok", "1.0.0")] + by_lock.get(lock, [])
        results.append({"source": {"path": str(ROOT / lock)}, "packages": packages})
    return {"results": results}


def entry(**overrides):
    base = {
        "id": "RUSTSEC-1",
        "package": "glib",
        "reason": "Linux only",
        "evidence": "cargo tree",
        "review_by": datetime.date(2027, 1, 1),
    }
    return {**base, **overrides}


class ReadScanTest(unittest.TestCase):
    def test_an_advisory_with_its_ids_severity_and_fix(self):
        glib = package(
            "glib",
            "0.18.5",
            groups=[{"ids": ["RUSTSEC-1"], "aliases": ["GHSA-1"], "max_severity": "6.9"}],
            vulns=[vuln("RUSTSEC-1", "glib", fixed=True)],
        )
        advisories, problems = tool.read_scan(report(**{"Cargo.lock": [glib]}), ROOT)
        self.assertEqual(problems, [])
        self.assertEqual(advisories, [Advisory("Cargo.lock", "glib", "0.18.5", ("GHSA-1", "RUSTSEC-1"), 6.9, True)])

    def test_unknown_severity_and_no_fix(self):
        groups = [{"ids": ["R-2"], "max_severity": ""}]
        old = package("old", "1.0.0", groups=groups, vulns=[vuln("R-2", "old", fixed=False)])
        (advisory,), _ = tool.read_scan(report(**{"requirements.txt": [old]}), ROOT)
        self.assertIsNone(advisory.severity)
        self.assertFalse(advisory.fixed)

    def test_a_fix_for_another_package_does_not_count(self):
        groups = [{"ids": ["X-1"], "max_severity": "2.0"}]
        pkg = package("a", "1.0.0", groups=groups, vulns=[vuln("X-1", "b", fixed=True)])
        (advisory,), _ = tool.read_scan(report(**{"package-lock.json": [pkg]}), ROOT)
        self.assertFalse(advisory.fixed)

    def test_a_missing_or_empty_lockfile_is_a_problem(self):
        scan = report()
        scan["results"] = [r for r in scan["results"] if not r["source"]["path"].endswith("requirements.txt")]
        scan["results"][0]["packages"] = []
        _, problems = tool.read_scan(scan, ROOT)
        self.assertIn("requirements.txt: not in the scan results", problems)
        self.assertIn("Cargo.lock: no packages found", problems)

    def test_an_empty_report_is_four_problems(self):
        _, problems = tool.read_scan({}, ROOT)
        self.assertEqual(len(problems), len(tool.LOCKFILES))


class PolicyTest(unittest.TestCase):
    def adv(self, severity, fixed):
        return Advisory("Cargo.lock", "x", "1", ("X-1",), severity, fixed)

    def test_only_low_without_a_fix_passes(self):
        self.assertFalse(tool.blocks(self.adv(3.9, False)))
        self.assertTrue(tool.blocks(self.adv(3.9, True)))
        self.assertTrue(tool.blocks(self.adv(4.0, False)))
        self.assertTrue(tool.blocks(self.adv(None, False)))

    def test_an_ignore_needs_the_same_package(self):
        adv = Advisory("Cargo.lock", "glib", "0.18.5", ("RUSTSEC-1",), 9.0, True)
        blocking, _, used = tool.evaluate([adv], [entry(package="gtk")])
        self.assertEqual(len(blocking), 1)
        self.assertEqual(used, [])
        blocking, passed, used = tool.evaluate([adv], [entry()])
        self.assertEqual(blocking, [])
        self.assertEqual(len(used), 1)
        self.assertIn("ignored until 2027-01-01", passed[0])

    def test_an_alias_matches_the_ignore(self):
        adv = Advisory("Cargo.lock", "glib", "0.18.5", ("GHSA-1", "RUSTSEC-1"), 9.0, True)
        blocking, _, _ = tool.evaluate([adv], [entry(id="GHSA-1")])
        self.assertEqual(blocking, [])


class IgnoreEntriesTest(unittest.TestCase):
    def test_a_complete_entry_is_fine(self):
        self.assertEqual(tool.check_ignores([entry()], TODAY), [])

    def test_every_field_is_required(self):
        (problem,) = tool.check_ignores([entry(reason="", evidence=None)], TODAY)
        self.assertIn("missing reason, evidence", problem)

    def test_a_past_review_date_fails(self):
        (problem,) = tool.check_ignores([entry(review_by=datetime.date(2026, 10, 22))], TODAY)
        self.assertIn("has passed", problem)
        self.assertEqual(tool.check_ignores([entry(review_by=TODAY)], TODAY), [])

    def test_the_review_date_must_be_a_date(self):
        (problem,) = tool.check_ignores([entry(review_by="soon")], TODAY)
        self.assertIn("must be a date", problem)

    def test_the_committed_file_reads_and_is_complete(self):
        import tomllib

        entries = tomllib.loads(tool.IGNORES.read_text(encoding="utf-8"))["ignore"]
        self.assertTrue(entries)
        self.assertEqual(tool.check_ignores(entries, datetime.date(2026, 10, 23)), [])
        for item in entries:
            self.assertTrue(item.get("absent_from"), f"{item['id']}: a crate entry names the builds it is not in")


def metadata(edges, members, names=None):
    """Synthetic `cargo metadata` output: ids are names unless `names` maps them."""
    names = names or {}
    ids = set(edges) | {d for deps in edges.values() for d in deps} | set(members)
    return {
        "packages": [{"id": i, "name": names.get(i, i)} for i in sorted(ids)],
        "workspace_members": list(members),
        "resolve": {"nodes": [{"id": i, "deps": [{"pkg": d} for d in edges.get(i, [])]} for i in sorted(ids)]},
    }


class ReachableTest(unittest.TestCase):
    META = metadata({"desktop": ["tauri"], "tauri": ["gtk"], "gtk": ["glib"], "cards": ["serde"]}, ["desktop", "cards"])

    def test_follows_dependencies_from_every_member(self):
        self.assertEqual(tool.reachable(self.META, set()), {"desktop", "tauri", "gtk", "glib", "cards", "serde"})

    def test_an_excluded_member_drops_what_only_it_brings(self):
        self.assertEqual(tool.reachable(self.META, {"desktop"}), {"cards", "serde"})

    def test_a_crate_that_appears_in_a_build_breaks_the_ignore(self):
        item = entry(absent_from=[{"target": "linux"}, {"target": "linux", "exclude": ["desktop"]}])
        (problem,) = tool.check_absent(item, metadata_for=lambda target: self.META)
        self.assertIn("glib is now built for linux", problem)

    def test_an_absent_crate_keeps_the_ignore(self):
        item = entry(absent_from=[{"target": "linux", "exclude": ["desktop"]}])
        self.assertEqual(tool.check_absent(item, metadata_for=lambda target: self.META), [])

    def test_an_absent_from_without_target_is_a_problem(self):
        (problem,) = tool.check_absent(entry(absent_from=[{}]), metadata_for=lambda target: self.META)
        self.assertIn("no target", problem)


class FunctionsTest(unittest.TestCase):
    def folder(self, files):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        root = Path(tmp.name)
        for name, text in files.items():
            (root / name).parent.mkdir(parents=True, exist_ok=True)
            (root / name).write_text(text, encoding="utf-8")
        return root

    def test_local_imports_only_is_fine(self):
        root = self.folder({"a/index.ts": 'import { h } from "./handler.ts";\nimport x from "../_shared/x.ts";\n'})
        self.assertEqual(tool.check_functions(root), [])

    def test_npm_jsr_and_url_imports_fail(self):
        root = self.folder(
            {
                "a/index.ts": 'import { z } from "npm:zod@3";\n',
                "b/index.ts": "export { x } from 'jsr:@std/x';\n",
                "c/index.ts": 'const m = await import("https://deno.land/x/m.ts");\n',
            }
        )
        problems = tool.check_functions(root)
        self.assertEqual(len([p for p in problems if "third-party import" in p]), 3)

    def test_a_deno_lockfile_fails(self):
        root = self.folder({"deno.lock": "{}", "a/index.ts": ""})
        self.assertTrue(any("Deno config or lockfile" in p for p in tool.check_functions(root)))

    def test_the_repo_functions_import_nothing_third_party(self):
        self.assertEqual(tool.check_functions(), [])


class MainTest(unittest.TestCase):
    def run_main(self, scan=None, error=None, entries=()):
        out, err = io.StringIO(), io.StringIO()
        runner = mock.Mock(side_effect=error) if error else mock.Mock(return_value=scan)
        ignores = mock.Mock(return_value={"ignore": list(entries)})
        with (
            mock.patch.object(tool, "run_scanner", runner),
            mock.patch.object(tool.tomllib, "loads", ignores),
            mock.patch.object(tool, "check_absent", lambda e: []),
            mock.patch.object(tool, "check_functions", lambda: []),
            redirect_stdout(out),
            redirect_stderr(err),
        ):
            code = tool.main(["--scanner", "osv-scanner", "--today", "2026-10-23"])
        return code, out.getvalue() + err.getvalue()

    def test_a_clean_scan_passes(self):
        code, text = self.run_main(report())
        self.assertEqual(code, 0, text)

    def test_a_blocking_advisory_fails(self):
        groups = [{"ids": ["G-1"], "max_severity": "9.1"}]
        bad = package("certifi", "1", groups=groups, vulns=[vuln("G-1", "certifi", True)])
        code, text = self.run_main(report(**{"requirements.txt": [bad]}))
        self.assertEqual(code, 1)
        self.assertIn("FAIL: requirements.txt: certifi 1: G-1", text)

    def test_a_scanner_error_is_exit_2(self):
        code, text = self.run_main(error=tool.ScanError("osv-scanner exited 127: network"))
        self.assertEqual(code, 2)
        self.assertIn("network", text)

    def test_an_unused_ignore_fails(self):
        code, text = self.run_main(report(), entries=[entry()])
        self.assertEqual(code, 1)
        self.assertIn("no advisory matches it; remove it", text)


class ScannerRunTest(unittest.TestCase):
    def test_an_error_exit_code_raises(self):
        result = mock.Mock(returncode=127, stdout="", stderr="api unreachable")
        with mock.patch.object(tool.subprocess, "run", return_value=result):
            with self.assertRaisesRegex(tool.ScanError, "exited 127"):
                tool.run_scanner("osv-scanner")

    def test_vulnerabilities_found_is_not_an_error(self):
        result = mock.Mock(returncode=1, stdout='{"results": []}', stderr="")
        with mock.patch.object(tool.subprocess, "run", return_value=result):
            self.assertEqual(tool.run_scanner("osv-scanner"), {"results": []})

    def test_unreadable_output_raises(self):
        result = mock.Mock(returncode=0, stdout="not json", stderr="")
        with mock.patch.object(tool.subprocess, "run", return_value=result):
            with self.assertRaisesRegex(tool.ScanError, "unreadable"):
                tool.run_scanner("osv-scanner")

    def test_a_missing_binary_raises(self):
        with mock.patch.object(tool.subprocess, "run", side_effect=FileNotFoundError("osv-scanner")):
            with self.assertRaises(tool.ScanError):
                tool.run_scanner("osv-scanner")


if __name__ == "__main__":
    unittest.main()
