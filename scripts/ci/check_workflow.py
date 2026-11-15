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
NATIVE_PREFLIGHT = ROOT / ".github/workflows/c07-native-preflight.yml"
NIGHTLY = ROOT / ".github/workflows/ci-nightly.yml"

# Candidate CI is proportional to the change. The `changes` job selects lanes
# from the changed paths; every partition declares the lane that runs it, and
# `ci-gate` requires success for a selected lane and a skip for an unselected
# one. A job without a lane condition would run unconditionally and a job
# with any other condition could be skipped without the gate noticing.
FULL_LANE_IF = "    if: ${{ needs.changes.outputs.full == 'true' }}\n"
SDK_LANE_IF = "    if: ${{ needs.changes.outputs.full == 'true' || needs.changes.outputs.sdk == 'true' }}\n"
SCRIPT_LANE_POLICY_COMMANDS = (
    "python3 scripts/ci/test_merge_alarm.py",
    "python3 scripts/ci/test_candidate_admission.py",
    "python3 scripts/ci/test_linux_arm64_policy.py",
    "python3 scripts/ci/test_native_sample_policy.py",
    "python3 scripts/ci/test_sdk_attunement_policy.py",
    "python3 scripts/ci/test_rrflowdb_standalone.py",
    "python3 scripts/ci/test_cache_warm_policy.py",
    "python3 scripts/ci/test_inspect_merge_conflicts.py",
    "python3 scripts/ci/test_ci_lanes_policy.py",
    "python3 scripts/ci/test_merge_conflict_policy.py",
    "python3 scripts/ci/check_workflow.py",
    "python3 scripts/ci/test_inspect_candidate_runs.py",
)

LANE_CONDITIONS = {
    "docs-policy": "    if: ${{ needs.changes.outputs.docs == 'true' }}\n",
    "scripts-policy": "    if: ${{ needs.changes.outputs.scripts == 'true' }}\n",
    "engine-fast": "    if: ${{ needs.changes.outputs.engine == 'true' }}\n",
    "sdk-conformance": SDK_LANE_IF,
    # Repository-wide invariants run on every candidate the full matrix does not cover.
    "repository-invariants": "    if: ${{ needs.changes.outputs.full != 'true' }}\n",
}
FULL_LANE_JOBS = (
    "native-arm64", "topology-smoke", "engram-portability", "verify", "engine-suites",
    "optional-features", "pgvector-feature", "storage-contract-preflight", "installed-reopen",
    "installed-crash",
)
GATE_EXPECTATIONS = {
    "docs-policy": ("DOCS_LANE", "DOCS_POLICY_RESULT"),
    "scripts-policy": ("SCRIPTS_LANE", "SCRIPTS_POLICY_RESULT"),
    "engine-fast": ("ENGINE_LANE", "ENGINE_FAST_RESULT"),
    "native-arm64": ("FULL_LANE", "ARM64_RESULT"),
    "topology-smoke": ("FULL_LANE", "TOPOLOGY_RESULT"),
    "engram-portability": ("FULL_LANE", "PORTABILITY_RESULT"),
    "verify": ("FULL_LANE", "VERIFY_RESULT"),
    "engine-suites": ("FULL_LANE", "ENGINE_SUITES_RESULT"),
    "sdk-conformance": ("SDK_SELECTED", "SDK_CONFORMANCE_RESULT"),
    "optional-features": ("FULL_LANE", "OPTIONAL_FEATURES_RESULT"),
    "pgvector-feature": ("FULL_LANE", "PGVECTOR_FEATURE_RESULT"),
    "installed-crash": ("FULL_LANE", "INSTALLED_CRASH_RESULT"),
    "storage-contract-preflight": ("FULL_LANE", "STORAGE_PREFLIGHT_RESULT"),
    "installed-reopen": ("FULL_LANE", "INSTALLED_REOPEN_RESULT"),
    "repository-invariants": ("LIGHT_SELECTED", "INVARIANTS_RESULT"),
}
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


def verify_candidate_admission(caller: str) -> None:
    """Keep the thin caller's admission envelope explicit and fail closed on drift.

    This intentionally accepts the small checked-in YAML shape, not arbitrary
    workflow syntax. GitHub evaluates the condition; this policy prevents lost
    transitions, path filters, or an alternate caller from bypassing that shape.
    """
    trigger = re.search(r"(?ms)^on:\n(.*?)(?=^\S)", caller)
    require(trigger is not None, "candidate CI must declare its event envelope")
    events = re.fullmatch(
        r"  push:\n    branches: \[main\]\n"
        r"  pull_request:\n    types: \[([a-z_, ]+)\]\n"
        r"  merge_group:\n  workflow_dispatch:\n\n",
        trigger.group(1),
    )
    require(events is not None, "candidate events must run on pushes to main and must not add filters or omit merge/manual dispatch")
    activities = [item.strip() for item in events.group(1).split(",")]
    required = {"opened", "synchronize", "reopened", "ready_for_review", "converted_to_draft"}
    require(
        len(activities) == len(required) and set(activities) == required,
        "candidate PR events must include ready and draft transitions exactly once",
    )
    jobs = job_blocks(caller)
    require(set(jobs) == {"pipeline"}, "candidate admission must have one pipeline caller")
    conditions = re.findall(r"(?m)^    if: (.+)$", jobs["pipeline"])
    require(
        conditions == ["${{ github.event_name != 'pull_request' || github.event.pull_request.draft == false }}"],
        "draft PRs must defer full CI; ready, merge-group and manual candidates must run it",
    )
    require(
        "concurrency:\n"
        "  group: ci-${{ github.workflow }}-${{ github.event.pull_request.number || github.ref }}-"
        "${{ github.event_name == 'pull_request' && github.event.pull_request.draft && 'draft' || 'candidate' }}\n"
        "  cancel-in-progress: true\n" in caller,
        "candidate concurrency must isolate draft and ready events for the same PR or ref",
    )


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


