#!/usr/bin/env python3
"""Qualify real installed capacity failures on disposable native filesystems.

Never accepts an existing disk, partition or mount root. This runner creates its
own capped image, proves its attachment identity, and removes only that image
after verified detachment. Missing privileges/tools are not-run, never a pass.
"""
from __future__ import annotations

import argparse
from contextlib import contextmanager
import json
import os
from pathlib import Path
import plistlib
import re
import secrets
import shutil
import sys
import tempfile
import time

from run_filesystem_containment import detach_owned_image
from selected_tests import NATIVE_CAPACITY_CASE
from run_offline_lifecycle import (CommandDeadlineExceeded, CommandOutputNotClosed,
                                   filesystem_profile, require, run, sha256)

CASE = NATIVE_CAPACITY_CASE
POINTS = ("wal_before_write", "segment_before_create", "manifest_before_create", "current_before_create")
SCRIPT = Path(__file__).resolve()
# hdiutil create starts a diskimages-helper that leaves the command's process
# group and inherits its output. After a busy create that helper can outlive
# hdiutil while it serves the half-made image. This bounds how long a retry
# waits for it to let go after the owned image is released.
HELPER_EXIT_SECONDS = 20


class NotRun(RuntimeError):
    pass


class NativeCommandFailure(RuntimeError):
    """Failure from one owned native command, with bounded diagnostic output."""
    def __init__(self, tool, verb, result, stdout_open=False, stderr_open=False, timed_out=False):
        self.tool = tool
        self.verb = verb
        self.returncode = result.returncode
        self.stdout_open = stdout_open
        self.stderr_open = stderr_open
        self.timed_out = timed_out
        # Set when a busy create's output stayed open past the bounded wait.
        self.holder_outlived_release = False
        self.stdout_head = result.stdout[:8192]
        self.stdout_tail = result.stdout[-8192:]
        self.stderr_head = result.stderr[:8192]
        self.stderr_tail = result.stderr[-8192:]
        super().__init__(f"native {tool} {'timed out' if timed_out else 'failed'} ({result.returncode}): "
                         + result.stderr[-2048:] + result.stdout[-2048:])


