# POAM-035 — installed storage failure and growth qualification

**Status:** active parent-owned POA&M detail
**Coordinate:** `rrflow://rrflow-instance/data/poam/rrflow-1.0-alpha/poam-035`
**Owner:** [RRFlow 1.0 alpha POA&M](../rrflow-1.0-alpha.md)

## Observed deficiency

At `b6052feef8df2aa86e5a866ff0c8da196bc8f2ab`, accepted C-06 supplies the
physical reader/format/cache and D-01 supplies native installed lifecycle
evidence. The [C-07 planning baseline](../../evidence/change-journals/gate-c/c-07-planning-baseline.md)
confirms passing maintenance, compaction and failure fixtures, followed by
the full local `rrd-lsm` suite. These do not complete installed storage
qualification:

- `FailureMode::StorageFull`/`Crash` return injected errors at logical
  boundaries; they do not themselves exhaust a filesystem or discard volatile
  OS/device writes.
- Maintenance thresholds permit one oversized atomic batch and do not by
  themselves establish a hard aggregate process/queue/pinned-memory limit.
- Automatic compaction retains untracked snapshot history; growth and
  reclamation need explicit retention/pin admission and sustained evidence.
- `snapshot_memory.rs` is compiled only on Linux and executes zero tests on
  Windows. A green local suite does not establish three-platform RSS bounds.
- No joined C-07 accepted matrix presently binds native faults, independent
  acknowledged histories and installed sustained-capacity results.

These were observed coverage and resource-contract gaps at the planning
baseline, without a reproduced lost acknowledged write. POAM-034 separately
owns whole-scope semantic constraint cost;
POAM-025/026 retain broader repair and observability deficiencies.

### C-07b demonstrated publication deficiencies and bounded remediation

At baseline `fca1a980245a9ff8634fa70fe21ee79a27eb2047`, deterministic native-call
fault tests reproduced an append after failed standalone sync, silent retry
after a zero-byte vectored write, loss of a newly created WAL name in the
stable-directory model, and another acknowledged write through the obsolete
WAL after an uncertain CURRENT rename. A retry/loss fixture also reproduced a
selected segment missing after its earlier rename's directory barrier failed.

The [C-07b implementation record](../../evidence/change-journals/gate-c/c-07b-io-fault-oracle.md)
binds regression tests and repairs: poison failed WAL sync, reject zero writes,
stabilize newly created/reused WAL and immutable names, fence dependent effects
after uncertain publication, and reconcile selected CURRENT/checkpoint names
on reopen before reclamation. These are abstract I/O-loss and native-call-error
fixtures, not observed device power failures. Exact-candidate CI is required
for landing; this remediation does not close POAM-035 or reopen C-06/D-01.

## Impact

### Encoded staging admission deficiency

At accepted main `952ec21b03a9e701967f2f15c0a37e3d34051fed`, two valid
individual values could exceed the existing 16 MiB encoded batch ceiling in
native and profile-neutral staged maps. Final batch validation rejected them
only at commit. A public semantic request with 4,096 claims also reproduced
an internal-error classification. Inspection showed that this generic error
retained prepared intent despite known pre-publication rejection. The [admission baseline](../../evidence/change-journals/gate-c/c-07d-write-admission-baseline.md)
and [repair evidence](../../evidence/change-journals/gate-c/c-07d-write-admission.md)
bind the reproduction, one format-owned incremental byte/count accountant,
typed rejection and engine abort/reopen/reuse qualification.

This bounded repair covers encoded staging and existing physical ceilings.
Caller inputs, serialization copies, aggregate concurrent owners, installed
configuration, waiting queues, maintenance headroom, replay and native RSS
remain separate C-07d obligations. It does not close this POA&M row.

### Installed reclamation call-path observation

At `cae8f3d9e3e0f733720a2f60ff5a8a3ab3094bf3`, ordinary writes already
reach automatic flush and compaction through `Database::prepare_write`.
Native `Database::garbage_collect` is exposed by `RrflowKvStore`, but its
remaining callers are tests and examples; no installed engine operation or
automatic native write path invokes reclamation. Component GC success cannot
therefore establish installed reclamation or bounded disk growth. C-07c/d/e
must supply and qualify that composition with authorization, budgets, pins
and recovery preserved. This observation does not select a second scheduler
or require a particular new public endpoint.

### Qualification impact

Component success could be mistaken for safe production growth, a true
disk-full result, power-loss durability or bounded multi-estate operation.
Storage-level success cannot establish native graph/vector scale, complete
SQL compatibility, model execution or UI/provider conformance.

## Owning gates

[C-07's ordered plan](../../roadmap/rrflow-1.0/gate-c-07.md) owns remediation
through packages a-h under OBJ-02.

Roadmap owner: [Gate C](../../roadmap/rrflow-1.0/gate-c.md).

## Closure evidence

C-07 requires independent exact histories,
I/O fault and native process/exhaustion evidence, hard resource admission,
snapshot/buffer/GC lifetime, and installed scale/soak qualification at declared
profiles. Gate C must accept the complete matrix at exact tested revisions
before this row closes. Missing facilities or unrun large profiles remain
visible gaps. J-02/J-03 retain OS/power-cut and distribution-profile claims;
J-04 retains full-product competitive evidence.
