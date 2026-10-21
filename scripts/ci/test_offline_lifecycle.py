"""Adversarial input tests for the offline qualification runner."""

import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import run_offline_lifecycle as runner


class OfflineLifecyclePolicyTests(unittest.TestCase):
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
        runner.require_lifecycle_tests(fresh + "\n" + existing)
        for listing in ["", fresh, existing, "5 tests, 0 benchmarks"]:
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
            for system in ["/usr", "/etc", "/proc", "/tmp"]:
                path = Path(system)
                if path.exists():
                    with self.subTest(path=path), self.assertRaises(RuntimeError):
                        runner.mask_paths([path], stage)

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
