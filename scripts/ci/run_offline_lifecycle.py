#!/usr/bin/env python3
"""Qualify the real installed lifecycle with asserted native offline enforcement.

This is a test runner, not an installer or a sandbox for hostile code. Build
inputs are acquired before isolation. Only the primary binary and test harness
enter the staged payload; ordinary operating-system libraries remain available.
Isolation setup failure is fatal. Windows qualification uses a disposable hosted
CI principal, scoped ACLs and a dynamic WFP session; no global firewall rule is changed.
"""

from __future__ import annotations

import argparse
from contextlib import ExitStack
import errno
import hashlib
import json
import os
import re
import signal
import platform
import plistlib
from pathlib import Path
import shutil
import socket
import subprocess
import sys
import tempfile
import threading
import time


def require(condition: bool, message: str) -> None:
    if not condition:
        raise RuntimeError(message)


def artifact_paths(output: str) -> tuple[Path, Path]:
    binaries: set[Path] = set()
    tests: set[Path] = set()
    for line in output.splitlines():
        message = json.loads(line)
        if message.get("reason") != "compiler-artifact" or not message.get("executable"):
            continue
        target = message["target"]
        path = Path(message["executable"]).resolve(strict=True)
        if target["name"] == "rrflow" and target["kind"] == ["bin"]:
            binaries.add(path)
        if target["name"] == "installed_lifecycle" and target["kind"] == ["test"]:
            tests.add(path)
    require(len(binaries) == len(tests) == 1, "Cargo must identify one primary binary and one lifecycle harness")
    return binaries.pop(), tests.pop()


def mask_paths(paths: list[Path], stage: Path) -> list[Path]:
    """Reject broad/system mounts and keep only non-overlapping source roots."""
    resolved = sorted({path.resolve(strict=True) for path in paths}, key=lambda path: len(path.parts))
    selected: list[Path] = []
    if sys.platform == "win32":
        protected = [Path(os.environ[name]).resolve() for name in
                     ["SystemRoot", "ProgramFiles", "ProgramFiles(x86)", "ProgramData", "PUBLIC"]
                     if name in os.environ]
    else:
        protected = [Path(p).resolve() for p in
                     ["/usr", "/bin", "/sbin", "/lib", "/lib64", "/etc", "/proc", "/dev", "/sys", "/tmp", "/var"]]
    for path in resolved:
        require(path.is_dir(), f"mask target is not a directory: {path}")
        require(len(path.parts) >= 3, f"refuse broad mask target: {path}")
        require(not any(path == item or path.is_relative_to(item) or item.is_relative_to(path)
                        for item in protected),
                f"refuse operating-system mask target: {path}")
        require(not stage.is_relative_to(path) and not path.is_relative_to(stage),
                f"payload and mask target overlap: {path}")
        if not any(path.is_relative_to(parent) for parent in selected):
            selected.append(path)
    return selected


def sha256(path: Path) -> str:
    with path.open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


def filesystem_profile(path: Path) -> dict:
    if sys.platform == "linux":
        kind = run("findmnt", "--noheadings", "--output", "FSTYPE", "--target", str(path)).stdout.strip()
    elif sys.platform == "darwin":
        mount = run("df", "-P", str(path)).stdout.splitlines()[-1].split(None, 5)[5]
        data = plistlib.loads(run("diskutil", "info", "-plist", mount).stdout.encode())
        kind = data.get("FilesystemType", data.get("FilesystemName"))
    else:
        import ctypes
        from ctypes import wintypes
        kernel = ctypes.WinDLL("kernel32", use_last_error=True)
        volume, name = ctypes.create_unicode_buffer(1024), ctypes.create_unicode_buffer(128)
        get_path = kernel.GetVolumePathNameW
        get_path.argtypes = [wintypes.LPCWSTR, wintypes.LPWSTR, wintypes.DWORD]
        get_path.restype = wintypes.BOOL
        require(get_path(str(path), volume, len(volume)), "native volume path unavailable")
        get_info = kernel.GetVolumeInformationW
        get_info.argtypes = [wintypes.LPCWSTR, wintypes.LPWSTR, wintypes.DWORD, ctypes.c_void_p,
                            ctypes.c_void_p, ctypes.c_void_p, wintypes.LPWSTR, wintypes.DWORD]
        get_info.restype = wintypes.BOOL
        require(get_info(volume, None, 0, None, None, None, name, len(name)), "native filesystem identity unavailable")
        kind = name.value
    require(isinstance(kind, str) and 0 < len(kind) < 128, "filesystem identity is absent or unbounded")
    return {"type": kind, "device": os.stat(path).st_dev}


