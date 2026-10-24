#!/usr/bin/env python3
"""Regressions for the candidate scheduling boundary, using the actual caller."""

import unittest

from check_workflow import CALLER, PolicyError, verify_candidate_admission


class CandidateAdmissionTests(unittest.TestCase):
    def setUp(self):
        self.caller = CALLER.read_text(encoding="utf-8")

    def test_checked_in_caller_is_admitted(self):
        verify_candidate_admission(self.caller)

    def test_each_required_pr_transition_is_retained(self):
        for activity in ["opened", "synchronize", "reopened", "ready_for_review", "converted_to_draft"]:
            with self.subTest(activity=activity):
                changed = self.caller.replace(activity, "edited", 1)
                with self.assertRaisesRegex(PolicyError, "transitions"):
                    verify_candidate_admission(changed)

    def test_merge_group_and_manual_dispatch_cannot_be_omitted(self):
        for trigger in ["merge_group", "workflow_dispatch"]:
            with self.subTest(trigger=trigger):
                with self.assertRaisesRegex(PolicyError, "merge/manual"):
                    verify_candidate_admission(self.caller.replace(f"  {trigger}:\n", ""))

    def test_draft_guard_cannot_be_removed_or_reversed(self):
        condition = next(line for line in self.caller.splitlines(True) if line.startswith("    if:"))
        for replacement in ["", "    if: true\n", condition.replace("== false", "== true")]:
            with self.subTest(replacement=replacement):
                with self.assertRaisesRegex(PolicyError, "draft PRs"):
                    verify_candidate_admission(self.caller.replace(condition, replacement))

    def test_non_pr_candidates_cannot_be_skipped(self):
        with self.assertRaisesRegex(PolicyError, "merge-group and manual"):
            verify_candidate_admission(self.caller.replace("github.event_name != 'pull_request' || ", ""))

    def test_path_filters_cannot_skip_candidate_evidence(self):
        with self.assertRaisesRegex(PolicyError, "filters"):
            verify_candidate_admission(self.caller.replace("  merge_group:", "    paths: ['crates/**']\n  merge_group:"))

    def test_concurrency_cannot_cancel_another_pr_or_keep_stale_work(self):
        for old, new in [
            ("cancel-in-progress: true", "cancel-in-progress: false"),
            ("github.event.pull_request.number || github.ref", "github.workflow"),
        ]:
            with self.subTest(old=old):
                with self.assertRaisesRegex(PolicyError, "same PR or ref"):
                    verify_candidate_admission(self.caller.replace(old, new))

    def test_another_caller_cannot_bypass_admission(self):
        with self.assertRaisesRegex(PolicyError, "one pipeline"):
            verify_candidate_admission(self.caller + "\n  bypass:\n    uses: ./.github/workflows/ci-reusable.yml\n")


if __name__ == "__main__":
    unittest.main()
