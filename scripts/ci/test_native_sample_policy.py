#!/usr/bin/env python3
"""Reject CI changes that omit native worker lifecycle or image oracles."""
import unittest
from check_workflow import REUSABLE, PolicyError, job_blocks, verify_native_sample_worker_ci


class NativeSamplePolicyTests(unittest.TestCase):
    def setUp(self):
        jobs = job_blocks(REUSABLE.read_text(encoding="utf-8"))
        self.reopen = jobs["installed-reopen"]
        self.verify = jobs["verify"]
        self.step = (
            "      - name: Verify owned Windows native sampler lifecycle\n"
            "        if: runner.os == 'Windows'\n"
            "        timeout-minutes: 5\n"
            "        shell: bash\n"
            "        run: |\n"
            "          set -euo pipefail\n"
            "          python scripts/ci/test_windows_harness_image.py --profile release\n"
            "          python scripts/ci/test_native_sample_worker.py --profile release\n"
        )
        self.assertEqual(self.reopen.count(self.step), 1)
        self.linux_step = (
            "      - name: Verify Linux native sampler running image\n"
            "        if: runner.os == 'Linux'\n"
            "        timeout-minutes: 5\n"
            "        run: cargo test -p rrflow-cli --test installed_reopen --release --locked -- sustained::native_worker::tests::replaced_harness_path_cannot_redirect_linux_worker_or_receipt_digest --exact --nocapture\n"
        )
        self.assertEqual(self.reopen.count(self.linux_step), 1)
        self.failure_step = (
            "      - name: Require bounded native sampler failure\n"
            "        timeout-minutes: 5\n"
            "        run: cargo test -p rrflow-cli --test installed_reopen --release --locked -- sustained::native_worker::tests::acquisition_deadline_keeps_unfinished_cleanup_owned_and_blocks_reuse --exact --nocapture\n"
        )
        self.assertEqual(self.reopen.count(self.failure_step), 1)

    def test_installed_conflict_oracle_is_required(self):
        step = (
            "      - name: Require installed stale-snapshot conflict safety\n"
            "        timeout-minutes: 5\n"
            "        shell: bash\n"
            "        run: |\n"
            "          set -euo pipefail\n"
            "          cargo test -p rrflow-cli --test installed_reopen --release --locked -- --ignored --list | grep -Fx 'sustained::campaign::installed_disjoint_writes_from_one_stamp_conflict_without_partial_effect: test'\n"
            "          cargo test -p rrflow-cli --test installed_reopen --release --locked -- sustained::campaign::installed_disjoint_writes_from_one_stamp_conflict_without_partial_effect --exact --ignored --nocapture\n"
        )
        self.assertEqual(self.reopen.count(step), 1)
        verify_native_sample_worker_ci(self.reopen, self.verify)
        for changed in (
            "", step + step,
            step.replace("        shell: bash\n", "        if: runner.os == 'Windows'\n        shell: bash\n"),
            step.replace("          set -euo pipefail\n", ""),
            step.replace(" -- --ignored --list | grep -Fx", " -- --ignored --list | grep -F"),
            step.replace(" -- --ignored --list | grep -Fx", " -- --list | grep -Fx"),
            step.replace(" --exact --ignored --nocapture", " --exact --nocapture"),
            step.replace(" --exact --ignored --nocapture", " --ignored --nocapture"),
            step.replace("timeout-minutes: 5", "timeout-minutes: 0"),
            step + "        continue-on-error: true\n",
        ):
            with self.subTest(changed=changed):
                with self.assertRaises(PolicyError):
                    verify_native_sample_worker_ci(self.reopen.replace(step, changed), self.verify)

    def test_checked_in_lane_and_policy_are_accepted(self):
        verify_native_sample_worker_ci(self.reopen, self.verify)

    def test_removed_or_duplicated_oracle_is_rejected(self):
        for replacement in ("", self.step + self.step):
            with self.subTest(replacement=replacement):
                with self.assertRaises(PolicyError):
                    verify_native_sample_worker_ci(self.reopen.replace(self.step, replacement), self.verify)

    def test_weakened_condition_command_timeout_or_failure_is_rejected(self):
        for changed in (
            self.step.replace("runner.os == 'Windows'", "false"),
            self.step.replace("--profile release", "--profile debug"),
            self.step.replace("timeout-minutes: 5", "timeout-minutes: 0"),
            self.step.replace("          set -euo pipefail\n", ""),
            self.step.replace("          python scripts/ci/test_windows_harness_image.py --profile release\n", ""),
            self.step + "        continue-on-error: true\n",
        ):
            with self.subTest(changed=changed):
                with self.assertRaises(PolicyError):
                    verify_native_sample_worker_ci(self.reopen.replace(self.step, changed), self.verify)

    def test_missing_or_bypassed_regression_runner_is_rejected(self):
        command = "          python3 scripts/ci/test_native_sample_policy.py\n"
        for replacement in ("          true\n", "          # python3 scripts/ci/test_native_sample_policy.py\n"):
            with self.subTest(replacement=replacement):
                with self.assertRaises(PolicyError):
                    verify_native_sample_worker_ci(self.reopen, self.verify.replace(command, replacement))
        marker = "      - name: Enforce repository CI policy\n"
        with self.assertRaises(PolicyError):
            verify_native_sample_worker_ci(
                self.reopen, self.verify.replace(marker, marker + "        continue-on-error: true\n")
            )

    def test_linux_image_oracle_cannot_be_removed_or_bypassed(self):
        for changed in (
            "",
            self.linux_step + self.linux_step,
            self.linux_step.replace("runner.os == 'Linux'", "false"),
            self.linux_step.replace("--release", "--profile debug"),
            self.linux_step.replace("timeout-minutes: 5", "timeout-minutes: 0"),
            self.linux_step + "        continue-on-error: true\n",
        ):
            with self.subTest(changed=changed):
                with self.assertRaises(PolicyError):
                    verify_native_sample_worker_ci(self.reopen.replace(self.linux_step, changed), self.verify)

    def test_bounded_failure_oracle_cannot_be_removed_or_bypassed(self):
        for changed in (
            "",
            self.failure_step + self.failure_step,
            self.failure_step.replace("--release", "--profile debug"),
            self.failure_step.replace("timeout-minutes: 5", "timeout-minutes: 0"),
            self.failure_step + "        continue-on-error: true\n",
        ):
            with self.subTest(changed=changed):
                with self.assertRaises(PolicyError):
                    verify_native_sample_worker_ci(self.reopen.replace(self.failure_step, changed), self.verify)


if __name__ == "__main__":
    unittest.main()
