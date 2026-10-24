# C-07d mutable admission and accepted reader-history reconciliation

**Status:** active local dependency reconciliation; diagnostic oracle repair and candidate CI pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-c/c-07d-mutable-lifetime-reconciliation`
**Owner:** OBJ-02 prerequisite, [C-07d](../../../roadmap/rrflow-1.0/gate-c-07.md), [POAM-035](../../../poam/rrflow-1.0-alpha/poam-035.md)

[PR 65](https://github.com/rrflow/rrflow-development/pull/65) continues from
local `948f7170aae47b17a39cdfa08a1e062dd41f6cb7`, tree
`2af5931df67e9db4951a3dfeb668aab19c288dcb`. The preimplementation brief and
[prior reconciliation](c-07d-mutable-admission-reconciliation.md) still govern.
The reader-history candidate `909b0113357c917b9cb8285c1763dc660606bfed`
was merged locally while its CI finished. It subsequently passed all 23 checks
in run `39832685534` and PR 64 merged as
`c8d8e2e2aa1bc3f3e7ca28a9aa2603b36a010e58`, tree
`75bcff09727bb5882338465f28e9819ab85b758e`, on 2026-10-22.

Both the accepted reader histories and pending mutable-limit implementation
are preserved at their canonical owners. Three merge conflicts affected CI
notes, POAM-035 and generated inventory. The two prose records retain both
bounded requirements; the inventory was regenerated after staging resolution.
No runtime implementation was changed by this reconciliation. Reviewed the
combined workflow/policy, records, full affected reader fixtures and complete
diff; no alternate reader, storage or policy authority was introduced.

Downloaded PR 64 native receipts qualify fresh/existing histories on Linux,
macOS and Windows: 600 forced restarts and 30,900 reconciled attempts, with no
violations and explicit owned-estate cleanup. The three required interleaved
history steps passed. GitHub temporarily retained an in-progress macOS job
status despite completed steps, then finalized successfully without a bypass.
These receipts accept PR 64, not this pending mutable-limit candidate.

## Combined commands and exposed failure

Windows used the isolated `target/c07d-mutable-build` target.

- `cargo test -p rrd-lsm --test mutable_limits --test projected_read_adversarial --locked`:
  12 passed, 0 failed, 0 ignored.
- `cargo test -p rrd-lsm -p rrd-store --locked`: stopped with 298 passes,
  1 failure and 1 existing owned-child ignore across 44 result groups.
- The failed test was `rrflow_kv_operator::native_write_path_diagnostics_are_explicit_bounded_and_semantically_inert`:
  `commit_mutex_wait_ns > 0` was false for an uncontended acquisition. This is
  an observed zero measurement; no hardware cause is established.
- `cargo clippy -p rrd-lsm -p rrd-store --all-targets --locked -- -D warnings`:
  passed. Lint success does not override the failed test.

The failed timing assertion cannot be silently retried or replaced by fabricated
nonzero duration. [PR 69](https://github.com/rrflow/rrflow-development/pull/69)
separately binds its oracle repair, including exact zero/nonzero forwarded
context, outer elapsed bounds and unchanged native durable evidence. PR 65
must reconcile that accepted repair and repeat the affected combined suites
before final candidate CI. No suite completion or merge is claimed here.

An exploratory log summary ran before the subsequent clippy log existed and
reported FileNotFound; the later command result confirmed lint success.
No repository or runtime effect followed from that log read.

## Checklist and limits

This is the predeclared accepted-predecessor reconciliation: owners, baseline,
scope, traceability, failure oracle, actual results and stop are recorded.
No new algorithm/research or runtime diagnostic change is needed here; the
separate clock-oracle PR owns its primary-source record. Preserve runtime
traces, limits and all previous failure evidence. Regenerate navigation and
inventory, validate version/workflow/docs/format and staged diff before commit.
No objective, roadmap package, POAM or alpha completion status changes.

Not run on this combination: the remainder of the aborted store suite,
engine rerun, final native candidate lanes, installed custom-budget operation,
actual disk exhaustion, capacity profiles or 24-hour soak. The prior engine
suite result remains prior evidence, not a result for this source. Do not push
or merge this as accepted until the exposed oracle failure and candidate CI
are resolved. Installed binding, aggregate resources and maintenance progress
remain open under C-07d.
