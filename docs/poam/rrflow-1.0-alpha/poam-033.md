# POAM-033 — rejected native commits retain an unabortable intent

**Status:** active parent-owned POA&M detail
**Coordinate:** `rrflow://rrflow-instance/data/poam/rrflow-1.0-alpha/poam-033`
**Owner:** [RRFlow 1.0 alpha POA&M](../rrflow-1.0-alpha.md)

## Observed deficiency

The installed real-server local database test first failed when aborting a
transaction whose unique-constraint violation had already been rejected.
`RrdEngine::commit_transaction` persisted its commit intent before native
semantic validation; no authoritative commit outcome existed, but
`abort_transaction` returned `CommitInProgress`. The same path can retain a
typed-invalid or stale-cursor rejected transaction. Data atomicity remained
intact; transaction cleanup and quota availability did not.

## Impact

Rejected transactions retain an open slot and cannot be explicitly canceled,
eventually exhausting a session's transaction quota despite no data being
committed. Unknown-outcome recovery must remain distinct from definite rejection.

## Owning gates

C-03/C-07 transaction/recovery semantics and H-04 installed client operation.

Roadmap owners: [Gate C](../../roadmap/rrflow-1.0/gate-c.md) and
[Gate H](../../roadmap/rrflow-1.0/gate-h.md).

## Closure evidence

The candidate fix in [PR 26](https://github.com/rrflow/rrflow-development/pull/26)
clears the intent only after a typed definitive semantic/concurrency rejection
and an authoritative absent-outcome check. The transaction can then be
explicitly aborted. Unknown storage/I/O failures retain their intent for exact
retry and recovery. No storage format or second coordinator is introduced.

The [native database regression](../../../crates/transport/rrd-client/tests/rrflowdb_local.rs)
proves unique/type/stale-cursor rejection, unchanged rows/cursor, successful
explicit abort, subsequent valid work, receipt replay and reopen. Existing
engine recovery/function tests must also remain green. Local passing evidence
does not close the parent row: candidate CI and reviewed landing are required.

Owning boundaries: C-03/C-07 transaction/recovery semantics and H-04 installed
client operation. PostgreSQL isolation and savepoints remain H-08 work.
