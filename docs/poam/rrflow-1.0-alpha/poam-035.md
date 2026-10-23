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

### C-07e reproduced projected reader ownership defects

At `cae8f3d9e3e0f733720a2f60ff5a8a3ab3094bf3`, new public-reader fixtures
reproduced `PoisonedReadViewRegistry` when completing/canceling one of two
empty views, leaked admission after drop, and failure to release a populated
view while an empty generation remained live. The registry incorrectly equated
zero pinned bytes with zero remaining references. Independent weak-owner
fixtures also proved terminal streams retained their source memtables after
unregistering pins, for completion, cancellation and budget failure.

The [reader ownership repair](../../evidence/change-journals/gate-c/c-07e-read-view-release.md)
preserves zero-byte live references and drops captured generations/temporary
buffers before releasing the lease. Copied output batches remain independent.
Its focused local evidence does not close retained-buffer/native-memory or
installed maintenance qualification; exact-candidate native CI is required.

The subsequent [interleaved history evidence](../../evidence/change-journals/gate-c/c-07e-interleaved-lifetimes.md)
joins overlapping snapshot readers and transaction pins to checkpoint release,
compaction/GC, terminal admission reuse and reopen using the existing independent
model. Its bounded physical matrix does not close native memory, aggregate
output admission or installed reclamation obligations.

### C-07d native mutable admission and replay gap

At `f4e404be38a0c21a4a3665b1907bad2226f5c13a`, maintenance still exposes
only soft targets. Behavioral characterization with a new unenforced policy
surface accepted excessive atomic versions, grew hot-key state beyond its cap,
and materialized WAL state beyond a configured budget. The
[native mutable-limit record](../../evidence/change-journals/gate-c/c-07d-mutable-limits.md)
binds the repair and exact tests. Both native open paths must deny excessive
materialization without changing storage bytes. Active logical charge excludes
retained generations, allocator overhead and temporary decoding. Installed
bootstrap/configuration binding, aggregate memory and maintenance/queue/work
limits remain open; local native tests do not close this deficiency.

### Mutable admission adapter gap

Combining accepted main `dde6ef86d278b8c59cf1f61ebc2d3923186396d5`
with the native policy in PR 65 reproduced both byte and version rejections as
generic internal errors with prepared commit intent still present. Store open
wrappers also erased replay-limit classification, and store read-only inspection
could not receive explicit native options. The [propagation baseline](../../evidence/change-journals/gate-c/c-07d-limit-propagation-baseline.md)
and [implementation evidence](../../evidence/change-journals/gate-c/c-07d-limit-propagation.md)
bind typed propagation, absent-outcome intent cleanup and configured inspection.
Its joined histories must preserve prior acknowledgements and permit later work.
Installed startup/restore configuration, aggregate resource bounds and sustained
capacity remain open; this repair does not qualify or close C-07.

### Restore policy bypass

At the local PR 67 dependency, logical restore and nested application catalogue/
control replay opened native stores with defaults. Characterization accepted
work despite explicitly insufficient byte/version policies. The [restore
policy evidence](../../evidence/change-journals/gate-c/c-07d-restore-limits.md)
binds the reproduction and options-aware canonical restore/verify paths.
Partial-prefix rejection, exact resume and post-population replay checks are
component evidence. Installed configuration projection, aggregate recovery
resources, real exhaustion and sustained capacity remain open.

### Diagnostic timing oracle deficiency

During the pending mutable-limit/accepted reader-history combination, the
store diagnostics test rejected an observed zero uncontended mutex duration.
Its strictly-positive assumption exceeds the monotonic clock contract. The
[clock-oracle repair](../../evidence/change-journals/gate-c/c-07a-diagnostic-clock.md)
retains actual timings and checks independent outer bounds, exact forwarded
context and durable effects. It does not qualify contention performance,
resource enforcement or close this POA&M item.

### C-07d immutable maintenance admission

