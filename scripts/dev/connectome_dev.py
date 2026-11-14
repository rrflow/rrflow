#!/usr/bin/env python3
"""Start the development loop: this checkout's engine, built in debug, under Connectome.

    python scripts/dev/connectome_dev.py            # build the engine, then open Connectome
    python scripts/dev/connectome_dev.py --check    # build and report what would run; open nothing

The Connectome desktop app owns the lifecycle from there: it installs, serves and
stops one engine process per project from its Project engines panel, through the
`rrflow` it finds on PATH. This script only makes that `rrflow` the one built from
this checkout and says exactly which binary that is. It installs nothing, serves
nothing and writes nothing into a project.

Cargo hard-links its output into `target/debug/deps`, and the engine refuses to
install from an executable with more than one link. The launcher therefore
stages a private copy under `target/dev-loop/` and runs that.

A debug rebuild changes the executable's SHA-256. An installation records the
executable it was applied with, so a project installed by an earlier build refuses
to open until it is installed again. The report names that digest so the refusal
is explainable; this script does not bypass it.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CONNECTOME = ROOT / "connectome"
ENGINE_NAME = "rrflow.exe" if sys.platform == "win32" else "rrflow"


class DevLoopError(RuntimeError):
    pass


def require(condition: bool, message: str) -> None:
    if not condition:
        raise DevLoopError(message)


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def target_directory(root: Path = ROOT) -> Path:
    """Cargo's own answer, so a configured target directory is honored."""
    completed = subprocess.run(["cargo", "metadata", "--format-version", "1", "--no-deps", "--locked"],
                               cwd=root, check=True, capture_output=True, text=True)
    return Path(json.loads(completed.stdout)["target_directory"])


def build_engine(root: Path = ROOT, jobs: int | None = None) -> Path:
    command = ["cargo", "build", "--locked", "-p", "rrflow-cli", "--bin", "rrflow"]
    if jobs:
        command += ["--jobs", str(jobs)]
    require(subprocess.run(command, cwd=root).returncode == 0, "the engine did not build")
    target = target_directory(root)
    built = target / "debug" / ENGINE_NAME
    require(built.is_file(), f"cargo reported success but {built} is absent")
    return stage_engine(built, target / "dev-loop")


def stage_engine(built: Path, directory: Path) -> Path:
    """Copy the built engine to a file of its own: one link, identical bytes."""
    directory.mkdir(parents=True, exist_ok=True)
    staged = directory / ENGINE_NAME
    pending = directory / f"{ENGINE_NAME}.pending"
    shutil.copy2(built, pending)
    os.replace(pending, staged)
    require(sha256(staged) == sha256(built), "the staged engine differs from the built engine")
    require(staged.stat().st_nlink == 1, "the staged engine must have exactly one link")
    return staged


def path_with_engine_first(engine: Path, current: str) -> str:
    """PATH with the engine's directory first, without a duplicate entry."""
    directory = str(engine.parent)
    rest = [entry for entry in current.split(os.pathsep) if entry and Path(entry) != engine.parent]
    return os.pathsep.join([directory, *rest])


def engine_on_path(environment: dict[str, str]) -> Path | None:
    found = shutil.which("rrflow", path=environment.get("PATH", ""))
    return Path(found).resolve() if found else None


def connectome_ready(connectome: Path = CONNECTOME) -> None:
    require((connectome / "package.json").is_file(),
            "the Connectome submodule is not checked out; run `git submodule update --init`")
    require((connectome / "node_modules").is_dir(),
            "Connectome's dependencies are not installed; run `pnpm install` in connectome/")
    require(shutil.which("pnpm") is not None, "pnpm is not on PATH")


def revision(directory: Path) -> str:
    return subprocess.run(["git", "rev-parse", "HEAD"], cwd=directory, check=True,
                          capture_output=True, text=True).stdout.strip()


def report(engine: Path, environment: dict[str, str]) -> dict[str, str]:
    resolved = engine_on_path(environment)
    require(resolved == engine.resolve(),
            f"PATH resolves rrflow to {resolved}, not the engine this checkout built ({engine})")
    version = subprocess.run([str(engine), "--json", "version"], check=True, capture_output=True, text=True)
    return {
        "engine": str(engine),
        "engine_sha256": sha256(engine),
        "engine_version": json.loads(version.stdout),
        "engine_revision": revision(ROOT),
        "connectome_revision": revision(CONNECTOME),
        "profile": "debug",
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--check", action="store_true", help="build and report; do not open Connectome")
    parser.add_argument("--jobs", type=int, help="cargo build jobs")
    args = parser.parse_args(argv)
    try:
        connectome_ready()
        engine = build_engine(jobs=args.jobs)
        environment = dict(os.environ, PATH=path_with_engine_first(engine, os.environ.get("PATH", "")))
        print(json.dumps(report(engine, environment), indent=2), flush=True)
        if args.check:
            return 0
        pnpm = shutil.which("pnpm")
        return subprocess.run([pnpm, "tauri:dev"], cwd=CONNECTOME, env=environment).returncode
    except (DevLoopError, subprocess.CalledProcessError, json.JSONDecodeError) as error:
        print(f"dev loop: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
