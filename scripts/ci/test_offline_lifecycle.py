"""Adversarial input tests for the offline qualification runner."""

import json
import copy
import os
import signal
import subprocess
from pathlib import Path
import tempfile
import sys
import threading
import time
import unittest
from unittest.mock import patch

import run_offline_lifecycle as runner
from selected_tests import LIFECYCLE_CASES


def held_output_command(pid_file, stdout, stderr, status=1, message="hdiutil: create failed - Resource busy"):
    """A child that ends (or, with no status, overruns) while its descendant keeps streams open."""
    def stream(held, name):
        return f"sys.{name}" if held else "subprocess.DEVNULL"
    finish = "__import__('time').sleep(60)" if status is None else f"sys.exit({status})"
    return (sys.executable, "-c",
            "import pathlib,subprocess,sys; "
            "holder=subprocess.Popen([sys.executable,'-c','import time; time.sleep(60)'],"
            f"stdin=subprocess.DEVNULL,stdout={stream(stdout, 'stdout')},stderr={stream(stderr, 'stderr')}); "
            "pathlib.Path(sys.argv[1]).write_text(str(holder.pid)); "
            f"print({message!r},flush=True); print({message!r},file=sys.stderr,flush=True); {finish}",
            str(pid_file))


def end_holder(pid_file):
    # The test owns this descendant's lifetime, so no assertion races its timing.
    if pid_file.exists():
        try:
            os.kill(int(pid_file.read_text()), signal.SIGTERM)
        except (OSError, ValueError):
            pass


def recorded_popen(started):
    """Record the group leader's PID without depending on the child having started."""
    real = subprocess.Popen

    def popen(*arguments, **kwargs):
        child = real(*arguments, **kwargs)
        started.append(child.pid)
        return child
    return patch.object(runner.subprocess, "Popen", side_effect=popen)


