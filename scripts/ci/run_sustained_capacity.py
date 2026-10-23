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
import re
from pathlib import Path
import secrets
import sys
import time

from run_offline_lifecycle import require, run, sha256

ROOT = Path(__file__).resolve().parents[2]
CASE = "sustained::installed_sustained_capacity"
MAX_DOCUMENT = 8 * 1024 * 1024
MAX_LAUNCH_RECEIPT = 8192


class NotRun(RuntimeError):
    pass


def bounded_json(path):
    require(path.is_file() and not path.is_symlink(), "campaign must be a regular file")
    with path.open("rb") as source:
        raw = source.read(MAX_DOCUMENT + 1)
    require(len(raw) <= MAX_DOCUMENT, "campaign exceeds evidence document bound")
    return json.loads(raw)


def proof(kind, allocation, harness, nonce=None):
    record = {"format_version": 1, "kind": kind, "nonce": nonce or secrets.token_hex(32),
              "engine_bytes": allocation["engine_bytes"], "host_bytes": allocation["host_bytes"],
              "harness_sha256": sha256(harness), "vcpus": allocation["vcpus"],
              "host_logical_cpus": os.cpu_count()}
    require(isinstance(record["host_logical_cpus"], int) and 0 < record["vcpus"] <= record["host_logical_cpus"] <= 65536,
            "host CPU capacity cannot satisfy frozen allocation")
    digest = hashlib.sha256(b"rrflow-native-allocation-v1\0" + json.dumps(record, separators=(",", ":")).encode()).hexdigest()
    return record, {"kind": kind, "receipt_sha256": digest}


@contextmanager
def linux_allocation(parent, allocation):
    root = Path("/sys/fs/cgroup").resolve(strict=True)
    parent = parent.resolve(strict=True)
    require(parent != root and parent.is_relative_to(root), "explicit delegated cgroup below root required")
    owned = parent / ("rrflow-capacity-" + secrets.token_hex(32))
    try:
        owned.mkdir()
    except PermissionError as error:
        raise NotRun("delegated cgroup creation unavailable") from error
    try:
        # Aggregate bound includes the harness and its installed engine children;
        # it is stricter than an engine-only resident allocation.
        for name, value in [("memory.max", allocation["engine_bytes"]), ("memory.swap.max", 0),
                            ("memory.oom.group", 1), ("pids.max", 512),
                            ("cpu.max", f'{allocation["vcpus"] * 100000} 100000')]:
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



