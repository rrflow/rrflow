#!/usr/bin/env python3
"""Require native mount adversaries with bounded, test-owned cleanup."""
import errno
import json
import os
from pathlib import Path
import plistlib
import re
import sys

from selected_tests import MOUNT_CROSSING_LINUX, MOUNT_CROSSING_OTHER
import tempfile
import time

from run_offline_lifecycle import require, run


def detach_owned_image(image: Path) -> None:
    # Only this fixture's currently verified attachment can be detached. Never
    # cache a device name across retries: macOS can recycle those names.
    deadline = time.monotonic() + 30
    original_device = None
    attempts = 0
    detach_succeeded = False
    last_error = ""

    def command(*arguments):
        remaining = deadline - time.monotonic()
        require(remaining > 0, "owned image cleanup deadline exceeded: " + last_error)
        result = run("/usr/bin/hdiutil", *arguments, timeout=remaining,
                     process_group=True, check=False)
        return result

    while True:
        info = command("info", "-plist")
        require(info.returncode == 0, "owned image inspection failed: " + info.stderr[-2048:])
        rows = plistlib.loads(info.stdout.encode()).get("images", [])
        matches = [row for row in rows if Path(row.get("image-path", "")).resolve() == image]
        require(len(matches) <= 1, "ambiguous owned image identity")
        if not matches:
            return  # Verified absent, including a lost/busy detach response.
        require(not detach_succeeded, "owned image remains attached after successful detach")
        devices = [row["dev-entry"] for row in matches[0].get("system-entities", [])
                   if re.fullmatch(r"/dev/disk[0-9]+", row.get("dev-entry", ""))]
        require(bool(devices), "owned image has no whole-device identity")
        # The attachment precedes any synthesized APFS container device.
        require(original_device is None or original_device == devices[0],
                "owned image attachment identity changed during cleanup")
        original_device = devices[0]
        require(attempts < 5, "owned image remains attached after bounded cleanup: " + last_error)
        result = command("detach", "-force", original_device)
        attempts += 1
        detach_succeeded = result.returncode == 0
        last_error = f"hdiutil detach ({result.returncode}): " + result.stderr[-2048:] + result.stdout[-2048:]
        # Darwin EBUSY=16 is the only retryable command failure. A success also
        # needs a fresh absence observation; it never authorizes blind deletion.
        require(result.returncode in (0, 16), last_error)
        if result.returncode == 16 and attempts < 5:
            remaining = deadline - time.monotonic()
            require(remaining > 0, "owned image cleanup deadline exceeded: " + last_error)
            # Give asynchronous native teardown time to settle instead of
            # exhausting all five attempts in a tight burst. The same deadline
            # and attempt cap apply; the next iteration rechecks ownership.
            time.sleep(min(2 ** (attempts - 1), remaining))


def cleanup_owned_mount_fixture(fixture: Path) -> None:
    """Remove only known host paths; never enumerate a mounted volume."""
    project = fixture / "project"
    mount = project / "mounted"
    image = fixture / "fixture.sparseimage"
    pending = project / "pending"
    require(fixture.is_dir() and not fixture.is_symlink(), "invalid fixture root")
    require(project.is_dir() and not project.is_symlink(), "invalid fixture project")
    require(mount.is_dir() and not mount.is_symlink(), "invalid fixture mountpoint")
    require({entry.name for entry in fixture.iterdir()} <= {"project", "fixture.sparseimage"},
            "unexpected fixture entries; retained")
    project_entries = {entry.name for entry in project.iterdir()}
    require("mounted" in project_entries and project_entries <= {"mounted", "pending"},
            "unexpected project entries; retained")
    require(not pending.is_symlink() and (not pending.exists() or pending.is_file()),
            "invalid pending file; retained")
    require(not image.is_symlink() and (not image.exists() or image.is_file()),
            "invalid fixture image; retained")
    deadline = time.monotonic() + 30
    for attempt in range(5):
        require(time.monotonic() < deadline, "fixture mount cleanup deadline exceeded; retained")
        require(not mount.is_symlink(), "fixture mountpoint changed; retained")
        try:
            # rmdir is non-recursive and must fail on a busy or nonempty mount.
            mount.rmdir()
            break
        except OSError as error:
            if error.errno not in (errno.EBUSY, errno.ENOTEMPTY) or attempt == 4:
                raise
            remaining = deadline - time.monotonic()
            require(remaining > 0, "fixture mount cleanup deadline exceeded; retained")
            time.sleep(min(2 ** attempt, remaining))
    if pending.exists():
        require(not pending.is_symlink() and pending.is_file(), "pending file changed; retained")
        pending.unlink()
    project.rmdir()
    if image.exists():
        require(not image.is_symlink() and image.is_file(), "fixture image changed; retained")
        image.unlink()
    fixture.rmdir()


def main() -> None:
    require(sys.platform in {"linux", "darwin"}, "mount qualification requires native Linux or macOS")
    build = run("cargo", "test", "--locked", "-p", "rrd-lsm", "--test", "filesystem",
                "--no-run", "--message-format=json", timeout=600, process_group=True)
    artifacts = {
        Path(row["executable"]).resolve(strict=True)
        for line in build.stdout.splitlines()
        if (row := json.loads(line)).get("reason") == "compiler-artifact"
        and row.get("executable") and row["target"]["name"] == "filesystem"
        and row["profile"]["test"]
    }
    require(len(artifacts) == 1, "Cargo must identify one filesystem test executable")
    executable = str(artifacts.pop())
    case = MOUNT_CROSSING_LINUX if sys.platform == "linux" else MOUNT_CROSSING_OTHER
    listing = run(executable, "--list").stdout
    require(listing.splitlines().count(f"{case}: test") == 1, "required mount case is absent")
    args = [executable, "--ignored", "--exact", case, "--nocapture"]
    env = {**os.environ, "RUST_BACKTRACE": "1", "RRFLOW_PRIVATE_MOUNT_FIXTURE": "1"}
    if sys.platform == "linux":
        # Private propagation prevents fixture mounts reaching the host.
        result = run("sudo", "-n", "timeout", "--kill-after=5s", "60s", "unshare", "--mount",
                     "--propagation", "private", "env", "RRFLOW_PRIVATE_MOUNT_FIXTURE=1",
                     *args, timeout=75, env=env, process_group=True, check=False)
    else:
        fixture = Path(tempfile.mkdtemp(prefix="rrflow-native-mount-")).resolve(strict=True)
        image = fixture / "fixture.sparseimage"
        (fixture / "project" / "mounted").mkdir(parents=True)
        try:
            run("/usr/bin/hdiutil", "create", "-size", "64m", "-fs", "APFS", "-type", "SPARSE",
                "-volname", "RRFlowFixture", "-quiet", str(image), timeout=60, process_group=True)
            result = run(*args, timeout=90, process_group=True, check=False,
                         env={**env, "RRFLOW_MAC_MOUNT_FIXTURE": str(fixture)})
            # Preserve the test result even when later fixture cleanup fails.
            print(result.stdout, end="", flush=True)
            print(result.stderr, end="", file=sys.stderr, flush=True)
        finally:
            detach_owned_image(image)
            cleanup_owned_mount_fixture(fixture)
    if sys.platform == "linux":
        print(result.stdout, end="")
        print(result.stderr, end="", file=sys.stderr)
    require(result.returncode == 0 and f"test {case} ... ok" in result.stdout
            and "1 passed; 0 failed; 0 ignored;" in result.stdout,
            "required native mount adversary did not pass")


if __name__ == "__main__":
    main()