class OfflineLifecyclePolicyTests(unittest.TestCase):
    def assert_group_gone(self, group):
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            try:
                os.killpg(group, 0)
            except (ProcessLookupError, PermissionError):
                return  # Darwin reports EPERM once only zombies remain.
            time.sleep(0.01)
        self.fail(f"owned process group {group} outlived run()")

    def test_open_output_metadata_rejects_unrecognized_category_and_status(self):
        error = runner.CommandOutputNotClosed("private-path", "private-status", True, False)
        self.assertEqual(error.command_category, "other")
        self.assertIsNone(error.child_returncode)
        self.assertNotIn("private", str(error))

    def test_failed_child_with_held_output_keeps_its_text_out_of_closed_metadata(self):
        for stdout, stderr in [(True, False), (False, True), (True, True)]:
            with self.subTest(stdout=stdout, stderr=stderr), tempfile.TemporaryDirectory() as directory:
                pid_file = Path(directory) / "holder.pid"
                completed = []
                try:
                    with self.assertRaises(runner.CommandOutputNotClosed) as caught:
                        runner.run(*held_output_command(pid_file, stdout, stderr, 3, "private native detail"),
                                   check=False, timeout=30, on_completion=completed.append)
                finally:
                    end_holder(pid_file)
                error = caught.exception
                self.assertEqual(error.child_returncode, 3)
                self.assertEqual((error.stdout_open, error.stderr_open), (stdout, stderr))
                self.assertEqual(error.captured.returncode, 3)
                self.assertIn("private native detail", error.captured.stdout)
                self.assertIn("private native detail", error.captured.stderr)
                self.assertNotIn("private", str(error))
                self.assertNotIn("private", repr(error))
                self.assertEqual(completed, [])

    @unittest.skipUnless(os.name == "posix", "requires a real POSIX process group")
    def test_successful_group_command_lets_its_descendant_finish(self):
        finish = '(sleep 0.5; echo finished; : > "$1")'
        for name, script, stdout in [("holding stdout", f"{finish} & exit 0", "finished\n"),
                                     ("detached", f"{finish} >/dev/null 2>&1 & exit 0", "")]:
            with self.subTest(name), tempfile.TemporaryDirectory() as directory:
                marker = Path(directory) / "finished"
                started = []
                with recorded_popen(started), patch.object(runner, "_OUTPUT_CLOSE_SECONDS", 30):
                    result = runner.run("sh", "-c", script, "sh", str(marker), process_group=True, timeout=30)
                self.assertEqual((result.returncode, result.stdout), (0, stdout))
                self.assertTrue(marker.exists(), "descendant was killed because its parent exited")
                self.assert_group_gone(started[0])

    @unittest.skipUnless(os.name == "posix", "requires a real POSIX process group")
    def test_successful_group_command_without_descendant_does_not_wait(self):
        begin = time.monotonic()
        with patch.object(runner, "_OUTPUT_CLOSE_SECONDS", 60):
            runner.run("sh", "-c", "exit 0", process_group=True, timeout=30)
        self.assertLess(time.monotonic() - begin, 20)

    @unittest.skipUnless(os.name == "posix", "requires a real POSIX process group")
    def test_successful_group_command_kills_descendant_that_outlives_the_output_bound(self):
        started = []
        begin = time.monotonic()
        with recorded_popen(started), patch.object(runner, "_OUTPUT_CLOSE_SECONDS", 0.2):
            result = runner.run("sh", "-c", "sleep 30 & exit 0", process_group=True, timeout=30)
        self.assertEqual(result.returncode, 0)
        self.assert_group_gone(started[0])
        self.assertLess(time.monotonic() - begin, 20)

    @unittest.skipUnless(os.name == "posix", "requires a real POSIX process group")
    def test_successful_group_command_wait_cannot_outlast_its_deadline(self):
        started = []
        begin = time.monotonic()
        with recorded_popen(started), patch.object(runner, "_OUTPUT_CLOSE_SECONDS", 60):
            result = runner.run("sh", "-c", "sleep 30 & exit 0", process_group=True, timeout=5)
        self.assertEqual(result.returncode, 0)
        self.assert_group_gone(started[0])
        self.assertLess(time.monotonic() - begin, 20)

    @unittest.skipUnless(os.name == "posix", "requires a real POSIX process group")
    def test_output_failure_ends_the_group_wait_at_once(self):
        # The reader can see an overflow after the child has already exited.
        failed = threading.Event()
        failed.set()
        begin = time.monotonic()
        runner._await_process_group_exit(os.getpgrp(), time.monotonic() + 60, failed)
        self.assertLess(time.monotonic() - begin, 20)

    @unittest.skipUnless(os.name == "posix", "requires a real POSIX process group")
    def test_failed_group_command_loses_its_descendant_at_once(self):
        slow = '(sleep 5; : > "$1") >/dev/null 2>&1 &'
        for name, script, kwargs, failure in [
            ("exit status", f"{slow} exit 3", {"check": False, "timeout": 30}, None),
            ("deadline", f"{slow} sleep 30", {"timeout": 0.3}, "deadline"),
            ("output limit", f"{slow} printf %02000d 0; exit 0", {"timeout": 30, "output_limit": 1024}, "output"),
        ]:
            with self.subTest(name), tempfile.TemporaryDirectory() as directory:
                marker = Path(directory) / "finished"
                started = []
                with recorded_popen(started), patch.object(runner, "_OUTPUT_CLOSE_SECONDS", 30):
                    if failure is None:
                        self.assertEqual(runner.run("sh", "-c", script, "sh", str(marker), process_group=True,
                                                    **kwargs).returncode, 3)
                    else:
                        with self.assertRaisesRegex(RuntimeError, failure):
                            runner.run("sh", "-c", script, "sh", str(marker), process_group=True, **kwargs)
                self.assertFalse(marker.exists(), "failed command's descendant was allowed to finish")
                self.assert_group_gone(started[0])
                self.assertFalse(marker.exists())

    @unittest.skipUnless(os.name == "posix", "requires a real inherited POSIX pipe")
    def test_exited_child_with_inherited_stdout_reports_bounded_closed_metadata(self):
        with tempfile.TemporaryDirectory() as directory:
            marker = Path(directory) / "private-marker"
            grandchild = "import pathlib,sys,time; time.sleep(4); pathlib.Path(sys.argv[1]).write_text('done')"
            parent = ("import subprocess,sys; "
                      "subprocess.Popen([sys.executable,'-c',sys.argv[1],sys.argv[2]], stderr=subprocess.DEVNULL)")
            started = time.monotonic()
            completed = []
            with self.assertRaises(RuntimeError) as captured:
                runner.run(sys.executable, "-c", parent, grandchild, str(marker), timeout=10,
                           check=False, on_completion=completed.append)
            error = captured.exception
            self.assertEqual(type(error).__name__, "CommandOutputNotClosed")
            self.assertEqual(error.command_category, "other")
            self.assertEqual(error.child_returncode, 0)
            self.assertTrue(error.stdout_open)
            self.assertFalse(error.stderr_open)
            self.assertEqual(completed, [])
            self.assertNotIn(str(marker), str(error))
            self.assertLess(time.monotonic() - started, 4)
            deadline = time.monotonic() + 5
            while not marker.exists() and time.monotonic() < deadline:
                time.sleep(0.05)
            self.assertTrue(marker.exists(), "controlled descendant did not exit")

    def test_timeout_budget_keeps_namespace_wrapper_above_harness_deadline(self):
        harness, wrapper = runner.lifecycle_timeouts()
        self.assertEqual((harness, wrapper), (540, 600))
        self.assertGreater(wrapper, harness)

    def test_storage_history_requires_both_modes_and_complete_checked_ledgers(self):
        # Deliberately incomplete history must not become evidence because its
        # producer printed a passing summary. The real fixture supplies a
        # canonical ledger for validation by the existing Rust evaluator.
        for output in ["", "native_storage_history={}",
                       'native_storage_history={"mode":"fresh","report":{"violations":[]}}']:
            with self.subTest(output=output), self.assertRaises(RuntimeError):
                runner.storage_history_receipts(output, [])

    def test_storage_history_rejects_changed_build_schedule_and_nested_payloads(self):
        build = {key: "b" * 64 for key in ["executable_sha256", "metadata_sha256",
                                         "public_contract_sha256", "signal_catalogue_sha256"]}
        expected = [{"logical_id": name, "request_sha256": "a" * 64,
                     "effect_sha256": "c" * 64, "resolution": "absent" if name == "c" else "present"}
                    for name in "abcd"]
        schedule = [("a", {"kind": "acknowledged", "receipt_sha256": "d" * 64}),
                    ("b", {"kind": "uncertain", "reason": "response-lost"}),
                    ("c", {"kind": "rejected", "reason": "stale-cursor"}),
                    ("d", {"kind": "acknowledged", "receipt_sha256": "d" * 64})]
        schedule += [(name, {"kind": "acknowledged", "receipt_sha256": "d" * 64})
                     for _ in range(3) for name in "abd"]
        attempts = [{"attempt_id": f"attempt-{index}", "logical_id": name,
                     "request_sha256": "a" * 64, "started_ns": index, "finished_ns": index + 1,
                     "outcome": outcome} for index, (name, outcome) in enumerate(schedule)]
        readback = [{"kind": "present", "logical_id": name, "request_sha256": "a" * 64,
                     "effect_sha256": "c" * 64, "receipt_sha256": "d" * 64, "applications": 1}
                    for name in "abd"] + [{"kind": "absent", "logical_id": "c"}]
        first = {"mode": "fresh", "build": build, "forced_reopens": 3, "expected_sha256": "e" * 64,
                 "ledger": {"expected": expected, "attempts": attempts, "readback": readback},
                 "report": {"logical_calls": 4, "attempts": 13, "acknowledged": 11, "rejected": 1,
                            "uncertain": 1, "resolved_present": 3, "resolved_absent": 1,
                            "unresolved": 0, "violations": []}}
        second = {**copy.deepcopy(first), "mode": "existing"}
        durable = [{"mode": mode, "build": build} for mode in ["fresh", "existing"]]
        def encode(receipts):
            return "\n".join("native_storage_history=" + json.dumps(item) for item in receipts)
        self.assertEqual(runner.storage_history_receipts(encode([first, second]), durable), [first, second])
        invalid = [[], [first], [first, first], [first, second, second]]
        changes = [(["forced_reopens"], 2), (["build", "executable_sha256"], "0" * 64),
                   (["report", "violations"], ["lost-acknowledged-operation"]),
                   (["ledger", "attempts", 0, "credential"], "never retain"),
                   (["ledger", "attempts", 0, "started_ns"], True),
                   (["ledger", "attempts", 0, "outcome", "receipt_sha256"], "invalid"),
                   (["ledger", "expected", 0, "logical_id"], "b"),
                   (["ledger", "readback", 0, "applications"], 2),
                   (["ledger", "readback", 0, "logical_id"], "b"),
                   (["ledger", "attempts", 0, "outcome"], {"kind": "uncertain", "reason": "response-lost"})]
        for path, value in changes:
            altered = copy.deepcopy(second)
            parent = altered
            for key in path[:-1]:
                parent = parent[key]
            parent[path[-1]] = value
            invalid.append([first, altered])
        for receipts in invalid:
            with self.subTest(receipts=receipts), self.assertRaises(RuntimeError):
                runner.storage_history_receipts(encode(receipts), durable)

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
        summary = f"test result: ok. {len(runner.REQUIRED_CASES)} passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.00s"
        output = "\n".join(lines + [summary])
        self.assertEqual(runner.require_lifecycle_results(output), sorted(runner.REQUIRED_CASES))
        source_case = "plans::installed_source_read_choice_is_explicit_and_reopens"
        duplicate_source_result = "\n".join(
            lines + [f"test {source_case} ... ok", summary]
        )
        with self.assertRaisesRegex(RuntimeError, "exactly once"):
            runner.require_lifecycle_results(duplicate_source_result)
        missing_source_result = "\n".join(
            [line for line in lines if line != f"test {source_case} ... ok"] + [summary]
        )
        with self.assertRaisesRegex(RuntimeError, "exactly once"):
            runner.require_lifecycle_results(missing_source_result)
        for invalid in ["", "\n".join(lines), "\n".join(lines[1:] + [summary]),
                        output + "\n" + lines[0], output.replace("0 ignored", "1 ignored"),
                        output.replace("0 filtered out", "1 filtered out"),
                        output.replace(lines[0], lines[0].replace("ok", "FAILED")),
                        output + "\n" + summary]:
            with self.subTest(output=invalid), self.assertRaises(RuntimeError):
                runner.require_lifecycle_results(invalid)

    def test_preflight_checks_compiled_harness_listing_without_relaxing_set(self):
        listing = "\n".join(f"{case}: test" for case in sorted(runner.REQUIRED_CASES))
        with tempfile.TemporaryDirectory() as directory:
            harness = Path(directory) / "harness"
            harness.write_bytes(b"bounded fixture")
            completed = subprocess.CompletedProcess([], 0, listing, "")
            with patch.object(runner, "run", return_value=completed) as invoke:
                receipt = runner.preflight_lifecycle_harness(harness)
            self.assertEqual(receipt, {"harness_sha256": runner.sha256(harness),
                                       "required_cases": len(runner.REQUIRED_CASES)})
            invoke.assert_called_once_with(str(harness.resolve()), "--list", "--format=terse",
                                           timeout=30, output_limit=1024 * 1024)
            for bad in [listing + "\nunknown: test", "\n".join(listing.splitlines()[1:]),
                        listing + "\n" + listing.splitlines()[0]]:
                with self.subTest(listing=bad), patch.object(
                    runner, "run", return_value=subprocess.CompletedProcess([], 0, bad, "")
                ), self.assertRaisesRegex(RuntimeError, "exactly the required lifecycle cases"):
                    runner.preflight_lifecycle_harness(harness)

    def test_child_deadline_and_output_limit_are_enforced(self):
        start = time.monotonic()
        observed = []
        with self.assertRaisesRegex(RuntimeError, "deadline"):
            runner.run(sys.executable, "-c", "import time; print('before timeout', flush=True); time.sleep(30)",
                       timeout=0.2, on_completion=observed.append)
        self.assertEqual(len(observed), 1)
        self.assertNotEqual(observed[0].returncode, 0)
        self.assertLess(time.monotonic() - start, 5)
        with self.assertRaisesRegex(RuntimeError, "output"):
            runner.run(sys.executable, "-c", "import sys; sys.stdout.write('x' * 1000000)",
                       output_limit=1024, on_completion=observed.append)
        self.assertEqual(len(observed), 2)
        self.assertLessEqual(len(observed[-1].stdout), 1024)
        with self.assertRaisesRegex(RuntimeError, "exit status"):
            runner.run(sys.executable, "-c", "raise SystemExit(3)")
        output = runner.run(sys.executable, "-c", "import sys; print('out'); print('err', file=sys.stderr)")
        self.assertEqual(output.stdout.strip(), "out")
        self.assertEqual(output.stderr.strip(), "err")

    def test_partial_progress_cannot_retain_arbitrary_output_or_credentials(self):
        case = sorted(runner.REQUIRED_CASES)[0]
        result = runner.subprocess.CompletedProcess([], 1,
            f'test {case} ... ok\ntest unknown_secret_case ... FAILED\n'
            'native_lifecycle_receipt={"mode":"fresh","credential":"never retain me"}\n'
            'native_storage_history={"mode":"not a fixture","private":"never retain me"}\n'
            'native_storage_history={invalid}\nprivate output never retain me\n',
            'native readiness output stdout_bytes=123 stderr_bytes=0\nsecret never retain me\n')
        progress = runner.partial_lifecycle_progress(result)
        self.assertEqual(progress["cases"], [{"case": case, "status": "ok", "occurrences": 1}])
        self.assertEqual(progress["markers"]["native_lifecycle_receipt"], {"fresh": 1, "existing": 0})
        self.assertEqual(progress["markers"]["native_storage_history"], {"fresh": 0, "existing": 0})
        self.assertEqual(progress["readiness_observations"], 1)
        self.assertNotIn("never retain me", json.dumps(progress))
        self.assertNotIn("unknown_secret_case", json.dumps(progress))

    def test_every_primary_lifecycle_case_is_required(self):
        cases = set(LIFECYCLE_CASES)
        runner.require_lifecycle_tests("\n".join(f"{case}: test" for case in sorted(cases)))
        for missing in sorted(cases):
            with self.subTest(missing=missing), self.assertRaises(RuntimeError):
                runner.require_lifecycle_tests(
                    "\n".join(f"{case}: test" for case in sorted(cases - {missing}))
                )
        source_case = "plans::installed_source_read_choice_is_explicit_and_reopens"
        duplicate_listing = "\n".join(
            [f"{case}: test" for case in sorted(cases)] + [f"{source_case}: test"]
        )
        with self.assertRaisesRegex(RuntimeError, "exactly the required lifecycle cases"):
            runner.require_lifecycle_tests(duplicate_listing)

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
