"""Tests for tools/check_updater_key.py: the decision, and a real git repo end to end."""

import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "tools"))

import check_updater_key as tool  # noqa: E402


def conf(key):
    return json.dumps({"plugins": {"updater": {"pubkey": key}}})


class DecisionTest(unittest.TestCase):
    def test_pubkey_is_read_and_a_missing_one_is_none(self):
        self.assertEqual(tool.pubkey(conf("AAA")), "AAA")
        self.assertIsNone(tool.pubkey(json.dumps({"plugins": {}})))
        self.assertIsNone(tool.pubkey(json.dumps({})))
        self.assertIsNone(tool.pubkey(None))

    def test_an_unchanged_key_passes_without_a_marker(self):
        self.assertIsNone(tool.problem("AAA", "AAA", [""]))

    def test_a_changed_key_fails_without_a_marker(self):
        self.assertIn("changed", tool.problem("AAA", "BBB", ["feat: x", "body"]))

    def test_a_removed_or_added_key_counts_as_a_change(self):
        self.assertIsNotNone(tool.problem("AAA", None, [""]))
        self.assertIsNotNone(tool.problem(None, "AAA", [""]))

    def test_the_marker_in_any_text_allows_the_change(self):
        for text in ("updater-key-change: yes", "x\nUpdater-Key-Change:   YES  \ny"):
            self.assertIsNone(tool.problem("A", "B", ["", text]), text)

    def test_the_marker_must_be_a_line_of_its_own_and_say_yes(self):
        for text in (
            "updater-key-change: no",
            "do not write updater-key-change: yes here",
            "updater-key-change: yes please",
            "updater-key-change yes",
        ):
            self.assertIsNotNone(tool.problem("A", "B", [text]), text)


class GitTest(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.root = Path(tmp.name)
        patcher = mock.patch.object(tool, "ROOT", self.root)
        patcher.start()
        self.addCleanup(patcher.stop)
        self.run_git("init", "-q")
        self.write("A")
        self.base = self.commit("chore: first")

    def run_git(self, *args):
        env = {
            **os.environ,
            "GIT_AUTHOR_NAME": "t",
            "GIT_AUTHOR_EMAIL": "t@example.com",
            "GIT_COMMITTER_NAME": "t",
            "GIT_COMMITTER_EMAIL": "t@example.com",
        }
        return subprocess.run(
            ["git", *args], cwd=self.root, check=True, env=env, capture_output=True, text=True
        ).stdout.strip()

    def write(self, key):
        path = self.root / tool.CONFIG
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(conf(key), encoding="utf-8")

    def commit(self, message):
        self.run_git("add", "-A")
        self.run_git("commit", "-q", "--allow-empty", "-m", message)
        return self.run_git("rev-parse", "HEAD")

    def check(self, body=""):
        with mock.patch.dict(os.environ, {"PR_BODY": body}):
            return tool.main(["--base", self.base])

    def test_no_change_passes(self):
        self.commit("feat: other")
        self.assertEqual(self.check(), 0)

    def test_a_changed_key_fails(self):
        self.write("B")
        self.commit("feat: new key")
        self.assertEqual(self.check(), 1)

    def test_the_marker_in_the_body_passes(self):
        self.write("B")
        self.commit("feat: new key")
        self.assertEqual(self.check("Rotation.\n\nupdater-key-change: yes\n"), 0)

    def test_the_marker_in_a_commit_message_passes(self):
        self.write("B")
        self.commit("feat: new key\n\nupdater-key-change: yes")
        self.assertEqual(self.check(), 0)

    def test_a_key_changed_and_changed_back_passes(self):
        self.write("B")
        self.commit("feat: new key")
        self.write("A")
        self.commit("fix: restore")
        self.assertEqual(self.check(), 0)

    def test_an_unknown_base_is_an_error_not_a_pass(self):
        with mock.patch.dict(os.environ, {"PR_BODY": ""}):
            self.assertEqual(tool.main(["--base", "0" * 40]), 2)

    def test_a_pubkey_in_an_override_config_fails(self):
        (self.root / "crates/desktop/tauri.windows.conf.json").write_text(conf("C"), encoding="utf-8")
        self.commit("feat: override")
        self.assertEqual(self.check("updater-key-change: yes"), 1)

    def test_the_repo_has_a_readable_key(self):
        text = (Path(__file__).resolve().parent.parent / tool.CONFIG).read_text(encoding="utf-8")
        self.assertTrue(tool.pubkey(text))


if __name__ == "__main__":
    unittest.main()
