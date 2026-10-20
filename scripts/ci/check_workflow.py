#!/usr/bin/env python3
"""Validate durable CI policy without coupling engine tests to workflow layout."""

from __future__ import annotations

import json
import re
import shlex
import sys
from pathlib import Path

import tomllib

ROOT = Path(__file__).resolve().parents[2]
WORKFLOW_DIRECTORY = ROOT / ".github/workflows"
CALLER = ROOT / ".github/workflows/ci.yml"
REUSABLE = ROOT / ".github/workflows/ci-reusable.yml"
DEVELOPMENT_CONTAINER = ROOT / ".devcontainer"
DIGEST_PINNED_IMAGE = re.compile(r"[^@\s]+@sha256:[0-9a-f]{64}")

# Containerfile ARG -> the one candidate-CI value it must equal.
CI_TOOLCHAIN_PINS = {
    "NODE_VERSION": r"node-version: (\S+)",
    "PNPM_VERSION": r"pnpm/action-setup@[^\n]*\n\s+with:\n\s+version: (\S+)",
    "PYTHON_VERSION": r"python-version: (\S+)",
    "UV_VERSION": r"astral-sh/setup-uv@[^\n]*\n\s+with:\n\s+version: (\S+)",
    "GO_VERSION": r"go-version: (\S+)",
    "JAVA_VERSION": r"java-version: (\S+)",
    "DOTNET_VERSION": r"dotnet-version: (\S+)",
    "MAVEN_VERSION": r"MAVEN_VERSION: (\S+)",
    "MAVEN_SHA512": r"MAVEN_SHA512: (\S+)",
}
CONTAINER_CHECKSUMS = {
    "RUSTUP_SHA256": r"[0-9a-f]{64}",
    "UV_SHA256": r"[0-9a-f]{64}",
    "PYTHON_SHA256": r"[0-9a-f]{64}",
    "NODE_SHA256": r"[0-9a-f]{64}",
    "PNPM_INTEGRITY": r"sha512-[A-Za-z0-9+/]{86}==",
    "GO_SHA256": r"[0-9a-f]{64}",
    "JAVA_SHA256": r"[0-9a-f]{64}",
    "MAVEN_SHA512": r"[0-9a-f]{128}",
    "DOTNET_SHA512": r"[0-9a-f]{128}",
}
DEVCONTAINER_LIFECYCLE_COMMANDS = {
    "initializeCommand",
    "onCreateCommand",
    "updateContentCommand",
    "postCreateCommand",
    "postStartCommand",
    "postAttachCommand",
}



class PolicyError(RuntimeError):
    pass


def require(condition: bool, message: str) -> None:
    if not condition:
        raise PolicyError(message)


def job_blocks(workflow: str) -> dict[str, str]:
    marker = "\njobs:\n"
    require(marker in workflow, "reusable workflow must define jobs")
    jobs = workflow.split(marker, 1)[1]
    matches = list(re.finditer(r"(?m)^  ([a-z][a-z0-9-]*):\s*$", jobs))
    require(matches, "reusable workflow must contain at least one job")
    return {
        match.group(1): jobs[match.end() : matches[index + 1].start()]
        if index + 1 < len(matches)
        else jobs[match.end() :]
        for index, match in enumerate(matches)
    }


def workspace_packages() -> set[str]:
    with (ROOT / "Cargo.toml").open("rb") as source:
        workspace = tomllib.load(source)["workspace"]
    return {Path(member).name for member in workspace["members"]}


def optional_feature_packages() -> set[str]:
    packages = set()
    for manifest in (ROOT / "crates").rglob("Cargo.toml"):
        with manifest.open("rb") as source:
            document = tomllib.load(source)
        features = set(document.get("features", {})) - {"default"}
        if features:
            packages.add(document["package"]["name"])
    return packages


def workflow_sources() -> dict[Path, str]:
    workflows = {
        path: path.read_text(encoding="utf-8")
        for pattern in ("*.yml", "*.yaml")
        for path in sorted(WORKFLOW_DIRECTORY.glob(pattern))
    }
    require(CALLER in workflows, "candidate CI caller is absent")
    require(REUSABLE in workflows, "candidate reusable CI workflow is absent")
    return workflows


