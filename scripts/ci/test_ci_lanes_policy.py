#!/usr/bin/env python3
"""Regressions for the proportional-lane contract of the reusable candidate CI."""

import unittest

from check_workflow import (
    FULL_LANE_IF,
    FULL_LANE_JOBS,
    GATE_EXPECTATIONS,
    LANE_CONDITIONS,
    NIGHTLY,
    REUSABLE,
    PolicyError,
    job_blocks,
    verify_lane_selection,
    verify_nightly_caller,
)


class LanePolicyTests(unittest.TestCase):
    def setUp(self):
        self.reusable = REUSABLE.read_text(encoding="utf-8")
        self.nightly = NIGHTLY.read_text(encoding="utf-8")

    def check(self, reusable):
        verify_lane_selection(job_blocks(reusable), reusable)

    def test_checked_in_workflow_is_admitted(self):
        self.check(self.reusable)
        verify_nightly_caller(self.nightly)

    def test_every_full_lane_job_declares_exactly_the_full_lane(self):
        for job in FULL_LANE_JOBS:
            marker = f"\n  {job}:\n"
            self.assertIn(marker, self.reusable)
            before, after = self.reusable.split(marker, 1)
            with self.subTest(job=job, change="condition removed"):
                self.assertIn(FULL_LANE_IF, after[:after.index("\n    steps:")])
                with self.assertRaisesRegex(PolicyError, "exactly the full lane"):
                    changed = self.reusable.replace(marker + after[:after.index("\n    steps:")],
                                                     marker + after[:after.index("\n    steps:")].replace(FULL_LANE_IF, "", 1), 1)
                    self.check(changed)
            with self.subTest(job=job, change="condition weakened"):
                weakened = self.reusable.replace(FULL_LANE_IF, "    if: ${{ always() }}\n", 1)
                with self.assertRaisesRegex(PolicyError, "exactly the full lane"):
                    self.check(before + marker + weakened)
            with self.subTest(job=job, change="second condition"):
                doubled = self.reusable.replace(FULL_LANE_IF, FULL_LANE_IF + "    if: false\n", 1)
                with self.assertRaisesRegex(PolicyError, "exactly the full lane"):
                    self.check(before + marker + doubled)

    def test_light_lane_jobs_declare_exactly_their_lane(self):
        for job, condition in LANE_CONDITIONS.items():
            marker = f"\n  {job}:\n"
            self.assertIn(marker, self.reusable)
            with self.subTest(job=job):
                self.assertIn(condition, self.reusable.split(marker, 1)[1])
                with self.assertRaisesRegex(PolicyError, "exactly its lane"):
                    self.check(self.reusable.replace(condition, FULL_LANE_IF, 1))

    def test_a_partition_cannot_drop_its_dependency_on_the_selection(self):
        with self.assertRaisesRegex(PolicyError, "depend on the lane selection"):
            self.check(self.reusable.replace("    needs: [changes, storage-contract-preflight]\n",
                                             "    needs: storage-contract-preflight\n", 1))

    def test_lane_selection_cannot_be_conditional_or_untested(self):
        marker = "\n  changes:\n    name: select CI lanes\n"
        self.assertIn(marker, self.reusable)
        with self.assertRaisesRegex(PolicyError, "cannot be conditional"):
            self.check(self.reusable.replace(marker, marker + "    if: false\n", 1))
        with self.assertRaisesRegex(PolicyError, "lane selection is missing"):
            self.check(self.reusable.replace("          python3 scripts/ci/test_select_ci_lanes.py\n", "", 1))

    def test_selection_must_see_the_base_revision_and_full_history(self):
        with self.assertRaisesRegex(PolicyError, "lane selection is missing"):
            self.check(self.reusable.replace(
                "BASE_SHA: ${{ github.event.pull_request.base.sha || github.event.merge_group.base_sha || github.event.before }}",
                "BASE_SHA: ${{ github.event.pull_request.base.sha }}", 1))
        marker = "\n  changes:\n"
        before, after = self.reusable.split(marker, 1)
        with self.assertRaisesRegex(PolicyError, "lane selection is missing"):
            self.check(before + marker + after.replace("          fetch-depth: 0\n", "", 1))

    def test_engine_fast_lane_tests_the_selection_and_compiles_dependents(self):
        for command in (
            'test -n "$PACKAGES$DEPENDENTS"',
            "cargo test $PACKAGES --locked --no-fail-fast",
            "cargo clippy $PACKAGES --all-targets --locked -- -D warnings",
            "cargo check $DEPENDENTS --all-targets --locked",
        ):
            with self.subTest(command=command):
                self.assertIn(command, self.reusable)
                with self.assertRaisesRegex(PolicyError, "engine fast lane is missing"):
                    self.check(self.reusable.replace(command, "true", 1))

    def test_engine_fast_lane_handles_a_workspace_only_selection(self):
        guard = "        if: ${{ env.PACKAGES != '' }}\n"
        self.assertEqual(self.reusable.count(guard), 2)
        with self.assertRaisesRegex(PolicyError, "workspace-only selection"):
            self.check(self.reusable.replace(guard, "", 1))

    def test_script_lane_enforces_the_ci_policy_candidates_no_longer_get_from_verify(self):
        for command in ("python3 scripts/ci/check_workflow.py", "python3 scripts/ci/test_merge_alarm.py",
                        "python3 scripts/ci/test_ci_lanes_policy.py"):
            with self.subTest(command=command):
                marker = "\n  scripts-policy:\n"
                before, after = self.reusable.split(marker, 1)
                with self.assertRaisesRegex(PolicyError, "script lane"):
                    self.check(before + marker + after.replace(f"          {command}\n", "", 1))

    def test_repository_invariants_run_on_every_light_candidate(self):
        jobs = job_blocks(self.reusable)
        self.assertIn("    if: ${{ needs.changes.outputs.full != 'true' }}\n", jobs["repository-invariants"])
        command = "cargo test -p rrd-engine --locked --test workspace_architecture"
        self.assertIn(command, jobs["repository-invariants"])
        with self.assertRaisesRegex(PolicyError, "repository invariants"):
            self.check(self.reusable.replace(command, "true", 1))
        with self.assertRaisesRegex(PolicyError, "exactly its lane"):
            self.check(self.reusable.replace(
                "    if: ${{ needs.changes.outputs.full != 'true' }}\n",
                "    if: ${{ needs.changes.outputs.docs == 'true' }}\n", 1))

    def test_gate_expectations_cover_every_partition(self):
        jobs = job_blocks(self.reusable)
        self.assertEqual(set(GATE_EXPECTATIONS), set(jobs) - {"ci-gate", "changes"})
        gate = jobs["ci-gate"]
        for job, (lane, variable) in GATE_EXPECTATIONS.items():
            with self.subTest(job=job):
                self.assertIn(f'expect "${lane}" "${variable}" {job}', gate)
        self.assertIn('test "$CHANGES_RESULT" = success', gate)
        self.assertNotIn('= success\n          test', gate.split("expect()", 1)[0].replace(
            'test "$CHANGES_RESULT" = success', ""), "no unconditional success tests remain")

    def test_nightly_caller_runs_the_full_matrix_on_a_schedule_only(self):
        with self.assertRaisesRegex(PolicyError, "schedule"):
            verify_nightly_caller(self.nightly.replace("  schedule:\n", "  push:\n  schedule:\n", 1))
        with self.assertRaisesRegex(PolicyError, "full matrix"):
            verify_nightly_caller(self.nightly.replace("      scope: full\n", "      scope: auto\n", 1))
        with self.assertRaisesRegex(PolicyError, "cancel"):
            verify_nightly_caller(self.nightly.replace("  cancel-in-progress: false\n", "  cancel-in-progress: true\n", 1))


if __name__ == "__main__":
    unittest.main()