def verify_linux_arm64_ci(caller: str, reusable: str) -> None:
    """Require a native ARM64 result without routing fork code to private hosts."""
    routing = (
        "      linux-arm64-runner: >-\n"
        "        ${{ (github.event_name != 'pull_request' || github.event.pull_request.head.repo.full_name == github.repository)\n"
        "            && vars.CI_SELF_HOSTED_ENABLED == 'true'\n"
        "            && (vars.CI_LINUX_ARM64_RUNNER || '[\"ubuntu-24.04-arm\"]')\n"
        "            || '[\"ubuntu-24.04-arm\"]' }}\n"
    )
    require(caller.count(routing) == 1, "native ARM64 routing or fork isolation is missing")
    require("      linux-arm64-runner:\n" in reusable,
            "reusable CI must require its ARM64 runner input")
    jobs = job_blocks(reusable)
    require("native-arm64" in jobs, "native ARM64 qualification job is missing")
    arm = jobs["native-arm64"]
    for required in [
        "runs-on: ${{ fromJSON(inputs.linux-arm64-runner) }}",
        'test "$RUNNER_OS" = Linux',
        'test "$RUNNER_ARCH" = ARM64',
        'test "$(uname -m)" = aarch64',
        "host: aarch64-unknown-linux-gnu",
        "cargo test -p rrd-lsm --test filesystem --test manifest --locked",
        "cargo test -p rrd-store --test installed_lifecycle --test semantic_commit_atomicity --locked",
        "cargo test -p rrd-attunement --locked",
        "cargo test -p rrflow-cli --test installed_lifecycle --locked -- --nocapture",
    ]:
        require(arm.count(required) == 1, f"native ARM64 qualification missing: {required}")
    require("continue-on-error:" not in arm
            and re.findall(r"(?m)^\s+if: .*$\n", arm) == [FULL_LANE_IF],
            "native ARM64 qualification runs on the full lane and cannot otherwise be skipped or ignored")
    gate = jobs.get("ci-gate", "")
    require("      - native-arm64\n" in gate
            and "ARM64_RESULT: ${{ needs.native-arm64.result }}" in gate
            and 'expect "$FULL_LANE" "$ARM64_RESULT" native-arm64' in gate,
            "candidate gate must require the native ARM64 result on the full lane")
    require(jobs["verify"].count("python3 scripts/ci/test_linux_arm64_policy.py") == 1
            and reusable.count("python3 scripts/ci/test_linux_arm64_policy.py") == 2,
            "native ARM64 policy regressions must run once in verify and once in the script lane")


def verify_native_sample_worker_ci(reopen: str, verify: str) -> None:
    """Keep bounded failure, Windows lifecycle and Linux image oracles required."""
    marker = "      - name: Require installed stale-snapshot conflict safety\n"
    require(reopen.count(marker) == 1, "installed conflict safety CI step must occur once")
    block = reopen.split(marker, 1)[1].split("      - name:", 1)[0]
    required = (
        "        timeout-minutes: 5\n"
        "        shell: bash\n"
        "        run: |\n"
        "          set -euo pipefail\n"
        "          cargo test -p rrflow-cli --test installed_reopen --release --locked -- --ignored --list | grep -Fx 'sustained::campaign::installed_disjoint_writes_from_one_stamp_conflict_without_partial_effect: test'\n"
        "          cargo test -p rrflow-cli --test installed_reopen --release --locked -- sustained::campaign::installed_disjoint_writes_from_one_stamp_conflict_without_partial_effect --exact --ignored --nocapture\n"
    )
    require(block == required, "installed conflict safety step must retain its exact matrix-wide list check, timeout and command")
    marker = "      - name: Require bounded native sampler failure\n"
    require(reopen.count(marker) == 1, "bounded native sampler failure CI step must occur once")
    block = reopen.split(marker, 1)[1].split("      - name:", 1)[0]
    required = (
        "        timeout-minutes: 5\n"
        "        run: cargo test -p rrflow-cli --test installed_reopen --release --locked -- sustained::native_worker::tests::acquisition_deadline_keeps_unfinished_cleanup_owned_and_blocks_reuse --exact --nocapture\n"
    )
    require(block == required, "bounded native sampler failure step must retain its exact timeout and command")
    marker = "      - name: Verify owned Windows native sampler lifecycle\n"
    require(reopen.count(marker) == 1, "native sampler Windows CI step must occur once")
    block = reopen.split(marker, 1)[1].split("      - name:", 1)[0]
    required = (
        "        if: runner.os == 'Windows'\n"
        "        timeout-minutes: 5\n"
        "        shell: bash\n"
        "        run: |\n"
        "          set -euo pipefail\n"
        "          python scripts/ci/test_windows_harness_image.py --profile release\n"
        "          python scripts/ci/test_native_sample_worker.py --profile release\n"
    )
    require(block == required, "native sampler CI step must retain its exact condition, timeout and command")
    marker = "      - name: Verify Linux native sampler running image\n"
    require(reopen.count(marker) == 1, "native sampler Linux image CI step must occur once")
    block = reopen.split(marker, 1)[1].split("      - name:", 1)[0]
    required = (
        "        if: runner.os == 'Linux'\n"
        "        timeout-minutes: 5\n"
        "        run: cargo test -p rrflow-cli --test installed_reopen --release --locked -- sustained::native_worker::tests::replaced_harness_path_cannot_redirect_linux_worker_or_receipt_digest --exact --nocapture\n"
    )
    require(block == required, "native sampler Linux image step must retain its exact condition, timeout and command")
    policy_marker = "      - name: Enforce repository CI policy\n"
    require(verify.count(policy_marker) == 1, "repository CI policy step must occur once")
    policy = verify.split(policy_marker, 1)[1].split("      - name:", 1)[0]
    require(policy.count("          python3 scripts/ci/test_native_sample_policy.py\n") == 1
            and "continue-on-error:" not in policy,
            "native sampler policy regressions must run without a bypass")


def verify_native_capacity_diagnostics(job: str) -> None:
    """Preserve native qualification and retain only bounded create diagnostics."""
    def step(name: str) -> str:
        marker = f"      - name: {name}\n"
        require(job.count(marker) == 1, f"native capacity step missing or duplicated: {name}")
        return job.split(marker, 1)[1].split("      - name:", 1)[0]

    first = step("Qualify installed native capacity failure and recovery")
    repeated = step("Require repeated APFS exhaustion and recovery")
    upload = step("Retain native capacity create diagnostics")
    destination = "RRFLOW_CAPACITY_DIAGNOSTICS: ${{ runner.temp }}/rrflow-capacity-diagnostics"
    require(destination in first and destination in repeated,
            "both capacity runs must bind their diagnostic directory")
    require("run: python3 scripts/ci/run_native_capacity.py" in first,
            "initial native capacity run must remain required")
    require("if: matrix.os == 'macos-latest'" in repeated
            and "python3 scripts/ci/run_native_capacity.py" in repeated
            and repeated.count("--mode fresh --mode existing") == 3,
            "macOS repeated native capacity cases must remain required")
    require("if: always() && matrix.os == 'macos-latest'" in upload
            and "path: ${{ runner.temp }}/rrflow-capacity-diagnostics/*.json" in upload
            and "if-no-files-found: ignore" in upload
            and "actions/upload-artifact@" in upload,
            "native failure artifact must retain only controlled diagnostics")