Inspection at `4dc1b43764c4f48f35f532ca2a575811e8587cb8` found count-only
compaction admission and complete hot-key history materialization. The
[bounded compaction implementation](../../evidence/change-journals/gate-c/c-07d-bounded-compaction.md)
adds installed byte/version/output/deadline/L0 limits, typed pressure and
ownership-preserving pruning. New history/pressure oracles are compiled but
unrun locally at the owner's direction. Candidate CI, native memory, installed
reclamation, real exhaustion and sustained capacity remain required; this is
implementation inventory, not closure evidence.

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

At accepted `dde6ef86d278b8c59cf1f61ebc2d3923186396d5`, projected reads
bound each batch and each stream's cumulative output but do not share output
admission across successive streams. Retained batches and individual Arrow
clones can outlive terminal streams. A policy-surface reproduction accepted a
second 97-byte output while the first remained retained under a nominal
97-byte pool, and accepted zero capacity. The
[retained-output baseline](../../evidence/change-journals/gate-c/c-07e-output-budget-baseline.md)
binds native preallocation reservation and last-allocation-owner release.
Its per-open-instance output cap is not a bound on independent reopen pools,
other memory families or installed total memory. C-07d/e remain open.

Component success could be mistaken for safe production growth, a true
disk-full result, power-loss durability or bounded multi-estate operation.
Storage-level success cannot establish native graph/vector scale, complete
SQL compatibility, model execution or UI/provider conformance.

## Owning gates

