#!/usr/bin/env python3
"""Capture actual Cargo compilation inputs and embed them in the primary binary.

Two identical build commands are intentional: the first discovers compiled units;
the second embeds that observation and must reproduce the same unit closure.
This is source/build provenance, never a signed or qualified distribution.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import tomllib

ROOT = Path(__file__).resolve().parents[1]
MAX_MANIFEST_BYTES = 256 * 1024


def canonical(value: object) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True).encode()


def digest_file(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def command(args: list[str], root: Path = ROOT, env: dict | None = None) -> bytes:
    return subprocess.run(args, cwd=root, env=env, check=True, stdout=subprocess.PIPE).stdout


def source_snapshot(root: Path, allow_dirty: bool) -> dict:
    def git(*args: str) -> bytes:
        return command(["git", *args], root)
    dirty = bool(git("status", "--porcelain=v1", "--untracked-files=all"))
    if dirty and not allow_dirty:
        raise ValueError("dirty source is not admitted; --allow-dirty is debugging only")
    stages = {}
    for entry in git("ls-files", "--stage", "-z").split(b"\0"):
        if entry:
            header, name = entry.split(b"\t", 1)
            mode, oid, stage = header.split()
            if stage != b"0":
                raise ValueError("unmerged source is not admitted")
            stages[name] = (mode, oid)
    paths = sorted(set(git("ls-files", "--cached", "--others", "--exclude-standard", "-z").split(b"\0")) - {b""})
    content = hashlib.sha256(b"rrflow.build-source.v1\0")
    for name in paths:
        path = root / os.fsdecode(name)
        mode, oid = stages.get(name, (b"untracked", b""))
        if mode == b"160000":
            value = b"gitlink:" + oid
        elif path.is_symlink():
            value = b"symlink:" + os.fsencode(os.readlink(path))
        elif path.is_file():
            value = b"file:" + digest_file(path).encode()
        elif not path.exists():
            value = b"deleted"
        else:
            raise ValueError("unsupported build source entry")
        for field in (name, mode, value):
            content.update(len(field).to_bytes(8, "big"))
            content.update(field)
    return {
        "source_commit": git("rev-parse", "HEAD").decode().strip(),
        "source_tree": git("rev-parse", "HEAD^{tree}").decode().strip(),
        "source_content_sha256": content.hexdigest(),
        "dirty": dirty,
        "cargo_lock_sha256": digest_file(root / "Cargo.lock"),
    }


def check_profiles(document: dict) -> None:
    expected = {"inherits": "release", "debug": "line-tables-only", "strip": "none"}
    if document.get("profile", {}).get("diagnostic") != expected:
        raise ValueError("diagnostic profile must inherit release with symbols-only overrides")


def build_environment(env: dict[str, str]) -> str:
    names = {"RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "RUSTC", "RUSTC_WRAPPER",
             "RUSTC_WORKSPACE_WRAPPER", "CC", "CXX", "CFLAGS", "CXXFLAGS", "LDFLAGS"}
    selected = {k: v for k, v in env.items()
                if k in names or k.startswith(("CARGO_PROFILE_", "CARGO_TARGET_"))}
    return hashlib.sha256(canonical(selected)).hexdigest()


def compilation(messages: list[dict], packages: dict) -> tuple[list[dict], Path]:
    units = {}
    binary = None
    for message in messages:
        if message.get("reason") != "compiler-artifact":
            continue
        package = packages.get(message.get("package_id"))
        if not package:
            continue
        target, profile = message["target"], message["profile"]
        unit = {
            "package": package["name"], "version": package["version"],
            "target": target["name"], "kind": sorted(target["kind"]),
            "features": sorted(message["features"]), "opt_level": profile["opt_level"],
            "debug_assertions": profile["debug_assertions"],
            "overflow_checks": profile["overflow_checks"],
        }
        units[canonical(unit)] = unit
        if package["name"] == "rrflow-cli" and target["name"] == "rrflow" and target["kind"] == ["bin"]:
            if profile["test"] or not message.get("executable"):
                raise ValueError("expected primary product executable")
            binary = Path(message["executable"])
    if not units or binary is None:
        raise ValueError("Cargo did not report the primary binary and first-party units")
    return [units[key] for key in sorted(units)], binary


def require_same(before: object, after: object, label: str) -> None:
    if before != after:
        raise ValueError(f"{label} changed during provenance capture")


def build(args: list[str], env: dict, packages: dict) -> tuple[list[dict], Path]:
    process = subprocess.Popen(args, cwd=ROOT, env=env, stdout=subprocess.PIPE, text=True)
    messages = []
    assert process.stdout is not None
    for line in process.stdout:
        try:
            message = json.loads(line)
        except json.JSONDecodeError:
            print(line, end="", file=sys.stderr)
            continue
        if message.get("reason") == "compiler-message":
            print(message["message"].get("rendered", ""), end="", file=sys.stderr)
        elif message.get("reason") == "compiler-artifact":
            messages.append(message)
    if process.wait() != 0:
        raise RuntimeError("Cargo build failed; no build receipt was accepted")
    return compilation(messages, packages)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--profile", choices=["dev", "release", "diagnostic"], default="release")
    parser.add_argument("--target-dir", type=Path, default=ROOT / "target")
    parser.add_argument("--output", type=Path, required=True, help="new JSON receipt path (must not exist)")
    parser.add_argument("--allow-dirty", action="store_true")
    args = parser.parse_args()
    output = args.output.resolve()
    if output.exists():
        raise ValueError("receipt already exists; choose a new output path")
    check_profiles(tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8")))
    source = source_snapshot(ROOT, args.allow_dirty)
    env = dict(os.environ)
    env.pop("RRFLOW_BUILD_MANIFEST", None)
    # One native primary binary. Cross-compilation/distribution are later gates.
    rustc = command(["rustc", "--version"], env=env).decode().strip()
    verbose = command(["rustc", "-vV"], env=env).decode()
    host = next(line.removeprefix("host: ") for line in verbose.splitlines() if line.startswith("host: "))
    metadata = json.loads(command(["cargo", "metadata", "--locked", "--no-deps", "--format-version", "1"], env=env))
    packages = {p["id"]: p for p in metadata["packages"] if p["id"] in metadata["workspace_members"]}
    target_dir = args.target_dir.resolve()
    target_dir.mkdir(parents=True, exist_ok=True)
    cargo = ["cargo", "build", "--locked", "-p", "rrflow-cli", "--bin", "rrflow",
             "--profile", args.profile, "--target-dir", str(target_dir),
             "--message-format", "json"]
    units, _ = build(cargo, env, packages)
    require_same(source, source_snapshot(ROOT, args.allow_dirty), "source")
    provenance = {
        "format_version": 1, **source, "rustc": rustc, "target": host,
        "cargo_profile": args.profile, "build_environment_sha256": build_environment(env),
        "first_party_units": units,
    }
    manifest = canonical(provenance)
    if len(manifest) > MAX_MANIFEST_BYTES:
        raise ValueError("build manifest exceeds budget")
    # Keep generated inputs in the ignored target tree, outside source inventory.
    with tempfile.TemporaryDirectory(prefix="rrflow-build-", dir=target_dir) as staging:
        manifest_path = Path(staging) / "inputs.json"
        manifest_path.write_bytes(manifest)
        env["RRFLOW_BUILD_MANIFEST"] = str(manifest_path)
        actual_units, binary = build(cargo, env, packages)
        require_same(units, actual_units, "compiled feature/semantic profile closure")
        require_same(source, source_snapshot(ROOT, args.allow_dirty), "source")
        report = json.loads(command([str(binary), "--json", "version"], env=env))
        require_same(provenance, report["build"]["provenance"], "embedded provenance")
        require_same(digest_file(binary), report["build"]["executable_sha256"], "executable")
        output.parent.mkdir(parents=True, exist_ok=True)
        with output.open("xb") as receipt:
            receipt.write(canonical({"format_version": 1, "binary": str(binary), "report": report}) + b"\n")
    print(f"Built {binary}; unqualified provenance receipt: {output}")


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, RuntimeError, subprocess.CalledProcessError) as error:
        print(f"build identity rejected: {error}", file=sys.stderr)
        sys.exit(1)
