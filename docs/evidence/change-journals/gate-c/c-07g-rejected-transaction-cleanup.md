# C-07g rejected transaction cleanup

**Status:** active implementation evidence; native characterization pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journals/gate-c/c-07g-rejected-transaction-cleanup`
**Owner:** [C-07g](../../../roadmap/rrflow-1.0/gate-c-07.md), OBJ-02 and POAM-035

Starting canonical revision: 65c97b5d0533b9f1cf0c82c69ec6287670029a6f.
PR72 pre-edit comment 6361850155 binds scope and failure oracle.
The preceding [failure-provenance correction](c-07g-native-failure-provenance.md)
enabled the second optimized Windows Job campaign at clean combined revision
788b344a259bf594ad0eebadd847164da720a826 to retain the actual causes.

That unchanged-shape campaign failed after 55.32s: its first measured segment
had 186 acknowledgements, three API conflicts, seven resource-exhaustion
responses and four driver concurrency rejections. Every acknowledgement had
independent readback. The existing error path retained open transaction leases
after failed commits, eventually exhausting the four-lease session admission.
The raw campaign digest is
aab3ba5f350a2755b0d7e3e840e15a0b6ab7c4bea3d345e6ecebc1c995636c18;
failed-segment digest is
ead996f9c372ac798f68596253986cb3864463062923b7738ea8399bcb011bf2.
Artifacts remain at `target/c07-windows-characterization-02/evidence`.
This is failed characterization, not a supported throughput or soak claim.

`support/sustained/workload.rs::Workload::commit` now invokes one explicit public
abort after a failed commit. An authenticated lease in Aborted state proves
the engine accepted cancellation and releases that lease. Only that outcome
is classified as rejection, with the original cause and abort receipt bound
into a digest. Failed or denied abort remains uncertain. There is no automatic
retry, broad-error-code absence inference, quota expansion or threshold change.
The engine's existing abort implementation refuses outstanding commit intent
and already-committed transactions; its authority is unchanged.

`support/sustained.rs::collect` carries that explicit rejection into the
existing independent ledger, preserving error counts and zero-error thresholds.
A rejected operation is not counted as successful load. Concurrent conflicts
and the sustainable envelope remain to be characterized.

Reviewed the complete workload begin/commit/error path, completion collection,
engine commit intent/known-pre-effect rejection and abort state transitions,
session admission, client error vocabulary and public error projection. These
existing owners supply the semantics; no external source adaptation or new
runtime/evidence schema is needed. No implementation is moved or deleted.
Changed paths are the two support files, this journal, POAM-035 and generated
navigation/inventory. Stop on implicit effect retry, ambiguous absence claims,
relaxed error thresholds or changed canonical transaction behavior.

`cargo clippy -p rrflow-cli --test installed_reopen --locked -- -D warnings`
passed (9.17s); formatting applied and the changed source was reviewed. The
same native campaign on a newly frozen source/binary remains the behavioral
oracle. No full suite or new capacity claim is inferred from Clippy. Candidate
CI and native macOS capacity diagnosis remain pending. Version, C-07 and alpha
status remain unchanged.