def verify_native_capacity_preflight(workflow: str) -> None:
    """A manual characterization cannot stand in for candidate or merge CI."""
    events = re.search(r"(?ms)^on:\n(.*?)(?=^\S)", workflow)
    require(
        events is not None
        and re.findall(r"(?m)^  ([a-z_]+):", events.group(1)) == ["workflow_dispatch"],
        "native preflight must be manual only",
    )
    for name, default in [("point", "all"), ("mode", "both"), ("repetitions", '"3"')]:
        pattern = rf"(?ms)^      {name}:\n(?:(?!^      \w+:).)*?^        default: {re.escape(default)}$"
        require(re.search(pattern, events.group(1)) is not None,
                f"native preflight default drifted: {name}")
    jobs = job_blocks(workflow)
    require(set(jobs) == {"native-preflight"}, "native preflight must remain isolated")
    job = jobs["native-preflight"]
    for required in ["runs-on: macos-latest", 'test "$(git rev-parse HEAD)" = "$GITHUB_SHA"',
                     "python3 scripts/ci/test_native_capacity.py", "python3 scripts/ci/test_macos_native_failure_preflight.py",
                     "python3 scripts/ci/macos_native_failure_preflight.py", "python3 scripts/ci/run_macos_capacity_preflight.py",
                     "python3 scripts/ci/test_macos_capacity_preflight.py",
                     "RRFLOW_PREFLIGHT_POINT: ${{ inputs.point }}",
                     "RRFLOW_PREFLIGHT_MODE: ${{ inputs.mode }}",
                     "RRFLOW_PREFLIGHT_REPETITIONS: ${{ inputs.repetitions }}",
                     '--point "$RRFLOW_PREFLIGHT_POINT" --mode "$RRFLOW_PREFLIGHT_MODE"',
                     '--repetitions "$RRFLOW_PREFLIGHT_REPETITIONS"',
                     "RRFLOW_CAPACITY_DIAGNOSTICS: ${{ runner.temp }}/rrflow-capacity-diagnostics",
                     "path: ${{ runner.temp }}/rrflow-capacity-diagnostics/*.json", "if: always()"]:
        require(required in job, f"native preflight missing: {required}")
    require("continue-on-error" not in job,
            "native preflight must not ignore failures")
    for name, artifact_path, missing in [
        ("Retain injected native failure receipt",
         "${{ runner.temp }}/rrflow-native-failure-probe/rrflow-capacity-diagnostics/*.json", "error"),
        ("Retain native capacity create diagnostics",
         "${{ runner.temp }}/rrflow-capacity-diagnostics/*.json", "ignore"),
    ]:
        marker = f"      - name: {name}\n"
        require(job.count(marker) == 1, f"native preflight artifact missing or duplicated: {name}")
        step = job.split(marker, 1)[1].split("      - name:", 1)[0]
        require("        if: always()\n" in step and "actions/upload-artifact@" in step
                and f"path: {artifact_path}" in step and f"if-no-files-found: {missing}" in step,
                f"native preflight artifact is not isolated and failure-preserving: {name}")
    require(not any(re.match(r"(?:ref|repository|path)\s*:", entry)
                    for _, body in checkout_steps(job) for entry in body),
            "native preflight must checkout the dispatched revision")


def verify_macos_reopen_cache_warm(workflow: str, reopen: str) -> None:
    """Admit one trusted-main cache experiment without changing CI requirements."""
    trigger = re.search(r"(?ms)^on:\n(.*?)(?=^\S)", workflow)
    require(trigger is not None and trigger.group(1) == "  workflow_dispatch:\n\n",
            "cache warmer must be manual-only")
    jobs = job_blocks(workflow)
    require(set(jobs) == {"cache-reopen"}, "cache warmer must contain one job")
    job = jobs["cache-reopen"]
    require(re.findall(r"(?m)^    if: (.+)$", job) == ["${{ github.ref == 'refs/heads/main' }}"],
            "cache warmer must be scoped to the main dispatch ref")
    require("    runs-on: macos-latest\n" in job and "- macos-latest" in reopen,
            "cache warmer must match the macOS reopen runner")
    for setting in ('CARGO_PROFILE_DEV_DEBUG: "0"', 'CARGO_PROFILE_TEST_DEBUG: "0"'):
        require(setting in job and setting in reopen, "cache build profile differs from reopen")
    require('CARGO_BUILD_JOBS: "1"' in job and
            "CARGO_BUILD_JOBS: ${{ matrix.os == 'windows-latest' && '2' || '1' }}" in reopen,
            "cache build jobs differ from macOS reopen")
    require('CARGO_INCREMENTAL: "0"' in workflow and 'CARGO_TERM_COLOR: always' in workflow,
            "cache warmer environment differs from reusable CI")
    require("shared-key: installed-reopen-macos-latest" in job and
            "shared-key: installed-reopen-${{ matrix.os }}" in reopen and
            "cache-on-failure: false" in job and "cache-on-failure: false" in reopen,
            "cache warmer key or failure behavior differs from reopen")
    isolation = """      - name: Isolate macOS reopen toolchain
        if: runner.os == 'macOS'
        shell: bash
        run: |
          set -euo pipefail
          toolchain_home="${RUNNER_TEMP}/rrflow-reopen-rustup"
          test ! -e "$toolchain_home"
          printf 'RUSTUP_HOME=%s\\n' "$toolchain_home" >> "$GITHUB_ENV"
"""
    for surface in (job, reopen):
        require(surface.count(isolation) == 1,
                "macOS reopen toolchain isolation must match exactly")
        require(surface.index(isolation) < surface.index("- uses: dtolnay/rust-toolchain@") <
                surface.index("- uses: Swatinem/rust-cache@"),
                "toolchain isolation must precede installation and cache lookup")
    actions = lambda text: re.findall(r"(?m)^\s*- uses: ([^\s#]+)", text)
    require(actions(job) == actions(reopen)[:3], "cache warmer actions differ from reopen")
    expected = "cargo test -p rrflow-cli --test installed_reopen --release --locked --no-run"
    require(re.findall(r"(?m)^\s*(?:-\s*)?run: (.+)$", job) == ["|", expected] and
            f"run: {expected}" in reopen, "cache warmer build differs from reopen")
    for _, body in checkout_steps(workflow):
        require(not any(re.match(r"(?:ref|repository|path)\s*:", item) for item in body),
                "cache warmer must use the triggering main revision")


