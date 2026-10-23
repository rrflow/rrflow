#!/usr/bin/env python3
"""Explicit installed sustained campaign with owned native process ceilings.

This is qualification tooling, never installed product policy. Missing native
facilities produce not_run. It never changes an existing cgroup or a host cap.
"""
from __future__ import annotations
import argparse
from contextlib import contextmanager
import hashlib
import json
import os
from pathlib import Path
import secrets
import sys
import time

from run_offline_lifecycle import require, run, sha256

ROOT = Path(__file__).resolve().parents[2]
CASE = "sustained::installed_sustained_capacity"
MAX_DOCUMENT = 8 * 1024 * 1024


class NotRun(RuntimeError):
    pass


def bounded_json(path):
    require(path.is_file() and not path.is_symlink(), "campaign must be a regular file")
    with path.open("rb") as source:
        raw = source.read(MAX_DOCUMENT + 1)
    require(len(raw) <= MAX_DOCUMENT, "campaign exceeds evidence document bound")
    return json.loads(raw)


def proof(kind, allocation, harness):
    record = {"format_version": 1, "kind": kind, "nonce": secrets.token_hex(32),
              "engine_bytes": allocation["engine_bytes"], "host_bytes": allocation["host_bytes"],
              "harness_sha256": sha256(harness)}
    digest = hashlib.sha256(json.dumps(record, sort_keys=True, separators=(",", ":")).encode()).hexdigest()
    return record, {"kind": kind, "receipt_sha256": digest}


@contextmanager
def linux_allocation(parent, allocation):
    root = Path("/sys/fs/cgroup").resolve(strict=True)
    parent = parent.resolve(strict=True)
    require(parent != root and parent.is_relative_to(root), "explicit delegated cgroup below root required")
    owned = parent / ("rrflow-capacity-" + secrets.token_hex(16))
    try:
        owned.mkdir()
    except PermissionError as error:
        raise NotRun("delegated cgroup creation unavailable") from error
    try:
        # Aggregate bound includes the harness and its installed engine children;
        # it is stricter than an engine-only resident allocation.
        for name, value in [("memory.max", allocation["engine_bytes"]), ("memory.swap.max", 0),
                            ("memory.oom.group", 1), ("pids.max", 512)]:
            target = owned / name
            if not target.is_file():
                raise NotRun("delegated native controller missing: " + name)
            target.write_text(str(value), encoding="ascii")
            require(target.read_text(encoding="ascii").strip() == str(value), "native cgroup limit did not bind")
        require((owned / "cgroup.kill").is_file(), "native owned-tree termination unavailable")
        yield owned
    finally:
        # Only the directory created above can be killed or removed.
        require(owned.parent == parent and owned.name.startswith("rrflow-capacity-")
                and owned.resolve(strict=True) == owned, "owned cgroup identity changed")
        if (owned / "cgroup.procs").read_text(encoding="ascii").strip():
            (owned / "cgroup.kill").write_text("1", encoding="ascii")
        deadline = time.monotonic() + 10
        while "populated 1" in (owned / "cgroup.events").read_text(encoding="ascii"):
            require(time.monotonic() < deadline, "owned cgroup remained populated")
            time.sleep(0.05)
        owned.rmdir()


def enter_cgroup(group, arguments):
    # Called only in the owned launch child. Membership is established before
    # the harness can run or spawn the engine; the parent remains outside.
    root = Path("/sys/fs/cgroup").resolve(strict=True)
    group = group.resolve(strict=True)
    require(group.is_relative_to(root) and group.name.startswith("rrflow-capacity-"), "invalid owned cgroup")
    expected = os.environ["RRFLOW_SUSTAINED_CGROUP"]
    require(str(group) == expected, "child cgroup differs from launch binding")
    (group / "cgroup.procs").write_text(str(os.getpid()), encoding="ascii")
    require(str(os.getpid()) in (group / "cgroup.procs").read_text(encoding="ascii").split(), "child membership absent")
    os.execve(arguments[0], arguments, os.environ)