def retain_native_failure(config, fixture, mode, point, case_index, error):
    """Retain closed capture metadata or controlled create output, never product data."""
    directory = os.environ.get("RRFLOW_CAPACITY_DIAGNOSTICS")
    runner_temp = os.environ.get("RUNNER_TEMP")
    if not directory or not runner_temp:
        return
    path = Path(directory)
    parent = Path(runner_temp).resolve(strict=True)
    require(path.name == "rrflow-capacity-diagnostics" and path.parent.resolve(strict=True) == parent,
            "capacity diagnostic directory must be directly inside runner temp")
    require(not path.is_symlink(), "capacity diagnostic directory is a symlink")
    path.mkdir(exist_ok=True)
    require(path.is_dir() and not path.is_symlink(), "invalid capacity diagnostic directory")
    source = error
    seen = set()
    def closed(value, pattern):
        return value if isinstance(value, str) and re.fullmatch(pattern, value) else None

    for _ in range(4):
        if source is None or id(source) in seen:
            break
        seen.add(id(source))
        if isinstance(source, CommandOutputNotClosed):
            size = config.get("size_mib")
            receipt = {"version": 1, "kind": "command-output-not-closed",
                       "mode": mode if mode in ("fresh", "existing") else "unknown",
                       "point": point if point in POINTS else "unknown",
                       "case_index": case_index if type(case_index) is int and 0 < case_index <= 1000 else None,
                       "size_mib": size if type(size) is int and 128 <= size <= 2048 else None,
                       "harness_sha256": closed(config.get("harness_sha256"), r"[0-9a-f]{64}"),
                       "github_sha": closed(os.environ.get("GITHUB_SHA"), r"[0-9a-f]{40}"),
                       "github_run_id": closed(os.environ.get("GITHUB_RUN_ID"), r"[0-9]{1,20}"),
                       "github_run_attempt": closed(os.environ.get("GITHUB_RUN_ATTEMPT"), r"[0-9]{1,8}"),
                       "command_category": source.command_category,
                       "child_returncode": source.child_returncode,
                       "timed_out": source.timed_out,
                       "stdout_open": source.stdout_open, "stderr_open": source.stderr_open,
                       "cleanup_state": "unknown"}
            encoded = json.dumps(receipt, ensure_ascii=False).encode("utf-8", errors="replace")
            require(len(encoded) <= 16 * 1024, "capacity capture diagnostic exceeds output bound")
            with (path / f"failure-{secrets.token_hex(16)}.json").open("xb") as output:
                output.write(encoded)
            return
        if isinstance(source, NativeCommandFailure) and (source.tool, source.verb) == ("hdiutil", "create"):
            receipt = {"version": 1, "mode": mode, "point": point, "case_index": case_index,
                       "fixture": fixture.name, "size_mib": config["size_mib"],
                       "harness_sha256": config["harness_sha256"],
                       "github_sha": os.environ.get("GITHUB_SHA"),
                       "github_run_id": os.environ.get("GITHUB_RUN_ID"),
                       "github_run_attempt": os.environ.get("GITHUB_RUN_ATTEMPT"),
                       "tool": source.tool, "verb": source.verb, "returncode": source.returncode,
                       "timed_out": source.timed_out,
                       "stdout_head": source.stdout_head, "stdout_tail": source.stdout_tail,
                       "stderr_head": source.stderr_head, "stderr_tail": source.stderr_tail,
                       "stdout_open": source.stdout_open, "stderr_open": source.stderr_open,
                       "holder_outlived_release": source.holder_outlived_release,
                       "cleanup_state": "unknown"}
            encoded = json.dumps(receipt, ensure_ascii=False).encode("utf-8", errors="replace")
            require(len(encoded) <= 256 * 1024, "capacity diagnostic exceeds output bound")
            with (path / f"failure-{secrets.token_hex(16)}.json").open("xb") as output:
                output.write(encoded)
            return
        if isinstance(source, NativeCommandFailure) and (source.tool, source.verb) == ("hdiutil", "attach"):
            # Attach output may contain host details. Retain closed metadata
            # only, including whether an unowned pipe was still open.
            receipt = {"version": 1, "kind": "native-command-failed",
                       "mode": mode if mode in ("fresh", "existing") else "unknown",
                       "point": point if point in POINTS else "unknown",
                       "case_index": case_index if type(case_index) is int and 0 < case_index <= 1000 else None,
                       "harness_sha256": closed(config.get("harness_sha256"), r"[0-9a-f]{64}"),
                       "github_sha": closed(os.environ.get("GITHUB_SHA"), r"[0-9a-f]{40}"),
                       "github_run_id": closed(os.environ.get("GITHUB_RUN_ID"), r"[0-9]{1,20}"),
                       "github_run_attempt": closed(os.environ.get("GITHUB_RUN_ATTEMPT"), r"[0-9]{1,8}"),
                       "command_category": "hdiutil-attach",
                       "child_returncode": source.returncode,
                       "timed_out": source.timed_out,
                       "stdout_open": source.stdout_open, "stderr_open": source.stderr_open,
                       "cleanup_state": "unknown"}
            encoded = json.dumps(receipt, ensure_ascii=False).encode("utf-8")
            require(len(encoded) <= 16 * 1024, "capacity capture diagnostic exceeds output bound")
            with (path / f"failure-{secrets.token_hex(16)}.json").open("xb") as output:
                output.write(encoded)
            return
        source = source.__context__