REQUIRED_CASES = frozenset({
    "primary_help_exposes_only_the_installed_product_lifecycle",
    "raw_database_and_development_commands_fail_during_argument_parsing",
    "install_requires_an_explicit_plan_digest_and_verify_is_read_only_by_shape",
    "fresh_installed_engine_survives_restart_and_restore",
    "installed_engine_plans_applies_serves_discovers_authenticates_and_reopens",
})


def require_lifecycle_tests(output: str) -> None:
    cases = [line.removesuffix(": test") for line in output.splitlines() if line.endswith(": test")]
    require(len(cases) == len(REQUIRED_CASES) and set(cases) == REQUIRED_CASES, "harness must contain exactly the five required lifecycle cases")


def require_lifecycle_results(output: str) -> list[str]:
    cases = re.findall(r"(?m)^test (\w+) \.\.\. ok$", output)
    summaries = re.findall(r"(?m)^test result: (.+)$", output)
    require(len(cases) == len(REQUIRED_CASES) and set(cases) == REQUIRED_CASES,
            "every required lifecycle case must execute exactly once and pass")
    require(len(summaries) == 1 and re.fullmatch(
        r"ok\. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in [0-9.]+s",
        summaries[0]) is not None, "lifecycle summary must have no failures, omissions or skips")
    return sorted(cases)


def lifecycle_receipts(output: str) -> list[dict]:
    receipts = [json.loads(line.removeprefix("native_lifecycle_receipt="))
                for line in output.splitlines() if line.startswith("native_lifecycle_receipt=")]
    require(len(receipts) == 2 and {receipt.get("mode") for receipt in receipts} == {"fresh", "existing"},
            "both lifecycle modes must publish their own accepted receipt")
    fields = {"mode", "build", "installation_plan_sha256", "restore_plan_sha256", "backup_sha256",
              "installed_runtime_cursor", "reopened_runtime_cursor", "restored_runtime_cursor",
              "committed_claim_sequence", "restored_read_manifest_sha256", "captured_output_bytes", "stream_limit_bytes"}
    for receipt in receipts:
        require(set(receipt) == fields, "lifecycle receipt has unknown or missing fields")
        for field in ["installation_plan_sha256", "restore_plan_sha256", "backup_sha256", "restored_read_manifest_sha256"]:
            require(re.fullmatch("[0-9a-f]{64}", receipt.get(field, "")) is not None, "lifecycle receipt digest is missing")
        require([receipt.get(field) for field in ["installed_runtime_cursor", "reopened_runtime_cursor", "restored_runtime_cursor"]]
                == [2, 4, 3], "lifecycle receipt does not bind the asserted durable outcomes")
        build = receipt.get("build")
        require(isinstance(build, dict) and set(build) == {"executable_sha256", "metadata_sha256", "public_contract_sha256", "signal_catalogue_sha256"}, "lifecycle receipt lacks closed build identity")
        require(all(isinstance(value, str) and re.fullmatch("[0-9a-f]{64}", value) for value in build.values()), "build identity digest is invalid")
        require(type(receipt["captured_output_bytes"]) is int and 0 < receipt["captured_output_bytes"] <= 128 * 1024 * 1024
                and receipt["stream_limit_bytes"] == 16 * 1024 * 1024
                and type(receipt["committed_claim_sequence"]) is int and receipt["committed_claim_sequence"] > 0,
                "lifecycle receipt resource or commit bounds are invalid")
    return receipts


