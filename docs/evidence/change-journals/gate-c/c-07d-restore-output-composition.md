# C-07d configured restore and native resource composition

**Status:** active local qualification; accepted prerequisites and candidate CI pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-c/c-07d-restore-output-composition`
**Owner:** OBJ-02 prerequisite, [C-07d](../../../roadmap/rrflow-1.0/gate-c-07.md), [POAM-035](../../../poam/rrflow-1.0-alpha/poam-035.md)

[PR 68](https://github.com/rrflow/rrflow-development/pull/68) continues from
`bb72fe654f771d2c4fb70fe17a2c8e3835555eaf`, tree
`82bc531402b9cb404641cea9f096cebb2bf7e8fc`, under its preimplementation brief
and [implementation journal](c-07d-restore-limits.md). Compose propagation
candidate `8be3907946c1c13dfd101261f103d82d4fcd8996`, which includes accepted
PR 64 and pending PR 65/66/69. PR 69 CI `39843394936` is still running.
Neither dependency acceptance nor this package's final CI is claimed.

## Scope, traceability and review

Four conflicts involved the generated journal index/inventory, POAM-035 and
workflow policy. Retain restore and diagnostic requirements, stage resolution,
then regenerate both projections. No production source conflicted. Restore
and backup implementation/tests are unchanged from this package's reviewed
head; staged path comparisons confirm that. Native/engine sources match the
reviewed propagation candidate exactly. The complete combined diff and
workflow/policy/POAM/CI changes were reviewed; no alternate restore, allocation
or transaction authority was introduced and no behavior was removed.

The canonical restore implementation still carries caller-selected native
options through initial/recovered staging, semantic replay, catalogue/control
population, post-population reopen and object verification. Default wrappers
delegate to it. Partial committed prefixes are not published as complete
targets, and retries reconcile exact receipts. The original evidence and
native output/clock research remain applicable; no new external algorithm,
runtime diagnostics or aggregate memory claim is introduced by composition.

## Combined Windows evidence

Used isolated `target/c07d-restore-build`:

- `cargo test -p rrd-store --test logical_archive_resilience --test backup_catalogue configured_ --locked`:
  6 passed, 0 failed, 0 ignored.
- `cargo test -p rrd-store --locked`: 212 passed, 0 failed, 0 ignored across
  36 result groups.
- `cargo clippy -p rrd-store --all-targets --locked -- -D warnings`: passed.
- Format, workflow policy and staged whitespace checks passed. Version,
  documentation, navigation and inventory checks precede commit.

No new compile, runtime-test or lint failure occurred. The original red
restore-policy bypass and fixture corrections remain in the implementation
journal. The composed histories preserve all-family materialized data,
relationships, schema, vectors and evidence; reject insufficient byte/version
budgets without publishing a partial target; resume exact prefixes under an
adequate policy; and preserve bytes on denied published verification. These
are component restore histories, not installed policy or inference quality.

The existing brief/checklist still binds baseline, outcome, scope, owners,
replacement evidence, research/trace decisions and stop conditions. This
immutable linked node records the composition and results. No objective,
roadmap or POAM completion change. Not run on this combination: engine/native
reruns, final native candidate CI, installed policy binding, whole-restore
memory bounds, real capacity exhaustion, capacity profiles or 24-hour soak.
Input candidates' suites are prior evidence only. Reconcile accepted
predecessors and pass the exact candidate before landing. Stop on policy
bypass, altered receipt identity, lost data, partial publication or failed CI.
