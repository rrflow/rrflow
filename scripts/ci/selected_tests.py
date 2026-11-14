"""The one owner of every test name CI selects, and the static resolver that checks them.

CI chooses tests by name in three places: Python harness constants, `cargo test`
filters in the workflows, and Rust tests that respawn themselves by name. A test's
name is its module path, so moving its body into a child module renames it, and a
filter that matches nothing passes silently. This module resolves names from the
Rust module tree without compiling anything; `check_selected_tests.py` runs it.
"""
from __future__ import annotations

import re
from dataclasses import dataclass, field
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]

# ---------------------------------------------------------------------------
# Selectors owned here. Harness scripts import these names instead of copying them.
# ---------------------------------------------------------------------------

LIFECYCLE_CASES = frozenset({
    "commands::primary_help_exposes_only_the_installed_product_lifecycle",
    "commands::raw_database_and_development_commands_fail_during_argument_parsing",
    "plans::install_requires_an_explicit_plan_digest_and_verify_is_read_only_by_shape",
    "plans::installed_plan_accepts_selected_attunement_and_rejects_tampering",
    "plans::fresh_installed_engine_survives_restart_and_restore",
    "plans::installed_engine_plans_applies_serves_discovers_authenticates_and_reopens",
    "engrams_and_history::a_served_engram_is_found_proved_and_not_served_twice",
    "commands::repository_discovery_is_read_only_without_an_installed_engine",
    "plans::installed_source_read_choice_is_explicit_and_reopens",
    "engrams_and_history::configured_cold_history_publishes_and_reads_after_process_restart",
    "adaptive_installed::selected_inventory_completes_and_replans_across_primary_process_restarts",
})
NATIVE_CAPACITY_CASE = "capacity::cases::installed_native_capacity_recovers_acknowledged_history"
SUSTAINED_CAPACITY_CASE = "sustained::campaign::installed_sustained_capacity"
NATIVE_SAMPLE_MEMBERSHIP = "sustained::native_worker::tests::native_job_contains_engine_harness_and_worker"
NATIVE_SAMPLE_ABRUPT = "sustained::native_worker::tests::native_job_abrupt_parent_fixture"
NATIVE_SAMPLE_TIMEOUT = "sustained::native_worker::tests::native_allocation_timeout_cleanup_fixture"
NATIVE_SAMPLE_HELPER = "sustained::native_worker::native_sample_helper"
MOUNT_CROSSING_LINUX = "bind_mount_crossing_is_denied_before_any_effect"
MOUNT_CROSSING_OTHER = "mounted_volume_crossing_is_denied_before_any_effect"


@dataclass(frozen=True)
class Selector:
    package: str
    target: str  # an integration-test name, or "lib"
    name: str
    exact: bool = True
    owner: str = ""


SELECTORS: tuple[Selector, ...] = (
    *(Selector("rrflow-cli", "installed_lifecycle", case, owner="run_offline_lifecycle.py")
      for case in sorted(LIFECYCLE_CASES)),
    Selector("rrflow-cli", "installed_crash", NATIVE_CAPACITY_CASE, owner="run_native_capacity.py"),
    Selector("rrflow-cli", "installed_reopen", SUSTAINED_CAPACITY_CASE, owner="run_sustained_capacity.py"),
    Selector("rrflow-cli", "installed_reopen", NATIVE_SAMPLE_MEMBERSHIP, owner="native sample tests"),
    Selector("rrflow-cli", "installed_reopen", NATIVE_SAMPLE_ABRUPT, owner="native sample tests"),
    Selector("rrflow-cli", "installed_reopen", NATIVE_SAMPLE_TIMEOUT, owner="native sample tests"),
    Selector("rrflow-cli", "installed_reopen", NATIVE_SAMPLE_HELPER, owner="native sample tests"),
    Selector("rrd-lsm", "filesystem", MOUNT_CROSSING_LINUX, owner="run_filesystem_containment.py"),
    Selector("rrd-lsm", "filesystem", MOUNT_CROSSING_OTHER, owner="run_filesystem_containment.py"),
)

# Targets whose complete test set CI requires to equal a declared set.
EXACT_TEST_SETS = {("rrflow-cli", "installed_lifecycle"): LIFECYCLE_CASES}

