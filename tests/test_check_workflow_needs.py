"""Tests for tools/check_workflow_needs.py with synthetic workflows, and the repo's own."""

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "tools"))

from check_workflow_needs import main, parse, problems  # noqa: E402

ROOT = Path(__file__).resolve().parent.parent

# The shape of the release workflow before its fix: the draft job was skipped
# on a real tag because the dry-run job before verify was.
SKIPPED_DRAFT = """\
on: push
jobs:
  build:
    runs-on: x
  sign:
    needs: build
    if: ${{ github.event_name == 'push' }}
  sign-dry-run:
    needs: build
    if: ${{ github.event_name != 'push' }}
  verify:
    needs: [build, sign, sign-dry-run]
    if: ${{ !cancelled() && needs.build.result == 'success' }}
  draft:
    needs: [build, verify]
    if: ${{ github.event_name == 'push' }}
"""


class CheckWorkflowNeedsTest(unittest.TestCase):
    def test_a_job_after_a_skippable_one_without_a_status_function_is_reported(self):
        found = problems(SKIPPED_DRAFT)
        self.assertEqual(len(found), 1)
        self.assertTrue(found[0].startswith("draft: follows sign, sign-dry-run, verify"))

    def test_a_status_function_says_how_the_skip_is_treated(self):
        fixed = SKIPPED_DRAFT.replace(
            "  draft:\n    needs: [build, verify]\n    if: ${{ github.event_name == 'push' }}",
            "  draft:\n    needs: [build, verify]\n    if: ${{ !cancelled() && needs.verify.result == 'success' }}",
        )
        self.assertEqual(problems(fixed), [])
        always = SKIPPED_DRAFT.replace("if: ${{ github.event_name == 'push' }}\n", "if: ${{ always() }}\n")
        self.assertEqual(problems(always), [])

    def test_a_job_with_no_if_after_a_skippable_one_is_reported_too(self):
        text = "jobs:\n  a:\n    if: ${{ false }}\n  b:\n    needs: a\n"
        self.assertEqual(len(problems(text)), 1)

    def test_jobs_after_jobs_that_always_run_need_nothing(self):
        text = "jobs:\n  a:\n    runs-on: x\n  b:\n    needs: a\n    if: ${{ github.ref == 'x' }}\n"
        self.assertEqual(problems(text), [])

    def test_unreadable_or_unknown_needs_are_reported_not_guessed(self):
        text = "jobs:\n  a:\n    runs-on: x\n  b:\n    needs:\n      - a\n  c:\n    needs: [a, nope]\n"
        found = problems(text)
        self.assertTrue(any("needs: ''" in p for p in found), found)
        self.assertTrue(any("unknown job 'nope'" in p for p in found), found)

    def test_steps_and_other_top_level_keys_are_not_jobs(self):
        text = "on:\n  push:\n    branches: [main]\njobs:\n  a:\n    steps:\n      - if: ${{ failure() }}\n        run: x\n"
        jobs, unreadable = parse(text)
        self.assertEqual(list(jobs), ["a"])
        self.assertEqual(unreadable, [])
        self.assertIsNone(jobs["a"].condition, "a step's if is not the job's")

    def test_the_repository_workflows_pass(self):
        self.assertEqual(main(["--root", str(ROOT)]), 0)


if __name__ == "__main__":
    unittest.main()