def checkout_steps(workflow: str) -> list[tuple[int, list[str]]]:
    """Return each checkout step line and normalized nested configuration."""
    steps: list[tuple[int, list[str]]] = []
    lines = workflow.splitlines()
    for index, line in enumerate(lines):
        if re.match(r"^\s*-\s+uses:\s+actions/checkout@", line) is None:
            continue
        indentation = len(line) - len(line.lstrip())
        body: list[str] = []
        for following in lines[index + 1 :]:
            if not following.strip():
                continue
            following_indentation = len(following) - len(following.lstrip())
            if following_indentation <= indentation:
                break
            body.append(following.strip())
        steps.append((index + 1, body))
    return steps


def verify_workflow_security(workflows: dict[Path, str]) -> None:
    for path, workflow in workflows.items():
        relative = path.relative_to(ROOT)
        require(
            "pull_request_target" not in workflow,
            f"workflow must not execute proposed code with pull_request_target: {relative}",
        )
        require(
            "permissions:\n  contents: read" in workflow,
            f"workflow must default to read-only contents: {relative}",
        )

        for line_number, body in checkout_steps(workflow):
            require(
                "persist-credentials: false" in body,
                f"checkout credentials must not persist: {relative}:{line_number}",
            )
            # A declared submodule is a pinned source input, so every checkout
            # must materialize it rather than build an incomplete tree.
            require(
                not (ROOT / ".gitmodules").is_file()
                or "submodules: recursive" in body,
                f"checkout must fetch declared submodules recursively: {relative}:{line_number}",
            )


def verify_action_pins(workflows: dict[Path, str]) -> None:
    for path, workflow in workflows.items():
        relative = path.relative_to(ROOT)
        for action in re.findall(r"(?m)^\s*(?:-\s*)?uses:\s+([^\s#]+)", workflow):
            if action.startswith("./"):
                continue
            require(
                "@" in action,
                f"external action is unpinned in {relative}: {action}",
            )
            repository, reference = action.rsplit("@", 1)
            require(
                "/" in repository,
                f"external action repository is invalid in {relative}: {action}",
            )
            require(
                re.fullmatch(r"[0-9a-fA-F]{40}", reference) is not None,
                f"external action must use a full immutable SHA in {relative}: {action}",
            )

        for image in re.findall(r"(?m)^\s+image:\s+([^\s#]+)", workflow):
            require(
                re.fullmatch(r"[^@]+@sha256:[0-9a-fA-F]{64}", image) is not None,
                f"workflow service image must be digest pinned in {relative}: {image}",
            )


def verify_arc_policy() -> None:
    deployment = ROOT / "deploy/ci/github-actions/arc"
    controller = (deployment / "controller-values.yaml").read_text()
    standard = (deployment / "rrflow-standard.values.yaml").read_text()
    heavy = (deployment / "rrflow-heavy.values.yaml").read_text()
    installer = (ROOT / "scripts/ci/install-arc.sh").read_text()

    require(
        "0.14.2@sha256:" in controller, "ARC controller image must be digest pinned"
    )
    require(
        "repository_visibility" in installer
        and '!= "PRIVATE"' in installer
        and "refusing to register self-hosted runners" in installer,
        "ARC installer must refuse non-private repositories",
    )
    for name, values, maximum in [
        ("standard", standard, "maxRunners: 2"),
        ("heavy", heavy, "maxRunners: 1"),
    ]:
        require(
            "minRunners: 0" in values and maximum in values,
            f"{name} ARC scale set must stay within physical capacity",
        )
        require(
            "githubConfigSecret: rrflow-arc-github" in values,
            f"{name} ARC scale set must use the pre-created secret",
        )
        require(
            "@sha256:" in values and ":latest" not in values,
            f"{name} ARC runner images must be immutable",
        )
    require(
        "runnerScaleSetName: rrflow-standard" in standard
        and 'cpu: "16"' in standard
        and "memory: 20Gi" in standard,
        "standard ARC allocation must fit the documented host capacity",
    )
    require(
        "runnerScaleSetName: rrflow-heavy" in heavy
        and 'cpu: "46"' in heavy
        and "memory: 60Gi" in heavy
        and 'cpu: "2"' in heavy
        and "memory: 4Gi" in heavy,
        "heavy ARC runner plus Docker sidecar must fit the documented host capacity",
    )