def verify_windows_reopen_cache_warm(workflow: str, reopen: str) -> None:
    """Warm only trusted main with the Windows candidate's dependency inputs."""
    require(re.findall(r"(?m)^([A-Za-z][A-Za-z0-9_-]*):", workflow) ==
            ["name", "on", "permissions", "concurrency", "env", "jobs"],
            "Windows cache warmer has unexpected or duplicate top-level settings")
    trigger = re.search(r"(?ms)^on:\n(.*?)(?=^\S)", workflow)
    require(trigger is not None and trigger.group(1) == "  workflow_dispatch:\n\n",
            "Windows cache warmer must be manual-only")
    permissions = re.search(r"(?ms)^permissions:\n(.*?)(?=^\S)", workflow)
    require(permissions is not None and permissions.group(1) == "  contents: read\n\n",
            "Windows cache warmer permissions must be read-only")
    require("concurrency:\n  group: trusted-windows-reopen-cache-warm\n"
            "  cancel-in-progress: false\n\n" in workflow,
            "Windows cache warming must not cancel another warm run")
    jobs = job_blocks(workflow)
    job_names = re.findall(r"(?m)^  ([a-z][a-z0-9-]*):\s*$", workflow.split("\njobs:\n", 1)[1])
    require(job_names == ["cache-reopen"] and set(jobs) == {"cache-reopen"},
            "Windows cache warmer must have one unique job")
    job = jobs["cache-reopen"]
    require(re.findall(r"(?m)^    ([A-Za-z][A-Za-z0-9_-]*):", job) ==
            ["if", "runs-on", "timeout-minutes", "env", "steps"],
            "Windows cache warmer has unexpected or duplicate job settings")
    require(re.findall(r"(?m)^    if: (.+)$", job) == ["${{ github.ref == 'refs/heads/main' }}"],
            "Windows cache warmer must use the trusted main ref")
    require("    runs-on: windows-latest\n" in job and "- windows-latest" in reopen,
            "Windows cache warmer must use the candidate runner")
    require("    timeout-minutes: 120\n" in job,
            "Windows cache warmer must have its own finite preparation deadline")
    root_env = re.search(r"(?ms)^env:\n(.*?)(?=^\S)", workflow)
    require(root_env is not None and root_env.group(1) ==
            '  CARGO_INCREMENTAL: "0"\n  CARGO_TERM_COLOR: always\n\n',
            "Windows cache warmer environment differs from reusable CI")
    expected_env = ('      CARGO_BUILD_JOBS: "2"\n'
                    '      CARGO_PROFILE_DEV_DEBUG: "0"\n'
                    '      CARGO_PROFILE_TEST_DEBUG: "0"\n')
    env = re.search(r"(?ms)^    env:\n(.*?)(?=^    \S)", job)
    require(env is not None and env.group(1) == expected_env,
            "Windows cache warmer job environment differs from the candidate")
    require("CARGO_BUILD_JOBS: ${{ matrix.os == 'windows-latest' && '2' || '1' }}" in reopen
            and 'CARGO_PROFILE_DEV_DEBUG: "0"' in reopen
            and 'CARGO_PROFILE_TEST_DEBUG: "0"' in reopen,
            "Windows candidate build settings differ")
    require("shared-key: installed-reopen-windows-latest" in job
            and "shared-key: installed-reopen-${{ matrix.os }}" in reopen
            and "cache-on-failure: false" in job and "cache-on-failure: false" in reopen,
            "Windows cache key or failure handling differs")
    actions = lambda text: re.findall(r"(?m)^\s*- uses: ([^\s#]+)", text)
    selected = actions(job)
    require(selected == actions(reopen)[:3],
            "Windows cache warmer actions differ from the reopen candidate")
    require(len(re.findall(r"(?m)^      - (?:uses|name):", job)) == 4,
            "Windows cache warmer contains unexpected steps")
    checkout = checkout_steps(workflow)
    require(len(checkout) == 1
            and checkout[0][1] == ["with:", "persist-credentials: false", "submodules: recursive"],
            "Windows cache warmer checkout must use the main revision and recursive submodules")
    for action, settings in [
        (selected[1], "        with:\n          components: clippy\n"),
        (selected[2], "        with:\n          shared-key: installed-reopen-windows-latest\n"
                      "          cache-on-failure: false\n"),
    ]:
        block = re.search(r"(?ms)^      - uses: " + re.escape(action) +
                          r"(?: #[^\n]*)?\n(.*?)(?=^      - |\Z)", job)
        require(block is not None and block.group(1) == settings,
                "Windows cache warmer action settings differ")
    expected = "cargo test -p rrflow-cli --test installed_reopen --release --locked --no-run"
    require(re.findall(r"(?m)^\s*(?:-\s*)?run: (.+)$", job) == [expected]
            and f"run: {expected}" in reopen,
            "Windows cache warmer build differs from the candidate")
    require(job.endswith("      - name: Build cache inputs only\n        run: " + expected + "\n"),
            "Windows cache warmer build step must not add other configuration")
    require("continue-on-error:" not in job and not re.search(r"(?m)^        if:", job),
            "Windows cache warmer cannot bypass or condition its steps")


def verify_execution_inventory_ci(verify: str) -> None:
    """Keep each retained inventory bound to the clean checked-out revision."""
    marker = "      - name: Enforce exhaustive RRFlow 1.0 file planning\n"
    upload = "      - name: Retain revision-bound execution inventory\n"
    require(verify.count(marker) == 1 and verify.count(upload) == 1,
            "CI must generate and retain the revision-bound inventory")
    block = verify.split(marker, 1)[1].split("      - name:", 1)[0]
    for required in (
        "set -euo pipefail",
        "git diff --exit-code",
        "git diff --cached --exit-code",
        'test -z "$(git ls-files --others --exclude-standard)"',
        "python3 scripts/ci/build_execution_inventory.py\n",
        "git rev-parse --verify HEAD > target/rrflow-inventory-revision.txt",
        'test "$(cat target/rrflow-inventory-revision.txt)" = "$GITHUB_SHA"',
        "python3 scripts/ci/build_execution_inventory.py --check",
        "          python3 scripts/ci/test_analyze_sustained_conflicts.py\n",
    ):
        require(block.count(required) == 1, f"CI inventory generation missing: {required.strip()}")
    require("continue-on-error:" not in block and "    if:" not in block,
            "CI inventory and conflict analysis cannot be bypassed")
    order = (
        "set -euo pipefail",
        "git diff --exit-code",
        "git diff --cached --exit-code",
        'test -z "$(git ls-files --others --exclude-standard)"',
        "python3 scripts/ci/build_execution_inventory.py\n",
        "git rev-parse --verify HEAD > target/rrflow-inventory-revision.txt",
        'test "$(cat target/rrflow-inventory-revision.txt)" = "$GITHUB_SHA"',
        "python3 scripts/ci/build_execution_inventory.py --check",
    )
    positions = [block.index(item) for item in order]
    require(positions == sorted(positions) and verify.index(marker) < verify.index(upload),
            "CI inventory must validate source, bind revision, check and retain in order")
    artifact = verify.split(upload, 1)[1].split("      - name:", 1)[0]
    for required in (
        "uses: actions/upload-artifact@cf430e030ddbb5b0abf93d22962f4752f3646cd9",
        "target/rrflow-1.0-file-plan.jsonl",
        "target/rrflow-inventory-revision.txt",
        "if-no-files-found: error",
    ):
        require(required in artifact, f"CI inventory artifact missing: {required}")
    require("continue-on-error:" not in artifact and "    if:" not in artifact,
            "CI inventory retention cannot be bypassed")


