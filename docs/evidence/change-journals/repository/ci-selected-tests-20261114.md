# One owner and a static check for the test names CI selects

**Status:** active change evidence

**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/repository/ci-selected-tests-20261114`

**Owner:** the [candidate CI pipeline](../../../operations/ci.md)

**Starting revision:** `development/main` at `b0cbc4df`.

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

**Evidence:**
- **Static against compiled.** The static resolver was compared with the real `--list` output of the compiled test binaries on the #384 branch. For `installed_lifecycle`, `installed_reopen`, `installed_crash`, `rrd-lsm` `filesystem` and the `rrd-engine`, `rrd-store`, `rrd-lsm`, `rrd-query` and `rrd-vector` libraries, as well as `http_process` and `workspace_architecture`, every listed test was found statically. The only extra static names were cfg-gated tests compiled out on Windows.
- **Negative oracle.** Run against `ae02a970e`, the #384 head before its names were restored, the check reports 17 errors, each naming the test's new module path.
- **Commands:**
  - `python scripts/ci/test_selected_tests.py`: 7 passed. These cover module resolution, helpers, a moved selector, exact-set drift, workflow exact and substring filters, a stale self-spawn literal, and the real repository.
  - `python scripts/ci/check_selected_tests.py`: OK, with 19 owned selectors, 14 workflow filters and 5 test-path literals.
  - `test_offline_lifecycle.py`, `test_native_capacity.py`, `check_workflow.py` and the CI policy tests: OK.
  - `check_documentation.py` and `check_code_shape.py`: OK.

**Not established:**
- macOS and Linux runs of the refactored harness scripts happen only in candidate CI.
- The resolver does not expand `macro_rules!`-generated tests; none of the selected targets has them.
