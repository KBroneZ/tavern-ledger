"""Tests for tools/release.py with synthetic release data (no network, no real key)."""

import base64
import datetime as dt
import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "tools"))

import release  # noqa: E402
from release import Refused  # noqa: E402

ROOT = Path(__file__).resolve().parent.parent
SHA = "e52f3a76a4ffb7717f017e431a6e56df7d86826e95d0372e75f5e3215e9386ee"
OTHER = "0" * 64


def public_key(raw: bytes = b"Ed" + bytes(8) + bytes(range(32))) -> str:
    """A key shaped like `tauri signer generate`'s public key (not a real one)."""
    text = f"untrusted comment: minisign public key 0000000000000000\n{base64.b64encode(raw).decode()}\n"
    return base64.b64encode(text.encode()).decode()


def conf(version: str = "0.2.0", key: str | None = None) -> str:
    return json.dumps({"version": version, "plugins": {"updater": {"pubkey": key or public_key()}}})


CRATE = '[package]\nname = "desktop"\nversion = "0.2.0"\n\n[dependencies]\nserde = { version = "1" }\n'


def asset(name: str, sha: str = SHA, size: int = 6_847_192) -> dict:
    return {"name": name, "size": size, "digest": f"sha256:{sha}", "state": "uploaded"}


def gh_release(tag: str = "v0.2.0", assets: list | None = None, draft: bool = False) -> dict:
    version = tag[1:]
    return {
        "tag_name": tag,
        "draft": draft,
        "prerelease": False,
        "published_at": "2026-10-20T10:00:00Z",
        "assets": assets if assets is not None else [asset(f"TavernLedger_{version}_x64-setup.exe")],
    }


class PrepareTest(unittest.TestCase):
    def test_a_tag_that_matches_both_versions_and_a_real_key_passes(self):
        out = release.prepare("refs/tags/v0.2.0", False, conf(), CRATE)
        self.assertEqual(out, {"version": "0.2.0", "file": "TavernLedger_0.2.0_x64-setup.exe"})

    def test_the_tag_must_be_the_config_version(self):
        with self.assertRaisesRegex(Refused, "not v0.2.0"):
            release.prepare("refs/tags/v0.2.1", False, conf(), CRATE)
        with self.assertRaisesRegex(Refused, "not v0.2.0"):
            release.prepare("refs/heads/main", False, conf(), CRATE)

    def test_the_crate_version_must_match_too(self):
        with self.assertRaisesRegex(Refused, "Cargo.toml"):
            release.prepare("refs/tags/v0.3.0", False, conf("0.3.0"), CRATE)

    def test_the_placeholder_key_stops_a_release_but_not_a_dry_run(self):
        placeholder = conf(key="NOT-SET-the-release-public-key-goes-here")
        with self.assertRaisesRegex(Refused, "placeholder"):
            release.prepare("refs/tags/v0.2.0", False, placeholder, CRATE)
        self.assertEqual(release.prepare("refs/pull/1/merge", True, placeholder, CRATE)["version"], "0.2.0")

    def test_the_committed_config_has_the_release_key_and_agrees_with_the_crate(self):
        text = (ROOT / "crates/desktop/tauri.conf.json").read_text(encoding="utf-8")
        crate = (ROOT / "crates/desktop/Cargo.toml").read_text(encoding="utf-8")
        version = json.loads(text)["version"]
        self.assertEqual(release.crate_version(crate), version)
        self.assertTrue(release.is_public_key(json.loads(text)["plugins"]["updater"]["pubkey"]))
        self.assertEqual(release.prepare(f"refs/tags/v{version}", False, text, crate)["version"], version)

    def test_only_a_minisign_shaped_key_counts(self):
        self.assertTrue(release.is_public_key(public_key()))
        for bad in [None, "", "not base64!", base64.b64encode(b"hello").decode(), public_key(b"Ed" + bytes(10)), public_key(b"XX" + bytes(40))]:
            self.assertFalse(release.is_public_key(bad), bad)


