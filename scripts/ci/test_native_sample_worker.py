#!/usr/bin/env python3
"""Exercise test-owned native sampler membership and interruption on Windows.

These are synthetic Job Object regression checks, not C-07 qualification. The
runner creates its own named job and can terminate only handles verified inside
that exact job. A failure retains the fixture rather than deleting unknown data.
"""
from __future__ import annotations

import argparse
import ctypes as c
from ctypes import wintypes as w
import json
import os
from pathlib import Path
import sys
import tempfile
import threading
import time

from owned_windows_process import run_owned
from run_offline_lifecycle import require, run
from selected_tests import NATIVE_SAMPLE_ABRUPT, NATIVE_SAMPLE_MEMBERSHIP, NATIVE_SAMPLE_TIMEOUT
from run_sustained_capacity import windows_allocation_proof
from windows_offline import P, U32, api, boolean, close, kernel

ROOT = Path(__file__).resolve().parents[2]
MEMBERSHIP = NATIVE_SAMPLE_MEMBERSHIP
ABRUPT = NATIVE_SAMPLE_ABRUPT
TIMEOUT = NATIVE_SAMPLE_TIMEOUT
ENGINE_BYTES = 2 * 1024**3
HOST_BYTES = 4 * 1024**3


def artifact(profile: str, offline: bool) -> Path:
    command = ["cargo", "test", "--locked", "-p", "rrflow-cli", "--test", "installed_reopen",
               "--no-run", "--message-format=json"]
    if profile == "release":
        command.append("--release")
    if offline:
        command.append("--offline")
    result = run(*command, cwd=ROOT, timeout=1800)
    messages = (json.loads(line) for line in result.stdout.splitlines())
    paths = {Path(row["executable"]).resolve(strict=True) for row in messages
             if row.get("reason") == "compiler-artifact" and row.get("executable")
             and row["target"]["name"] == "installed_reopen" and row["profile"]["test"]}
    require(len(paths) == 1, "Cargo did not identify one installed-reopen harness")
    harness = paths.pop()
    tests = run(str(harness), "--list", timeout=60).stdout.splitlines()
    require(all(tests.count(f"{case}: test") == 1 for case in (MEMBERSHIP, ABRUPT, TIMEOUT)),
            "native worker test entries are missing")
    return harness


def allocation():
    cpus = os.cpu_count()
    require(isinstance(cpus, int) and 0 < cpus <= 65536, "host CPU count unavailable")
    limits = {"engine_bytes": ENGINE_BYTES, "host_bytes": HOST_BYTES, "vcpus": cpus}
    return limits


def launch_args(harness: Path, case: str) -> list[str]:
    return [str(harness), "--exact", case, "--ignored", "--nocapture"]


def owned(args, env, job_name):
    return run_owned(args, env=env, timeout=90, job_name=job_name, cpu_rate=10000,
                     process_memory_bytes=ENGINE_BYTES, job_memory_bytes=ENGINE_BYTES)


def membership(harness: Path, receipt: dict) -> None:
    name = "Local\\RRFlowCapacity-" + receipt["nonce"]
    env = dict(os.environ)
    env["RRFLOW_SUSTAINED_ALLOCATION_RECEIPT"] = json.dumps(receipt, separators=(",", ":"))
    result = owned(launch_args(harness, MEMBERSHIP), env, name)
    require(result.returncode == 0 and "test result: ok. 1 passed; 0 failed;" in result.stdout,
            "owned native allocation test failed: " + result.stderr[-2048:])
    print("native_sample_job=" + json.dumps({"status": "passed", "receipt": receipt}), flush=True)