def workspace_packages() -> set[str]:
    with (ROOT / "Cargo.toml").open("rb") as source:
        workspace = tomllib.load(source)["workspace"]
    return {Path(member).name for member in workspace["members"]}


def optional_feature_packages() -> set[str]:
    packages = set()
    diagnostic_packages = set()
    for manifest in (ROOT / "crates").rglob("Cargo.toml"):
        with manifest.open("rb") as source:
            document = tomllib.load(source)
        features = set(document.get("features", {})) - {"default"}
        # This explicit diagnostic feature changes the primary's private stdin
        # contract. Its separate native job owns the diagnostic corpus; ordinary
        # installed suites continue to qualify the default artifact.
        if "crash-test" in features:
            diagnostic_packages.add(document["package"]["name"])
            features.remove("crash-test")
        if features:
            packages.add(document["package"]["name"])
    require(diagnostic_packages == {"rrd-lsm", "rrd-store", "rrd-engine", "rrflow-cli"},
            "diagnostic feature must remain within its four-crate composition")
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
            if path == REUSABLE:
                require(
                    not any(re.match(r"(?:ref|repository|path)\s*:", entry) for entry in body),
                    f"candidate checkout must use its triggering source revision: {relative}:{line_number}",
                )
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
        and "cargo build -p rrflow-cli --bin rrd-engram-controller --locked" in block,
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


def verify_sdk_conformance(block: str, verify: str) -> None:
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

    marker = "      - name: Exercise installed Python attunement recovery\n"
    require(block.count(marker) == 1, "installed Python attunement step must occur once")
    step = block.split(marker, 1)[1].split("      - name:", 1)[0]
    expected = (
        "        timeout-minutes: 10\n"
        "        shell: bash\n"
        "        run: |\n"
        "          set -euo pipefail\n"
        "          cargo build -p rrflow-cli --bin rrflow --profile test --locked\n"
        '          export RRFLOW_TEST_BINARY="$GITHUB_WORKSPACE/target/debug/rrflow"\n'
        '          test -x "$RRFLOW_TEST_BINARY"\n'
        '          uv --directory sdks/python run --frozen pytest -o addopts= --color=no -q tests/test_installed_attunement.py | tee "$RUNNER_TEMP/rrflow-python-attunement.txt"\n'
        '          grep -Eq \'^2 passed in [0-9.]+s[[:space:]]*$\' "$RUNNER_TEMP/rrflow-python-attunement.txt"\n'
    )
    require(step == expected, "installed Python attunement step must run the exact unskipped test")
    policy_marker = "      - name: Enforce repository CI policy\n"
    require(verify.count(policy_marker) == 1, "repository CI policy step must occur once")
    policy = verify.split(policy_marker, 1)[1].split("      - name:", 1)[0]
    require(policy.count("          python3 scripts/ci/test_sdk_attunement_policy.py\n") == 1
            and "continue-on-error:" not in policy and "        if:" not in policy,
            "SDK attunement policy regressions must run without a bypass")


def verify_standalone_rrflowdb(jobs: dict[str, str]) -> None:
    build = "cargo build --locked -p rrflow-cli --bin rrflow -p rrd-client --example rrflowdb"
    run = '"$python_command" scripts/ci/run_rrflowdb_standalone.py "target/debug/rrflow${suffix}" "target/debug/examples/rrflowdb${suffix}"'
    step = ("      - name: Build and exercise the standalone rrflowDB example\n"
            "        shell: bash\n"
            "        run: |\n"
            f"          {build}\n"
            '          suffix=""\n'
            '          python_command=python3\n'
            '          if [ "$RUNNER_OS" = Windows ]; then suffix=".exe"; python_command=python; fi\n'
            f"          {run}\n")
    for name in ("topology-smoke", "engram-portability"):
        block = jobs[name]
        require(block.count(step) == 1 and block.count(run) == 1,
                f"{name} must unconditionally build and run the native standalone rrflowDB example")
    require(jobs["verify"].count("python3 scripts/ci/test_rrflowdb_standalone.py") == 1,
            "verify must run standalone rrflowDB oracle and process tests")


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


def verify_merge_group_rust_cache_scope(reusable: str) -> None:
    """Keep transient queue-ref Rust caches from evicting reusable entries."""
    actions = list(re.finditer(r"(?m)^      - uses: Swatinem/rust-cache@[^\n]+\n", reusable))
    require(actions, "reusable CI must retain its Rust cache actions")
    expected = "${{ github.event_name != 'merge_group' }}"
    for action in actions:
        rest = reusable[action.end():]
        end = re.search(r"(?m)^(?:      - |  [a-z0-9-]+:)", rest)
        block = rest[:end.start()] if end else rest
        conditions = re.findall(r"(?m)^          save-if:\s*(.+)$", block)
        require(conditions == [expected], "Rust cache must not save merge-group entries")


