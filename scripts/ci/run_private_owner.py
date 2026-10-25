#!/usr/bin/env python3
"""Execute the required native foreign-owner case; unavailable privilege fails."""
import json
import os
from pathlib import Path
import sys

from run_offline_lifecycle import require, run, sha256


def main():
    require(sys.platform in {"linux", "darwin", "win32"}, "unsupported native ownership platform")
    build = run("cargo", "test", "--locked", "-p", "rrd-engine", "--lib", "--no-run",
                "--message-format=json", timeout=900)
    artifacts = {Path(row["executable"]).resolve(strict=True)
                 for line in build.stdout.splitlines()
                 if (row := json.loads(line)).get("reason") == "compiler-artifact"
                 and row.get("executable") and row["target"]["name"] == "rrd_engine"
                 and row["profile"]["test"]}
    require(len(artifacts) == 1, "expected exactly one native engine test harness")
    harness = artifacts.pop()
    listing = run(str(harness), "foreign_owner_is_denied_without_repair", "--list", "--format=terse")
    cases = [line.removesuffix(": test") for line in listing.stdout.splitlines() if line.endswith(": test")]
    require(len(cases) == 1, "required native foreign-owner fixture is absent or ambiguous")
    arguments = [str(harness), "--ignored", "--exact", cases[0], "--nocapture"]
    if sys.platform != "win32":
        arguments = ["sudo", "-n", "env", "RRFLOW_PRIVATE_OWNER_FIXTURE=1", *arguments]
    result = run(*arguments, timeout=30, env={**os.environ, "RRFLOW_PRIVATE_OWNER_FIXTURE": "1"})
    require(f"test {cases[0]} ... ok" in result.stdout and
            "1 passed; 0 failed; 0 ignored; 0 measured;" in result.stdout,
            "required native ownership fixture did not execute successfully")
    print(json.dumps({"platform": sys.platform, "case": cases[0], "passed": 1,
                      "ignored": 0, "harness_sha256": sha256(harness)}))


if __name__ == "__main__":
    main()