def tools_available():
    names = {"linux": ["sudo", "unshare", "losetup", "mkfs.ext4", "mount", "umount", "findmnt"],
             "darwin": ["hdiutil", "diskutil"], "win32": ["powershell.exe"]}
    if sys.platform not in names:
        raise NotRun("native ext4, APFS or NTFS host required")
    missing = [name for name in names[sys.platform] if not shutil.which(name)]
    if missing:
        raise NotRun("required native image tools unavailable: " + ", ".join(missing))
    if sys.platform == "linux" and os.geteuid() != 0:
        if run("sudo", "-n", "true", check=False).returncode != 0:
            raise NotRun("private ext4 namespace requires noninteractive sudo on a qualification host")
    if sys.platform == "linux":
        if run("sudo", "-n", "unshare", "--mount", "--propagation", "private", "true", check=False).returncode != 0:
            raise NotRun("private native mount namespace unavailable")
    if sys.platform == "win32":
        import ctypes
        if not ctypes.windll.shell32.IsUserAnAdmin():
            raise NotRun("owned NTFS VHD requires an elevated qualification host")


def artifact():
    result = run("cargo", "test", "-p", "rrflow-cli", "--test", "installed_crash", "--features",
                 "crash-test", "--locked", "--no-run", "--message-format=json", timeout=1800)
    paths = {Path(row["executable"]).resolve(strict=True)
             for line in result.stdout.splitlines() if (row := json.loads(line)).get("reason") == "compiler-artifact"
             and row.get("executable") and row["target"]["name"] == "installed_crash" and row["profile"]["test"]}
    require(len(paths) == 1, "Cargo must identify one installed diagnostic harness")
    executable = paths.pop()
    require(run(str(executable), "--list").stdout.splitlines().count(f"{CASE}: test") == 1,
            "required capacity case absent")
    return executable


def loop_binding(device: str, image: Path):
    data = json.loads(run("losetup", "--json", "--output", "NAME,BACK-FILE", device).stdout)
    rows = data.get("loopdevices", [])
    require(len(rows) == 1 and rows[0]["name"] == device
            and Path(rows[0]["back-file"]).resolve(strict=True) == image, "loop device is not our image")


def native_command(*arguments, **kwargs):
    # These are owned-image OS commands only, never credential-bearing product
    # output. Keep the failing tool and bounded native message in CI evidence.
    # POSIX callers own a fresh process group. Its remaining members are
    # terminated on failure; an escaped pipe holder is reported, not killed.
    # A successful command must finish its group before it can be accepted.
    if os.name == "posix":
        kwargs["process_group"] = True
        kwargs["require_clean_process_group"] = True
    kwargs["capture_deadline_output"] = True
    try:
        result = run(*arguments, check=False, **kwargs)
    except CommandDeadlineExceeded as deadline:
        raise NativeCommandFailure(arguments[0], arguments[1], deadline.captured,
                                   timed_out=True) from deadline
    except CommandOutputNotClosed as unclosed:
        # A descendant holding the pipe must not hide this command's own failure.
        if unclosed.captured is None:
            raise
        raise NativeCommandFailure(arguments[0], arguments[1], unclosed.captured,
                                   unclosed.stdout_open, unclosed.stderr_open,
                                   timed_out=unclosed.timed_out) from unclosed
    if result.returncode != 0:
        raise NativeCommandFailure(arguments[0], arguments[1], result)
    return result


def is_busy_darwin_create(error):
    # A timed-out create is never retried: its work may still be running. One
    # whose output is still open is retried only after its holder is seen to
    # let go; `darwin_template` owns that wait.
    if not (isinstance(error, NativeCommandFailure)
            and sys.platform == "darwin"
            and (error.tool, error.verb) == ("hdiutil", "create")
            and not error.timed_out):
        return False
    native_output = (error.stderr_tail + "\n" + error.stdout_tail).lower()
    return "resource busy" in native_output


def held_output(error):
    """The open-output record behind a create whose output a process still holds."""
    if not (error.stdout_open or error.stderr_open):
        return None
    cause = error.__cause__
    return cause if isinstance(cause, CommandOutputNotClosed) else None


def associated_loops(image):
    # util-linux can return a successful empty stdout when no matching loop
    # remains after autoclear. Only that successful empty result means none.
    response = run("losetup", "--json", "--associated", str(image),
                   "--output", "NAME,BACK-FILE")
    return json.loads(response.stdout)["loopdevices"] if response.stdout.strip() else []


