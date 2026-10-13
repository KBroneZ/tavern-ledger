"""Tests for tools/check_action_pins.py with synthetic workflow text (no repo files)."""

import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "tools"))

import check_action_pins  # noqa: E402
from check_action_pins import find_unpinned, main  # noqa: E402

SHA = "3d3c42e5aac5ba805825da76410c181273ba90b1"


def problems(text: str) -> list[str]:
    return [p.reason for p in find_unpinned("ci.yml", text)]


class FindUnpinnedTest(unittest.TestCase):
    def test_full_sha_with_version_comment_is_pinned(self):
        self.assertEqual(problems(f"      - uses: actions/checkout@{SHA} # v7.0.1\n"), [])

    def test_step_with_other_keys_after_dash_is_read(self):
        text = f"      - name: Links\n        uses: lycheeverse/lychee-action@{SHA} # v2.9.0\n"
        self.assertEqual(problems(text), [])

    def test_tag_is_not_pinned(self):
        self.assertEqual(len(problems("      - uses: actions/checkout@v4\n")), 1)

    def test_branch_is_not_pinned(self):
        self.assertEqual(len(problems("      - uses: actions/checkout@main\n")), 1)

    def test_short_sha_is_not_pinned(self):
        self.assertEqual(len(problems(f"      - uses: actions/checkout@{SHA[:7]} # v4\n")), 1)

    def test_no_ref_at_all_is_not_pinned(self):
        self.assertEqual(len(problems("      - uses: actions/checkout\n")), 1)

    def test_uppercase_sha_is_not_pinned(self):
        self.assertEqual(len(problems(f"      - uses: actions/checkout@{SHA.upper()} # v4\n")), 1)

    def test_sha_without_version_comment_is_reported(self):
        self.assertEqual(len(problems(f"      - uses: actions/checkout@{SHA}\n")), 1)

    def test_comment_without_a_version_is_reported(self):
        self.assertEqual(len(problems(f"      - uses: actions/checkout@{SHA} # latest\n")), 1)

    def test_quoted_value_is_read(self):
        self.assertEqual(len(problems("      - uses: 'actions/checkout@v4'\n")), 1)
        self.assertEqual(problems(f'      - uses: "actions/checkout@{SHA}" # v7.0.1\n'), [])

    def test_local_action_needs_no_pin(self):
        self.assertEqual(problems("      - uses: ./.github/actions/setup\n"), [])

    def test_docker_image_needs_a_digest(self):
        self.assertEqual(len(problems("      - uses: docker://alpine:3.20\n")), 1)
        digest = "a" * 64
        self.assertEqual(problems(f"      - uses: docker://alpine@sha256:{digest} # 3.20\n"), [])

    def test_reusable_workflow_is_checked_too(self):
        self.assertEqual(len(problems("    uses: org/repo/.github/workflows/x.yml@main\n")), 1)
        self.assertEqual(problems(f"    uses: org/repo/.github/workflows/x.yml@{SHA} # v1.2.3\n"), [])

    def test_commented_out_use_is_ignored(self):
        self.assertEqual(problems("      # - uses: actions/checkout@v4\n"), [])

    def test_uses_inside_a_run_block_is_ignored(self):
        text = "      - run: |\n          echo uses: actions/checkout@v4\n"
        self.assertEqual(problems(text), [])

    def test_forms_the_check_cannot_read_are_reported_not_skipped(self):
        for line in [
            "      - {uses: a/b@v1}",
            "      - { name: x, uses: a/b@v1 }",
            '      - "uses": a/b@v1',
            "      - 'uses': a/b@v1",
            "      - uses: &x a/b@v1",
            "      - uses: !!str a/b@v1",
            "        uses:",
            f"      - {{uses: a/b@{SHA}}}",
        ]:
            with self.subTest(line=line):
                self.assertEqual(len(problems(line + "\n")), 1)

    def test_comment_stuck_to_the_ref_is_not_a_pin(self):
        self.assertEqual(len(problems(f"      - uses: a/b@{SHA}#v1\n")), 1)

    def test_reports_line_number_and_action(self):
        text = f"steps:\n  - uses: actions/checkout@{SHA} # v7.0.1\n  - uses: actions/cache@v4\n"
        (found,) = find_unpinned("ci.yml", text)
        self.assertEqual((found.path, found.line), ("ci.yml", 3))
        self.assertIn("actions/cache@v4", found.action)

    def test_crlf_line_ends_are_read(self):
        text = f"  - uses: actions/checkout@{SHA} # v7.0.1\r\n  - uses: actions/cache@v4\r\n"
        self.assertEqual(len(problems(text)), 1)


class MainTest(unittest.TestCase):
    def run_in(self, files: dict[str, str]) -> int:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            workflows = root / ".github" / "workflows"
            workflows.mkdir(parents=True)
            for name, text in files.items():
                (workflows / name).write_text(text, encoding="utf-8")
            return main(["--root", str(root)])

    def test_local_action_files_are_scanned_too(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / ".github" / "workflows").mkdir(parents=True)
            (root / ".github" / "workflows" / "ci.yml").write_text(f"- uses: a/b@{SHA} # v1\n")
            action = root / ".github" / "actions" / "setup"
            action.mkdir(parents=True)
            (action / "action.yml").write_text("- uses: x/y@v1\n")
            self.assertEqual(main(["--root", str(root)]), 1)

    def test_pinned_workflows_pass(self):
        self.assertEqual(self.run_in({"ci.yml": f"- uses: actions/checkout@{SHA} # v7.0.1\n"}), 0)

    def test_one_unpinned_action_in_any_file_fails(self):
        files = {"a.yml": f"- uses: actions/checkout@{SHA} # v7.0.1\n", "b.yaml": "- uses: x/y@v1\n"}
        self.assertEqual(self.run_in(files), 1)

    def test_no_workflows_is_an_error(self):
        self.assertEqual(self.run_in({}), 2)

    def test_the_real_workflows_are_pinned(self):
        self.assertEqual(main([]), 0)


if __name__ == "__main__":
    unittest.main()
