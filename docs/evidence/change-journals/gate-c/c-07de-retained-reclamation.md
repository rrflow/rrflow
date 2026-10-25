# C-07d/e retained mutable generations and installed reclamation

**Status:** active implemented candidate; native qualification pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-c/c-07de-retained-reclamation`
**Owner:** OBJ-02, C-07d/e, POAM-035

Starting revision
`fcb564efd009ee070c4af1ffa23d5215662ba37c`, tree
`fd5b79ab068cd8fab781798c00b5c296c9d28e62`. PR:
[65](https://github.com/rrflow/rrflow-development/pull/65).
The continuation brief was added to that PR before implementation.

Previously captured mutable sources could force unaccounted copies and ordinary
installed writes never invoked native garbage collection. The canonical native
write path now reserves active plus retained mutable versions, logical bytes
and generations before copy/WAL effects. Payload ownership retains reservations
through reader cancellation/drop and database close. Unexpected publication
failure after WAL append fences the writer and requires recovery.

Reclamation uses the existing manifest authority and anchored filesystem,
protects current/captured/checkpoint manifests and files, validates its entire
admitted inventory before deletion, and bounds metadata bytes, entries,
deletions and cooperative time. Partial successful work is synchronized and
retried before subsequent incoming WAL writes. Metadata reads retain the
existing native fault-observation boundary. No parallel collector or scheduler
was introduced. Eight required installed configuration controls lower through
the existing configuration/locator/install/reopen/restore composition. Required
protocol and five-language projections advance together; version remains 1.0.0.

Implementation traceability: Arc<Memtable> ownership becomes MutableGeneration
inside database and projected reader sources, preserving Memtable semantics;
the existing GC/checkpoint readers gain bounded variants within ManifestStore.
Existing compaction, fault, lifetime and recovery characterizations remain.
New maintenance definitions cover no-WAL pressure rejection, cancellation and
post-close source ownership, partial installed reclamation, and inventory
pressure preserving orphan evidence. They were compiled, not run locally.

Reads/review: README; C-07 and its implementation plan; POAM-035; change-authoring
procedure; complete new mutable_memory and reclamation modules; DatabaseOptions,
create/open/write/prepare/flush/compact/snapshot/GC and candidate removal;
ManifestStore load/checkpoints; read-view ownership/registry; native filesystem
read wrappers; native/store/engine error and absent-outcome mappings; installed
StorageLimits/default configuration/lowering; SDK capability fixture shapes;
contract installation/public fixture construction and evaluation refresh example.
Source diffs and generated projection differences were reviewed. Mechanical
changed-file identities are supplied by the execution inventory, not a second
prose authority. No source tree was moved or removed.

Research/adaptation: no external code or algorithm imported. This extends the
existing RRFlow authority and reference-counted lifetime model. Diagnostics:
retain typed pressure errors, expose pool current/peak/denied counters and GC
work counts without content-bearing metric labels. Logical reservations are
not RSS, allocator or independent-reopen host limits. Stop conditions remain
unsafe deletion, lost acknowledged data, hidden unbounded fallback or failed CI.

Validation performed:
- `cargo check -p rrd-lsm -p rrd-store -p rrd-engine -p rrflow-cli --all-targets --locked`:
  passed, 21.50 seconds.
- `cargo check -p rrd-lsm --all-targets --locked`: passed with added definitions,
  2.39 seconds.
- `cargo clippy -p rrd-lsm -p rrd-store -p rrd-contract -p rrd-engine -p rrflow-cli -p rrflow-eval --all-targets --locked -- -D warnings`:
  passed, 21.95 seconds.
- Offline typed installation and synthetic storage fixture refresh completed;
  five SDK generators completed; generated-surface parity passed with OpenAPI
  `68a17b8757c21aa38a881d75b8c3340dafafed0b8d6d2875be1a1e86dcab653c`.
- `cargo fmt --all`, Go formatting and `git diff --check`: passed.

Surfaced/corrected: incompatible String/str lookup branches; wrong native I/O
module name; an incorrect source path in a one-shot edit script (remaining
paths then updated); filesystem traversal hit a stale Python environment link
and was replaced with tracked changed-file enumeration. Earlier candidate
fcb564e CI run 39881827942 failed two public-contract checks because its default
configuration fixture and OpenAPI constant were stale; this change updates both.
Its successful lifecycle/crash jobs are prior-candidate evidence only.

Local runtime suites, benchmarks and 24-hour soak were not run, following the
owner's instruction to implement without another prolonged local testing cycle.
Required exact-candidate CI remains the merge gate. No runtime/scale success is
inferred from compilation or synthetic fixture generation.

Checklist: owner/scope and failure oracles bound in PR; source boundaries and
replacement behavior reviewed; research and resource decisions explicit;
regression definitions preserved/extended; failures retained; generated
projections refreshed; linked journal/index/inventory updated before commit.
Roadmap and POAM remain open. Pending acceptance, queue admission, recovery and
disk headroom, native exhaustion, profile sweeps and actual soak remain owned
by C-07. No merge or completed-alpha claim is made here.