def verify_storage_contract_preflight(jobs: dict[str, str]) -> None:
    """The expensive reopen matrix requires the candidate's synthetic contract."""
    preflight = jobs.get("storage-contract-preflight", "")
    reopen = jobs.get("installed-reopen", "")
    gate = jobs.get("ci-gate", "")
    require(bool(preflight), "storage contract preflight is missing")
    require(
        "runs-on: ${{ fromJSON(inputs.linux-standard-runner) }}" in preflight
        and "cache-on-failure: false" in preflight
        and re.findall(r"(?m)^\s+if\s*:.*$\n", preflight) == [FULL_LANE_IF]
        and "continue-on-error:" not in preflight,
        "storage preflight must run on the selected trusted lane without bypass",
    )
    command = (
        "      - name: Validate candidate catalogue and synthetic storage projections\n"
        "        shell: bash\n"
        "        run: |\n"
        "          set -euo pipefail\n"
        "          cargo test --locked -p rrflow-qualification-storage --test storage_contract "
        "generated_schemas_and_synthetic_receipts_match_goldens -- --exact --list | "
        "grep -Fx 'generated_schemas_and_synthetic_receipts_match_goldens: test'\n"
        "          cargo test --locked -p rrflow-qualification-storage --test storage_contract "
        "generated_schemas_and_synthetic_receipts_match_goldens -- --exact"
    )
    require(preflight.count(command + "\n") == 1, "storage preflight must run the exact golden test")
    require(
        re.search(r"(?m)^    needs: \[changes, storage-contract-preflight\]\s*$", reopen) is not None
        and re.findall(r"(?m)^    if:.*$\n", reopen) == [FULL_LANE_IF]
        and "continue-on-error:" not in reopen,
        "installed reopen must depend on successful storage preflight without bypass",
    )
    step_sections = gate.split("    steps:\n")
    require(len(step_sections) == 2, "ci-gate must have one steps block")
    steps = step_sections[1]
    require(
        len(re.findall(r"(?m)^      - ", steps)) == 1
        and not re.search(r"(?m)^        if\s*:", steps),
        "ci-gate reduction step must be unconditional and unique",
    )
    result_variables = re.findall(
        r"(?m)^          ([A-Z][A-Z0-9_]+):\s+\$\{\{\s*needs\.[a-z0-9-]+\.result\s*}}\s*$",
        steps,
    )
    run_sections = steps.split("        run: |\n")
    require(len(run_sections) == 2 and result_variables, "ci-gate reduction body is missing")
    run_lines = [line for line in run_sections[1].splitlines()
                 if line.strip() and not line.strip().startswith("#")]
    expected_lines = [
        "          set -euo pipefail",
        '          test "$CHANGES_RESULT" = success',
        "          expect() {",
        '            if [ "$1" = true ]; then',
        '              test "$2" = success || { echo "$3: selected, expected success, got $2"; exit 1; }',
        "            else",
        '              test "$2" = skipped || { echo "$3: not selected, expected skipped, got $2"; exit 1; }',
        "            fi",
        "          }",
        '          if [ "$FULL_LANE" = true ] || [ "$SDK_LANE" = true ]; then SDK_SELECTED=true; else SDK_SELECTED=false; fi',
        '          if [ "$FULL_LANE" = true ]; then LIGHT_SELECTED=false; else LIGHT_SELECTED=true; fi',
    ] + [f'          expect "${lane}" "${variable}" {job}' for job, (lane, variable) in GATE_EXPECTATIONS.items()]
    require(
        run_lines == expected_lines,
        "ci-gate reduction must contain only the lane-aware success and skip checks",
    )
    require(
        "    if: ${{ always() }}\n" in gate
        and "continue-on-error:" not in gate
        and "      - name: Require every selected CI lane\n        env:\n" in gate
        and "      - storage-contract-preflight\n" in gate
        and "STORAGE_PREFLIGHT_RESULT: ${{ needs.storage-contract-preflight.result }}" in gate
        and 'expect "$FULL_LANE" "$STORAGE_PREFLIGHT_RESULT" storage-contract-preflight' in gate,
        "ci-gate must require the storage preflight result to be success on the full lane",
    )


def verify_lane_selection(jobs: dict[str, str], reusable: str) -> None:
    """Every partition runs on exactly its lane, and the selection is tested."""
    require("changes" in jobs, "candidate CI must select lanes before any partition")
    changes = jobs["changes"]
    for required in (
        "fetch-depth: 0",
        "python3 scripts/ci/test_select_ci_lanes.py",
        'python3 scripts/ci/select_ci_lanes.py --full --github-output "$GITHUB_OUTPUT"',
        'python3 scripts/ci/select_ci_lanes.py --base "$BASE_SHA" --head "$HEAD_SHA" --github-output "$GITHUB_OUTPUT"',
        'if [ "$SCOPE" = full ] || [ -z "$BASE_SHA" ]; then',
        "BASE_SHA: ${{ github.event.pull_request.base.sha || github.event.merge_group.base_sha || github.event.before }}",
    ):
        require(changes.count(required) == 1, f"lane selection is missing: {required}")
    require(not re.search(r"(?m)^    if:", changes) and "continue-on-error:" not in changes,
            "lane selection itself cannot be conditional or ignored")
    for required in (
        "      - name: Enforce pull-request limits\n",
        "          python3 scripts/ci/test_pr_governance.py\n",
        "          python3 scripts/ci/check_pr_governance.py\n",
        "      pull-requests: read\n",
    ):
        require(changes.count(required) == 1, f"pull-request limits are missing from lane selection: {required.strip()}")
    require(not re.search(r"(?m)^        if:.*\n(?:.*\n){0,3}.*check_pr_governance", changes),
            "pull-request limits cannot be conditional")
    for name in ("full", "docs", "scripts", "engine", "sdk", "packages", "dependents", "binaries"):
        require(f"      {name}: ${{{{ steps.lanes.outputs.{name} }}}}" in changes,
                f"lane selection must publish the {name} output")
    for job in FULL_LANE_JOBS:
        conditions = re.findall(r"(?m)^    if: .*$\n", jobs[job])
        require(conditions == [FULL_LANE_IF], f"{job} must run on exactly the full lane")
        require(re.search(r"(?m)^    needs: (\[changes(, [a-z0-9-]+)*\]|changes)\s*$", jobs[job]) is not None,
                f"{job} must depend on the lane selection")
    for job, condition in LANE_CONDITIONS.items():
        conditions = re.findall(r"(?m)^    if: .*$\n", jobs[job])
        require(conditions == [condition], f"{job} must run on exactly its lane")
        require("    needs: [changes]\n" in jobs[job], f"{job} must depend on the lane selection")
    fast = jobs["engine-fast"]
    for required in (
        "PACKAGES: ${{ needs.changes.outputs.packages }}",
        "DEPENDENTS: ${{ needs.changes.outputs.dependents }}",
        "BINARIES: ${{ needs.changes.outputs.binaries }}",
        "cargo build $BINARIES --locked",
        'test -n "$PACKAGES$DEPENDENTS"',
        "cargo test $PACKAGES --locked --no-fail-fast",
        "cargo clippy $PACKAGES --all-targets --locked -- -D warnings",
        "cargo check $DEPENDENTS --all-targets --locked",
        "cargo fmt --all -- --check",
        "python3 scripts/ci/check_generated_surfaces.py",
    ):
        require(fast.count(required) == 1, f"engine fast lane is missing: {required}")
    require(jobs["repository-invariants"].count(
        "cargo test -p rrd-engine --locked --test workspace_architecture") == 1,
        "repository invariants must run the workspace architecture tests on every light candidate")
    require(fast.count("        if: ${{ env.PACKAGES != '' }}\n") == 2
            and fast.count("        if: ${{ env.DEPENDENTS != '' }}\n") == 1
            and fast.count("        if: ${{ env.BINARIES != '' }}\n") == 1,
            "engine fast lane must skip package steps for a workspace-only selection and "
            "dependent compilation when nothing depends on the change")
    # Candidates never run the full matrix, so workflow and CI-script edits
    # must meet the CI policy in the script lane.
    for command in SCRIPT_LANE_POLICY_COMMANDS:
        require(jobs["scripts-policy"].count(command) == 1 and jobs["verify"].count(command) == 1,
                f"CI policy must run once in the script lane and once in verify: {command}")
    require(reusable.count("python3 scripts/ci/test_select_ci_lanes.py") == 2
            and jobs["verify"].count("python3 scripts/ci/test_ci_lanes_policy.py") == 1,
            "lane selection regressions must run in the changes job and the verify policy step")
    light_checkouts = checkout_steps(jobs["docs-policy"]) + checkout_steps(jobs["scripts-policy"])
    require(all("fetch-depth: 0" in body for _, body in light_checkouts),
            "documentation and script lanes need complete history for the inventory and knowledge checks")


