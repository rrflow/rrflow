# C-07d: reclaim superseded physical history under owned reader lifetimes

**Status:** active evidence record; installed comparison and candidate CI pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/c-07d-tracked-history-maintenance`
**Owner:** [C-07](../../../roadmap/rrflow-1.0/gate-c-07.md), OBJ-02 and POAM-035
**Pull request:** [95](https://github.com/rrflow/rrflow-development/pull/95)

Starting revision `1399f709d1bd7ca10a28d7b9daa0fb3d01935372`, tree
`e8f5323b56f6ed9ad534445c4fde6ca37ef346a6`, integrates exact PR94 from fetched
main. The pre-edit brief maps conservative idle retention to explicit owned
snapshot retention through the existing compactor. `RrflowKvStore` keeps its
database behind a private mutex: transactions register their snapshots, reads
hold the mutex or own projected generations, and logical snapshot handles own
physical checkpoints. No untracked physical snapshot escapes that store.

## Scope and replacement evidence

`Database::maintenance_step_with_protected_snapshots` shares the same inner
quantum, input admission, immutable merge, publisher, cleanup and counters as
ordinary maintenance. It uses the existing explicit-compaction pruning rule
without flushing the mutable tail. Registered transactions and durable head
are included automatically; the caller supplies additional untracked snapshots.
Projected views and named checkpoints independently retain their original
generations. `RrflowKvStore::try_maintenance_step` selects this contract with no
additional untracked snapshots. Standalone `maintenance_step` and foreground
automatic compaction retain their conservative behavior. Logical versioned
keys are not obsolete physical versions and remain intact.

Changed native `database.rs`, `tests/maintenance.rs`, store `rrflow_kv.rs` and
`tests/rrflow_kv_operator.rs`, installed-lifecycle reference and POAM-035.
Created this journal and regenerated navigation/inventory. No format, default
limit, publisher, scheduler, public operation or deployment authority changed.

Read README, alpha OBJ-02/03, Gate C/C-07, POAM-035, installed storage limits,
authoring/traceability procedures and previous maintenance evidence. Reviewed
complete maintenance/compaction selection, merge, publication and reclamation
functions, database projected-read capture, store construction/private lock,
transaction and checkpoint reconciliation paths, physical evidence projection,
complete `inspection_cursor.rs`, engine and server maintenance modules, native
snapshot/compaction/maintenance histories, store operator/model-soak tests, and
claim append/key ownership. Reread changed functions/tests and final diff.
Local contracts and reproduced retention suffice; no external code adaptation
or speculative research optimization. Preserve resource limits and native
measurements. Stop on lost reader/history, untracked store snapshot escape,
unknown publication state or unsupported performance claims.

## Executed evidence

The first store oracle failed because 197 physical entries remained 197 after
maintenance. The repaired history retains 110 while preserving 33 distinct
logical claims, 32 immutable history keys, current/deleted values, a registered
old transaction, every page of a retained inspection stream and an acknowledged
unflushed write after reopen. The test compares logical history before and after
maintenance. During fixture correction, identical temporal claim coordinates
were found to overwrite the same logical key; distinct valid/transaction times
and the API's newest-first ordering corrected that expectation. Those failed
attempts are retained in the local logs, not reported as engine data loss.

The native history runs both retention choices over 12 versions. Conservative
maintenance keeps all 12; explicit retention keeps the three observable versions
for a supplied snapshot, registered transaction and durable head. A named
checkpoint's original segment remains readable after compaction/reclamation
and reopen, then is removed only after checkpoint release. The mutable tail
reopens unchanged and neither path advances its durable flush coordinate.

- `cargo test --locked -p rrd-store --test rrflow_kv_operator idle_maintenance_reclaims -- --nocapture`:
  initial red, two fixture corrections, then passed.
- `cargo test --locked -p rrd-lsm --test maintenance idle_pruning -- --nocapture`:
  passed, covering conservative and explicitly protected histories.
- `cargo test --locked -p rrd-lsm -p rrd-store`: 433 passed, two existing ignored.
- `cargo test --locked -p rrd-engine --lib idle_maintenance -- --nocapture`:
  one passed; logical cursors and mutable acknowledgements survive upkeep/reopen.
- `cargo clippy --locked -p rrd-lsm -p rrd-store --all-targets -- -D warnings`:
  passed.

Raw logs: `target/c07d-tracked-history-{red,green,corrected,fixed,native,suite}.log`.
Engine/lint logs use the same prefix. Formatting, documentation, navigation,
execution inventory and diff checks passed before commit. No native performance
or full-profile pass follows from these component histories.

## Native comparison retained from the prerequisite

Linux12 used source `4a64e773fb7e62b319946440b7527ff614b18469`, ELF
`a42c51af715900bae1005e4db41fa9ec6638949662dc426775d616545391a3d6`, campaign
`8202e9eef0bbc3517b46848cb24d39dd0bb88744c4468d73098d2b9e42c0e83c`, harness
`d2075b3ab3a6f16ca767628d763fdfb8d2fe53a47762e42998f56d215c9190d9`.
Its unchanged four-CPU/2-GiB characterization completed 1000 record
acknowledgements/readbacks, 5000 edges, 1000 512D vectors and two forced reopens.
Warmup and the first measured segment had zero failures; this does not qualify
the remaining load. Initial debt was 39. Backup interference exceeded the
unchanged deadline; the campaign failed after 432474 ms and three retained
segments, with cleanup confirmed. Receipts are locally retained under
`target/c07-linux-scale-evidence-12`; this repair still requires a fresh exact
candidate comparison. Windows17 remains an active earlier-candidate campaign,
not a 24-hour pass or evidence for this source revision.

Checklist: pre-edit PR brief, owner/source inspection, explicit replacement map,
red/green and protected-read histories, owning suite, journal and projections
are recorded. C-07 debt/latency convergence, macOS native-I/O receipt failure,
sampling coverage, full profiles, 24-hour soak and exact candidate CI remain
open. No roadmap/objective/POA&M closure, alpha or promotion claim.
