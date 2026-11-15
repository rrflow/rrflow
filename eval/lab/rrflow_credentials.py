#!/usr/bin/env python3
"""Print shell exports for an installed RRFlow engram's lab credentials.

Only run_system.sh should call this, inside an eval. It reads the installed locator
and the owner-only API key document it names, and writes neither anywhere.
"""
import json
import shlex
import sys
import tomllib
from pathlib import Path

LOCATOR = ".rrflow/config.toml"


def main():
    project = Path(sys.argv[1])
    locator = tomllib.loads((project / LOCATOR).read_text(encoding="utf-8"))
    key = json.loads((project / locator["operator_credential"]).read_text(encoding="utf-8"))
    for name, value in (
        ("RRFLOW_LAB_PRINCIPAL", key["principal_id"]),
        ("RRFLOW_LAB_SECRET", key["credential"]),
        ("RRFLOW_LAB_INSTANCE", locator["identity"]["instance_id"]),
    ):
        print(f"export {name}={shlex.quote(value)}")


if __name__ == "__main__":
    main()
