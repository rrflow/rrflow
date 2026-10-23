# C-07g sustained evidence and installed capacity driver

**Status:** active implementation candidate; native profile qualification pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-c/c-07g-sustained-capacity`
**Owner:** OBJ-02, C-07g, POAM-035

[PR 72](https://github.com/rrflow/rrflow-development/pull/72) carries the
pre-effect brief. Starting revision is
`459e60fd8d7c27e41098cf069aa38146f888875f`; the native-capacity prerequisite
advanced to `521a37c6827fcd2c72b20b1367a79f197b61119e` while this package was
being authored. PRs 65 and 71 must pass and land before this dependent package.
Version remains 1.0.0. C-07, OBJ-02 and POAM-035 remain open.

## Brief, boundaries and implementation traceability

The fixed ordinary-primary 100-reopen corpus cannot establish capacity or a
24-hour soak. The existing C-07a manifest/ledger caps one document at 8 MiB and
10,000 attempts. Repeating it without segmentation would either grow memory
without bound or lose raw evidence. This package extends that owner with
frozen operating points, budgets, lifecycle lanes, schedules and a bounded
hash-linked segment writer/verifier. It preserves the paired-provider runner
and the fixed installed-reopen corpus. No new engine lifecycle authority,
provider dependency, runtime hook or external code generator is introduced.

The evidence contract binds candidate/configuration/environment/corpus,
explicit offered-rate timestamps, sequence continuity, a deterministic
50/20/25/5 operation mix, operation deadlines, raw timings, samples, declared
native enforcement, native unavailable measurements, and independent ledger
outcomes. A read or write timing record must correspond to the ledger request,
interval and outcome. Segment chains, fixed-count aggregates and total artifact
quotas are rechecked by offline verification. The first invalid segment is
retained separately and cannot become passing totals. Qualification requires
all declared phases, offered work, P99 threshold, error envelope, measurements,
lanes, seeds, reopens and minimum 24-hour duration. Characterization cannot
qualify a profile. A contract validator does not authenticate physical evidence.

The ordinary installed harness now accepts explicit configuration and a bounded
campaign lifetime while retaining its previous defaults. Its separate ignored
campaign entry invokes public session, transaction, query and vector APIs.
Prepared record/relationship/vector mutations share one atomic commit;
readback compares independent input values and relationships at the vector
read's captured cursor. Successful cancellation is planned maintenance work.
The driver uses bounded concurrent tasks and explicit offered timestamps;
concurrency rejection and deadline/unknown outcomes stay in the ledger. It
samples during load and separates the post-burst drain from offered work.

Live logical-owner and queue observations are projected from
`Database::mutable_memory`, `Database::projected_output_memory`, and
`RrdEngine::transaction_admission_snapshot` through the existing physical
storage port and `DiagnosticStorageSnapshot.admission`. Missing counters do
not become zero. Compaction debt is reported in its actual segment-count unit.
The new optional projection does not change installed configuration identity.
OpenAPI and all existing SDK projections are regenerated from the same owner.

The explicit campaign launcher binds clean source and the actual ordinary
harness, enforces a finite wall-clock duration and bounded captured output, and
sets native memory controls before the harness can spawn engine processes.
Linux creates only a new child below an explicitly delegated cgroup; it never
changes an existing host allocation. Windows adds per-process and aggregate
committed-memory ceilings to the existing owned job controller. It preserves
kill-on-close and bounded pipes. The macOS path is explicitly unavailable until
an independently verified native VM allocation is supplied. No automatic
privilege escalation, host provisioning, disk fill or service starts occur as
part of ordinary test discovery.

## Grounded reads and research

Read owners: README/alpha objective/roadmap/POA&M and the execution routine,
C-07 plan, J-04 capacity requirements, PR 72 brief, and C-07f journal. Code reads:
rrflow-eval storage mod/model/policy/ledger/validation/CLI and schema generator;
storage_contract fixtures and golden checks; installed_reopen, installed_crash,
support/process and support/capacity; rrd-contract lib and diagnostic types;
rrd-client operation/session; rrd-engine diagnostic/transaction/admission and
query_transaction/vector_index fixtures; rrd-query syntax/parser/query tests;
rrd-store engine/rrflow_kv physical evidence; rrd-lsm database/mutable_memory and
segment/output; run_offline_lifecycle, run_native_capacity,
owned_windows_process and windows_offline; generated-surface tooling.

Primary native references:
[Linux cgroup v2](https://docs.kernel.org/admin-guide/cgroup-v2.html),
[Linux proc measurements](https://docs.kernel.org/filesystems/proc.html),
[Windows job limits](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-jobobject_extended_limit_information),
[Windows private committed bytes](https://learn.microsoft.com/en-us/windows/win32/api/psapi/ns-psapi-process_memory_counters_ex),
and [Windows mapped regions](https://learn.microsoft.com/en-us/windows/win32/api/memoryapi/nf-memoryapi-virtualqueryex).
Linux memory.max may transiently exceed its limit; sampled overshoots still
fail the frozen envelope. Windows job limits govern committed memory, not RSS.
Private/mapped/resident observations remain distinct; platform-native semantics
must remain visible in comparisons. No upstream engine code was copied.

## Authoring commands, outcomes and surfaced failures

- `cargo check -p rrflow-eval --all-targets --locked` passed.
- `cargo clippy -p rrflow-eval --all-targets --locked -- -D warnings` passed
  before driver integration (18.83 seconds).
- The installed-driver check surfaced a missing CreateSession import and a
  typed Limit conversion; both were corrected. Reusing a target directory
  across dependency worktrees also exposed stale rrflow-eval fingerprints;
  `cargo clean -p rrflow-eval` invalidated that package only before rebuilding.
- Adding the public projection initially omitted its explicit root re-export;
  the compiler exposed and the implementation corrected that boundary.
- `cargo check -p rrflow-cli --test installed_reopen --locked` passed after
  public projection integration (14.42 seconds).
- `cargo clippy -p rrflow-eval -p rrflow-cli -p rrd-engine -p rrd-store
  -p rrd-contract --all-targets --locked -- -D warnings` passed (43.47 seconds).
- Native sampler lint then requested `is_null()` instead of a null comparison;
  corrected. The final native-sampler/launcher edits require the next recorded
  lint result, not inheritance of the earlier pass.
- The declared storage schema generator ran successfully; it regenerates
  synthetic contract projections and the campaign/segment/result schemas,
  never native workload evidence.
- TypeScript, Python, Java, .NET and Go generators ran successfully. Changed SDK
  files were normalized to LF. Generated-surface write/check passed: 42 HTTP
  operations, unchanged signal identity, OpenAPI
  `58273fc2418b1e8d2085f8f91fe05be06fdc42eafbeb7d6a8a24082affdbb163`.
- Python source compilation passed for the launcher and owned-process helper.
  It does not execute cgroup, job, engine or filesystem qualification effects.

No local runtime suite, workload, privileged resource control, disk fill or
24-hour soak ran. The owner requested implementation progress without repeated
local runtime campaigns. Required candidate CI remains the merge gate. New
contract regression definitions cover failed-segment retention/tampering,
sequence/schedule gaps, latency/rate admission, native missing measurements,
short-run rejection and quota admission before directory creation.

## Current limitations and stop conditions

This is an in-progress implementation, not C-07 completion. The installed driver
admits a single-estate configurable linked characterization corpus;
it explicitly denies full qualification/fleet execution until lifecycle lanes,
native allocation binding and remaining native measurements are integrated. macOS native memory
enforcement is unavailable. Exhaustion/recovery and 100-cycle evidence belong
to their prerequisite candidates. No 24-hour evidence exists for this change.

Stop on incorrect candidate/configuration, unsafe ownership, unknown effects,
missing ACK, partial mixed batch, changed receipt, lost failure record,
resource/output overrun, missing native facility or failed candidate pipeline.
Continue implementing at these owners; do not mark the roadmap complete based
on the existence or compilation of this code.

Final pre-candidate authoring update: the native sampler/driver strict Clippy
command `cargo clippy -p rrflow-eval -p rrflow-cli --all-targets --locked -- -D
warnings` passed in 3.03 seconds. Storage schema regeneration passed after the
raw-metric semantics and pre-operation evidence admission were documented.
The native Windows memory walker now distinguishes reaching the supported
address-space end from other query failures; partial mapping walks stay
unavailable. Evidence capacity is reserved before the next operating point,
with actual encoded/native write checks still applied at append.

## Configurable corpus and long-lived session implementation

Continued from `5060bcb88aefb757d3872b95d1e46d03061e425d`; merged the
native-capacity dependency correction `117c388385db3c2f6bc879c1b8bc4dc54192e471`.
The generated inventory conflicted during the merge. Staging the resolved
index and regenerating it restored the mechanical projection; it was not
resolved by choosing a stale branch copy.

The ordinary sustained driver now seeds the requested document count with
configured relation/vector fanout and dimensions, a minimum 1 KiB payload,
and deterministic varying ASCII source data. The one-record admission guard
is removed. Ceil-divided fanout can exceed a requested family minimum; the
receipt records the actual counts, including the separate graph anchor.
A complete second independent readback pass precedes measured load. Rolling
input, ACK and readback digests keep one corpus request in memory instead of
retaining every generated document. Typed corpus evidence is bound into the
first warmup segment and qualification requires its presence. Source payload
bytes and encoded mutation bytes remain separate; neither is physical disk
footprint or cold-cache evidence.

Setup has a frozen finite deadline, included in launcher/process lifetime.
Single-record dimensions/fanout/payload and query output are admitted before
allocation. Segment preparation is bounded by a conservative request-owner
allowance, retained-byte budget and one quarter of the engine allocation;
active requests have deadlines and segments have an additional bounded wall
clock. These are driver bounds, not substituted engine enforcement.

Session replacement occurs between joined segments and during streaming setup,
after explicitly closing the previous session. Segment/drain intervals are
bounded well below the absolute session lifetime. Query budgets now come from
the sealed installation instead of requesting one million keys against the
default one-hundred-thousand-key ceiling. Point reads include uniform and hot
sets; bounded ranges validate every returned identity/value. Existing graph
and vector readback checks now cover the configured fanout at one known cursor.

Additional grounded reads: client session/backup operations, engine transaction
and backup authority, installed lifecycle/reopen, contract deployment/query
limits, sustained evidence writer/validator and J-04/C-07 workload owners.
No new third-party implementation or research assumption was introduced.

Validation: strict Clippy initially rejected a manual modulo predicate; changed
to `is_multiple_of`. `cargo clippy -p rrflow-eval -p rrflow-cli --all-targets
--locked -- -D warnings` then passed (0.91 seconds). The declared storage
schema generator passed (3.65 seconds). Regression definitions cover complete
corpus readback, missing families and setup deadline rejection. No local
runtime suite or native campaign ran. Candidate CI remains required. Full
lifecycle lanes, native allocation binding, fleet execution, update/delete
histories and complete native qualification remain open; no objective,
roadmap or POA&M completion status changed.


## Installed reopen and backup interference lanes

Continued from `1681b098a65a0972118ee52ff55b97062fff8c87`. The same configured
estate now performs its frozen number of forced-process kills before warmup.
Every cycle commits a linked record/relation/vector operation, reaps the owned
server, reconnects, reads every preceding lifecycle effect before retry and
requires each original receipt plus an explicit idempotent replay. A final
complete seed-corpus readback checks the immutable project data across all
kills. Bounded raw cycle observations bind process IDs, monotonic times,
expected/effect chains and original/replayed receipt chains. A lane summary
without those matching observations is rejected. Lifecycle setup has its own
finite deadline (at most thirty minutes) and bounded cycle count, included in
the runner deadline. It is separate from the offered-load duration.

One bounded backup management client begins alongside the first mixed write
of measured load. It verifies the saved archive through the existing public
backup catalogue, binds the exact complete backup receipt and requires actual
client-observed overlap with an acknowledged write. Independent segment
readback still checks the live ACKs. This is backup-under-write observation,
not restored-backup readback or proof of simultaneous disk instructions.
The first measured readback supplies the warm-lane receipt. Cold-cache control
and externally retained reader interference remain explicitly unavailable;
requesting those lanes fails admission rather than silently omitting them.

The candidate 100-cycle workflow previously selected every ignored test in
installed_reopen. That would also select the new manually configured campaign
without its required manifest. Its command now selects the two existing
`installed_100_acknowledged_reopens` cases explicitly. This preserves the
required 200-cycle candidate corpus and keeps the optional sustained entry
behind its own explicit frozen invocation. No required case was removed.

Reads: installed lifecycle backup/reopen paths, client session/backup methods,
engine transaction read-stamp and backup authority, evaluation ledger and
sustained validators, native process ownership, and CI owner/workflow. Native
transaction leases contain logical read stamps; they are not evidence that a
client holds a physical page pin. No such equivalence was introduced.

Validation: `cargo clippy -p rrflow-eval -p rrflow-cli --all-targets --locked
-- -D warnings` passed after the reopen extension (4.29 seconds), backup
integration (2.51 seconds), and final lane guards (2.72 seconds). Storage
schema regeneration passed (6.07 seconds). Added regression definitions deny
missing prior ACK counts and changed replay receipts. No local runtime suite,
campaign, backup, process-kill corpus or soak ran. Native/full-size acceptance
still depends on explicit candidate facilities and evidence, including failure
artifact retention during preparation, native allocation binding, reader/cold
lanes, fleet, update/delete histories and the complete profile/24-hour matrix.
PR 65's resource-limit prerequisite passed its complete candidate chain and
merged as `6de0c5be56a6b7c888586186babb3b1648eb55ae`; C-07 remains open.


## Live native allocation verification

Continued from `8d8dfde02f1a87436d2831a80c6ca060a74b4737`. Resource samples
retain a typed launcher receipt identifying the exact harness, unpredictable
allocation nonce, memory limits, CPU quota and host logical processor count.
The driver hashes its own executable in bounded chunks before installation.
Each sample checks actual kernel membership and effective controls; a supplied
environment string alone cannot establish native enforcement. Evidence
validation requires the matching retained receipt and frozen campaign limits.

Linux checks both harness and engine cgroup membership, the canonical owned
nonce path, memory.max, disabled swap, grouped OOM behavior, PID bound and
cpu.max. Windows opens only the nonce-named job for query, checks both process
memberships, committed-memory ceilings, kill-on-close/no-breakaway flags and
CPU hard-cap settings. The launcher refuses a pre-existing named job and binds
all controls before resuming the suspended harness. Windows CPU rate uses the
measured logical-processor count; this is a quota, not dedicated-core affinity.
Missing platform controls remain failure/not-run, and macOS VM admission is
still unavailable. These controls do not authenticate a third-party benchmark
producer or make host RSS equivalent to Windows committed memory.

The owned Windows controller now admits invalid limit/name input before native
ownership and registers pipe write ends immediately. Every exceptional path
closes writer handles before joining bounded readers and quiescing its job.
The original anonymous-job callers retain their default behavior. Evidence
capacity admission now also precedes corpus/estate setup, not only measured
segments.

Read boundaries: native sampler, sustained driver/model/writer, explicit
campaign runner, owned_windows_process, windows_offline native structures and
job teardown. Primary references: [Linux cgroup v2](https://docs.kernel.org/admin-guide/cgroup-v2.html),
[Windows CPU rate control](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-jobobject_cpu_rate_control_information),
[Windows job queries](https://learn.microsoft.com/en-us/windows/win32/api/jobapi2/nf-jobapi2-queryinformationjobobject),
and [Windows extended job limits](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-jobobject_extended_limit_information).
These APIs are qualification infrastructure only, not new installed dependencies.

The compiler exposed that this workspace's SHA-256 output does not implement
LowerHex; encoding now uses the same explicit per-byte format as the installed
harness. Strict Clippy for rrflow-eval/rrflow-cli all targets passed (1.71
seconds). Python syntax compilation passed for both changed runner modules.
Storage schema generation passed (4.50 seconds). Regression definitions deny
native claims lacking receipts, substituted limits and mismatched native kinds.
No native control or runtime campaign executed locally. Full C-07 acceptance,
preparation-failure artifact retention, cold/pin/fleet lanes, update/delete
histories and actual profile/24-hour results remain open.