class TrustedKeyTest(unittest.TestCase):
    OLD = public_key(b"Ed" + bytes(8) + bytes(32))
    NEW = public_key()

    def git(self, confs: dict, earlier: list[str]):
        def run(args):
            if args[0] == "tag":
                return "".join(f"{name}\n" for name in earlier)
            return confs.get(args[1].split(":")[0])
        return run

    def test_the_first_updater_release_is_checked_against_its_own_key(self):
        git = self.git({"v0.2.0": conf(key=self.NEW), "v0.1.0": json.dumps({"version": "0.1.0"})}, ["v0.1.0"])
        self.assertEqual(release.trusted_key("v0.2.0", git)[0], self.NEW)
        self.assertEqual(release.trusted_key("v0.2.0", self.git({"v0.2.0": conf(key=self.NEW)}, []))[0], self.NEW)

    def test_a_release_that_changes_the_key_is_checked_against_the_old_one(self):
        git = self.git({"v0.3.0": conf("0.3.0", self.NEW), "v0.2.0": conf(key=self.OLD)}, ["v0.2.0"])
        key, note = release.trusted_key("v0.3.0", git)
        self.assertEqual(key, self.OLD)
        self.assertIn("changes the updater key", note)

    def test_a_stray_or_keyless_tag_in_between_does_not_hide_the_trusted_key(self):
        confs = {
            "v0.4.0": conf("0.4.0", self.NEW),
            "v0.3.1": json.dumps({"version": "0.3.1"}),
            "vtest": conf("0.0.0", self.NEW),
            "v0.3.0": conf("0.3.0", self.OLD),
        }
        git = self.git(confs, ["vtest", "v0.3.1", "v0.3.0", "v0.2.0"])
        self.assertEqual(release.trusted_key("v0.4.0", git)[0], self.OLD)

    def test_a_tag_without_a_real_key_is_refused(self):
        git = self.git({"v0.3.0": conf("0.3.0", "NOT-SET")}, ["v0.2.0"])
        with self.assertRaisesRegex(Refused, "no release public key"):
            release.trusted_key("v0.3.0", git)


class FilesTest(unittest.TestCase):
    def test_sums_are_written_sha256sum_style(self):
        with tempfile.TemporaryDirectory() as d:
            folder = Path(d)
            (folder / "b.exe").write_bytes(b"installer")
            (folder / "a.txt").write_bytes(b"")
            text = release.write_sums(folder, ["b.exe", "a.txt"])
            self.assertEqual(
                text,
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855  a.txt\n"
                f"{release.sha256_of(folder / 'b.exe')}  b.exe\n",
            )
            self.assertEqual((folder / "SHA256SUMS").read_bytes(), text.encode())

    def test_latest_json_points_only_at_the_tag_s_installer(self):
        sig = base64.b64encode(b"untrusted comment: x\nRUQ=\ntrusted comment: y\nZZ==\n").decode()
        text = release.latest_json("0.2.0", sig + "\n", "Notes.", dt.datetime(2026, 10, 20, 12, 0, tzinfo=dt.timezone.utc))
        doc = json.loads(text)
        self.assertEqual(doc["version"], "0.2.0")
        self.assertEqual(doc["pub_date"], "2026-10-20T12:00:00Z")
        url = "https://github.com/KBroneZ/tavern-ledger/releases/download/v0.2.0/TavernLedger_0.2.0_x64-setup.exe"
        for target in ["windows-x86_64-nsis", "windows-x86_64"]:
            self.assertEqual(doc["platforms"][target], {"signature": sig, "url": url})
        with self.assertRaises(Refused):
            release.latest_json("v0.2.0", sig, "", dt.datetime.now(dt.timezone.utc))

    def test_minisig_is_the_plain_signature_text(self):
        text = "untrusted comment: signature from tauri secret key\nRUQ=\ntrusted comment: timestamp:1\tversion:0.2.0\nZZ=="
        self.assertEqual(release.minisig(base64.b64encode(text.encode()).decode()), text + "\n")
        with self.assertRaises(Refused):
            release.minisig(base64.b64encode(b"something else").decode())