@contextmanager
def retain_launch(evidence, campaign, identity):
    """Seal the outer outcome only after owned process/control cleanup returns.

    The inner writer still creates the evidence directory. If it never starts,
    preserve only the frozen campaign and explicit launch failure, not a result.
    """
    started = time.monotonic()
    state = {"format_version": 1, "campaign_sha256": identity, "status": "failed",
             "exit_code": None, "cleanup_confirmed": False, "allocation": None,
             "elapsed_ms": 0, "reason": "launch-not-completed"}
    try:
        yield state
    except NotRun:
        state.update(status="not_run", reason="native-facility-unavailable")
        raise
    except KeyboardInterrupt:
        state.update(status="interrupted", reason="launcher-interrupted")
        raise
    except BaseException:
        state.update(status="failed", reason="launch-failed" if state["cleanup_confirmed"] else "native-cleanup-unconfirmed")
        raise
    finally:
        state["elapsed_ms"] = int((time.monotonic() - started) * 1000)
        raw = (json.dumps(state, separators=(",", ":")) + "\n").encode()
        require(len(raw) <= MAX_LAUNCH_RECEIPT, "launch receipt exceeds reservation")
        if not evidence.exists():
            evidence.mkdir()
        require(evidence.is_dir() and not evidence.is_symlink(), "launch evidence directory changed")
        frozen = evidence / "campaign.json"
        if not frozen.exists():
            with frozen.open("xb") as output:
                output.write((json.dumps(campaign, separators=(",", ":")) + "\n").encode())
                output.flush()
                os.fsync(output.fileno())
        with (evidence / "launch.json").open("xb") as output:
            output.write(raw)
            output.flush()
            os.fsync(output.fileno())


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
    validated = run("cargo", "run", "--quiet", "--locked", "-p", "rrflow-eval", "--", "storage", "campaign", "validate", str(args.campaign.resolve()), timeout=1800)
    identity = re.fullmatch(r"valid storage campaign: ([0-9a-f]{64}) purpose=(?:Characterization|Qualification); no workload executed\s*", validated.stdout)
    require(identity is not None, "canonical campaign identity absent")
    require(not args.evidence.exists() and args.evidence.parent.is_dir(), "evidence directory must be new with an existing parent")
    with retain_launch(args.evidence, campaign, identity[1]) as state:
        candidate = campaign["storage"]["candidate"]
        require(run("git", "rev-parse", "HEAD").stdout.strip() == candidate["commit"]
                and run("git", "rev-parse", "HEAD^{tree}").stdout.strip() == candidate["tree"]
                and not run("git", "status", "--porcelain").stdout, "campaign source is not the frozen clean candidate")
        output = run("cargo", "test", "--release", "--locked", "-p", "rrflow-cli", "--test", "installed_reopen", "--no-run", "--message-format=json", timeout=3600)
        messages = [json.loads(line) for line in output.stdout.splitlines()]
        paths = {Path(row["executable"]).resolve(strict=True) for row in messages
                 if row.get("reason") == "compiler-artifact" and row.get("executable")
                 and row["target"]["name"] == "installed_reopen" and row["profile"]["test"]
                 and not row["profile"]["debug_assertions"] and row["profile"]["opt_level"] in {"2", "3", "s", "z"}}
        require(len(paths) == 1, "Cargo must select one ordinary installed harness")
        harness = paths.pop()
        require(f"{CASE}: test" in run(str(harness), "--list").stdout.splitlines(), "installed sustained entry missing")
        schedule = campaign["schedule"]
        duration = sum(schedule[name] for name in ["setup_ms", "lifecycle_ms", "warmup_ms", "measured_ms", "burst_ms", "drain_ms", "soak_ms"])
        # Lifecycle admission is per lane: pre-load forced reopens and final readback.
        duration += schedule["lifecycle_ms"]
        if "cold" in campaign["required_lanes"]:
            duration += schedule["lifecycle_ms"]
        timeout = (duration // 1000 + 600) * len(campaign["points"]) * len(campaign["seeds"])
        if campaign["allocation"]["estates"] > 1:
            timeout *= 3  # quiet, equal and abusive trials, each with full durations
            timeout += ((campaign["allocation"]["estates"] - 1) *
                        (schedule["measured_ms"] + 120_000) // 1000) * len(campaign["points"]) * len(campaign["seeds"])
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
                receipt, binding = proof("cgroup_v2", allocation, harness, group.name.removeprefix("rrflow-capacity-"))
                environment["RRFLOW_SUSTAINED_CGROUP"] = str(group)
                environment["RRFLOW_SUSTAINED_ALLOCATION_RECEIPT"] = json.dumps(receipt, separators=(",", ":"))
                print("native_sustained_enforcement=" + json.dumps(receipt), flush=True)
                state["allocation"] = receipt
                result = run(sys.executable, str(Path(__file__).resolve()), "--enter-cgroup", str(group), *command,
                             env=environment, timeout=timeout, process_group=True, check=False)
            state["cleanup_confirmed"] = True
        elif sys.platform == "win32":
            from owned_windows_process import run_owned
            receipt, binding = proof("windows_job", allocation, harness)
            environment["RRFLOW_SUSTAINED_ALLOCATION_RECEIPT"] = json.dumps(receipt, separators=(",", ":"))
            state["allocation"] = receipt
            result = run_owned(command, env=environment, timeout=timeout,
                               job_name="Local\\RRFlowCapacity-" + receipt["nonce"],
                               cpu_rate=10000 * receipt["vcpus"] // receipt["host_logical_cpus"],
                               process_memory_bytes=allocation["engine_bytes"], job_memory_bytes=allocation["engine_bytes"],
                               on_bound=lambda: print("native_sustained_enforcement=" + json.dumps(receipt), flush=True))
            state["cleanup_confirmed"] = True
        else:
            raise NotRun("this platform needs an independently verified native VM memory allocation")
        state["exit_code"] = result.returncode
        print(result.stdout, end="")
        print(result.stderr, end="", file=sys.stderr)
        require(result.returncode == 0 and f"test {CASE} ... ok" in result.stdout, "installed campaign failed; retained artifacts required")
        run("cargo", "run", "--quiet", "--locked", "-p", "rrflow-eval", "--", "storage", "campaign", "verify", str(args.evidence.resolve()), timeout=1800)
        state.update(status="completed", reason=None)



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