def storage_history_receipts(output: str, durable: list[dict]) -> list[dict]:
    """Admit bounded collector output, not another semantic history oracle.

    The native fixture calls the canonical rrflow-eval ledger reconciler at
    every reopen. Here we reject incomplete schedules and unknown/secret-bearing
    fields before retaining its evidence; a producer summary alone is insufficient.
    """
    lines = [line.removeprefix("native_storage_history=") for line in output.splitlines()
             if line.startswith("native_storage_history=")]
    require(len(lines) == 2 and all(len(line) <= 64 * 1024 for line in lines),
            "both bounded storage histories are required")
    receipts = [json.loads(line) for line in lines]
    require(all(isinstance(item, dict) for item in receipts)
            and {item.get("mode") for item in receipts} == {"fresh", "existing"},
            "storage history modes must be distinct")
    builds = {item["mode"]: item["build"] for item in durable}
    expected_report = {"logical_calls": 4, "attempts": 13, "acknowledged": 11,
                       "rejected": 1, "uncertain": 1, "resolved_present": 3,
                       "resolved_absent": 1, "unresolved": 0, "violations": []}
    def fields(item, names):
        require(isinstance(item, dict) and set(item) == set(names.split()),
                "storage history has unknown or missing fields")
    def hash_value(value):
        require(isinstance(value, str) and re.fullmatch("[0-9a-f]{64}", value) is not None,
                "storage history digest is invalid")
    for receipt in receipts:
        fields(receipt, "mode build forced_reopens expected_sha256 ledger report")
        require(receipt["mode"] in builds and receipt["build"] == builds[receipt["mode"]],
                "history build differs from installed lifecycle")
        require(type(receipt["forced_reopens"]) is int and receipt["forced_reopens"] == 3,
                "storage history reopen schedule is incomplete")
        hash_value(receipt["expected_sha256"])
        require(receipt["report"] == expected_report
                and all(type(receipt["report"][key]) is int for key in expected_report if key != "violations"),
                "storage history failed or omitted an outcome")
        ledger = receipt["ledger"]
        fields(ledger, "expected attempts readback")
        for key, count in [("expected", 4), ("attempts", 13), ("readback", 4)]:
            require(isinstance(ledger[key], list) and len(ledger[key]) == count,
                    "storage history ledger is incomplete")
        for item in ledger["expected"]:
            fields(item, "logical_id request_sha256 effect_sha256 resolution")
            require(item["logical_id"] in {"a", "b", "c", "d"}
                    and item["resolution"] in {"present", "absent"}, "history expected identity")
            hash_value(item["request_sha256"])
            hash_value(item["effect_sha256"])
        require({item["logical_id"] for item in ledger["expected"]} == {"a", "b", "c", "d"},
                "history expected identities are duplicated or omitted")
        for index, item in enumerate(ledger["attempts"]):
            fields(item, "attempt_id logical_id request_sha256 started_ns finished_ns outcome")
            require(item["attempt_id"] == f"attempt-{index}" and item["logical_id"] in {"a", "b", "c", "d"},
                    "history attempt identity")
            hash_value(item["request_sha256"])
            require(type(item["started_ns"]) is int and type(item["finished_ns"]) is int
                    and 0 <= item["started_ns"] <= item["finished_ns"] <= 600_000_000_000,
                    "history timing bound")
            outcome = item["outcome"]
            require(isinstance(outcome, dict), "history outcome shape")
            if outcome.get("kind") == "acknowledged":
                fields(outcome, "kind receipt_sha256")
                hash_value(outcome["receipt_sha256"])
            else:
                fields(outcome, "kind reason")
                require((outcome["kind"], outcome["reason"]) in
                        {("rejected", "stale-cursor"), ("uncertain", "response-lost")}, "history outcome kind")
        for item in ledger["readback"]:
            require(isinstance(item, dict), "history readback shape")
            if item.get("kind") == "absent":
                fields(item, "kind logical_id")
                require(item["logical_id"] == "c", "history absent identity")
            else:
                fields(item, "kind logical_id request_sha256 effect_sha256 receipt_sha256 applications")
                require(item["kind"] == "present" and item["logical_id"] in {"a", "b", "d"}
                        and type(item["applications"]) is int and item["applications"] == 1,
                        "history recovered identity")
                for key in ["request_sha256", "effect_sha256", "receipt_sha256"]:
                    hash_value(item[key])
        require({item["logical_id"] for item in ledger["readback"]} == {"a", "b", "c", "d"},
                "history readback identities are duplicated or omitted")
        observed = {kind: sum(item["outcome"]["kind"] == kind for item in ledger["attempts"])
                    for kind in ["acknowledged", "rejected", "uncertain"]}
        require(all(observed[kind] == expected_report[kind] for kind in observed),
                "history outcome counts disagree with the retained attempts")
    return receipts


