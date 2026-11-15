# The fast lane builds the binaries a changed crate's tests run

**Status:** active change evidence

**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/repository/ci-fast-lane-test-binaries-20261114`

**Owner:** the [candidate CI pipeline](../../../operations/ci.md)

**Starting revision:** `main` at `81d6ae1e5`, red: [run 43665714352](https://github.com/rrflow/rrflow-development/actions/runs/43665714352).

**What failed.** `engine fast lane`, `cargo test -p rrd-engine -p rrd-query -p rrd-server`, test `controller_process_kill_matrix_converges_across_start_and_stop_effect_gaps` in `rrd-server/tests/local_estate_driver.rs`:

```text
required workspace binary rrd-estate-controller is not built at
…/target/debug/rrd-estate-controller: No such file or directory
```

**Cause.** That test runs the `rrd-estate-controller` binary, which belongs to `rrflow-cli`. `cargo test -p rrd-server` builds only `rrd-server`'s own binaries. The full-matrix jobs build the controller first (`cargo build -p rrflow-cli --bin rrd-estate-controller --locked`); the fast lane, written on 2026-11-14, did not. The product change in the push was not at fault: this was the first push to change `rrd-server` under the development gate. It passed on the author's machine because the binary was already in `target/`.

**Change:**

- `scripts/ci/select_ci_lanes.py`: `TEST_BINARIES` maps a crate to the workspace binary its tests execute, and the selector prints a `binaries` output for the changed crates.
- `.github/workflows/ci-reusable.yml`: the lane selection publishes `binaries`; `engine-fast` runs `cargo build $BINARIES --locked` before testing, when there is one.
- `scripts/ci/check_workflow.py`, `test_ci_lanes_policy.py`: the output, the environment variable, the build command and its guard are required.
- `scripts/ci/test_select_ci_lanes.py`: a change to `rrd-server` selects the controller; and every crate whose sources call `workspace_binary("…")` must be declared with exactly the binaries it names, so a new such test cannot be added without the lane building its binary.
- `docs/operations/ci.md` states the rule.

**Before and after, reproduced locally on 2026-11-14:**

- With `target/debug/rrd-estate-controller.exe` removed, `cargo test -p rrd-server --locked --test local_estate_driver` fails with the gate's message: 3 passed, 1 failed.
- After `cargo build -p rrflow-cli --bin rrd-estate-controller --locked`, the same command passes: 4 passed.
- Replaying the failing push's paths through the selector prints `binaries=-p rrflow-cli --bin rrd-estate-controller`.
- `python -m unittest test_select_ci_lanes test_ci_lanes_policy`: 32 passed. `python scripts/ci/check_workflow.py`: passed.

**Not covered.** The declaration test recognises only `workspace_binary("name")` with a literal name. A test that finds another package's binary some other way would not be caught. A search for `CARGO_BIN_EXE_` uses outside a crate's own binaries was not done.

**Research decision:** none. **Trace decision:** none. **Roadmap/POA&M change:** none.
