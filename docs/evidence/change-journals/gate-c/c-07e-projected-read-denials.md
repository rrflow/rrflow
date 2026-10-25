# C-07e: preserve read-only resource denial

**Status:** active evidence record; candidate CI pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/c-07e-projected-read-denials`
**Owner:** [C-07](../../../roadmap/rrflow-1.0/gate-c-07.md), OBJ-02 and POAM-035
**Pull request:** [87](https://github.com/rrflow/rrflow-development/pull/87)

Starting revision `10850d13343f3506cf057693421fc5e28fa5c520` integrates
PR86 from fetched main. The pre-edit brief binds this bounded classification
repair, scope, oracles and stop conditions. No version or gate status changes.

## Behavior, scope and traceability

Native projected-read admission previously became a generic substrate string.
An authenticated public inspection request with a valid one-byte page budget
reported Internal instead of ResourceExhausted. `rrd-store::Error` now retains
the native resource, limit and observed amount. Existing query read conversion
and engine `storage_read_error`/`query_storage_error` map this typed denial to
the existing StorageAdmission/ResourceExhausted surface.

Ordinary durable conversion intentionally remains Storage/Internal and is not
automatically retryable. A read can fail during reconciliation after an effect
has published; resource pressure alone cannot prove rejection. This package
does not alter transaction intents, backup outcomes, retry policy, configuration,
wire schemas or operation catalogues. Corruption remains an internal failure.

Production paths: `rrd-store/src/error.rs` and its native resource re-export in
`lib.rs`; `rrd-query/src/error.rs`; `rrd-engine/src/engine/error.rs`. Regression
paths: existing store control-journal integration and engine storage-cursor/
write-admission tests. No implementation moved or deleted. The same canonical
conversions own the replacement; no string matching or parallel error system.

Read README and the owner chain, C-07/POAM-035, change-authoring procedure,
native and store error definitions/conversions, projected-read budgets,
query error conversion, engine error/cursor lifecycle, relevant existing tests
and durable transaction outcome reconciliation. Complete changed functions,
new tests and diff were reread. Generated inventory supplies file identities.
Research/source adaptation: none; the local contracts and reproduced native
denial define the behavior. Trace/resource decision: use existing public cursor
and ownership counters; preserve all limits and avoid payload logging.
Stop on weakened admission, misclassified durable outcomes, changed retry
semantics, expanded wire surface or unsupported completion claims.

## Failure, implementation and acceptance

The real public cursor oracle failed before implementation:
`batch_allocated_bytes` limit 1, observed 820, mapped to Internal. After repair
it returns ResourceExhausted, leaves the cursor registry empty with zero active
views/projected bytes, admits a 4096-byte retry and releases both owners.
An initial missing test import caused compilation failure and was corrected;
the subsequent red run proves the behavioral defect.

The store's actual native oversized journal page now verifies typed
OutputBufferBytes with limit 8192 and observed greater than the cap, before
and after reopen. Conversion tests preserve the resource amounts, read-only
pressure classification, generic durable uncertainty, nonretryability and
corruption classification.

| Command | Result |
|---|---|
| `cargo test --locked -p rrd-engine --lib physical_cursor_denial -- --nocapture` before repair | Failed: Internal versus ResourceExhausted |
| `cargo test --locked -p rrd-engine --lib denial -- --nocapture` after repair | Four passed, including two existing ACL denial tests |
| `cargo test --locked -p rrd-store --test control_journal small_journal_pages` | Passed |
| `cargo test --locked -p rrd-store -p rrd-query` | 291 passed |
| `cargo test --locked -p rrd-engine --lib` | 165 passed, two existing ignored |
| `cargo clippy --locked -p rrd-store -p rrd-query -p rrd-engine --all-targets -- -D warnings` | Passed |
| `cargo fmt --all` | Passed |

Local command evidence is retained under `target/c07e-read-denial-*`.
Navigation, inventory, documentation and diff checks accompany commit.
Checklist: pre-edit brief, canonical replacement, red oracle, implementation,
focused and owning suites, explicit resource/uncertainty decisions, full diff
review and linked evidence are recorded. Exact candidate CI is still required
for merge. The independent Linux08 comparison runs frozen PR86 source; it is
not evidence for this later candidate. Full profiles and a successful 24-hour
soak are not complete. C-07, OBJ-02 and POAM-035 remain open.
