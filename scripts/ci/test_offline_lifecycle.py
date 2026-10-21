"""Adversarial input tests for the offline qualification runner."""

import json
import copy
import os
from pathlib import Path
import tempfile
import sys
import time
import unittest
from unittest.mock import patch

import run_offline_lifecycle as runner


class OfflineLifecyclePolicyTests(unittest.TestCase):
    def test_windows_ipv6_probe_requires_a_native_nonzero_scope(self):
        with patch.object(runner.sys, "platform", "win32"):
            for scope in [None, False, 0, -1, "12", 2 ** 32]:
                with self.subTest(scope=scope), self.assertRaises(RuntimeError):
                    runner.native_probe_targets({"ipv6_probe_scope_id": scope})
            targets = runner.native_probe_targets({"ipv6_probe_scope_id": 12})
            self.assertEqual(targets[1][1], ("fe80::ffff:ffff:fffe", 9, 0, 12))
        with patch.object(runner.sys, "platform", "darwin"):
            self.assertEqual(runner.native_probe_targets({})[1][1], ("2001:db8::1", 9))

    def test_preflight_and_cleanup_failures_cannot_lose_or_preserve_success_receipts(self):
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary) / "receipts" / "result.json"
            runner.record_outer_failure(output, RuntimeError("fixture failure" * 200))
            receipt = json.loads(output.read_text())
            self.assertEqual(receipt["status"], "failed")
            self.assertEqual(receipt["phase"], "preflight")
            self.assertEqual(len(receipt["outer_failure"]["diagnostic"]), 1024)
            output.write_text(json.dumps({"status": "passed", "sha256": {"rrflow": "a" * 64}}))
            runner.record_outer_failure(output, RuntimeError("cleanup failed"))
            receipt = json.loads(output.read_text())
            self.assertEqual(receipt["status"], "failed")
            self.assertEqual(receipt["sha256"], {"rrflow": "a" * 64})
            self.assertEqual(receipt["outer_failure"]["diagnostic"], "cleanup failed")

    def test_durable_receipts_reject_omission_tampering_and_unknown_fields(self):
        first = {"mode": "fresh", "build": {key: "b" * 64 for key in ["executable_sha256", "metadata_sha256", "public_contract_sha256", "signal_catalogue_sha256"]},
                 "installation_plan_sha256": "a" * 64, "restore_plan_sha256": "b" * 64,
                 "backup_sha256": "c" * 64, "restored_read_manifest_sha256": "d" * 64,
                 "installed_runtime_cursor": 2, "reopened_runtime_cursor": 4, "restored_runtime_cursor": 3,
                 "committed_claim_sequence": 5, "captured_output_bytes": 1, "stream_limit_bytes": 16 * 1024 * 1024}
        second = {**first, "mode": "existing"}
        def encode(receipts):
            return "\n".join("native_lifecycle_receipt=" + json.dumps(receipt) for receipt in receipts)
        self.assertEqual(runner.lifecycle_receipts(encode([first, second])), [first, second])
        invalid = [[], [first], [first, first], [first, second, second]]
        for field, value in [("restored_runtime_cursor", 4), ("backup_sha256", "bad"),
                             ("captured_output_bytes", 2 ** 32), ("credential", "must never be retained")]:
            altered = copy.deepcopy(second)
            altered[field] = value
            invalid.append([first, altered])
        altered = copy.deepcopy(second)
        altered["build"]["credential"] = "must never be retained"
        invalid.append([first, altered])
        for receipts in invalid:
            with self.subTest(receipts=receipts), self.assertRaises(RuntimeError):
                runner.lifecycle_receipts(encode(receipts))

    def test_result_requires_each_case_once_without_skips(self):
        lines = [f"test {case} ... ok" for case in sorted(runner.REQUIRED_CASES)]
        summary = "test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.00s"
        output = "\n".join(lines + [summary])
        self.assertEqual(runner.require_lifecycle_results(output), sorted(runner.REQUIRED_CASES))
        for invalid in ["", "\n".join(lines), "\n".join(lines[1:] + [summary]),
                        output + "\n" + lines[0], output.replace("0 ignored", "1 ignored"),
                        output.replace("0 filtered out", "1 filtered out"),
                        output.replace(lines[0], lines[0].replace("ok", "FAILED")),
                        output + "\n" + summary]:
            with self.subTest(output=invalid), self.assertRaises(RuntimeError):
                runner.require_lifecycle_results(invalid)

    def test_child_deadline_and_output_limit_are_enforced(self):
        start = time.monotonic()
        with self.assertRaisesRegex(RuntimeError, "deadline"):
            runner.run(sys.executable, "-c", "import time; time.sleep(30)", timeout=0.2)
        self.assertLess(time.monotonic() - start, 5)
        with self.assertRaisesRegex(RuntimeError, "output"):
            runner.run(sys.executable, "-c", "import sys; sys.stdout.write('x' * 1000000)", output_limit=1024)
        with self.assertRaisesRegex(RuntimeError, "exit status"):
            runner.run(sys.executable, "-c", "raise SystemExit(3)")
        output = runner.run(sys.executable, "-c", "import sys; print('out'); print('err', file=sys.stderr)")
        self.assertEqual(output.stdout.strip(), "out")
        self.assertEqual(output.stderr.strip(), "err")

    def test_every_primary_lifecycle_case_is_required(self):
        cases = {
            "primary_help_exposes_only_the_installed_product_lifecycle",
            "raw_database_and_development_commands_fail_during_argument_parsing",
            "install_requires_an_explicit_plan_digest_and_verify_is_read_only_by_shape",
            "fresh_installed_engine_survives_restart_and_restore",
            "installed_engine_plans_applies_serves_discovers_authenticates_and_reopens",
        }
        runner.require_lifecycle_tests("\n".join(f"{case}: test" for case in sorted(cases)))
        for missing in sorted(cases):
            with self.subTest(missing=missing), self.assertRaises(RuntimeError):
                runner.require_lifecycle_tests(
                    "\n".join(f"{case}: test" for case in sorted(cases - {missing}))
                )

    def test_zero_or_partial_lifecycle_corpus_is_not_a_pass(self):
        fresh = "fresh_installed_engine_survives_restart_and_restore: test"
        existing = "installed_engine_plans_applies_serves_discovers_authenticates_and_reopens: test"
        for listing in ["", fresh, existing, fresh + "\n" + existing, "5 tests, 0 benchmarks"]:
            with self.subTest(listing=listing), self.assertRaises(RuntimeError):
                runner.require_lifecycle_tests(listing)

    def test_artifact_selection_requires_both_actual_executables(self):
        with tempfile.TemporaryDirectory() as temporary:
            binary = Path(temporary) / "rrflow"
            harness = Path(temporary) / "installed_lifecycle"
            binary.touch()
            harness.touch()
            def artifact(name, kind, path):
                return json.dumps({"reason": "compiler-artifact", "target": {"name": name, "kind": [kind]},
                                   "executable": str(path)})
            primary = artifact("rrflow", "bin", binary)
            test = artifact("installed_lifecycle", "test", harness)
            self.assertEqual(runner.artifact_paths(primary + "\n" + test), (binary.resolve(), harness.resolve()))
            for output in [primary, test, "", artifact("rrflow", "test", binary) + "\n" + test]:
                with self.subTest(output=output), self.assertRaises(RuntimeError):
                    runner.artifact_paths(output)
            duplicate = artifact("rrflow", "bin", harness)
            with self.assertRaises(RuntimeError):
                runner.artifact_paths(primary + "\n" + duplicate + "\n" + test)

    def test_mask_rejects_system_roots_and_payload_overlap(self):
        with tempfile.TemporaryDirectory() as temporary:
            stage = Path(temporary).resolve()
            child = stage / "child"
            child.mkdir()
            for path in [Path(stage.anchor), stage, child]:
                with self.subTest(path=path), self.assertRaises(RuntimeError):
                    runner.mask_paths([path], stage)
            systems = ([os.environ[name] for name in
                        ["SystemRoot", "ProgramFiles", "ProgramFiles(x86)", "ProgramData", "PUBLIC"]
                        if name in os.environ] if sys.platform == "win32"
                       else ["/usr", "/etc", "/proc", "/tmp"])
            for system in systems:
                path = Path(system)
                if path.exists():
                    with self.subTest(path=path), self.assertRaises(RuntimeError):
                        runner.mask_paths([path], stage)

    def test_current_drive_source_is_not_an_operating_system_root(self):
        with tempfile.TemporaryDirectory(prefix="rrflow-mask-", dir=Path.cwd()) as source, \
                tempfile.TemporaryDirectory() as payload:
            root = Path(source).resolve()
            self.assertEqual(runner.mask_paths([root], Path(payload).resolve()), [root])

    def test_hash_observes_changed_payload(self):
        with tempfile.TemporaryDirectory() as temporary:
            payload = Path(temporary) / "rrflow"
            payload.write_bytes(b"original")
            original = runner.sha256(payload)
            payload.write_bytes(b"changed")
            self.assertNotEqual(original, runner.sha256(payload))

    def test_nested_masks_are_deduplicated_without_hiding_payload(self):
        with tempfile.TemporaryDirectory(prefix="rrflow-mask-", dir=Path.home()) as source, \
                tempfile.TemporaryDirectory() as payload:
            parent = Path(source).resolve()
            nested = parent / "checkout"
            nested.mkdir()
            self.assertEqual(runner.mask_paths([nested, parent, parent], Path(payload).resolve()), [parent])

    def test_namespace_mismatch_fails_before_mount_or_privilege_change(self):
        with tempfile.TemporaryDirectory() as temporary:
            stage = Path(temporary)
            (stage / "isolation.json").write_text(json.dumps({"namespaces": {"net": "unchanged"}}))
            with patch.object(runner.os, "geteuid", return_value=0, create=True), \
                    patch.object(runner.os, "readlink", return_value="unchanged"), \
                    patch.object(runner, "run") as command:
                with self.assertRaisesRegex(RuntimeError, "net namespace was not isolated"):
                    runner.inside(stage)
                command.assert_not_called()


if __name__ == "__main__":
    unittest.main()