def windows_volume(action, fixture, size):
    from owned_windows_process import run_owned
    result = run_owned([str(Path(shutil.which("powershell.exe")).resolve()), "-NoLogo", "-NoProfile", "-NonInteractive", "-File",
                 str(SCRIPT.with_name("owned_capacity_volume.ps1")), "-Action", action,
                 "-Fixture", str(fixture), "-SizeMiB", str(size)], env=dict(os.environ), timeout=120)
    require(result.returncode == 0, "owned NTFS image control failed: " + result.stderr[-2048:])
    require(bool(result.stdout.strip()), f"owned NTFS {action} returned no structured receipt; stderr=" + result.stderr[-2048:])
    marker = "RRFLOW_CAPACITY_VOLUME="
    rows = [line.removeprefix(marker) for line in result.stdout.splitlines() if line.startswith(marker)]
    require(len(rows) == 1, f"owned NTFS {action} omitted its framed receipt: " + repr(result.stdout[:512]))
    try:
        return json.loads(rows[0])
    except json.JSONDecodeError as error:
        raise RuntimeError(f"owned NTFS {action} receipt malformed at column {error.colno}") from error


DARWIN_TEMPLATE = "capacity-template.dmg"


def darwin_template(fixture: Path, size: int) -> Path:
    """The suite's pristine APFS image: created once, never attached by the lab, removed at the end."""
    template = fixture / DARWIN_TEMPLATE
    if template.exists():
        require(template.is_file() and not template.is_symlink()
                and template.resolve().parent == fixture.resolve(), "owned APFS template escaped fixture")
        return template
    for attempt in range(5):
        try:
            native_command("hdiutil", "create", "-verbose", "-size", f"{size}m", "-fs", "APFS", "-type", "UDIF",
                "-volname", "RRFlowCapacity", str(template), timeout=120)
            break
        except NativeCommandFailure as error:
            if not is_busy_darwin_create(error) or attempt == 4:
                raise
            # A busy create can leave a partial image behind, possibly
            # still attached; the next attempt must start from no image,
            # and only an ordinary file inside the fixture is ever removed.
            leftover = template.exists()
            if leftover:
                require(template.is_file() and not template.is_symlink()
                        and template.resolve().parent == fixture.resolve(),
                        "owned APFS image escaped fixture")
                detach_owned_image(template)
            # Never create over a live holder. A create whose output stayed
            # open is retried only after the holder is seen to let go; if it
            # does not, the image stays for inspection and the failure is kept.
            released_ms = None
            if error.stdout_open or error.stderr_open:
                held = held_output(error)
                began = time.monotonic()
                if held is None or not held.wait_closed(HELPER_EXIT_SECONDS):
                    error.holder_outlived_release = True
                    raise
                released_ms = round((time.monotonic() - began) * 1000)
            if template.exists():
                template.unlink()
            print(json.dumps({"capacity_create_retry": {
                "attempt": attempt + 1, "leftover_image": leftover,
                "held_output": released_ms is not None, "released_ms": released_ms}}),
                flush=True)
            time.sleep(2 ** attempt)
    return template