def verify_engine_suites(block: str) -> tuple[int, int]:
    suites = re.findall(
        r'(?m)^\s+- name: ([a-z][a-z0-9-]*)\s*$\n\s+packages: "([^"]+)"\s*$',
        block,
    )
    required_suites = {
        "kernel-storage",
        "query-retrieval",
        "engine-authority",
        "service-protocol",
        "product-operations",
    }
    observed_suites = {name for name, _ in suites}
    require(
        observed_suites == required_suites,
        "engine suites must follow product boundaries",
    )

    routed: list[str] = []
    for name, selection in suites:
        arguments = shlex.split(selection)
        require(
            len(arguments) % 2 == 0
            and all(arguments[index] == "-p" for index in range(0, len(arguments), 2)),
            f"engine suite {name} must contain only explicit -p package selections",
        )
        routed.extend(arguments[index] for index in range(1, len(arguments), 2))

    expected = workspace_packages()
    duplicates = sorted({package for package in routed if routed.count(package) > 1})
    require(
        not duplicates,
        f"workspace packages appear in multiple engine suites: {duplicates}",
    )
    require(
        set(routed) == expected,
        f"engine suite coverage mismatch: missing={sorted(expected - set(routed))} "
        f"unexpected={sorted(set(routed) - expected)}",
    )
    require(
        "cargo test $PACKAGE_SELECTION --all-targets --locked" in block
        and "cargo clippy $PACKAGE_SELECTION --all-targets --locked -- -D warnings"
        in block,
        "every engine suite must run locked all-target tests and strict Clippy",
    )
    require(
        "matrix.name == 'service-protocol'" in block
        and "cargo build -p rrflow-cli --bin rrd-estate-controller --locked" in block,
        "the service-protocol suite must explicitly build its controller fixture",
    )
    return len(suites), len(routed)


def verify_optional_features(optional: str, pgvector: str) -> int:
    matrix = optional.split("        package:\n", 1)
    require(len(matrix) == 2, "optional-feature job must define a package matrix")
    package_lines = matrix[1].split("    env:\n", 1)[0]
    routed = set(re.findall(r"(?m)^\s+- ([a-z][a-z0-9-]*)\s*$", package_lines))
    expected = optional_feature_packages()
    require(
        routed | {"rrd-operator-knowledge"} == expected,
        f"optional-feature coverage mismatch: expected={sorted(expected)} routed={sorted(routed)}",
    )
    require(
        "cargo test -p ${{ matrix.package }} --all-targets --all-features --locked"
        in optional
        and "cargo clippy -p ${{ matrix.package }} --all-targets --all-features --locked -- -D warnings"
        in optional,
        "optional-feature packages must run locked all-feature tests and strict Clippy",
    )
    require(
        "RRFLOW_PGVECTOR_TEST_URL" in pgvector
        and "cargo test -p rrd-operator-knowledge --all-targets --all-features --locked"
        in pgvector,
        "PostgreSQL operator knowledge must retain its real pgvector qualification",
    )
    return len(expected)


def verify_sdk_conformance(block: str) -> None:
    required = [
        "uses: pnpm/action-setup@ea17c68df8912ef543352723c149a84f56e3d413",
        "uses: actions/setup-node@949feb2413d6458794dcd2491c4babbbce0c15c1",
        "uses: actions/setup-python@5fda3b95a4ea91299a34e894583c3862153e4b97",
        "uses: astral-sh/setup-uv@1c37ad07a6a961277cf70c0d37d6f313000f5884",
        "uses: actions/setup-go@b7ad1dad31e06c5925ef5d2fc7ad053ef454303e",
        "uses: actions/setup-java@b843ebf9b913146b38da2eaa7aeddf76074ae1ad",
        "uses: actions/setup-dotnet@a98b56852c35b8e3190ac28c8c2271da59106c68",
        "python3 scripts/ci/run_sdk_conformance.py",
        "node-version: 24.22.0",
        "version: 12.14.0",
        "python-version: 3.15.0",
        "version: 0.13.2",
        "go-version: 1.27.2",
        "cache: false",
        "java-version: 25.0.4+101.0.LTS",
        "dotnet-version: 10.0.402",
        "MAVEN_VERSION: 3.10.0",
        "MAVEN_SHA512: 908b1501bfb420bf7c8affb855534a9c407fd6099367bfb9f2f2dcb8e9799102bffb84518cde74c679bd76870247c6528683abdd620581bffa90f95d92d175aa",
        "maven-3/$MAVEN_VERSION/binaries/",
        "sha512sum --check --strict",
    ]
    missing = [fragment for fragment in required if fragment not in block]
    require(
        not missing,
        f"SDK conformance toolchain or runner contract is incomplete: {missing}",
    )
    require(
        block.count("python3 scripts/ci/run_sdk_conformance.py") == 1,
        "SDK qualification must execute through one cohesive orchestrator",
    )