def main():
    if len(sys.argv) > 3 and sys.argv[1] == "--enter-cgroup":
        enter_cgroup(Path(sys.argv[2]), sys.argv[3:])
        return
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--campaign", required=True, type=Path)
    parser.add_argument("--configuration", required=True, type=Path)
    parser.add_argument("--evidence", required=True, type=Path)
    parser.add_argument("--cgroup-parent", type=Path)
    args = parser.parse_args()
    campaign = bounded_json(args.campaign)
    run("cargo", "run", "--quiet", "--locked", "-p", "rrflow-eval", "--", "storage", "campaign", "validate", str(args.campaign.resolve()), timeout=1800)
    require(not args.evidence.exists() and args.evidence.parent.is_dir(), "evidence directory must be new with an existing parent")
    candidate = campaign["storage"]["candidate"]
    require(run("git", "rev-parse", "HEAD").stdout.strip() == candidate["commit"]
            and run("git", "rev-parse", "HEAD^{tree}").stdout.strip() == candidate["tree"]
            and not run("git", "status", "--porcelain").stdout, "campaign source is not the frozen clean candidate")
    output = run("cargo", "test", "--locked", "-p", "rrflow-cli", "--test", "installed_reopen", "--no-run", "--message-format=json", timeout=3600)
    messages = [json.loads(line) for line in output.stdout.splitlines()]
    paths = {Path(row["executable"]).resolve(strict=True) for row in messages
             if row.get("reason") == "compiler-artifact" and row.get("executable")
             and row["target"]["name"] == "installed_reopen" and row["profile"]["test"]}
    require(len(paths) == 1, "Cargo must select one ordinary installed harness")
    harness = paths.pop()
    require(f"{CASE}: test" in run(str(harness), "--list").stdout.splitlines(), "installed sustained entry missing")
    schedule = campaign["schedule"]
    duration = sum(schedule[name] for name in ["warmup_ms", "measured_ms", "burst_ms", "drain_ms", "soak_ms"])
    timeout = (duration // 1000 + 600) * len(campaign["points"]) * len(campaign["seeds"])
    require(0 < timeout <= 31 * 86400, "campaign exceeds explicit one-month run bound")
    allocation = campaign["allocation"]
    environment = dict(os.environ, RRFLOW_SUSTAINED_CAMPAIGN=str(args.campaign.resolve()),
                       RRFLOW_SUSTAINED_CONFIGURATION=str(args.configuration.resolve(strict=True)),
                       RRFLOW_SUSTAINED_EVIDENCE=str(args.evidence.resolve()))
    command = [str(harness), CASE, "--exact", "--ignored", "--nocapture"]
    if sys.platform == "linux":
        if args.cgroup_parent is None:
            raise NotRun("explicit delegated --cgroup-parent required for native ceilings")
        with linux_allocation(args.cgroup_parent, allocation) as group:
            receipt, binding = proof("cgroup_v2", allocation, harness)
            environment["RRFLOW_SUSTAINED_CGROUP"] = str(group)
            environment["RRFLOW_SUSTAINED_ENFORCEMENT"] = json.dumps(binding)
            print("native_sustained_enforcement=" + json.dumps(receipt), flush=True)
            result = run(sys.executable, str(Path(__file__).resolve()), "--enter-cgroup", str(group), *command,
                         env=environment, timeout=timeout, process_group=True, check=False)
    elif sys.platform == "win32":
        from owned_windows_process import run_owned
        receipt, binding = proof("windows_job", allocation, harness)
        environment["RRFLOW_SUSTAINED_ENFORCEMENT"] = json.dumps(binding)
        result = run_owned(command, env=environment, timeout=timeout,
                           process_memory_bytes=allocation["engine_bytes"], job_memory_bytes=allocation["host_bytes"],
                           on_bound=lambda: print("native_sustained_enforcement=" + json.dumps(receipt), flush=True))
    else:
        raise NotRun("this platform needs an independently verified native VM memory allocation")
    print(result.stdout, end="")
    print(result.stderr, end="", file=sys.stderr)
    require(result.returncode == 0 and f"test {CASE} ... ok" in result.stdout, "installed campaign failed; retained artifacts required")
    run("cargo", "run", "--quiet", "--locked", "-p", "rrflow-eval", "--", "storage", "campaign", "verify", str(args.evidence.resolve()), timeout=1800)


if __name__ == "__main__":
    try:
        os.chdir(ROOT)
        main()
    except NotRun as error:
        print(json.dumps({"native_sustained_status": "not_run", "reason": str(error)}), file=sys.stderr)
        sys.exit(2)
    except Exception as error:
        print(json.dumps({"native_sustained_status": "failed", "reason": str(error)}), file=sys.stderr)
        sys.exit(1)