@contextmanager
def volume(fixture: Path, size: int):
    require(fixture.name.startswith("rrflow-capacity-") and fixture == fixture.resolve(strict=True),
            "invalid fixture ownership")
    require(128 <= size <= 2048 and shutil.disk_usage(fixture).free >= 2 * size * 1024 * 1024,
            "insufficient declared host headroom for capped image")
    image = fixture / ("capacity.vhd" if os.name == "nt" else "capacity.dmg" if sys.platform == "darwin" else "capacity.img")
    root = fixture / "volume"
    root.mkdir()
    device = None
    attached = False
    try:
        if sys.platform == "linux":
            require(os.geteuid() == 0, "private namespace controller must own mount privilege")
            with image.open("xb") as output:
                output.truncate(size * 1024 * 1024)
            # Format only the new bounded regular image. mount's loop owner
            # uses autoclear, so namespace teardown cannot leave an attached
            # persistent loop device if the controller is forcibly terminated.
            run("mkfs.ext4", "-q", "-F", "-m", "0", "-N", "4096", str(image), timeout=60)
            run("mount", "-t", "ext4", "-o", "loop,nodev,nosuid", str(image), str(root))
            attached = True
            info = json.loads(run("findmnt", "--json", "--mountpoint", str(root), "--output", "SOURCE,FSTYPE,TARGET").stdout)["filesystems"]
            require(len(info) == 1 and info[0]["fstype"] == "ext4"
                    and Path(info[0]["target"]).resolve() == root, "owned ext4 attachment differs")
            device = info[0]["source"]
            require(device.startswith("/dev/loop") and device.removeprefix("/dev/loop").isdigit(), "invalid owned loop identity")
            loop_binding(device, image)
            inode_count = os.statvfs(root).f_files
            require(0 < inode_count <= 8192, "owned ext4 inode count exceeds bounded filler")
            identity = {"device": device, "image": str(image), "mount": info[0], "inodes": inode_count}
        elif sys.platform == "darwin":
            # hdiutil create attaches, formats and detaches a new image itself, and on
            # macOS runners that internal cycle intermittently fails "Resource busy" with
            # its helper still holding the output (POAM-074). It therefore runs once per
            # suite, for a pristine template the lab never attaches, and each case starts
            # from a fresh copy: a new, never-written filesystem without a per-case create.
            shutil.copyfile(darwin_template(fixture, size), image)
            attach = plistlib.loads(native_command("hdiutil", "attach", "-nobrowse", "-mountpoint", str(root),
                                        "-plist", str(image), timeout=60).stdout.encode())
            attached = True
            rows = [row for row in attach["system-entities"] if row.get("mount-point") == str(root)]
            require(len(rows) == 1, "owned APFS image has no exact mount")
            info = plistlib.loads(native_command("diskutil", "info", "-plist", str(root)).stdout.encode())
            require(info.get("FilesystemType", "").lower() == "apfs"
                    and info.get("DeviceNode") == rows[0]["dev-entry"], "APFS device identity differs")
            identity = {"image": str(image), "device": info["DeviceNode"], "volume_uuid": info["VolumeUUID"]}
        else:
            identity = windows_volume("Attach", fixture, size)
            root = Path(identity["root"]).resolve(strict=True)
            attached = True
            require(Path(identity["image"]).resolve(strict=True) == image, "NTFS image identity differs")
        root = root.resolve(strict=True)
        profile = filesystem_profile(root)
        require(profile["type"].lower() == {"linux": "ext4", "darwin": "apfs", "win32": "ntfs"}[sys.platform],
                "wrong native filesystem")
        require(os.stat(root).st_dev != os.stat(fixture).st_dev, "fixture did not become a separate filesystem")
        total = shutil.disk_usage(root).total
        require(32 * 1024 * 1024 < total <= size * 1024 * 1024, "native capacity exceeds owned image")
        nonce = secrets.token_hex(32)
        with (root / "RRFLOW-CAPACITY-OWNER").open("x", encoding="utf-8") as marker:
            marker.write(nonce)
            marker.flush()
            os.fsync(marker.fileno())
        descriptor = {"version": 1, "root": str(root), "nonce": nonce,
                      "filesystem": profile["type"].lower(), "max_bytes": size * 1024 * 1024}
        path = fixture / "volume.json"
        path.write_text(json.dumps(descriptor), encoding="utf-8")
        yield path, {**identity, **profile, "total_bytes": total, "max_bytes": descriptor["max_bytes"]}
    finally:
        # Never remove a directory tree or image while a device can reference it.
        # Attachment failures also require inspection: the OS may have attached
        # successfully before its control response was lost.
        if sys.platform == "linux":
            if image.exists():
                associated = associated_loops(image)
                require(len(associated) <= 1, "ambiguous owned loop attachment")
                mounted = run("findmnt", "--json", "--mountpoint", str(root), "--output", "SOURCE", check=False)
                if mounted.returncode == 0:
                    rows = json.loads(mounted.stdout)["filesystems"]
                    require(len(rows) == len(associated) == 1 and rows[0]["source"] == associated[0]["name"],
                            "refuse detachment of an unowned mount")
                    loop_binding(associated[0]["name"], image)
                    run("umount", str(root), timeout=30)
                require(run("findmnt", "--noheadings", "--mountpoint", str(root), check=False).returncode != 0,
                        "owned ext4 volume remains mounted")
                remaining = associated_loops(image)
                for row in remaining:
                    loop_binding(row["name"], image)
                    run("losetup", "--detach", row["name"])
                deadline = time.monotonic() + 10
                while associated_loops(image):
                    require(time.monotonic() < deadline, "owned loop remains attached")
                    time.sleep(0.05)
        elif sys.platform == "darwin":
            if image.exists():
                detach_owned_image(image)
        else:
            windows_volume("Detach", fixture, size)
        if attached:
            print(json.dumps({"capacity_cleanup": "detached", "image": str(image)}), flush=True)


