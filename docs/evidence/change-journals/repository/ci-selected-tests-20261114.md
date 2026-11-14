# One owner and a static check for the test names CI selects

**Status:** active change evidence

**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/repository/ci-selected-tests-20261114`

**Owner:** the [candidate CI pipeline](../../../operations/ci.md)

**Starting revision:** `development/main` at `b0cbc4df`, with the #384 branch (`36cae1f37`) merged in.

**Owner direction, 2026-11-14:** fix the source of why CI drifted, not the evidence that recorded it.

**Cause.** PR #384 moved test bodies into child modules, which renamed the tests. CI selects tests by name in three places, and nothing checked that the names still existed:
- Python harness constants, the lifecycle set copied into several files;
- `cargo test … -- FILTER` lines in the workflows;
- Rust tests that respawn themselves by name.

Every test passed. Nine native jobs then failed when their harnesses could not find their cases ([run 43574694617](https://github.com/rrflow/rrflow-development/actions/runs/43574694617)). A substring filter that matched nothing would have passed silently.

**Change:**
- **`scripts/ci/selected_tests.py`** owns every name the harnesses select, and resolves test paths statically from the Rust module tree: `mod` files, `#[path]` attributes, inline modules, and `#[test]` or `#[tokio::test]` functions. It counts cfg-gated tests on every platform.
- **`scripts/ci/check_selected_tests.py`** fails when:
  - an owned selector names no test;
  - the installed lifecycle target's test set differs from its required set;
  - a workflow `--exact` filter names no test, or a substring filter matches none;
  - a test-path string literal in a selected target no longer resolves.

  Each failure names the moved test's new path.
- **Harness scripts** import their names from the owner: `run_offline_lifecycle.py`, `run_native_capacity.py`, `run_sustained_capacity.py`, `run_filesystem_containment.py`, `test_linux_native_sample.py`, `test_native_sample_worker.py` and `test_windows_harness_image.py`. `test_offline_lifecycle.py` keeps its own copy of the lifecycle set on purpose, as the independent expectation the runner is tested against.
- **`ci-reusable.yml`** runs the unit tests and the check next to the code-shape ratchet in `verify` and in `repository-invariants`.
- **`docs/operations/ci.md`** describes the check.

**Removing #384's compatibility entries.** Owner, 2026-11-14: "we are in pre-release and need to get this existing code properly authored". To unblock its merge, #384 put 22 forwarding `#[test]` entries back in the old parent modules, each calling a body that had moved to a child. That kept the old names alive for CI. With the names now checked, that layer is removed:
- Each test's `#[test]` and `#[ignore]` attributes are back on its real function in the child module:
  - `installed_lifecycle`: `commands`, `plans`, `engrams_and_history`;
  - `sustained`: `campaign`, `capacity_workload`, `point_run`;
  - `capacity`: `cases`, `filler`.
- The filler unit tests are an ordinary `tests` module inside `capacity/filler.rs`.
- The selectors follow the tests to their real paths, for example `plans::installed_source_read_choice_is_explicit_and_reopens`, `sustained::campaign::installed_sustained_capacity` and `capacity::cases::installed_native_capacity_recovers_acknowledged_history`. They were updated in the owner, the workflow and its listing guard, and the workflow policy checks that pin workflow text.
- `test_offline_lifecycle.py` tests the runner against the owner's set instead of its own copy.
- The check also reads listing guards (`--list | grep -Fx 'NAME: test'`), a form the first version missed.

**Evidence:**
- **Static against compiled.** The static resolver was compared with the real `--list` output of the compiled test binaries on the #384 branch. For `installed_lifecycle`, `installed_reopen`, `installed_crash`, `rrd-lsm` `filesystem` and the `rrd-engine`, `rrd-store`, `rrd-lsm`, `rrd-query` and `rrd-vector` libraries, as well as `http_process` and `workspace_architecture`, every listed test was found statically. The only extra static names were cfg-gated tests compiled out on Windows.
- **Negative oracle.** Run against `ae02a970e`, the #384 head before its names were restored, the check reports 17 errors, each naming the test's new module path.
- **Commands:**
  - `python scripts/ci/test_selected_tests.py`: 7 passed. These cover module resolution, helpers, a moved selector, exact-set drift, workflow exact, substring and listing-guard filters, a stale self-spawn literal, and the real repository.
  - `python scripts/ci/check_selected_tests.py`: OK, with 19 owned selectors, 16 workflow filters and 5 test-path literals. Before the selectors were updated, it listed every moved name with its new path.
  - `test_offline_lifecycle.py`, `test_native_capacity.py`, `check_workflow.py` and the CI policy tests: OK.
  - `check_documentation.py` and `check_code_shape.py`: OK.

- **Rust harnesses after the entries were removed, on Windows:** `cargo fmt --check`: clean.
  - `installed_lifecycle`: 11 passed.
  - `installed_reopen`: 39 passed, 14 ignored.
  - `installed_crash` with `crash-test`: 25 passed.
  - clippy on those targets: clean.
  - `run_offline_lifecycle.require_lifecycle_tests` accepts the compiled harness's `--list` output.

**First candidate CI failure, and its fix.** Run [43624082216](https://github.com/rrflow/rrflow-development/actions/runs/43624082216) failed installed lifecycle and engram process on Windows and macOS. The preflight passed with 11 cases. The isolated qualification stage copies only the binary, the harness and `runner.py`, so the runner's new `selected_tests` import failed inside the sandbox. A local reproduction with the same file set showed `ModuleNotFoundError`.
- The fix stages the runner's imported modules, listed once as `STAGED_MODULES`.
- The Windows runtime probe checks them.
- `test_the_staged_runner_imports_only_what_is_staged` runs the staged runner from a directory holding only the staged files. It fails when `STAGED_MODULES` is empty.
- `test_offline_lifecycle.py` now runs 28 tests.

**Not established:**
- macOS and Linux runs of the refactored harness scripts happen only in candidate CI.
- The resolver does not expand `macro_rules!`-generated tests; none of the selected targets has them.
