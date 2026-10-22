# C-07a honest write diagnostic timing oracle

**Status:** active locally verified oracle repair; candidate CI pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-c/c-07a-diagnostic-clock`
**Owner:** OBJ-02 prerequisite, [C-07a](../../../roadmap/rrflow-1.0/gate-c-07.md), [POAM-035](../../../poam/rrflow-1.0-alpha/poam-035.md)

## Baseline, brief and observed failure

[PR 69](https://github.com/rrflow/rrflow-development/pull/69) bound this repair
before implementation. Accepted main is
`c8d8e2e2aa1bc3f3e7ca28a9aa2603b36a010e58`, tree
`75bcff09727bb5882338465f28e9819ab85b758e`. The [baseline record](c-07a-diagnostic-clock-baseline.md)
is committed at `bdc0b9b90e2df10767fa4dc79cb8b6200dce3a21`.
PR 64 passed all 23 candidate checks in CI `39832685534`; this package changes
its tests and evidence, not runtime implementation.

While composing pending PR 65 with the accepted reader histories, the full
native/store suite stopped after 298 passes, one failure and one existing
owned-child ignore. The store diagnostic test observed zero nanoseconds for
an uncontended commit mutex acquisition and failed its `> 0` assertion.
This failure was retained rather than rerun until green. The hardware cause
of the sample is not established. Positive elapsed time is not a valid oracle
for whether an uncontended operation occurred.

## Canonical repair and traceability

`rrd-store/tests/rrflow_kv_operator.rs` captures an independent outer duration
around the complete append. The sum of its disjoint begin/commit lock waits
and its physical phase duration must fit that operation. Zero is permitted.
Capacity, one observed/retained sample, no drops, three physical mutations,
authoritative durability and the exact reopened semantic claim stay required.

`rrd-lsm/tests/write_diagnostics.rs` checks measured physical phases against
outer write durations; buffered sync remains zero and durability classes stay
exact. Neither WAL synchronization nor maintenance execution is inferred from
positive clock time. Actual maintenance counters remain asserted. The context
fixture now commits three known values with `(11,13)`, `(0,0)` and maximal
lock contexts, preserves the supplied fields exactly, checks saturating totals
24/0/MAX and reopens all three values. These are explicit forwarding fixtures,
not claimed real contention measurements. Existing collection capacity/drop
and actual physical I/O/durability tests retain their responsibilities.

The native workflow and policy require both targets on Linux/macOS/Windows.
CI notes and POAM-035 describe the failure and scope; this journal and generated
navigation/file inventory retain evidence. No runtime source, public schema,
format, product version, dependency or durable trace behavior changed. No
measurement is clamped, fabricated or delayed to obtain a positive result.
There is no second implementation or runtime authority.

## Research and review

Primary source: [Rust Instant](https://doc.rust-lang.org/std/time/struct.Instant.html),
reviewed 2026-10-22. Its clock is nondecreasing, not necessarily strictly
increasing between observations; elapsed operations also have documented
zero-duration behavior. This supports accepting zero, not a diagnosis of a
particular machine. No source code was copied or new dependency introduced.

Read README and its C-07/POAM owner chain and change-authoring procedure;
complete native/store diagnostic test files, native collector/phase clocks,
store transaction begin/commit timing and forwarding call sites, and the
existing durability/I/O responsibilities. Reread changed tests, workflow/policy,
CI and POAM records and the complete diff. Source and engine behavior remain
unchanged. Preserve existing trace capture; this test repair adds no runtime
observability overhead. Retain workflow-policy executable mode `100755`.

## Exact local verification

Windows commands use the isolated `target/c07a-clock-build` target.

| Command | Result |
|---|---|
| `cargo test -p rrd-lsm --test write_diagnostics --locked` | 3 passed, 0 failed. |
| `cargo test -p rrd-store --test rrflow_kv_operator --locked` | 3 passed, 0 failed. |
| `cargo test -p rrd-lsm -p rrd-store --locked` | 350 passed, 0 failed, 1 existing owned crash-child ignore across 56 result groups. |
| `cargo clippy -p rrd-lsm -p rrd-store --all-targets --locked -- -D warnings` | Passed. |

No new compile/test/lint failure occurred after this correction. The original
zero-duration failure remains recorded above and in PR 65's reconciliation
journal; it is not erased by these passing runs. The ignored child remains
executed by its existing parent crash tests; no ignore or allowance was added.

Format, frozen-version, workflow, documentation, navigation/inventory and diff
checks passed: 17 indexes, 409 nodes, 1,784 edges and 1,403 inventory records.

## Checklist and remaining gates

The preimplementation brief binds outcome/owner, exact current-to-target
behavior, source/test scope, failure oracle, acceptance commands, primary
research, trace/resource decisions and stop conditions. This record retains
baseline, changed paths, full-file/diff review, failures, test results and
not-run checks. POAM-035 gains the verified diagnostic-oracle deficiency;
no objective, roadmap package, POAM closure or alpha status changes.

Not run locally: full engine/workspace/SDK/optional-feature suites, native
Linux/macOS candidate lanes, installed custom resource limits, real exhaustion,
capacity profiles or 24-hour soak. The seed CI `39842741838` was deliberately
canceled and is not acceptance. Final exact-candidate CI must pass before merge;
PR 65 then reconciles this accepted repair and repeats its failed combined
suite. Stop on fabricated timing, lost bounded samples/effects, changed runtime
semantics, physical durability-check weakening, expanded scope or failed CI.