# ---------------------------------------------------------------------------
# Static Rust module resolution.
# ---------------------------------------------------------------------------

_TOKEN = re.compile(
    r"""
    (?P<comment>//[^\n]*|/\*.*?\*/)
  | (?P<raw>b?r(?P<hashes>\#*)"(?P<rawbody>.*?)"(?P=hashes))
  | (?P<string>b?"(?P<body>(?:\\.|[^"\\])*)")
  | (?P<char>b?'(?:\\.|[^'\\\n])')
  | (?P<lifetime>'[A-Za-z_][A-Za-z0-9_]*)
  | (?P<ident>[A-Za-z_][A-Za-z0-9_]*)
  | (?P<punct>[{}\[\]();\#=!:,])
  | (?P<other>\S)
    """,
    re.VERBOSE | re.DOTALL,
)


def _tokens(text: str):
    for match in _TOKEN.finditer(text):
        kind = match.lastgroup
        if kind == "comment":
            continue
        if kind == "raw":
            yield ("str", match.group("rawbody"))
        elif kind == "string":
            yield ("str", match.group("body"))
        elif kind in ("char", "lifetime", "other"):
            yield ("other", match.group(0))
        else:
            yield (kind, match.group(0))


@dataclass
class Target:
    tests: set[str] = field(default_factory=set)
    modules: set[str] = field(default_factory=set)
    files: list[Path] = field(default_factory=list)
    literals: list[tuple[Path, str]] = field(default_factory=list)


def _attribute_text(tokens, start):
    """Return (compact text, index after it) for an attribute starting at tokens[start] == '#'."""
    index = start + 1
    if index < len(tokens) and tokens[index][1] == "!":
        index += 1
    if index >= len(tokens) or tokens[index][1] != "[":
        return None, start + 1
    depth, parts = 0, []
    while index < len(tokens):
        kind, value = tokens[index]
        if kind == "punct" and value == "[":
            depth += 1
        elif kind == "punct" and value == "]":
            depth -= 1
            if depth == 0:
                return "".join(f'"{v}"' if k == "str" else v for k, v in parts[1:]), index + 1
        parts.append((kind, value))
        index += 1
    return None, index


def _is_test_attribute(text: str) -> bool:
    return text == "test" or text.endswith("::test") or "::test(" in text


def _child_file(current: Path, name: str, path_attr: str | None, mod_rs: bool, inline: list[str]) -> tuple[Path, bool]:
    if path_attr is not None:
        base = current.parent.joinpath(*inline)
        return (base / path_attr).resolve(), True
    base = current.parent if mod_rs else current.parent / current.stem
    base = base.joinpath(*inline)
    flat, nested = base / f"{name}.rs", base / name / "mod.rs"
    if flat.exists():
        return flat, False
    if nested.exists():
        return nested, True
    raise FileNotFoundError(f"{current}: module `{name}` has no file at {flat} or {nested}")


def _walk(path: Path, prefix: list[str], mod_rs: bool, target: Target) -> None:
    target.files.append(path)
    tokens = list(_tokens(path.read_text(encoding="utf-8")))
    stack: list[tuple[int, str]] = []  # (brace depth when an inline module opened, its name)
    depth = 0
    pending: list[str] = []  # attributes awaiting the item they annotate
    index = 0
    while index < len(tokens):
        kind, value = tokens[index]
        if kind == "punct" and value == "#":
            text, index = _attribute_text(tokens, index)
            if text is not None:
                pending.append(text)
            continue
        if kind == "str":
            if "::" in value:
                target.literals.append((path, value))
        elif kind == "ident" and value == "mod" and index + 1 < len(tokens) and tokens[index + 1][0] == "ident":
            name = tokens[index + 1][1]
            inline = [entry[1] for entry in stack]
            module_path = prefix + inline + [name]
            target.modules.add("::".join(module_path))
            following = tokens[index + 2][1] if index + 2 < len(tokens) else ""
            path_attr = next((re.search(r'"(.*)"', a).group(1) for a in pending if a.startswith("path=")), None)
            pending = []
            if following == ";":
                child, child_mod_rs = _child_file(path, name, path_attr, mod_rs, inline)
                _walk(child, module_path, child_mod_rs, target)
                index += 3
                continue
            if following == "{":
                stack.append((depth, name))
                depth += 1
                index += 3
                continue
        elif kind == "ident" and value == "fn" and index + 1 < len(tokens) and tokens[index + 1][0] == "ident":
            if any(_is_test_attribute(a) for a in pending):
                target.tests.add("::".join(prefix + [entry[1] for entry in stack] + [tokens[index + 1][1]]))
            pending = []
        elif kind == "punct" and value == "{":
            depth += 1
            pending = []
        elif kind == "punct" and value == "}":
            depth -= 1
            if stack and stack[-1][0] == depth:
                stack.pop()
            pending = []
        elif kind == "punct" and value == ";":
            pending = []
        index += 1