def parent_cleanup(harness: Path, receipt: dict, *, timed_out: bool) -> None:
    base = Path(os.environ.get("RUNNER_TEMP") or tempfile.gettempdir()).resolve(strict=True)
    fixture = Path(tempfile.mkdtemp(prefix="rrflow-native-parent-", dir=base)).resolve(strict=True)
    require(fixture.parent == base and fixture.name.startswith("rrflow-native-parent-"),
            "native interruption fixture escaped its parent")
    name = "Local\\RRFlowCapacity-" + receipt["nonce"]
    env = dict(os.environ)
    case = TIMEOUT if timed_out else ABRUPT
    env["RRFLOW_NATIVE_TIMEOUT_FIXTURE" if timed_out else "RRFLOW_NATIVE_ABORT_FIXTURE"] = str(fixture)
    if timed_out:
        env["RRFLOW_SUSTAINED_ALLOCATION_RECEIPT"] = json.dumps(receipt, separators=(",", ":"))
    state: dict = {}

    def launch():
        try:
            state["result"] = owned(launch_args(harness, case), env, name)
        except BaseException as error:
            state["error"] = str(error)[:2048]

    thread = threading.Thread(target=launch)
    thread.start()
    job = parent = worker = controller = None
    success = False
    try:
        deadline = time.monotonic() + 25
        while True:
            try:
                parent_pid = int((fixture / "parent.pid").read_text(encoding="ascii").strip())
                worker_pid = int((fixture / "worker.pid").read_text(encoding="ascii").strip())
                break
            except (OSError, ValueError):
                require(thread.is_alive() and time.monotonic() < deadline,
                        "owned parent/helper markers missing: " + str(state.get("error", "")))
                time.sleep(0.02)
        require(parent_pid > 0 and worker_pid > 0 and parent_pid != worker_pid,
                "invalid owned process identities")
        job = api(kernel, "OpenJobObjectW", P, U32, w.BOOL, w.LPCWSTR)(0x000c, False, name)
        boolean(job, "open exact owned job")
        open_process = api(kernel, "OpenProcess", P, U32, w.BOOL, U32)
        parent = open_process(0x100401, False, parent_pid)
        worker = open_process(0x100400, False, worker_pid)
        boolean(parent, "open owned parent")
        boolean(worker, "open owned worker")
        controller = open_process(0x100400, False, os.getpid())
        boolean(controller, "open independent controller")
        is_member = api(kernel, "IsProcessInJob", w.BOOL, P, P, P)
        outside = w.BOOL()
        boolean(is_member(controller, job, c.byref(outside)), "query independent controller")
        require(not outside.value, "controller unexpectedly belongs to child job")
        for handle in (parent, worker):
            member = w.BOOL()
            boolean(is_member(handle, job, c.byref(member)), "query owned job membership")
            require(bool(member.value), "refuse to terminate a process outside the owned job")
        wait = api(kernel, "WaitForSingleObject", U32, P, U32)
        if timed_out:
            require(wait(worker, 0) == 258 and wait(parent, 0) == 258,
                    "timeout fixture parent and helper must both still be live")
        else:
            boolean(api(kernel, "TerminateProcess", w.BOOL, P, U32)(parent, 73), "terminate owned parent")
        require(wait(parent, 15000) == 0, "owned parent did not terminate")
        thread.join(timeout=30)
        require(not thread.is_alive() and "error" not in state, "owned launcher did not finish cleanly")
        require(state["result"].returncode == 73, "unexpected owned parent exit")
        if timed_out:
            require(f"native_timeout_cleanup_unconfirmed_worker={worker_pid}" in state["result"].stdout,
                    "timeout fixture did not report the live unconfirmed worker")
        require(wait(worker, 15000) == 0, "held worker survived parent interruption and job cleanup")
        success = True
    finally:
        cleanup_error = None
        try:
            if thread.is_alive():
                if not job:
                    job = api(kernel, "OpenJobObjectW", P, U32, w.BOOL, w.LPCWSTR)(0x0008, False, name)
                if job:
                    boolean(api(kernel, "TerminateJobObject", w.BOOL, P, U32)(job, 1), "terminate owned job")
        except Exception as error:
            cleanup_error = str(error)
        for handle in (controller, worker, parent, job):
            if handle:
                close(handle)
        thread.join(timeout=100)
        require(not thread.is_alive(), "owned launcher remained live after termination")
        require(cleanup_error is None, "owned job cleanup failed: " + str(cleanup_error))
        if success:
            require(not fixture.is_symlink() and fixture.resolve(strict=True).parent == base,
                    "fixture identity changed")
            require({item.name for item in fixture.iterdir()} <= {"parent.pid", "worker.pid"},
                    "unexpected fixture content; retained")
            for filename in ("parent.pid", "worker.pid"):
                path = fixture / filename
                if path.exists():
                    require(path.is_file() and not path.is_symlink() and path.resolve().parent == fixture,
                            "fixture marker changed; retained")
                    path.unlink()
            fixture.rmdir()
        else:
            print("native_sample_fixture_retained=" + str(fixture), file=sys.stderr, flush=True)
    label = "native_sample_timeout_cleanup=" if timed_out else "native_sample_parent_death="
    print(label + json.dumps({"status": "passed", "parent_pid": parent_pid,
          "worker_pid": worker_pid, "receipt": receipt}), flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--profile", choices=("debug", "release"), default="release")
    parser.add_argument("--offline", action="store_true")
    args = parser.parse_args()
    require(sys.platform == "win32", "Windows Job Object host required")
    harness = artifact(args.profile, args.offline)
    limits = allocation()
    with windows_allocation_proof(limits, harness) as (receipt, _):
        membership(harness, receipt)
    with windows_allocation_proof(limits, harness) as (receipt, _):
        parent_cleanup(harness, receipt, timed_out=False)
    with windows_allocation_proof(limits, harness) as (receipt, _):
        parent_cleanup(harness, receipt, timed_out=True)


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        print("native_sample_worker_failure=" + str(error), file=sys.stderr, flush=True)
        raise SystemExit(1)
