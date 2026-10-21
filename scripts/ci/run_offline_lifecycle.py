#!/usr/bin/env python3
"""Qualify the real installed lifecycle in Linux mount/network/PID namespaces.

This is a test runner, not an installer or a sandbox for hostile code. Build
inputs are acquired before isolation. Only the primary binary and test harness
enter the staged payload; ordinary operating-system libraries remain available.
Isolation setup failure is fatal. No host mount or firewall is changed.
"""

from __future__ import annotations

import argparse
import errno
import hashlib
import json
import os
from pathlib import Path
import shutil
import socket
import subprocess
import sys
import tempfile


def require(condition: bool, message: str) -> None:
    if not condition:
        raise RuntimeError(message)


def lifecycle_timeouts() -> tuple[int, int]:
    """Return (harness_timeout_seconds, namespace_wrapper_timeout_seconds)."""
    harness = 720
    wrapper = harness + 60
    return harness, wrapper


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
    protected = [Path(p).resolve() for p in ["/", "/usr", "/bin", "/sbin", "/lib", "/lib64", "/etc", "/proc", "/dev", "/sys", "/tmp", "/var"]]
    for path in resolved:
        require(path.is_dir(), f"mask target is not a directory: {path}")
        require(len(path.parts) >= 3, f"refuse broad mask target: {path}")
        require(not any(path == item or path.is_relative_to(item) or item.is_relative_to(path)
                        for item in protected if item != Path("/")),
                f"refuse operating-system mask target: {path}")
        require(not stage.is_relative_to(path) and not path.is_relative_to(stage),
                f"payload and mask target overlap: {path}")
        if not any(path.is_relative_to(parent) for parent in selected):
            selected.append(path)
    return selected


def sha256(path: Path) -> str:
    with path.open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


def require_lifecycle_tests(output: str) -> None:
    cases = {line.removesuffix(": test") for line in output.splitlines() if line.endswith(": test")}
    require({"fresh_installed_engine_survives_restart_and_restore",
             "installed_engine_plans_applies_serves_discovers_authenticates_and_reopens"} <= cases,
            "harness must contain both fresh and existing installed-process cases")


def run(*arguments: str, **kwargs: object) -> subprocess.CompletedProcess:
    return subprocess.run(arguments, check=True, timeout=60, **kwargs)


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
    # Run the actual product and harness as the original unprivileged user.
    os.setgroups([])
    os.setgid(configuration["gid"])
    os.setuid(configuration["uid"])
    require(os.geteuid() != 0, "product qualification must not run as root")
    binary = stage / "rrflow"
    harness = stage / "installed_lifecycle"
    for artifact in [binary, harness]:
        require(sha256(artifact) == configuration["sha256"][artifact.name], "staged artifact changed")
    environment = {
        "PATH": "/usr/bin:/bin",  # harness SIGINT helper; product gets an empty PATH
        "HOME": str(stage / "home"),
        "TMPDIR": str(stage / "temporary"),
        "LANG": "C",
        "RRFLOW_TEST_BINARY": str(binary),
    }
    listing = run(str(harness), "--list", "--format=terse", cwd=stage,
                  env=environment, capture_output=True, text=True)
    require_lifecycle_tests(listing.stdout)
    print(json.dumps({"isolation": "checkout-hidden-network-loopback-only", "uid": os.geteuid(),
                      "masked_roots": [str(path) for path in masks], "sha256": configuration["sha256"]}), flush=True)
    # The two independent project fixtures may run together; serial execution
    # duplicates their full restore/verification latency inside one deadline.
    harness_timeout, _ = lifecycle_timeouts()
    subprocess.run([str(harness), "--nocapture", "--test-threads=2"], cwd=stage,
                   env=environment, check=True, timeout=harness_timeout)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--inside", type=Path, help=argparse.SUPPRESS)
    arguments = parser.parse_args()
    require(sys.platform == "linux", "offline namespace qualification requires Linux; it cannot skip")
    if arguments.inside:
        inside(arguments.inside.resolve(strict=True))
        return
    require(os.geteuid() != 0, "invoke as an unprivileged runner with noninteractive sudo")
    root = Path(__file__).resolve().parents[2]
    for command in ["sudo", "unshare", "timeout", "mount", "ip"]:
        require(shutil.which(command) is not None, f"required isolation tool is missing: {command}")
    build = subprocess.run(["cargo", "test", "--locked", "-p", "rrflow-cli", "--test",
                            "installed_lifecycle", "--no-run", "--message-format=json"],
                           cwd=root, check=True, stdout=subprocess.PIPE, text=True, timeout=900)
    binary, harness = artifact_paths(build.stdout)
    with tempfile.TemporaryDirectory(prefix="rrflow-offline-", dir="/tmp") as temporary:
        stage = Path(temporary)
        candidates = [root.parent]
        for name, default in [("CARGO_HOME", Path.home() / ".cargo"), ("RUSTUP_HOME", Path.home() / ".rustup")]:
            path = Path(os.environ.get(name, str(default)))
            if path.exists():
                candidates.append(path)
        masks = mask_paths(candidates, stage)
        source_hashes = {"rrflow": sha256(binary), "installed_lifecycle": sha256(harness)}
        for source, name in [(binary, "rrflow"), (harness, "installed_lifecycle"), (Path(__file__), "runner.py")]:
            shutil.copy2(source, stage / name)
        for name, expected in source_hashes.items():
            require(sha256(stage / name) == expected, f"staging changed the source artifact: {name}")
        for name in ["home", "temporary"]:
            (stage / name).mkdir()
        configuration = {"uid": os.getuid(), "gid": os.getgid(),
                         "namespaces": {kind: os.readlink(f"/proc/self/ns/{kind}") for kind in ["net", "mnt", "pid"]},
                         "masks": [str(path) for path in masks],
                         "sha256": source_hashes}
        (stage / "isolation.json").write_text(json.dumps(configuration), encoding="utf-8")
        # timeout is privileged too, so it can reap the namespace initializer.
        # Killing that PID namespace's initializer reaps every test-owned child.
        _, wrapper_timeout = lifecycle_timeouts()
        subprocess.run(["sudo", "-n", "/usr/bin/timeout", "--signal=TERM", "--kill-after=10s", f"{wrapper_timeout}s",
                        "/usr/bin/unshare", "--mount", "--net", "--pid", "--fork", "--kill-child=KILL",
                        "--mount-proc", "--propagation", "private", "/usr/bin/python3", str(stage / "runner.py"),
                        "--inside", str(stage)], cwd=stage, check=True, timeout=wrapper_timeout + 30)
        print("Offline installed lifecycle passed; no alpha or signed-distribution qualification claimed.")


if __name__ == "__main__":
    main()