def package_dir(package: str) -> Path:
    for manifest in sorted(ROOT.joinpath("crates").rglob("Cargo.toml")):
        if "target" in manifest.relative_to(ROOT).parts:
            continue
        if re.search(rf'(?m)^name\s*=\s*"{re.escape(package)}"', manifest.read_text(encoding="utf-8")):
            return manifest.parent
    raise LookupError(f"no workspace package named {package}")


def resolve_target(package: str, target: str) -> Target:
    directory = package_dir(package)
    if target == "lib":
        root = directory / "src" / "lib.rs"
    else:
        flat, nested = directory / "tests" / f"{target}.rs", directory / "tests" / target / "main.rs"
        root = flat if flat.exists() else nested
    if not root.exists():
        raise LookupError(f"{package}: no test target `{target}`")
    resolved = Target()
    _walk(root, [], True, resolved)
    return resolved


def matches(target: Target, name: str, exact: bool) -> list[str]:
    if exact:
        return [name] if name in target.tests else []
    return sorted(test for test in target.tests if name in test)


_WORKFLOW_TEST = re.compile(
    r"cargo (?:test|nextest run)\b(?P<args>[^\n|&;]*?)\s--\s(?P<rest>[^\n|&;]*)")


def workflow_selectors(text: str) -> list[Selector]:
    """`cargo test -p P (--test T | --lib) … -- FILTER [--exact]` lines with one package and target."""
    found = []
    for match in _WORKFLOW_TEST.finditer(text):
        args, rest = match.group("args"), match.group("rest").split()
        packages = re.findall(r"-p\s+(\S+)", args)
        targets = re.findall(r"--test\s+(\S+)", args) + (["lib"] if re.search(r"--lib\b", args) else [])
        filters = [word for word in rest if not word.startswith("-")]
        if len(packages) != 1 or len(targets) != 1 or not filters:
            continue
        exact = "--exact" in rest
        for word in filters:
            found.append(Selector(packages[0], targets[0], word.strip("'\""), exact, owner="workflow"))
    # Filters placed before `--` ("cargo test -p P --test T NAME -- --exact").
    for match in re.finditer(r"cargo test\s+(?P<args>[^\n|&;]*?)\s--\s(?P<rest>[^\n|&;]*)", text):
        args = match.group("args").split()
        packages = re.findall(r"-p\s+(\S+)", match.group("args"))
        targets = re.findall(r"--test\s+(\S+)", match.group("args"))
        positional = [word for i, word in enumerate(args) if not word.startswith("-")
                      and (i == 0 or args[i - 1] not in ("-p", "--test", "--features", "--package"))
                      and word != "test"]
        if len(packages) == 1 and len(targets) == 1:
            for word in positional:
                found.append(Selector(packages[0], targets[0], word, "--exact" in match.group("rest"),
                                      owner="workflow"))
    # Listing guards ("cargo test … -- --list | grep -Fx 'NAME: test'") name one test exactly.
    for match in re.finditer(
            r"cargo test\s+(?P<args>[^\n|&;]*?)\s--\s[^\n|]*--list[^\n|]*\|\s*grep -Fx '(?P<name>[^']+): test'",
            text):
        packages = re.findall(r"-p\s+(\S+)", match.group("args"))
        targets = re.findall(r"--test\s+(\S+)", match.group("args"))
        if len(packages) == 1 and len(targets) == 1:
            found.append(Selector(packages[0], targets[0], match.group("name"), True, owner="workflow"))
    return found