class FactsTest(unittest.TestCase):
    def test_facts_come_from_github_s_own_record_of_the_files(self):
        entry = release.facts_of(gh_release(), "v0.2.0", None)
        self.assertEqual(entry["file"], "TavernLedger_0.2.0_x64-setup.exe")
        self.assertEqual((entry["bytes"], entry["sha256"], entry["date"], entry["sums"]), (6_847_192, SHA, "2026-10-20", False))
        self.assertEqual(entry["assets"], {"TavernLedger_0.2.0_x64-setup.exe": {"bytes": 6_847_192, "sha256": SHA}})

    def test_the_sums_file_must_agree_with_github(self):
        r = gh_release(assets=[asset("TavernLedger_0.2.0_x64-setup.exe"), asset("SHA256SUMS", OTHER, 100)])
        entry = release.facts_of(r, "v0.2.0", f"{SHA}  TavernLedger_0.2.0_x64-setup.exe\n")
        self.assertTrue(entry["sums"])
        with self.assertRaisesRegex(Refused, "does not list"):
            release.facts_of(r, "v0.2.0", f"{OTHER}  TavernLedger_0.2.0_x64-setup.exe\n")

    def test_a_draft_a_strange_file_or_a_missing_digest_is_refused(self):
        with self.assertRaisesRegex(Refused, "draft"):
            release.facts_of(gh_release(draft=True), "v0.2.0", None)
        with self.assertRaisesRegex(Refused, "unexpected file"):
            release.facts_of(gh_release(assets=[asset("TavernLedger_0.2.0_x64-setup.exe"), asset("evil.dll")]), "v0.2.0", None)
        no_digest = asset("TavernLedger_0.2.0_x64-setup.exe")
        no_digest["digest"] = None
        with self.assertRaisesRegex(Refused, "no SHA-256"):
            release.facts_of(gh_release(assets=[no_digest]), "v0.2.0", None)
        with self.assertRaisesRegex(Refused, "no TavernLedger"):
            release.facts_of(gh_release(assets=[]), "v0.2.0", None)
        with self.assertRaisesRegex(Refused, "not the release"):
            release.facts_of(gh_release("v0.1.0"), "v0.2.0", None)

    def test_recording_keeps_newest_first_and_replaces_the_same_tag(self):
        old = release.facts_of(gh_release("v0.1.0"), "v0.1.0", None)
        fetched = []

        def fetch(url):
            fetched.append(url)
            return gh_release("v0.10.0")

        entries = release.record("v0.10.0", fetch, lambda url: "", [old])
        self.assertEqual([e["tag"] for e in entries], ["v0.10.0", "v0.1.0"])
        self.assertEqual(fetched, ["https://api.github.com/repos/KBroneZ/tavern-ledger/releases/tags/v0.10.0"])
        again = release.record("v0.10.0", fetch, lambda url: "", entries)
        self.assertEqual(len(again), 2)


class CheckTest(unittest.TestCase):
    def setUp(self):
        self.known = [release.facts_of(gh_release(), "v0.2.0", None)]

    def test_unchanged_files_pass(self):
        self.assertEqual(release.check_all(self.known, lambda url: gh_release()), [])

    def test_a_swapped_added_or_vanished_file_is_reported(self):
        swapped = gh_release(assets=[asset("TavernLedger_0.2.0_x64-setup.exe", OTHER)])
        self.assertEqual(len(release.check_all(self.known, lambda url: swapped)), 1)
        added = gh_release(assets=[asset("TavernLedger_0.2.0_x64-setup.exe"), asset("latest.json", OTHER, 10)])
        self.assertIn("added", release.check_all(self.known, lambda url: added)[0])

        def gone(url):
            raise Refused(f"{url}: HTTP 404")

        self.assertIn("404", release.check_all(self.known, gone)[0])
        self.assertEqual(release.check_all([], lambda url: gh_release()), ["releases.json lists no release"])

    def test_the_committed_facts_are_well_formed(self):
        entries = json.loads((ROOT / "web/src/lib/site/releases.json").read_text(encoding="utf-8"))
        self.assertTrue(entries)
        for entry in entries:
            self.assertEqual(entry["tag"], f"v{entry['version']}")
            self.assertEqual(entry["assets"][entry["file"]], {"bytes": entry["bytes"], "sha256": entry["sha256"]})


if __name__ == "__main__":
    unittest.main()
