#!/usr/bin/env python3
"""Exercise native sampler containment in an explicitly delegated cgroup.

This is a synthetic qualification-tool regression, not the frozen capacity
profile. The caller supplies a delegated parent; only a newly created child
of that parent is limited, killed and removed by the existing launcher.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import select
import sys
import tempfile
import threading
import time

from run_offline_lifecycle import require, run
from selected_tests import NATIVE_SAMPLE_MEMBERSHIP, NATIVE_SAMPLE_TIMEOUT
from run_sustained_capacity import linux_allocation, proof

ROOT = Path(__file__).resolve().parents[2]
CASE = NATIVE_SAMPLE_MEMBERSHIP
TIMEOUT = NATIVE_SAMPLE_TIMEOUT


def artifact(profile: str, offline: bool) -> Path:
    command = ["cargo", "test", "--locked", "-p", "rrflow-cli", "--test", "installed_reopen",
               "--no-run", "--message-format=json"]
    if profile == "release":
        command.append("--release")
    if offline:
        command.append("--offline")
    output = run(*command, cwd=ROOT, timeout=1800)
    messages = (json.loads(line) for line in output.stdout.splitlines())
    paths = {Path(row["executable"]).resolve(strict=True) for row in messages
             if row.get("reason") == "compiler-artifact" and row.get("executable")
             and row["target"]["name"] == "installed_reopen" and row["profile"]["test"]
             and (profile != "release" or (not row["profile"]["debug_assertions"]
                                             and row["profile"]["opt_level"] in {"2", "3", "s", "z"}))}
    require(len(paths) == 1, "Cargo did not identify one installed-reopen harness")
    harness = paths.pop()
    listing = run(str(harness), "--list", timeout=60).stdout.splitlines()
    require(all(listing.count(f"{case}: test") == 1 for case in (CASE, TIMEOUT)),
            "native allocation test entry missing")
    return harness


def exited(pidfd: int) -> bool:
    poll = select.poll()
    poll.register(pidfd, select.POLLIN)
    return bool(poll.poll(0))


def timeout_cleanup(parent: Path, limits: dict, harness: Path) -> dict:
    require(hasattr(os, "pidfd_open"), "native pidfd identity is unavailable")
    base = Path(tempfile.gettempdir()).resolve(strict=True)
    fixture = Path(tempfile.mkdtemp(prefix="rrflow-native-cgroup-", dir=base)).resolve(strict=True)
    require(fixture.parent == base and fixture.name.startswith("rrflow-native-cgroup-"),
            "native cgroup fixture escaped its parent")
    state: dict = {}
    success = False
    try:
        with linux_allocation(parent, limits) as group:
            controller = Path("/sys/fs/cgroup") / Path("/proc/self/cgroup").read_text().strip().split(":", 2)[2].lstrip("/")
            require(not controller.resolve(strict=True).is_relative_to(group),
                    "controller belongs to the owned test cgroup")
            receipt, _ = proof("cgroup_v2", limits, harness, group.name.removeprefix("rrflow-capacity-"))
            env = dict(os.environ, RRFLOW_SUSTAINED_CGROUP=str(group),
                       RRFLOW_NATIVE_TIMEOUT_FIXTURE=str(fixture),
                       RRFLOW_SUSTAINED_ALLOCATION_RECEIPT=json.dumps(receipt, separators=(",", ":")))
            membership = "0::/" + group.relative_to("/sys/fs/cgroup").as_posix()

            def observe() -> None:
                deadline = time.monotonic() + 15
                while time.monotonic() < deadline:
                    opened = []
                    try:
                        ids = [int((fixture / name).read_text(encoding="ascii").strip())
                               for name in ("parent.pid", "worker.pid")]
                        require(all(pid > 0 and pid != os.getpid() for pid in ids) and ids[0] != ids[1],
                                "invalid native fixture identities")
                        for pid in ids:
                            opened.append(os.pidfd_open(pid))
                        require(all(not exited(fd) for fd in opened), "native fixture exited before observation")
                        require(all(membership in Path(f"/proc/{pid}/cgroup").read_text().splitlines()
                                    for pid in ids), "native fixture escaped the owned cgroup")
                        require(all(not exited(fd) for fd in opened), "native fixture exited during observation")
                        state.update(ids=ids, pidfds=opened)
                        return
                    except (OSError, ValueError, RuntimeError) as error:
                        state["last_error"] = str(error)
                        for fd in opened:
                            os.close(fd)
                        time.sleep(0.02)
                state["error"] = "could not retain live native fixture identities: " + state.get("last_error", "")

            observer = threading.Thread(target=observe, daemon=True)
            observer.start()
            try:
                # The owned cgroup, not a process-group kill, performs cleanup.
                result = run(sys.executable, str(ROOT / "scripts/ci/run_sustained_capacity.py"),
                             "--enter-cgroup", str(group), str(harness), "--exact", TIMEOUT,
                             "--ignored", "--nocapture", cwd=ROOT, env=env, timeout=180,
                             process_group=False, check=False)
            finally:
                observer.join(timeout=16)
            require(not observer.is_alive() and "pidfds" in state,
                    "native fixture observation failed: " + state.get("error", ""))
            parent_pid, worker_pid = state["ids"]
            require(result.returncode == 73 and
                    f"native_timeout_cleanup_unconfirmed_worker={worker_pid}" in result.stdout,
                    "native timeout fixture did not report its expected failure")
            require(exited(state["pidfds"][0]) and not exited(state["pidfds"][1]),
                    "parent must have exited and held helper must remain live before cgroup cleanup")
        # The existing launcher has killed, quiesced and removed its cgroup.
        deadline = time.monotonic() + 10
        while not exited(state["pidfds"][1]) and time.monotonic() < deadline:
            time.sleep(0.01)
        require(exited(state["pidfds"][1]), "held helper survived owned cgroup cleanup")
        success = True
        return {"status": "passed", "parent_pid": parent_pid, "worker_pid": worker_pid, "receipt": receipt}
    finally:
        for fd in state.get("pidfds", []):
            os.close(fd)
        if success:
            require(fixture.resolve(strict=True) == fixture and fixture.parent == base,
                    "native cgroup fixture identity changed")
            require({entry.name for entry in fixture.iterdir()} <= {"parent.pid", "worker.pid"},
                    "unexpected native cgroup fixture content; retained")
            for name in ("parent.pid", "worker.pid"):
                path = fixture / name
                if path.exists():
                    require(path.is_file() and not path.is_symlink(), "native fixture marker changed; retained")
                    path.unlink()
            fixture.rmdir()
        else:
            print("native_cgroup_fixture_retained=" + str(fixture), file=sys.stderr, flush=True)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cgroup-parent", required=True, type=Path)
    parser.add_argument("--profile", choices=("debug", "release"), default="release")
    parser.add_argument("--offline", action="store_true")
    args = parser.parse_args()
    require(sys.platform == "linux", "Linux cgroup v2 host required")
    root = Path("/sys/fs/cgroup").resolve(strict=True)
    parent = args.cgroup_parent.resolve(strict=True)
    require(parent != root and parent.is_relative_to(root),
            "explicit delegated cgroup below root required")
    cpus = os.cpu_count()
    require(isinstance(cpus, int) and cpus >= 4, "four host CPUs required for this test allocation")
    harness = artifact(args.profile, args.offline)
    limits = {"engine_bytes": 2 * 1024**3, "host_bytes": 4 * 1024**3, "vcpus": 4}
    with linux_allocation(parent, limits) as group:
        current = Path("/proc/self/cgroup").read_text(encoding="ascii").strip().split(":", 2)
        require(len(current) == 3 and current[:2] == ["0", ""], "controller cgroup identity unavailable")
        controller = (Path("/sys/fs/cgroup") / current[2].lstrip("/")).resolve(strict=True)
        require(not controller.is_relative_to(group), "controller belongs to the owned test cgroup")
        receipt, _ = proof("cgroup_v2", limits, harness, group.name.removeprefix("rrflow-capacity-"))
        env = dict(os.environ, RRFLOW_SUSTAINED_CGROUP=str(group),
                   RRFLOW_SUSTAINED_ALLOCATION_RECEIPT=json.dumps(receipt, separators=(",", ":")))
        result = run(sys.executable, str(ROOT / "scripts/ci/run_sustained_capacity.py"),
                     "--enter-cgroup", str(group), str(harness), "--exact", CASE,
                     "--ignored", "--nocapture", cwd=ROOT, env=env, timeout=180,
                     process_group=True, check=False)
        if result.returncode != 0 or f"test {CASE} ... ok" not in result.stdout \
                or "test result: ok. 1 passed; 0 failed;" not in result.stdout:
            stdout_digest = hashlib.sha256(result.stdout.encode()).hexdigest()
            stderr_digest = hashlib.sha256(result.stderr.encode()).hexdigest()
            raise RuntimeError(f"native cgroup sample test failed: exit={result.returncode}; "
                               f"stdout_sha256={stdout_digest}; stderr_sha256={stderr_digest}")
    # linux_allocation must have confirmed quiescence and removed its child.
    print("native_sample_cgroup=" + json.dumps({"status": "passed", "receipt": receipt}), flush=True)
    outcome = timeout_cleanup(parent, limits, harness)
    print("native_sample_cgroup_timeout_cleanup=" + json.dumps(outcome), flush=True)


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        print("native_sample_cgroup_failure=" + str(error), file=sys.stderr, flush=True)
        raise SystemExit(1)
