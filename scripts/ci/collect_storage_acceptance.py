#!/usr/bin/env python3
"""Collect exact GitHub candidate provenance; no engine or capacity workload.

Raw bounded API records are retained before the canonical Rust projection is
validated. Historical files are not authenticated server state on later reads.
"""
from __future__ import annotations
import argparse
import json
import os
from pathlib import Path
import re

from run_offline_lifecycle import require, run

ROOT = Path(__file__).resolve().parents[2]
REPOSITORY = "rrflow/rrflow-development"
MAX_BYTES = 8 * 1024 * 1024


def api(endpoint):
    result = run("gh", "api", "repos/" + REPOSITORY + "/" + endpoint,
                 timeout=60, output_limit=MAX_BYTES)
    raw = result.stdout.encode("utf-8")
    require(len(raw) <= MAX_BYTES, "GitHub API record exceeds bound")
    return json.loads(raw), raw


def tested_source(record, run_id):
    require(record.get("id") == run_id and record.get("status") == "completed"
            and record.get("conclusion") == "success", "candidate run is not complete and successful")
    require(record.get("repository", {}).get("full_name") == REPOSITORY
            and record.get("path") == ".github/workflows/ci.yml", "candidate pipeline/repository differs")
    require(record.get("event") in {"pull_request", "merge_group", "workflow_dispatch"}, "not a candidate event")
    workflows = record.get("referenced_workflows")
    require(isinstance(workflows, list) and len(workflows) == 1, "ambiguous tested reusable-workflow source")
    revision = workflows[0].get("sha", "")
    require(re.fullmatch(r"[0-9a-f]{40}", revision) is not None
            and workflows[0].get("path") == f"{REPOSITORY}/.github/workflows/ci-reusable.yml@{revision}",
            "tested source is not the owning reusable workflow")
    require(type(record.get("run_attempt")) is int and 0 < record["run_attempt"] <= 9007199254740991,
            "invalid run attempt")
    return revision, record["run_attempt"]


def collect(run_id, request=api):
    record, run_raw = request(f"actions/runs/{run_id}")
    revision, attempt = tested_source(record, run_id)
    jobs = []
    expected = None
    for page in range(1, 3):
        response, _ = request(f"actions/runs/{run_id}/attempts/{attempt}/jobs?per_page=100&page={page}")
        count = response.get("total_count")
        require(type(count) is int and 0 < count <= 128, "candidate job count exceeds bound")
        require(expected is None or expected == count, "job inventory changed during collection")
        expected = count
        rows = response.get("jobs")
        require(isinstance(rows, list) and 0 < len(rows) <= 100, "candidate jobs page is empty or oversized")
        jobs.extend(rows)
        require(len(jobs) <= expected, "candidate job inventory exceeds declared count")
        if len(jobs) == expected:
            break
    require(len(jobs) == expected and len({row.get("id") for row in jobs}) == expected,
            "candidate jobs omitted or duplicated")
    for row in jobs:
        require(row.get("run_id") == run_id and row.get("run_attempt") == attempt
                and row.get("head_sha") == record.get("head_sha"), "job source/attempt changed")
    commit, commit_raw = request(f"git/commits/{revision}")
    require(commit.get("sha") == revision, "commit API source differs")
    final, _ = request(f"actions/runs/{run_id}")
    require(tested_source(final, run_id) == (revision, attempt)
            and final.get("head_sha") == record.get("head_sha"), "candidate reran or changed during collection")
    jobs_raw = (json.dumps(jobs, separators=(",", ":"), ensure_ascii=False) + "\n").encode("utf-8")
    require(len(jobs_raw) <= MAX_BYTES, "combined CI job inventory exceeds bound")
    return {"run.json": run_raw, "jobs.json": jobs_raw, "commit.json": commit_raw}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--run", type=int, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    require(0 < args.run <= 9007199254740991, "run ID outside exact integer domain")
    require(not args.output.exists() and args.output.parent.is_dir(), "new output directory and existing parent required")
    records = collect(args.run)
    args.output.mkdir()
    for name, raw in records.items():
        with (args.output / name).open("xb") as output:
            output.write(raw)
            output.flush()
            os.fsync(output.fileno())
    # The executable evidence owner checks raw API provenance and mandatory
    # native steps; this collector does not duplicate those acceptance rules.
    result = run("cargo", "run", "--quiet", "--locked", "-p", "rrflow-eval", "--",
                 "storage", "acceptance", "ci-receipt", str(args.output.resolve()),
                 cwd=ROOT, timeout=1800, output_limit=MAX_BYTES)
    print(result.stdout, end="")


if __name__ == "__main__":
    main()
