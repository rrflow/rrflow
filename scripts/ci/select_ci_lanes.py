#!/usr/bin/env python3
"""Choose which candidate CI lanes a change needs.

Candidates run the development gate: light, Linux-only lanes proportional to
the change. A documentation edit runs the documentation checks; a script,
workflow or repository-policy edit runs the script and CI-policy tests; a crate
edit tests that crate and compiles everything that depends on it; a workspace
manifest, lock file or toolchain change compiles the whole workspace. A path
this module does not recognize runs every light lane.

Compiling a dependent cannot see a golden file in it going stale. A crate
whose tests pin another crate's public identity is therefore tested, not only
compiled, whenever that crate changes: see `PINNED_BY`.

No changed path selects the full matrix. The full matrix is release
qualification (native installed lifecycle on three operating systems, crash
injection, reopen and capacity): it runs nightly against main and on demand
(`scope: full`), and a failure there sounds the main alarm instead of holding
a candidate.

The classifier is pure: it maps a list of repository-relative paths to lanes.
Dependents are resolved from `cargo metadata`, which the caller supplies or
this module runs. The workflow reads the printed `key=value` lines into job
outputs; `ci-gate` then requires every selected lane to succeed and every
unselected lane to be skipped.
"""

from __future__ import annotations

import argparse
import fnmatch
import json
import subprocess
import sys
from pathlib import Path, PurePosixPath

from workspace_packages import member_directories, package_of

ROOT = Path(__file__).resolve().parents[2]
LANES = ("full", "docs", "scripts", "engine", "sdk")

# Inputs every crate builds against: the whole workspace is compiled, and the
# nightly full matrix tests it.
WORKSPACE_PATTERNS = (
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain.toml",
    ".cargo/**",
)
# Workflows, CI scripts and repository policy files: the script lane runs the
# script tests and the workflow policy checks.
POLICY_PATTERNS = (
    ".github/workflows/*",
    ".github/workflows/**",
    ".devcontainer/**",
    ".gitmodules",
    ".gitignore",
    ".gitattributes",
    "connectome",
    "VERSION",
    "deploy/**",
)
SDK_PATTERNS = ("sdks/**", "fixtures/**", "crates/transport/**")
ENGINE_PATTERNS = ("crates/**",)
SCRIPTS_PATTERNS = ("scripts/**", "eval/**")
DOCS_PATTERNS = (
    "docs/**",
    "*.md",
    "**/*.md",
    ".github/ISSUE_TEMPLATE/**",
    ".github/PULL_REQUEST_TEMPLATE.md",
    "LICENSE",
)
# Owner crate -> crates whose tests hold a golden derived from the owner's
# public identity. A change to the owner tests them too. Without this, the
# 2026-11-14 catalogue change passed its gate while the storage qualification
# fixture's `public_catalogue_sha256` went stale; the next push that touched a
# crate reading that fixture found main already broken.
PINNED_BY: dict[str, tuple[str, ...]] = {
    "rrd-contract": ("rrflow-qualification-storage",),
}
# Crate -> the workspace binary of another package that its tests execute.
# `cargo test -p <crate>` builds only that crate's own binaries, so the lane
# builds these first. The first push that changed rrd-server under the
# development gate failed for want of one (run 43665714352).
TEST_BINARIES: dict[str, tuple[str, str]] = {
    "rrd-server": ("rrflow-cli", "rrd-engram-controller"),
}


def matches(path: str, patterns: tuple[str, ...]) -> bool:
    posix = PurePosixPath(path).as_posix()
    for pattern in patterns:
        if fnmatch.fnmatchcase(posix, pattern):
            return True
        # fnmatch's `*` crosses `/`, so `a/**` also needs the directory prefix.
        if pattern.endswith("/**") and posix.startswith(pattern[:-2]):
            return True
    return False


def crate_of(path: str, root: Path = ROOT) -> str | None:
    """The workspace package that owns `path`, from the workspace member list at any depth."""
    if not path.startswith("crates/"):
        return None
    return package_of(path, member_directories(root))


def workspace_graph(metadata: dict) -> dict[str, set[str]]:
    """Reverse dependency edges between workspace members: package -> dependents."""
    members = {package["id"]: package for package in metadata["packages"]}
    member_ids = set(metadata.get("workspace_members", members))
    names = {package["name"] for pid, package in members.items() if pid in member_ids}
    dependents: dict[str, set[str]] = {name: set() for name in names}
    for pid, package in members.items():
        if pid not in member_ids:
            continue
        for dependency in package.get("dependencies", []):
            if dependency["name"] in names:
                dependents[dependency["name"]].add(package["name"])
    return dependents