def execute(executable: Path, descriptor: Path, point: str, mode: str):
    env = {**os.environ, "RRFLOW_CAPACITY_DESCRIPTOR": str(descriptor), "RRFLOW_CAPACITY_POINT": point,
           "RRFLOW_CAPACITY_MODE": mode, "RUST_BACKTRACE": "1"}
    args = [str(executable), "--exact", CASE, "--ignored", "--nocapture", "--quiet", "--test-threads=1"]
    if os.name == "nt":
        from owned_windows_process import run_owned
        result = run_owned(args, env=env, timeout=900)
    else:
        result = run(*args, env=env, timeout=900, process_group=True, check=False)
    print(result.stdout, end="", flush=True)
    print(result.stderr, end="", file=sys.stderr, flush=True)
    require(result.returncode == 0 and "test result: ok. 1 passed; 0 failed; 0 ignored;" in result.stdout,
            "installed native capacity case failed")
    rows = [json.loads(line.removeprefix("native_capacity_receipt=")) for line in result.stdout.splitlines()
            if line.startswith("native_capacity_receipt=")]
    require(len(rows) == 1 and rows[0]["point"] == point and rows[0]["mode"] == mode
            and not rows[0]["report"]["violations"] and rows[0]["report"]["unresolved"] == 0,
            "capacity receipt omitted or unresolved")
    return rows[0]


def cleanup_detached_volume(fixture):
    # Check every target before deleting any. Never traverse a mount, symlink
    # or unknown artifact; failure leaves the owned image available for review.
    require(fixture == fixture.resolve(strict=True) and fixture.name.startswith("rrflow-capacity-"),
            "cleanup target identity changed")
    mount = fixture / "volume"
    require(not mount.is_symlink() and not os.path.ismount(mount), "refuse cleanup of attached mount")
    if mount.exists():
        require(mount.is_dir() and not any(mount.iterdir()), "detached mount directory is not empty")
    paths = [fixture / name for name in
             ("capacity.img", "capacity.dmg", "capacity.vhd", "volume.json", "create-owned-vhd.txt")]
    for path in paths:
        require(not path.is_symlink(), "cleanup file is a symlink")
        if path.exists():
            require(path.is_file() and path.resolve().parent == fixture, "cleanup file escaped fixture")
    for path in paths:
        if path.exists():
            path.unlink()
    if mount.exists():
        mount.rmdir()


