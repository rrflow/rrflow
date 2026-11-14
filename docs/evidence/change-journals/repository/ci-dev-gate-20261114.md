# Development gate for candidates; qualification matrix nightly and on demand

**Status:** active change evidence

**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/repository/ci-dev-gate-20261114`

**Owner:** the [candidate CI pipeline](../../../operations/ci.md)

**Starting revision:** the `agent/ci-test-selectors-20261114` branch at `51389f81` (PR #387), on `development/main` at `a4e32942`.

**Owner direction, 2026-11-14:**
- "is this seriously the CI we need for getting our prototype actually built".
- "we should have dev build, normal build later".
- The owner answered "go" to the proposal to make the qualification jobs non-blocking during pre-release.

**Current behavior.** `select_ci_lanes.py` held a `FULL_PATTERNS` list that sent a candidate to the full matrix whenever it touched any of these:
- the kernel or persistence crates;
- the CLI;
- most engine effect modules;
- any engine test;
- a workflow or CI script;
- the manifest or lock file;
- any unrecognized path.

The full matrix is native installed lifecycle on three operating systems plus ARM64, crash injection, the 100-cycle reopen and capacity. Each such candidate waited 60 to 90 minutes, twice (pull request and merge queue).

**Change:**
- **No changed path selects `full`.** `select_ci_lanes.py` now routes:
  - workspace inputs (`Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `.cargo/**`) to the engine lane, with a whole-workspace compile;
  - workflow, CI-script and repository-policy files to the script lane;
  - an unrecognized path to every light lane plus a whole-workspace compile.

  `full` is selected only by `scope: full` or an event without a base revision: the nightly and a manual dispatch.
- **`engine-fast`** skips its package test and clippy steps when the selection is workspace-only, and requires a package or dependent selection.
- **`scripts-policy`** now runs the CI policy tests and `check_workflow.py`, which candidates previously got only from the full matrix's `verify` job.
- **Nightly failures sound the alarm.** `merge-alarm.yml` also watches `nightly full matrix`, and `merge_alarm.py` treats a scheduled or manually dispatched run of `main` like a push to `main`: `Main is red: <workflow>` opens on failure and closes on success.
- **`check_workflow.py`** requires the CI policy commands once in the script lane and once in `verify`, and requires the engine lane's workspace-only guards.
- **`docs/operations/ci.md`** describes the development gate, the qualification matrix, when it runs, and what the gate does not prove.

**Commands, on Windows:**
- `python scripts/ci/test_select_ci_lanes.py`: 16 passed.
- `python scripts/ci/test_ci_lanes_policy.py`: 12 passed, including two new tests.
- `python scripts/ci/test_merge_alarm.py`: 10 passed. The new nightly test fails when the `merge_alarm.py` change is reverted.
- `python scripts/ci/check_workflow.py`: OK.
- `test_candidate_admission.py`, `test_linux_arm64_policy.py`, `test_native_sample_policy.py`, `test_cache_warm_policy.py`: OK.
- `check_selected_tests.py`, `check_code_shape.py`, `check_documentation.py`: OK.

**Not established:**
- The wall time of the development gate is not measured yet; this pull request's own candidate run is the first measurement.
- GitHub Actions behavior of the changed workflow is proven only by that run. The policy tests check the workflow text, not its execution.
- Candidates no longer prove macOS, Windows, ARM64, crash or capacity behavior. That evidence now comes only from nightly or manual qualification runs.