[C-07's ordered plan](../../roadmap/rrflow-1.0/gate-c-07.md) owns remediation
through packages a-h under OBJ-02.

Roadmap owner: [Gate C](../../roadmap/rrflow-1.0/gate-c.md).

The [retained-generation and installed reclamation change](../../evidence/change-journals/gate-c/c-07de-retained-reclamation.md)
adds configured pre-WAL copy admission and bounded native reclamation to normal
writes. Compilation is recorded separately from pending candidate CI. This
remediates implementation gaps without closing queue admission, recovery space,
real exhaustion, capacity profiles, soak or aggregate host-memory qualification.

The [finite transaction-admission change](../../evidence/change-journals/gate-c/c-07d-transaction-admission.md)
adds configured count/byte/wait admission to the existing serialization owner
and denies growth while bounded reclamation still has work. Its native runtime
acceptance remains pending; transport cancellation, recovery headroom and the
capacity/soak obligations are not closed by compilation.

The [recovery-admission change](../../evidence/change-journals/gate-c/c-07d-recovery-admission.md)
bounds native reopen and inspection metadata, file/read work, WAL size/frames
and cooperative time using installed policy. It preserves bytes on denial;
compilation does not qualify recovery-space reserve, an installed repair UX,
actual exhaustion, host limits or capacity/soak.

The [C-07f native capacity implementation](../../evidence/change-journals/gate-c/c-07f-native-capacity.md)
adds owned native images, installed failure/recovery histories and explicit
incomplete-tail candidate repair. Native acceptance, damage/backup matrix,
recovery reserve, operating profiles and 24-hour soak remain open; compiling
these paths is not qualification evidence.

The [C-07g sustained implementation](../../evidence/change-journals/gate-c/c-07g-sustained-capacity.md)
adds bounded campaign evidence, independent mixed-family installed readback,
live retained/queue telemetry and explicit native resource launch controls.
Its full corpus/lifecycle lanes and native qualification remain in progress;
no actual 24-hour result is implied by these implementation files.

Native candidate run 39893191950 also exposed an interrupted-repair publication
lease that could remain unavailable immediately after its owner returned.
The [publication-owner correction](../../evidence/change-journals/gate-c/c-07f-native-capacity.md#publication-owner-release-after-interrupted-repair)
adds explicit owner-bound unlock and duplicated-handle characterization;
corrected candidate acceptance is pending. This does not authorize bypassing
an active owner or changing the persistent lock inode.

The sustained sampler was found to invoke complete graph/history diagnostics
for physical counters, with a million-change ceiling and cross-model retries.
This can distort load or prevent sampling at the required corpus sizes.
The [C-07g storage observation change](../../evidence/change-journals/gate-c/c-07g-sustained-capacity.md)
adds a scope-bound authenticated physical-counter read using the same storage
projection. Candidate acceptance and large-corpus measurement remain pending;
full diagnostic consistency remains POAM-029's separate responsibility.

Native projected reads also accepted caller-controlled aggregate view/pin
ceilings. The [C-07g retention correction](../../evidence/change-journals/gate-c/c-07g-sustained-capacity.md#per-open-read-retention-ceilings-and-ownership-observations)
binds per-open ceilings to sealed installation policy and exposes actual
owner observations for drain validation. Candidate runtime acceptance is
pending; this does not establish the installed retained-reader workload.

Native physical transactions also lacked aggregate owner/write-buffer admission,
and the store retained a second copy of runtime snapshot writes. The
[C-07g transaction-owner correction](../../evidence/change-journals/gate-c/c-07g-sustained-capacity.md#aggregate-native-transaction-ownership-and-copy-removal)
adds sealed native ceilings and actual-owner release, removes duplicate intent
buffers, and preserves uncertainty when receipt lookup is resource-starved.
Candidate runtime acceptance remains pending.

The native transaction scan's requested limit was applied after full-range
materialization. The [bounded transaction-read correction](../../evidence/change-journals/gate-c/c-07d-bounded-transaction-scans.md)
reuses the projected reader and ordered pending overlay, with bounded result
copies and explicit work denial. Native candidate acceptance is pending;
other full-range APIs and caller-owned result residency remain distinct.

The installed mixed-model workload also exposed a real backup-under-write
failure: application export reread live source watermarks and rejected a
concurrent commit. The [candidate correction record](../../evidence/change-journals/gate-c/c-07g-candidate-workload-corrections.md)
binds one admitted read-only physical snapshot across archive, object closure,
catalogue and control reads. Native candidate acceptance remains pending;
this implementation does not qualify large-corpus backup latency or soak.

The combined two-estate flow exposed concurrent session-state replacement
between transaction begin/abort and commit. The [session admission correction](../../evidence/change-journals/gate-c/c-07g-session-admission.md)
extends the existing bounded transaction gate to begin, preview, abort, renewal
and closure, preserving exact identities and explicit conflicts. Native
concurrent characterization passes; full candidate acceptance remains pending.

The first optimized Windows Job characterization of the combined candidate
failed its measured backup/reader segment with nine uncertain requests. The
driver discarded request errors and omitted failed-case provenance; the
[native failure record](../../evidence/change-journals/gate-c/c-07g-native-failure-provenance.md)
binds the reporting repair and retained failure. The actual load failure remains
open. The current macOS capacity candidate also times out awaiting its first
WAL native-error event; bounded owned-process diagnostics are being retained
without weakening the real filesystem-error requirement.

The subsequent classified campaign identified three commit conflicts followed
by seven resource-exhaustion responses, with all acknowledgements read back.
The [rejected-transaction cleanup](../../evidence/change-journals/gate-c/c-07g-rejected-transaction-cleanup.md)
uses explicit engine-approved abort to release a failed workload lease and
classify rejection; ambiguous outcomes stay uncertain. Its real load acceptance
and sustainable envelope remain pending.

Characterization03 also exposed a scheduler pause: serial independent readback
after each offered segment manufactured overdue-load rejections and a resource
coverage gap. The [bounded-worker readback correction](../../evidence/change-journals/gate-c/c-07g-bounded-worker-readback.md)
moves the same independent oracle into existing admitted workers while keeping
real API latency and final restart readback. Actual timing acceptance is pending.

The lower-rate campaign reached drain and exposed an invalid probe ledger:
it claimed a present read without a recorded attempt. The
[drain receipt correction](../../evidence/change-journals/gate-c/c-07g-drain-probe-receipt.md)
records an executed public request, real receipt and independent verification
without counting idle-drain probes as offered throughput. Native flow acceptance
remains pending; the earlier failed resource policies remain retained.

## Closure evidence

C-07 requires independent exact histories,
I/O fault and native process/exhaustion evidence, hard resource admission,
snapshot/buffer/GC lifetime, and installed scale/soak qualification at declared
profiles. Gate C must accept the complete matrix at exact tested revisions
before this row closes. Missing facilities or unrun large profiles remain
visible gaps. J-02/J-03 retain OS/power-cut and distribution-profile claims;
J-04 retains full-product competitive evidence.

The [installed physical cursor candidate](../../evidence/change-journals/gate-c/c-07e-installed-storage-cursors.md)
adds session-owned native reader/output retention and raw sustained-interference
reconciliation. Runtime acceptance, complete capacity profiles and soak remain
open; a schema or compiled capability does not establish this proof.


The acceptance review found that author-supplied campaign SLOs could be looser
than J-04's owning nominal latency/error thresholds. The
[C-07h acceptance implementation](../../evidence/change-journals/gate-c/c-07h-acceptance-reconciliation.md)
projects those thresholds from their owner and checks raw measured observations
in addition to campaign qualification. It also binds native CI to the actual
tested revision and reports absent/failed operating points explicitly. Candidate
acceptance and the full profile/24-hour matrix remain pending; POAM-035 is open.

The same acceptance inspection found that a successful inner workload result
could outlive failed outer native-allocation cleanup. C-07h now requires a
separately bound launcher outcome/cleanup receipt and matches its allocation
against all raw samples. Missing or failed cleanup remains unqualified; the
native capacity/soak acceptance gap stays open.
The canonical ten-estate J-04 declaration was itself inadmissible: the shared
8 GiB ceiling was multiplied into an 80 GiB host reservation. The
[shared fleet budget correction](../../evidence/change-journals/gate-c/c-07g-shared-fleet-budget.md)
binds an explicit aggregate pool and rejects true summed-reservation overcommit.
Contract acceptance now passes; actual native fleet qualification remains open.
The [native flow and damage-policy record](../../evidence/change-journals/gate-c/c-07g-native-flow-and-damage-policy.md)
retains a successful small installed flow and all three platforms' capacity
cases on their exact candidates. CI also exposed a stale damage-fixture
assumption that backup publishes segments; the fixture now installs an explicit
flush policy. Updated native damage qualification, high-load storage-admission
resolution, complete profiles and the 24-hour run remain open.

The [budget-admitted compaction repair](../../evidence/change-journals/gate-c/c-07d-budgeted-compaction-selection.md)
reproduces a count-only selection failure even when a smaller native merge fits.
Selection now admits complete overlap closures and lower-level relief within
the existing limits. All 183 native storage tests pass; the old long-run candidate
was stopped with retained incomplete evidence. Repaired installed capacity and
full soak qualification remain required.

The [large-page acquisition repair](../../evidence/change-journals/gate-c/c-07e-large-page-acquisition.md)
reproduces a valid compacted key/history page exceeding the per-request I/O
cap. Bounded chunks now preserve the full-page checksum and exact historical
reads without raising any limit. Native component tests pass; attribution of
the retained installed Internal failure and updated installed capacity/soak
acceptance remain open.

The first actual shared-allocation fleet campaign found an idle-neighbour
segment that spent its whole time budget before the required verification
probe. The [idle deadline reservation repair](../../evidence/change-journals/gate-c/c-07g-idle-segment-deadlines.md)
derives bounded idle spans and enforces an outer deadline without changing
ledger acceptance. Forty evaluation tests and CLI lint pass; the repaired
quiet/equal/abusive installed campaign remains required.

The [combined native characterization record](../../evidence/change-journals/gate-c/c-07h-native-flow-characterization.md)
retains completed repaired Windows compaction flow, all three two-estate
interference trials, and a Linux WSL ext4 cold flow with zero resident pages
before restart. Fleet and Linux small runs have exact histories and no rejected
or unresolved work; the Windows 20 rps point exceeds error/latency thresholds.
All remain characterization. Required corpus profiles, complete 24-hour evidence,
current-candidate CI and final acceptance reconciliation remain open.

The [retained native failure record](../../evidence/change-journals/gate-c/c-07h-retained-native-failures.md)
preserves Windows13's coverage failure and Linux04/05's frozen setup deadline
failures with exact source/binary/campaign identities and confirmed cleanup.
Combined component checks at d209218d pass but do not override those native
failures. The repaired pipeline candidate and ongoing long characterization
remain separate evidence; full profiles and completed soak are still required.