def cases(config, fixture):
    results = []
    case_index = 0
    for mode in config["modes"]:
        for point in config["points"]:
            case_index += 1
            # A new filesystem per case prevents prior exhaustion/deletion
            # allocation state from influencing the next native error oracle.
            try:
                with volume(fixture, config["size_mib"]) as (descriptor, identity):
                    receipt = execute(Path(config["executable"]), descriptor, point, mode)
                    results.append({"mode": mode, "point": point, "build": receipt["build"],
                                    "expected_sha256": receipt["expected_sha256"], "filesystem": identity})
                cleanup_detached_volume(fixture)
            except Exception as error:
                try:
                    retain_native_failure(config, fixture, mode, point, case_index, error)
                except Exception as diagnostic_error:
                    print("native capacity diagnostic retention failed: " + str(diagnostic_error)[:1024], file=sys.stderr)
                raise
    # This statement is reachable only after owned process and volume cleanup.
    print("native_capacity_suite=" + json.dumps({"version": 2, "status": "passed",
          "harness_sha256": config["harness_sha256"], "cases": results}), flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--size-mib", type=int, choices=range(128, 2049), default=256)
    parser.add_argument("--point", choices=POINTS, action="append")
    parser.add_argument("--mode", choices=["fresh", "existing"], action="append")
    parser.add_argument("--linux-owned-context", type=Path, help=argparse.SUPPRESS)
    args = parser.parse_args()
    if args.linux_owned_context:
        require(sys.platform == "linux", "private Linux context on wrong platform")
        fixture = args.linux_owned_context.resolve(strict=True)
        config = json.loads((fixture / "runner.json").read_text(encoding="utf-8"))
        require(os.geteuid() == 0 and os.readlink("/proc/self/ns/mnt") != config["parent_mount_namespace"],
                "native image controller requires a distinct mount namespace")
        require(sha256(Path(config["executable"])) == config["harness_sha256"], "harness changed before admission")
        cases(config, fixture)
        return
    tools_available()
    executable = artifact()
    fixture = Path(tempfile.mkdtemp(prefix="rrflow-capacity-")).resolve(strict=True)
    config = {"executable": str(executable), "harness_sha256": sha256(executable),
              "size_mib": args.size_mib, "points": args.point or list(POINTS), "modes": args.mode or ["fresh", "existing"]}
    if sys.platform == "linux":
        config["parent_mount_namespace"] = os.readlink("/proc/self/ns/mnt")
    (fixture / "runner.json").write_text(json.dumps(config), encoding="utf-8")
    complete = False
    try:
        if sys.platform == "linux":
            result = run("sudo", "-n", "unshare", "--mount", "--propagation", "private", sys.executable,
                         str(SCRIPT), "--linux-owned-context", str(fixture), timeout=7500,
                         process_group=True, check=False)
            print(result.stdout, end="")
            print(result.stderr, end="", file=sys.stderr)
            require(result.returncode == 0 and "native_capacity_suite=" in result.stdout, "private capacity qualification failed")
        else:
            cases(config, fixture)
        complete = True
    finally:
        # Failed cleanup retains the exact image for diagnosis. No blanket rmtree
        # may traverse a residual mount. Only known host-owned regular files and
        # an empty, detached mount directory are removed after success.
        if complete:
            cleanup_detached_volume(fixture)
            template = fixture / DARWIN_TEMPLATE
            if template.exists():
                require(template.is_file() and not template.is_symlink()
                        and template.resolve().parent == fixture, "owned APFS template escaped fixture")
                template.unlink()
            runner = fixture / "runner.json"
            require(runner.is_file() and not runner.is_symlink() and runner.resolve().parent == fixture,
                    "runner configuration escaped fixture")
            runner.unlink()
            fixture.rmdir()
        else:
            print(json.dumps({"capacity_fixture_retained": str(fixture), "status": "failed"}), file=sys.stderr)


if __name__ == "__main__":
    try:
        main()
    except NotRun as error:
        print(json.dumps({"native_capacity_status": "not_run", "reason": str(error)}), file=sys.stderr)
        raise SystemExit(2)
    except Exception as error:
        reason = str(error)
        if error.__context__ is not None:
            reason += "; preceding failure: " + str(error.__context__)[:2048]
        print(json.dumps({"native_capacity_status": "failed", "reason": reason}), file=sys.stderr)
        raise SystemExit(1)
