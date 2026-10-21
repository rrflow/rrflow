#!/usr/bin/env python3
"""Run the required bind-mount adversary only in an isolated Linux namespace."""
import json
import os
from pathlib import Path
import subprocess
import sys


def main() -> None:
    if sys.platform != "linux":
        raise RuntimeError("bind-mount qualification requires native Linux")
    build = subprocess.run(
        ["cargo", "test", "--locked", "-p", "rrd-lsm", "--test", "filesystem",
         "--no-run", "--message-format=json"],
        check=True, stdout=subprocess.PIPE, text=True, timeout=600,
    )
    artifacts = {
        Path(row["executable"]).resolve(strict=True)
        for line in build.stdout.splitlines()
        if (row := json.loads(line)).get("reason") == "compiler-artifact"
        and row.get("executable") and row["target"]["name"] == "filesystem"
        and row["profile"]["test"]
    }
    if len(artifacts) != 1:
        raise RuntimeError("Cargo did not identify exactly one filesystem test executable")
    # Private propagation prevents any fixture mount reaching the host namespace.
    # timeout also bounds a blocked native syscall and kills the isolated test.
    subprocess.run(
        ["sudo", "-n", "timeout", "--kill-after=5s", "60s", "unshare", "--mount",
         "--propagation", "private", "env", "RRFLOW_PRIVATE_MOUNT_FIXTURE=1",
         str(artifacts.pop()), "--ignored", "--exact",
         "bind_mount_crossing_is_denied_before_any_effect", "--nocapture"],
        check=True, timeout=75, env={**os.environ, "RUST_BACKTRACE": "1"},
    )


if __name__ == "__main__":
    main()
