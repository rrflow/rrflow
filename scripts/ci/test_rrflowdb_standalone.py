"""Regression for the standalone rrflowDB process and independent result oracle."""
import copy
import sys
import unittest

import run_rrflowdb_standalone as runner
from check_workflow import PolicyError, REUSABLE, job_blocks, verify_standalone_rrflowdb


def view(status="done"):
    value = lambda text: {"type": "string", "value": text}
    common = {"scope": "instance:one", "known_at_cursor": 7, "schema_revision": 2,
              "execution": {"truncated": False, "returned_rows": 1}}
    return {
        "verified_expected_status": status,
        "tasks": {**copy.deepcopy(common), "rows": [{"values": {
            "id": value("first-task"), "title": value("Build with rrflowDB"), "status": value(status)}}]},
        "relationships": {**copy.deepcopy(common), "rows": [{"values": {
            "left.name": value("Local AI project"), "right.to_id": value("first-task")}}]},
        "documents": {**copy.deepcopy(common), "rows": [{"values": {
            "id": value("context-note"), "title": value("Project context"),
            "metadata": {"type": "map", "value": {
                "source": value("local"), "revision": {"type": "unsigned", "value": 1}}}}}]},
        "document_relationships": {**copy.deepcopy(common), "rows": [{"values": {
            "left.id": value("context-note"), "right.to_id": value("first-task"),
            "right.reason": value("supports")}}]},
    }


class ResultOracle(unittest.TestCase):
    def test_real_shape_and_independent_expected_values(self):
        runner.check_view(view(), "done")
        bad = view()
        bad["relationships"]["rows"][0]["values"]["right.to_id"]["value"] = "other"
        with self.assertRaisesRegex(ValueError, "relationship"):
            runner.check_view(bad, "done")
        bad = view()
        bad["documents"]["rows"][0]["values"]["metadata"]["value"]["revision"]["value"] = 2
        with self.assertRaisesRegex(ValueError, "document"):
            runner.check_view(bad, "done")
        bad = view()
        bad["document_relationships"]["rows"][0]["values"]["right.reason"]["value"] = "other"
        with self.assertRaisesRegex(ValueError, "document relationship"):
            runner.check_view(bad, "done")
        with self.assertRaisesRegex(ValueError, "task"):
            runner.check_view(view("open"), "done")
        with self.assertRaisesRegex(ValueError, "served instance"):
            runner.check_view(view(), "done", "another")

    def test_incomplete_mixed_and_missing_views_fail(self):
        for mutate in (
            lambda x: x["tasks"]["execution"].update(truncated=True),
            lambda x: x["relationships"].update(known_at_cursor=8),
            lambda x: x["relationships"].update(scope="instance:other"),
            lambda x: x["tasks"].update(rows=[]),
            lambda x: x.pop("relationships"),
            lambda x: x["documents"].update(known_at_cursor=8),
            lambda x: x["document_relationships"]["execution"].update(truncated=True),
            lambda x: x.pop("documents"),
            lambda x: x.pop("document_relationships"),
        ):
            bad = view()
            mutate(bad)
            with self.subTest(mutate=mutate), self.assertRaises((ValueError, KeyError)):
                runner.check_view(bad, "done")


class OwnedCommand(unittest.TestCase):
    def test_failure_and_timeout_are_not_success_or_raw_output(self):
        for code, timeout, expected in (("import sys;print('secret');sys.exit(3)", 5, "failed"),
                                        ("import time;time.sleep(5)", 0.1, "deadline")):
            with self.subTest(expected=expected):
                with self.assertRaisesRegex(RuntimeError, expected) as error:
                    runner.command([sys.executable, "-c", code], "controlled", timeout=timeout)
                self.assertNotIn("secret", str(error.exception))

    def test_output_limit_is_enforced_while_child_is_running(self):
        with self.assertRaisesRegex(RuntimeError, "output limit"):
            runner.command([sys.executable, "-c", "import sys;sys.stdout.write('x'*100000);sys.stdout.flush()"],
                           "oversized", timeout=5, limit=1024)

    def test_negative_oracle_rejects_an_unrelated_failure(self):
        args = [sys.executable, "-c", "import sys;print('other',file=sys.stderr);sys.exit(1)"]
        with self.assertRaisesRegex(RuntimeError, "unexpected reason"):
            runner.command(args, "negative", expect_failure=True, expected_error=b"expected")


class WorkflowPolicy(unittest.TestCase):
    def test_windows_uses_native_python_command(self):
        jobs = job_blocks(REUSABLE.read_text(encoding="utf-8"))
        expected = 'if [ "$RUNNER_OS" = Windows ]; then suffix=".exe"; python_command=python; fi'
        self.assertIn(expected, jobs["engram-portability"])
        altered = dict(jobs)
        altered["engram-portability"] = altered["engram-portability"].replace(
            "python_command=python; fi", "python_command=python3; fi", 1)
        with self.assertRaises(PolicyError):
            verify_standalone_rrflowdb(altered)

    def test_both_native_jobs_and_unit_oracle_are_required(self):
        jobs = job_blocks(REUSABLE.read_text(encoding="utf-8"))
        verify_standalone_rrflowdb(jobs)
        for job, marker in (("topology-smoke", "scripts/ci/run_rrflowdb_standalone.py"),
                            ("engram-portability", "scripts/ci/run_rrflowdb_standalone.py"),
                            ("verify", "scripts/ci/test_rrflowdb_standalone.py")):
            altered = dict(jobs)
            altered[job] = altered[job].replace(marker, "missing", 1)
            with self.subTest(job=job), self.assertRaises(PolicyError):
                verify_standalone_rrflowdb(altered)
        altered = dict(jobs)
        altered["engram-portability"] = altered["engram-portability"].replace(
            "      - name: Build and exercise the standalone rrflowDB example\n",
            "      - name: Build and exercise the standalone rrflowDB example\n        if: false\n", 1)
        with self.assertRaises(PolicyError):
            verify_standalone_rrflowdb(altered)


if __name__ == "__main__":
    unittest.main()