def single_ci_value(reusable: str, pattern: str, label: str) -> str:
    values = set(re.findall(pattern, reusable))
    require(
        len(values) == 1,
        f"candidate CI must declare exactly one {label} value: {sorted(values)}",
    )
    return values.pop()


def compose_services(compose: str) -> dict[str, str]:
    marker = "\nservices:\n"
    require(marker in compose, "development compose file must define services")
    services = compose.split(marker, 1)[1]
    services = re.split(r"(?m)^\S", services, maxsplit=1)[0]
    matches = list(re.finditer(r"(?m)^  ([a-z][a-z0-9-]*):\s*$", services))
    return {
        match.group(1): services[match.end() : matches[index + 1].start()]
        if index + 1 < len(matches)
        else services[match.end() :]
        for index, match in enumerate(matches)
    }


def verify_development_container(reusable: str, pgvector_job: str) -> int:
    """Keep the local workspace a faithful reproduction of candidate CI pins."""
    containerfile = (DEVELOPMENT_CONTAINER / "Containerfile").read_text(
        encoding="utf-8"
    )
    compose = (DEVELOPMENT_CONTAINER / "compose.yaml").read_text(encoding="utf-8")
    devcontainer = json.loads(
        (DEVELOPMENT_CONTAINER / "devcontainer.json").read_text(encoding="utf-8")
    )

    stages: set[str] = set()
    for image, stage in re.findall(
        r"(?m)^FROM\s+(\S+)\s+AS\s+([a-z][a-z0-9-]*)\s*$", containerfile
    ):
        require(
            image in stages or DIGEST_PINNED_IMAGE.fullmatch(image) is not None,
            f"development container base image must be digest pinned: {image}",
        )
        stages.add(stage)
    require(
        {"rust", "full"} <= stages,
        f"development container must provide rust and full targets: {sorted(stages)}",
    )
    require(
        len(re.findall(r"(?m)^FROM\s", containerfile)) == len(stages),
        "every development container stage must be named",
    )

    arguments = dict(re.findall(r"(?m)^ARG\s+([A-Z0-9_]+)=(\S+)\s*$", containerfile))
    with (ROOT / "rust-toolchain.toml").open("rb") as source:
        channel = tomllib.load(source)["toolchain"]["channel"]
    expected = {"RUST_VERSION": channel}
    for name, pattern in CI_TOOLCHAIN_PINS.items():
        expected[name] = single_ci_value(reusable, pattern, name)
    for name, value in expected.items():
        require(
            arguments.get(name) == value,
            f"development container {name} must equal candidate CI {value!r}: "
            f"{arguments.get(name)!r}",
        )
    for name, pattern in CONTAINER_CHECKSUMS.items():
        require(
            re.fullmatch(pattern, arguments.get(name, "")) is not None,
            f"development container checksum {name} is absent or malformed",
        )
        require(
            containerfile.count(f"${{{name}") >= 1,
            f"development container checksum {name} is declared but never verified",
        )
    downloads = len(re.findall(r"\bcurl\s+-fsSLo\b", containerfile))
    verifications = len(
        re.findall(r"\bsha(?:256|512)sum --check --strict\b", containerfile)
    )
    require(
        downloads == len(CONTAINER_CHECKSUMS) and verifications == downloads,
        "every development container download must be strictly checksum verified: "
        f"{downloads} downloads, {verifications} verifications",
    )
    require(
        re.fullmatch(r"\d{8}T\d{6}Z", arguments.get("UBUNTU_SNAPSHOT", "")) is not None
        and containerfile.count('--snapshot "${UBUNTU_SNAPSHOT}"') == 4,
        "development container packages must resolve from one fixed Ubuntu snapshot",
    )

    services = compose_services(compose)
    require(
        set(services) == {"workspace", "pgvector"},
        f"development compose services must be workspace and pgvector: {sorted(services)}",
    )
    for name, block in services.items():
        for image in re.findall(r"(?m)^\s+image:\s+(\S+)", block):
            local = image.startswith("localhost/") and "\n    build:\n" in block
            require(
                local or DIGEST_PINNED_IMAGE.fullmatch(image) is not None,
                f"development compose image must be locally built or digest pinned: {image}",
            )
    pgvector = services["pgvector"]
    ci_image = single_ci_value(pgvector_job, r"(?m)^\s+image:\s+(\S+)", "pgvector image")
    local_images = re.findall(r"(?m)^\s+image:\s+(\S+)", pgvector)
    require(
        len(local_images) == 1
        and local_images[0].removeprefix("docker.io/") == ci_image.removeprefix("docker.io/"),
        f"development pgvector image must equal candidate CI {ci_image}: {local_images}",
    )
    for variable in ("POSTGRES_USER", "POSTGRES_PASSWORD", "POSTGRES_DB"):
        value = single_ci_value(pgvector_job, rf"{variable}: (\S+)", variable)
        require(
            f"{variable}: {value}" in pgvector,
            f"development pgvector {variable} must equal candidate CI {value!r}",
        )
    require(
        "@127.0.0.1:5432/" in pgvector_job
        and "profiles: [\"pgvector\"]" in pgvector
        and "network_mode: host" in pgvector
        and "network_mode: host" in services["workspace"],
        "development pgvector must be opt-in and reachable at the candidate-CI loopback URL",
    )

    lifecycle = sorted(DEVCONTAINER_LIFECYCLE_COMMANDS & set(devcontainer))
    require(
        not lifecycle,
        f"devcontainer.json must not declare editor lifecycle commands: {lifecycle}",
    )
    build = devcontainer.get("build", {})
    require(
        build.get("dockerfile") == "Containerfile" and build.get("target") in stages,
        "devcontainer.json must build a named target of the one Containerfile",
    )
    return len(expected) + len(CONTAINER_CHECKSUMS)