def verify_nightly_caller(workflow: str) -> None:
    """The complete matrix still runs against main every day, as evidence about main."""
    trigger = re.search(r"(?ms)^on:\n(.*?)(?=^\S)", workflow)
    require(trigger is not None and re.fullmatch(
        r"  schedule:\n    - cron: \"[0-9 */,-]+\"\n  workflow_dispatch:\n\n", trigger.group(1)) is not None,
        "nightly caller must run on a schedule and on manual dispatch only")
    require("      scope: full\n" in workflow, "nightly caller must request the full matrix")
    require(workflow.count("uses: ./.github/workflows/ci-reusable.yml") == 1,
            "nightly caller must invoke exactly the reusable workflow")
    require("github.event.pull_request" not in workflow, "nightly caller never runs proposed code")
    require("  cancel-in-progress: false\n" in workflow, "a nightly result must not cancel another")


def main() -> None:
    workflows = workflow_sources()
    require(NATIVE_PREFLIGHT in workflows, "native macOS preflight workflow missing")
    verify_native_capacity_preflight(workflows[NATIVE_PREFLIGHT])
    caller = workflows[CALLER]
    reusable = workflows[REUSABLE]
    jobs = job_blocks(reusable)
    verify_merge_group_rust_cache_scope(reusable)
    verify_storage_contract_preflight(jobs)
    verify_candidate_admission(caller)
    verify_linux_arm64_ci(caller, reusable)
    warmer = workflows.get(WORKFLOW_DIRECTORY / "ci-cache-warm.yml")
    require(warmer is not None, "trusted macOS reopen cache warmer is absent")
    verify_macos_reopen_cache_warm(warmer, jobs["installed-reopen"])
    windows_warmer = workflows.get(WORKFLOW_DIRECTORY / "ci-windows-cache-warm.yml")
    require(windows_warmer is not None, "trusted Windows reopen cache warmer is absent")
    verify_windows_reopen_cache_warm(windows_warmer, jobs["installed-reopen"])
    verify_execution_inventory_ci(jobs["verify"])
    verify_native_sample_worker_ci(jobs["installed-reopen"], jobs["verify"])
    require(
        jobs["verify"].count("python3 scripts/ci/test_candidate_admission.py") == 1
        and reusable.count("python3 scripts/ci/test_candidate_admission.py") == 2,
        "candidate admission regressions must run once in verify and once in the script lane",
    )

    for trigger in ["pull_request:", "merge_group:", "workflow_dispatch:"]:
        require(
            re.search(rf"(?m)^  {re.escape(trigger)}\s*$", caller) is not None,
            f"candidate CI must include {trigger}",
        )
    # Pre-release work is committed straight to main (AGENTS.md), so the
    # development gate must run on exactly those pushes and on no other branch.
    require(
        len(re.findall(r"(?m)^  push:\n    branches: \[main\]$", caller)) == 1
        and len(re.findall(r"(?m)^\s*push:\s*$", caller)) == 1,
        "the development gate must run on pushes to main, and only main",
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
    def once_in(command: str, owners: tuple[str, ...], label: str) -> None:
        require(
            all(jobs[owner].count(command) == 1 for owner in owners)
            and reusable.count(command) == len(owners),
            f"{label} must run exactly once in each of {', '.join(owners)}",
        )

    once_in("python3 scripts/ci/check_documentation.py", ("verify", "docs-policy"),
            "documentation authority and local-link policy")
    once_in("python3 scripts/ci/build_execution_inventory.py --check", ("verify", "docs-policy"),
            "the exhaustive execution inventory check")
    once_in("python3 scripts/knowledge/render_capacity_profiles.py --check", ("verify", "docs-policy"),
            "the J-04 capacity projection rendering check")
    for command in (
        "python3 scripts/knowledge/test_capacity_profiles.py",
        "python3 scripts/ci/test_collect_storage_acceptance.py",
        "python3 scripts/ci/test_sustained_launch.py",
        "python3 scripts/ci/test_analyze_sustained_conflicts.py",
    ):
        once_in(command, ("verify", "scripts-policy"), "the J-04 capacity projection regression")
    verify_lane_selection(jobs, reusable)
    nightly = workflows.get(NIGHTLY)
    require(nightly is not None, "nightly full-matrix caller is absent")
    verify_nightly_caller(nightly)

    expected_jobs = {
        "changes",
        "docs-policy",
        "scripts-policy",
        "engine-fast",
        "repository-invariants",
        "native-arm64",
        "storage-contract-preflight",
        "installed-reopen",
        "installed-crash",
        "topology-smoke",
        "engram-portability",
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

    for job in ["topology-smoke", "engram-portability"]:
        for command in [
            "cargo test -p rrd-lsm --lib admission_tests --locked",
            "cargo test -p rrd-lsm --test write_admission --locked",
            "cargo test -p rrd-store --test write_admission --test semantic_commit_atomicity --locked",
            "cargo test -p rrd-engine --lib engine::tests::write_admission --locked",
        ]:
            require(jobs[job].count(command) == 1,
                    f"{job} must require encoded staging and rejection recovery: {command}")
        require(
            "cargo test -p rrflow-cli --test installed_lifecycle --locked -- --nocapture"
            in jobs[job],
            f"{job} must exercise the installed primary executable lifecycle",
        )
        for command in [
            "cargo test -p rrd-lsm --test mutable_limits --locked",
            "cargo test -p rrd-store --test mutable_admission --locked",
            "cargo test -p rrd-store --test logical_archive_resilience --test backup_catalogue configured_ --locked",
            "cargo test -p rrd-lsm --test write_diagnostics --locked",
            "cargo test -p rrd-store --test rrflow_kv_operator --locked",
            "cargo test -p rrd-lsm --test projected_lifetime --locked",
            "cargo test -p rrd-lsm --lib reader::lifetime_tests --locked",
            "cargo test -p rrd-lsm --test projected_output_limits --locked",
            "cargo test -p rrd-lsm --lib segment::output::tests --locked",
            "cargo test -p rrd-lsm --test projected_read_adversarial interleaved --locked -- --nocapture",
        ]:
            require(jobs[job].count(command) == 1, f"{job} must qualify native storage ownership and admission once: {command}")
    reopen = jobs["installed-reopen"]
    require(
        "CARGO_BUILD_JOBS: ${{ matrix.os == 'windows-latest' && '2' || '1' }}" in reopen,
        "ordinary reopen build must use two Windows workers and one elsewhere",
    )
    for platform in ["ubuntu-latest", "macos-latest", "windows-latest"]:
        require(f"- {platform}" in reopen, f"ordinary reopen corpus must qualify {platform}")
    for command in [
        "cargo test -p rrflow-cli --test installed_reopen --release --locked --no-run",
        "cargo test -p rrflow-cli --test installed_reopen --release --locked -- installed_100_acknowledged_reopens --ignored --nocapture --test-threads=2",
        "cargo clippy -p rrflow-cli --test installed_reopen --release --locked -- -D warnings",
        "cargo test -p rrflow-cli --test installed_reopen --release --locked -- sustained::capacity_workload::installed_capacity_workload_smoke --exact --ignored --nocapture",
        "cargo test -p rrflow-cli --test installed_reopen --release --locked -- sustained::fleet::installed_two_engram_isolation_under_concurrent_load --exact --ignored --nocapture",
        "run: >-\n          cargo test -p rrflow-cli --test installed_reopen --release --locked -- sustained::damage:: --ignored --nocapture --test-threads=2",
        "RRFLOW_REOPEN_RECEIPT_DIR:",
        "'attempts': 5150",
        "actions/upload-artifact@",
    ]:
        require(command in reopen, f"ordinary reopen qualification missing: {command}")
    require("--features" not in reopen and "--all-features" not in reopen,
            "ordinary reopen corpus must not activate diagnostic features")
    diagnostic = jobs["installed-crash"]
    verify_native_capacity_diagnostics(diagnostic)
    for platform in ["ubuntu-latest", "macos-latest", "windows-latest"]:
        require(f"- {platform}" in diagnostic, f"diagnostic primary must qualify {platform}")
    for command in [
        "cargo build -p rrflow-cli --bin rrflow --locked",
        "cargo test -p rrflow-cli --test installed_crash --features crash-test --locked --no-run",
        "cargo test -p rrflow-cli --test installed_crash --features crash-test --locked -- --nocapture --test-threads=2",
        "cargo test -p rrflow-cli --test lifecycle_process --features crash-test --locked",
        "cargo clippy -p rrflow-cli --bin rrflow --test installed_crash --test lifecycle_process --features crash-test --locked -- -D warnings",
        "C07C_NATIVE_PAUSE_V1=",
        "--skip publication_and_reclamation",
        "publication_and_reclamation --nocapture --test-threads=2",
        "Require both complete installed publication receipts",
        "RRFLOW_PUBLICATION_RECEIPT_DIR:",
    ]:
        require(command in diagnostic, f"installed diagnostic qualification missing: {command}")
    require(diagnostic.index("Verify diagnostic protocol") < diagnostic.index("Build explicit diagnostic"),
            "ordinary artifact exclusion must precede diagnostic compilation")
    require(
        "cargo clippy -p rrflow-cli --test installed_lifecycle --test lifecycle_process --locked -- -D warnings"
        in jobs["engram-portability"],
        "native installed lifecycle tests must not hide platform warnings",
    )
    require(
        jobs["topology-smoke"].count("python3 scripts/ci/run_offline_lifecycle.py") == 1
        and "python3 scripts/ci/test_offline_lifecycle.py" in jobs["verify"],
        "installed lifecycle must enforce offline isolation and test its artifact/mount policy",
    )

    for native in ["topology-smoke", "engram-portability"]:
        require("scripts/ci/run_filesystem_containment.py" in jobs[native],
                f"{native} must execute native mount containment qualification")
        require("scripts/ci/run_private_owner.py" in jobs[native]
                and "scripts/ci/run_offline_lifecycle.py --receipt" in jobs[native]
                and "cargo test -p rrflow-cli --test lifecycle_process --locked" in jobs[native]
                and "actions/upload-artifact@" in jobs[native],
                f"{native} must execute required native denials and bounded capture and retain sanitized receipts")

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
        'test "$CHANGES_RESULT" = success' in gate
        and all(
            f'expect "${lane}" "${variable}" {job}' in gate
            for job, (lane, variable) in GATE_EXPECTATIONS.items()
        )
        and set(result_variables) == {"CHANGES_RESULT", *(variable for _, variable in GATE_EXPECTATIONS.values())},
        "ci-gate must require success for every selected lane and a skip for every unselected one",
    )

    heavy_jobs = [
        "engine-fast",
        "repository-invariants",
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
    installed_lifecycle = (
        "cargo test -p rrflow-cli --test installed_lifecycle --locked -- --nocapture"
    )
    require(
        jobs["topology-smoke"].count(installed_lifecycle) == 1,
        "Linux topology smoke must run the primary installed lifecycle exactly once",
    )
    require(
        jobs["engram-portability"].count(installed_lifecycle) == 1,
        "macOS/Windows portability must run the same primary installed lifecycle exactly once",
    )
    require(
        "- windows-latest" in jobs["engram-portability"]
        and "- macos-latest" in jobs["engram-portability"],
        "engram portability must retain Windows and macOS development runners",
    )

    suite_count, package_count = verify_engine_suites(jobs["engine-suites"])
    verify_sdk_conformance(jobs["sdk-conformance"], jobs["verify"])
    verify_standalone_rrflowdb(jobs)
    feature_count = verify_optional_features(
        jobs["optional-features"], jobs["pgvector-feature"]
    )
    require(
        reusable.count("cargo build -p rrflow-cli --bin rrd-engram-controller --locked")
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
