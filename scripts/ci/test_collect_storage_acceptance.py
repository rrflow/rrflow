#!/usr/bin/env python3
"""Pure collector admission tests; not native engine or CI qualification."""
import copy
import json
import unittest
from collect_storage_acceptance import REPOSITORY, collect, tested_source
from check_workflow import REUSABLE, PolicyError, verify_workflow_security


def fixture():
    revision = "2" * 40
    run = {"id": 7, "status": "completed", "conclusion": "success", "path": ".github/workflows/ci.yml",
           "repository": {"full_name": REPOSITORY}, "event": "pull_request", "head_sha": "1" * 40, "run_attempt": 2,
           "referenced_workflows": [{"sha": revision, "path": f"{REPOSITORY}/.github/workflows/ci-reusable.yml@{revision}"}]}
    jobs = [{"id": 100 + i, "run_id": 7, "run_attempt": 2, "head_sha": "1" * 40} for i in range(101)]
    return run, jobs, {"sha": revision, "tree": {"sha": "3" * 40}}


class CollectorTests(unittest.TestCase):
    def test_candidate_checkout_cannot_substitute_source(self):
        base = "permissions:\n  contents: read\nsteps:\n  - uses: actions/checkout@abc\n    with:\n      persist-credentials: false\n      submodules: recursive\n"
        verify_workflow_security({REUSABLE: base})
        for key in ("ref", "repository", "path"):
            with self.assertRaises(PolicyError):
                verify_workflow_security({REUSABLE: base + f"      {key}: elsewhere\n"})

    def test_pr_head_is_not_substituted_for_tested_merge(self):
        run, _, _ = fixture()
        self.assertEqual(tested_source(run, 7), ("2" * 40, 2))

    def test_closed_bounded_pages_preserve_all_jobs(self):
        run, jobs, commit = fixture()
        def request(endpoint):
            value = run if endpoint == "actions/runs/7" else commit if endpoint.startswith("git/commits/") else {
                "total_count": len(jobs), "jobs": jobs[:100] if endpoint.endswith("page=1") else jobs[100:]}
            return value, json.dumps(value).encode()
        records = collect(7, request)
        self.assertEqual(len(json.loads(records["jobs.json"])), 101)
        self.assertEqual(json.loads(records["run.json"])["head_sha"], "1" * 40)

    def test_incomplete_foreign_and_ambiguous_runs_fail(self):
        run, _, _ = fixture()
        for key, value in [("status", "in_progress"), ("conclusion", "failure"), ("event", "push"),
                           ("repository", {"full_name": "rrflow/rrflow"}), ("referenced_workflows", [])]:
            changed = copy.deepcopy(run)
            changed[key] = value
            with self.assertRaises(RuntimeError):
                tested_source(changed, 7)

    def test_duplicate_jobs_and_changed_attempt_fail(self):
        run, jobs, commit = fixture()
        for duplicate in (False, True):
            changed = copy.deepcopy(jobs)
            if duplicate:
                changed[-1] = changed[0]
            else:
                changed[-1]["run_attempt"] = 1
            def request(endpoint):
                value = run if endpoint == "actions/runs/7" else commit if endpoint.startswith("git/commits/") else {
                    "total_count": len(changed), "jobs": changed[:100] if endpoint.endswith("page=1") else changed[100:]}
                return value, json.dumps(value).encode()
            with self.assertRaises(RuntimeError):
                collect(7, request)


if __name__ == "__main__":
    unittest.main()