def with_dependents(packages: set[str], dependents: dict[str, set[str]]) -> set[str]:
    selected = set(packages)
    frontier = list(packages)
    while frontier:
        for dependent in dependents.get(frontier.pop(), ()):
            if dependent not in selected:
                selected.add(dependent)
                frontier.append(dependent)
    return selected


def cargo_metadata(root: Path = ROOT) -> dict:
    completed = subprocess.run(
        ["cargo", "metadata", "--no-deps", "--format-version", "1", "--locked"],
        cwd=root, check=True, capture_output=True, text=True,
    )
    return json.loads(completed.stdout)


def select(paths: list[str], *, metadata: dict | None = None, root: Path = ROOT,
           force_full: bool = False) -> dict[str, object]:
    """Return lane flags and the cargo package selection for the engine lane."""
    lanes = {lane: False for lane in LANES}
    packages: set[str] = set()
    unknown: list[str] = []
    workspace = False
    for raw in paths:
        path = raw.strip().replace("\\", "/")
        if not path:
            continue
        known = False
        if matches(path, WORKSPACE_PATTERNS):
            lanes["engine"] = True
            workspace = True
            known = True
        if matches(path, POLICY_PATTERNS):
            lanes["scripts"] = True
            known = True
        if matches(path, SDK_PATTERNS):
            lanes["sdk"] = True
            known = True
        if matches(path, ENGINE_PATTERNS):
            lanes["engine"] = True
            known = True
            crate = crate_of(path, root)
            if crate is None:
                # A crate directory this checkout does not know: compile everything.
                workspace = True
                unknown.append(path)
            else:
                packages.add(crate)
        if matches(path, SCRIPTS_PATTERNS):
            lanes["scripts"] = True
            known = True
        if matches(path, DOCS_PATTERNS):
            lanes["docs"] = True
            known = True
        if not known:
            # Unrecognized: every light lane runs, and the workspace compiles.
            for lane in ("docs", "scripts", "engine", "sdk"):
                lanes[lane] = True
            workspace = True
            unknown.append(path)
    selection, dependents, binaries = "", "", ""
    if force_full:
        # The full matrix already covers every light lane.
        lanes = {lane: lane == "full" for lane in LANES}
    elif lanes["engine"]:
        packages |= {pinner for owner in sorted(packages) for pinner in PINNED_BY.get(owner, ())}
        selection = " ".join(f"-p {name}" for name in sorted(packages))
        binaries = " ".join(
            f"-p {package} --bin {binary}"
            for package, binary in sorted({TEST_BINARIES[name] for name in packages if name in TEST_BINARIES})
        )
        if workspace:
            dependents = "--workspace"
        elif packages:
            graph = workspace_graph(metadata if metadata is not None else cargo_metadata(root))
            closure = with_dependents(packages, graph) - packages
            dependents = " ".join(f"-p {name}" for name in sorted(closure))
    return {**lanes, "packages": selection, "dependents": dependents, "binaries": binaries,
            "unknown": sorted(unknown)}


def changed_paths(base: str, head: str, root: Path = ROOT) -> list[str]:
    merge_base = subprocess.run(["git", "merge-base", base, head], cwd=root, check=True,
                                capture_output=True, text=True).stdout.strip()
    completed = subprocess.run(["git", "diff", "--name-only", merge_base, head], cwd=root,
                               check=True, capture_output=True, text=True)
    return [line for line in completed.stdout.splitlines() if line]


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--base", help="base revision; with --head, the diff decides the lanes")
    parser.add_argument("--head", default="HEAD")
    parser.add_argument("--full", action="store_true", help="select the full matrix regardless of paths")
    parser.add_argument("--paths-from", type=Path, help="read changed paths from a file, one per line")
    parser.add_argument("--github-output", type=Path, help="append key=value lines to this file")
    args = parser.parse_args(argv)
    if args.full:
        paths: list[str] = []
    elif args.paths_from is not None:
        paths = [line for line in args.paths_from.read_text(encoding="utf-8").splitlines() if line.strip()]
    elif args.base:
        paths = changed_paths(args.base, args.head)
    else:
        parser.error("one of --full, --paths-from or --base is required")
    result = select(paths, force_full=args.full)
    lines = [f"{lane}={'true' if result[lane] else 'false'}" for lane in LANES]
    lines.append(f"packages={result['packages']}")
    lines.append(f"dependents={result['dependents']}")
    lines.append(f"binaries={result['binaries']}")
    for line in lines:
        print(line)
    if result["unknown"]:
        print("unknown paths routed to every light lane:", *result["unknown"], sep="\n  ", file=sys.stderr)
    if args.github_output is not None:
        with args.github_output.open("a", encoding="utf-8") as handle:
            handle.write("\n".join(lines) + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