def main() -> None:
    workflows = workflow_sources()
    caller = workflows[CALLER]
    reusable = workflows[REUSABLE]
    jobs = job_blocks(reusable)

    for trigger in ["pull_request:", "merge_group:", "workflow_dispatch:"]:
        require(
            re.search(rf"(?m)^  {re.escape(trigger)}\s*$", caller) is not None,
            f"candidate CI must include {trigger}",
        )
    require(
        re.search(r"(?m)^\s*push:\s*$", caller) is None,
        "candidate CI must not duplicate PR work on push",
    )
    require(
        "pull_request_target" not in caller,
        "candidate CI must not execute proposed code with pull_request_target",
    )
    require(
        caller.count("uses: ./.github/workflows/ci-reusable.yml") == 1,
        "candidate CI must invoke exactly one reusable workflow",
    )
    require(
        "cancel-in-progress: true" in caller, "stale candidate CI must be cancelled"
    )
    require(
        "github.event.pull_request.head.repo.full_name == github.repository" in caller
        and "vars.CI_SELF_HOSTED_ENABLED == 'true'" in caller,
        "self-hosted routing must preserve the fork trust boundary",
    )
    require(
        "permissions:\n  contents: read" in caller
        and "permissions:\n  contents: read" in reusable,
        "caller and reusable workflow must default to read-only contents",
    )
    require(
        reusable.count("python3 scripts/ci/check_documentation.py") == 1,
        "documentation authority and local-link policy must run exactly once",
    )
    require(
        reusable.count("python3 scripts/ci/build_execution_inventory.py --check") == 1,
        "the exhaustive execution inventory must be checked exactly once",
    )

    expected_jobs = {
        "topology-smoke",
        "estate-portability",
        "verify",
        "engine-suites",
        "sdk-conformance",
        "optional-features",
        "pgvector-feature",
        "ci-gate",
    }
    require(
        set(jobs) == expected_jobs,
        f"candidate CI jobs must have cohesive ownership: {sorted(jobs)}",
    )
    for job, block in jobs.items():
        require(
            "timeout-minutes:" in block, f"CI job {job} must have an explicit timeout"
        )

    gate = jobs["ci-gate"]
    needs_match = re.search(
        r"(?ms)^    needs:\s*$\n((?:^      - [a-z0-9-]+\s*$\n?)+)", gate
    )
    require(needs_match is not None, "ci-gate must enumerate every substantive job")
    needs = set(re.findall(r"(?m)^      - ([a-z0-9-]+)\s*$", needs_match.group(1)))
    require(
        needs == set(jobs) - {"ci-gate"},
        "ci-gate must reduce every substantive job exactly once",
    )
    result_dependencies = set(re.findall(r"needs\.([a-z0-9-]+)\.result", gate))
    require(
        result_dependencies == needs, "ci-gate must inspect every dependency result"
    )
    result_variables = re.findall(
        r"(?m)^\s+([A-Z][A-Z0-9_]+):\s+\$\{\{\s*needs\.[a-z0-9-]+\.result\s*}}\s*$",
        gate,
    )
    require(
        all(f'test "${variable}" = success' in gate for variable in result_variables),
        "ci-gate must require success for every reduced result",
    )

    heavy_jobs = [
        "verify",
        "engine-suites",
        "sdk-conformance",
        "optional-features",
        "pgvector-feature",
    ]
    for job in heavy_jobs:
        block = jobs[job]
        require(
            'CARGO_BUILD_JOBS: "2"' in block
            and 'CARGO_PROFILE_TEST_DEBUG: "0"' in block
            and 'RUSTFLAGS: "-C link-arg=-Wl,--threads=1"' in block,
            f"Linux-heavy job {job} must retain the bounded compiler/linker profile",
        )
        require(
            "cache-on-failure: false" in block,
            f"Linux-heavy job {job} must not publish failed artifacts",
        )
    require(
        "cargo test --workspace" not in reusable
        and "cargo clippy --workspace" not in reusable,
        "hosted fallback must not link the entire workspace in one target graph",
    )
    require(
        "cargo fmt --all -- --check" in jobs["verify"]
        and "python3 scripts/ci/check_workflow.py" in jobs["verify"],
        "repository policy must check formatting and this CI contract",
    )
    verify_checkouts = checkout_steps(jobs["verify"])
    require(len(verify_checkouts) == 1, "the verify job must have one checkout step")
    require(
        "fetch-depth: 0" in verify_checkouts[0][1],
        "the verify checkout must fetch complete Git history",
    )
    require(
        "python3 scripts/ci/check_generated_surfaces.py" in jobs["verify"],
        "repository policy must reject generated product-surface drift",
    )

    suite_count, package_count = verify_engine_suites(jobs["engine-suites"])
    verify_sdk_conformance(jobs["sdk-conformance"])
    feature_count = verify_optional_features(
        jobs["optional-features"], jobs["pgvector-feature"]
    )
    require(
        reusable.count("cargo build -p rrflow-cli --bin rrd-estate-controller --locked")
        == 2,
        "portability and service-protocol qualification must both declare the controller fixture",
    )

    verify_workflow_security(workflows)
    verify_action_pins(workflows)
    verify_arc_policy()
    container_pins = verify_development_container(reusable, jobs["pgvector-feature"])
    print(
        f"CI policy OK: {len(workflows)} workflows, {len(jobs) - 1} substantive jobs, "
        f"{suite_count} cohesive engine suites, "
        f"{package_count} default-feature packages, {feature_count} optional-feature packages, "
        f"{container_pins} development-container pins"
    )


if __name__ == "__main__":
    try:
        main()
    except PolicyError as error:
        print(f"CI policy violation: {error}", file=sys.stderr)
        raise SystemExit(1) from error