def run(*arguments: str, **kwargs: object) -> subprocess.CompletedProcess:
    """Capture a finite amount without communicate()'s unbounded allocation.

    The privileged outer runner owns the PID namespace and its descendants.
    This helper kills/reaps only the exact child it started. Never log arbitrary
    product output: it can include credentials if a redaction assertion fails.
    """
    check = kwargs.pop("check", True)
    timeout = kwargs.pop("timeout", 60)
    limit = kwargs.pop("output_limit", 8 * 1024 * 1024)
    kwargs.pop("capture_output", None)
    kwargs.pop("text", None)
    process_group = kwargs.pop("process_group", False)
    if process_group:
        require(os.name == "posix", "native process group is unavailable")
        kwargs["start_new_session"] = True
    child = subprocess.Popen(arguments, stdout=subprocess.PIPE, stderr=subprocess.PIPE, **kwargs)
    streams = [bytearray(), bytearray()]
    exceeded = threading.Event()
    failures = []

    def capture(pipe, destination):
        try:
            while chunk := pipe.read(65536):
                if len(destination) + len(chunk) > limit:
                    exceeded.set()
                    return
                destination.extend(chunk)
        except OSError:
            failures.append("capture failed")
            exceeded.set()
        finally:
            pipe.close()

    readers = [threading.Thread(target=capture, args=(pipe, destination), daemon=True)
               for pipe, destination in zip([child.stdout, child.stderr], streams)]
    for reader in readers:
        reader.start()
    deadline = time.monotonic() + timeout
    error = None
    try:
        while child.poll() is None:
            if exceeded.is_set():
                error = "command exceeded its output limit or capture failed"
                break
            if time.monotonic() >= deadline:
                error = "command exceeded its deadline"
                break
            time.sleep(0.01)
    finally:
        if process_group:
            try:
                os.killpg(child.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
        elif child.poll() is None:
            child.kill()
        child.wait(timeout=10)
        for reader in readers:
            reader.join(timeout=2)
    require(not any(reader.is_alive() for reader in readers), "command output did not close")
    require(error is None and not exceeded.is_set() and not failures, error or "command output limit exceeded")
    if check:
        require(child.returncode == 0, f"qualification command failed with exit status {child.returncode}")
    return subprocess.CompletedProcess(arguments, child.returncode,
                                       streams[0].decode("utf-8", errors="replace"),
                                       streams[1].decode("utf-8", errors="replace"))


def inside(stage: Path) -> None:
    configuration = json.loads((stage / "isolation.json").read_text())
    require(os.geteuid() == 0, "namespace setup requires its own privileged process")
    for namespace in ["net", "mnt", "pid"]:
        require(os.readlink(f"/proc/self/ns/{namespace}") != configuration["namespaces"][namespace],
                f"{namespace} namespace was not isolated")
    os.chdir(stage)
    masks = mask_paths([Path(path) for path in configuration["masks"]], stage)
    # unshare also sets private propagation; assert it by applying it before
    # any mask. These mounts exist only in this process's private namespace.
    run("/usr/bin/mount", "--make-rprivate", "/")
    for path in masks:
        run("/usr/bin/mount", "-t", "tmpfs", "-o", "ro,size=4k,mode=0755", "tmpfs", str(path))
        require(not any(path.iterdir()), f"source root remains visible: {path}")
    require(all(not Path(path).exists() for path in configuration["source_probes"]), "known source file escaped masking")
    ip = shutil.which("ip")
    require(ip is not None, "iproute2 is required to enable isolated loopback")
    run(ip, "link", "set", "lo", "up")
    interfaces = json.loads(run(ip, "-json", "link", "show", capture_output=True, text=True).stdout)
    require({entry["ifname"] for entry in interfaces} == {"lo"}, "non-loopback interface escaped isolation")
    for family in ["-4", "-6"]:
        routes = json.loads(run(ip, family, "-json", "route", "show", "default", capture_output=True, text=True).stdout)
        require(not routes, "isolated network has a default route")
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as probe:
        probe.settimeout(1)
        require(probe.connect_ex(("192.0.2.1", 9)) == errno.ENETUNREACH,
                "outbound IPv4 must fail with ENETUNREACH")
    with socket.socket(socket.AF_INET6, socket.SOCK_STREAM) as probe:
        probe.settimeout(1)
        require(probe.connect_ex(("2001:db8::1", 9)) == errno.ENETUNREACH,
                "outbound IPv6 must fail with ENETUNREACH")
    # Run the actual product and harness as the original unprivileged user.
    os.setgroups([])
    os.setgid(configuration["gid"])
    os.setuid(configuration["uid"])
    require(os.geteuid() != 0, "product qualification must not run as root")
    qualify_payload(stage, configuration)


def qualify_payload(stage: Path, configuration: dict) -> None:
    binary = stage / configuration.get("binary_name", "rrflow")
    harness = stage / configuration.get("harness_name", "installed_lifecycle")
    for artifact in [binary, harness]:
        require(sha256(artifact) == configuration["sha256"][artifact.name], "staged artifact changed")
    environment = {
        "PATH": "/usr/bin:/bin",  # harness SIGINT helper; product gets an empty PATH
        "HOME": str(stage / "home"),
        "TMPDIR": str(stage / "temporary"),
        "LANG": "C",
        "RRFLOW_TEST_BINARY": str(binary),
    }
    if sys.platform == "win32":
        environment.update({name: os.environ[name] for name in ["SystemRoot", "WINDIR"]})
        environment.update({"USERPROFILE": environment["HOME"], "TEMP": environment["TMPDIR"], "TMP": environment["TMPDIR"]})
    listing = run(str(harness), "--list", "--format=terse", cwd=stage,
                  env=environment, capture_output=True, text=True)
    require_lifecycle_tests(listing.stdout)
    # The two independent project fixtures may run together; serial execution
    # duplicates their full restore/verification latency inside one deadline.
    result = run(str(harness), "--nocapture", "--test-threads=2", cwd=stage,
                 env=environment, timeout=540, check=False)
    observed = re.findall(r"(?m)^test (\w+) \.\.\. (ok|FAILED|ignored[^\n]*)$", result.stdout)
    outcomes = [{"case": name, "status": status.split()[0]} for name, status in observed if name in REQUIRED_CASES]
    (stage / "receipt.json").write_text(json.dumps({"schema_version": 1, "status": "failed",
        "platform": sys.platform, "sha256": configuration["sha256"], "cases": outcomes,
        "exit_status": result.returncode, "isolation": configuration["isolation"]}, indent=2), encoding="utf-8")
    cases = require_lifecycle_results(result.stdout)
    require(result.returncode == 0, "lifecycle reported success with a failing process status")
    durable = lifecycle_receipts(result.stdout)
    histories = storage_history_receipts(result.stdout, durable)
    require(all(receipt["build"]["executable_sha256"] == configuration["sha256"][binary.name] for receipt in durable), "lifecycle receipt names a different executable")
    receipt = {"schema_version": 1, "status": "passed", "platform": sys.platform,
               "os_release": platform.release(), "architecture": platform.machine(),
               "sha256": configuration["sha256"], "passed_cases": cases,
               "filesystem": configuration["filesystem"], "durable_receipts": durable,
               "storage_histories": histories,
               "failed": 0, "ignored": 0, "outbound_denials": ["ipv4", "ipv6"],
               "masked_roots": configuration["masks"], "isolation": configuration["isolation"]}
    if sys.platform == "win32":
        receipt["startup_diagnostics"] = json.loads((stage / "startup.json").read_text())
    (stage / "receipt.json").write_text(json.dumps(receipt, indent=2), encoding="utf-8")
    print(json.dumps(receipt), flush=True)



def macos_profile(mask_count: int) -> str:
    require(mask_count > 0, "native isolation requires source masks")
    return "\n".join([
        "(version 1)", "(allow default)", "(deny network*)",
        '(allow network-bind (local ip "localhost:*"))',
        '(allow network-inbound (local ip "localhost:*"))',
        '(allow network-outbound (remote ip "localhost:*"))',
        *[f'(deny file-read* file-write* (subpath (param "MASK{index}")))' for index in range(mask_count)],
    ])


def native_probe_targets(configuration: dict) -> list[tuple]:
    ipv6 = ("2001:db8::1", 9)
    if sys.platform == "win32":
        scope = configuration.get("ipv6_probe_scope_id")
        require(type(scope) is int and 0 < scope < 2 ** 32,
                "native Windows IPv6 probe requires a configured interface scope")
        # A non-loopback link-local destination reaches IPv6 classification even
        # on hosted runners without a global IPv6 route. Never accept unreachable.
        ipv6 = ("fe80::ffff:ffff:fffe", 9, 0, scope)
    return [(socket.AF_INET, ("192.0.2.1", 9)), (socket.AF_INET6, ipv6)]


def native_denial_probes(stage: Path) -> None:
    configuration = json.loads((stage / "isolation.json").read_text())
    for root in configuration["masks"]:
        try:
            list(Path(root).iterdir())
        except PermissionError:
            pass
        else:
            raise RuntimeError("source root remains accessible")
    for path in configuration["source_probes"]:
        for mode in ["rb", "r+b"]:
            try:
                with open(path, mode):
                    pass
            except PermissionError:
                pass
            else:
                raise RuntimeError("a known source file remains readable or writable")
    for family, address in native_probe_targets(configuration):
        with socket.socket(family, socket.SOCK_STREAM) as probe:
            probe.settimeout(1)
            code = probe.connect_ex(address)
            require(code in {errno.EPERM, errno.EACCES, getattr(errno, "WSAEACCES", 10013)},
                    f"native outbound policy for {address} returned {code}, not an explicit permission denial")
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as listener:
        listener.bind(("127.0.0.1", 0))
        listener.listen(1)
        listener.settimeout(2)
        with socket.create_connection(listener.getsockname(), timeout=2), listener.accept()[0]:
            pass


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--inside", type=Path, help=argparse.SUPPRESS)
    parser.add_argument("--mac-inside", type=Path, help=argparse.SUPPRESS)
    parser.add_argument("--windows-inside", type=Path, help=argparse.SUPPRESS)
    parser.add_argument("--probe", type=Path, help=argparse.SUPPRESS)
    parser.add_argument("--receipt", type=Path)
    parser.add_argument("--cleanup-owned", type=Path, help=argparse.SUPPRESS)
    arguments = parser.parse_args()
    require(sys.platform in {"linux", "darwin", "win32"}, "native offline enforcement unavailable; it cannot skip")
    if arguments.cleanup_owned:
        stage = arguments.cleanup_owned.resolve(strict=True)
        require(sys.platform == "win32" and stage.name.startswith("rrflow-offline-")
                and stage.parent == Path(os.environ["PUBLIC"]).resolve(), "invalid cleanup fixture")
        for name in ["home", "temporary"]:
            path = stage / name
            require(path.resolve(strict=True) == path and not (path.lstat().st_file_attributes & 0x400), "cleanup fixture escaped its stage")
            shutil.rmtree(path)
        return
    if arguments.probe:
        native_denial_probes(arguments.probe.resolve(strict=True))
        return
    if arguments.mac_inside or arguments.windows_inside:
        stage = (arguments.mac_inside or arguments.windows_inside).resolve(strict=True)
        native_denial_probes(stage)
        run(sys.executable, str(stage / "runner.py"), "--probe", str(stage), cwd=stage)
        qualify_payload(stage, json.loads((stage / "isolation.json").read_text()))
        return
    if arguments.inside:
        inside(arguments.inside.resolve(strict=True))
        return
    if sys.platform != "win32":
        require(os.geteuid() != 0, "invoke as an unprivileged runner with noninteractive sudo")
    if sys.platform == "win32":
        require(os.environ.get("GITHUB_ACTIONS") == "true" and os.environ.get("RUNNER_ENVIRONMENT") == "github-hosted", "native Windows isolation requires an ephemeral hosted CI runner")
    root = Path(__file__).resolve().parents[2]
    for command in (["sudo", "unshare", "timeout", "mount", "ip", "findmnt"] if sys.platform == "linux" else ["sandbox-exec"] if sys.platform == "darwin" else []):
        require(shutil.which(command) is not None, f"required isolation tool is missing: {command}")
    build = run("cargo", "test", "--locked", "-p", "rrflow-cli", "--test",
                "installed_lifecycle", "--no-run", "--message-format=json", cwd=root, timeout=900)
    binary, harness = artifact_paths(build.stdout)
    stage_parent = "/tmp" if sys.platform == "linux" else os.environ["PUBLIC"] if sys.platform == "win32" else None
    with tempfile.TemporaryDirectory(prefix="rrflow-offline-", dir=stage_parent) as temporary, ExitStack() as probes:
        stage = Path(temporary).resolve()
        candidates = [root.parent]
        for name, default in [("CARGO_HOME", Path.home() / ".cargo"), ("RUSTUP_HOME", Path.home() / ".rustup")]:
            path = Path(os.environ.get(name, str(default)))
            if path.exists():
                candidates.append(path)
        masks = mask_paths(candidates, stage)
        source_probes = []
        for mask in masks:
            with tempfile.NamedTemporaryFile(prefix="rrflow-denial-", dir=mask, delete=False) as probe:
                probe.write(b"non-secret native source denial fixture")
                source_probes.append(probe.name)
            probes.callback(Path(probe.name).unlink)
        binary_name, harness_name = (("rrflow.exe", "installed_lifecycle.exe") if sys.platform == "win32" else ("rrflow", "installed_lifecycle"))
        source_hashes = {binary_name: sha256(binary), harness_name: sha256(harness)}
        for source, name in [(binary, binary_name), (harness, harness_name), (Path(__file__), "runner.py")]:
            shutil.copy2(source, stage / name)
        for name, expected in source_hashes.items():
            require(sha256(stage / name) == expected, f"staging changed the source artifact: {name}")
        for name in ["home", "temporary"]:
            (stage / name).mkdir()
        configuration = {"uid": os.getuid() if sys.platform != "win32" else None,
                         "gid": os.getgid() if sys.platform != "win32" else None,
                         "binary_name": binary_name, "harness_name": harness_name,
                         "namespaces": ({kind: os.readlink(f"/proc/self/ns/{kind}") for kind in ["net", "mnt", "pid"]} if sys.platform == "linux" else {}),
                         "isolation": "linux-private-namespaces" if sys.platform == "linux" else "macos-inherited-seatbelt" if sys.platform == "darwin" else "windows-principal-job-wfp",
                         "masks": [str(path) for path in masks],
                         "source_probes": source_probes,
                         "filesystem": filesystem_profile(stage),
                         "sha256": source_hashes}
        (stage / "isolation.json").write_text(json.dumps(configuration), encoding="utf-8")
        try:
            if sys.platform == "linux":
                # Privileged timeout owns the namespace initializer, whose exit
                # reaps every descendant, even if the unprivileged wrapper fails.
                result = run("sudo", "-n", "/usr/bin/timeout", "--signal=TERM", "--kill-after=10s", "600s",
                             "/usr/bin/unshare", "--mount", "--net", "--pid", "--fork", "--kill-child=KILL",
                             "--mount-proc", "--propagation", "private", "/usr/bin/python3", str(stage / "runner.py"),
                             "--inside", str(stage), cwd=stage, timeout=630)
            elif sys.platform == "darwin":
                profile = stage / "qualification.sb"
                profile.write_text(macos_profile(len(masks)), encoding="utf-8")
                parameters = [item for index, path in enumerate(masks) for item in ["-D", f"MASK{index}={path}"]]
                result = run("/usr/bin/sandbox-exec", *parameters, "-f", str(profile), sys.executable,
                             str(stage / "runner.py"), "--mac-inside", str(stage), cwd=stage,
                             timeout=600, process_group=True)
            else:
                from windows_offline import run_isolated
                run_isolated(stage, masks)
                result = subprocess.CompletedProcess([], 0, "")
            require((stage / "receipt.json").is_file(), "isolated execution produced no qualification receipt")
            print((stage / "receipt.json").read_text(encoding="utf-8"))
        finally:
            if arguments.receipt:
                arguments.receipt.parent.mkdir(parents=True, exist_ok=True)
                if (stage / "receipt.json").is_file():
                    shutil.copyfile(stage / "receipt.json", arguments.receipt)
                    if (stage / "startup.json").is_file():
                        receipt = json.loads(arguments.receipt.read_text(encoding="utf-8"))
                        receipt["startup_diagnostics"] = json.loads((stage / "startup.json").read_text())
                        arguments.receipt.write_text(json.dumps(receipt, indent=2), encoding="utf-8")
                else:
                    arguments.receipt.write_text(json.dumps({"schema_version": 1, "status": "failed",
                        "platform": sys.platform, "sha256": source_hashes,
                        "isolation": configuration["isolation"],
                        "diagnostic": "native qualification failed before complete acceptance",
                        "startup_diagnostics": (json.loads((stage / "startup.json").read_text()) if (stage / "startup.json").is_file() else None),
                        "native_failure": (json.loads((stage / "failure.json").read_text()) if (stage / "failure.json").is_file() else None)}, indent=2), encoding="utf-8")
        print("Offline installed lifecycle passed; no alpha or signed-distribution qualification claimed.")


def record_outer_failure(output: Path, error: Exception) -> None:
    receipt = (json.loads(output.read_text(encoding="utf-8")) if output.is_file() else
               {"schema_version": 1, "platform": sys.platform, "phase": "preflight"})
    receipt["status"] = "failed"
    receipt["outer_failure"] = {"type": type(error).__name__, "diagnostic": str(error)[:1024]}
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(receipt, indent=2), encoding="utf-8")


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        # Only structural diagnostic data escapes an isolated Windows helper.
        # Product output and test-owned credentials are never serialized here.
        inside_flag = next((flag for flag in ["--windows-inside", "--mac-inside", "--inside"] if flag in sys.argv), None)
        if "--receipt" in sys.argv:
            output = Path(sys.argv[sys.argv.index("--receipt") + 1])
            record_outer_failure(output, error)
        if inside_flag:
            stage = Path(sys.argv[sys.argv.index(inside_flag) + 1])
            (stage / "failure.json").write_text(json.dumps({"status": "failed", "error_type": type(error).__name__,
                "diagnostic": str(error)[:1024]}), encoding="utf-8")
        raise
