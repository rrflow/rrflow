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


## Interrupted preparation and workload evidence

Continued from `c9a17d2e67d3310e19e024b4c701c4c30e9c6cd7`. Added a
bounded append-only `progress.jsonl` under the existing campaign evidence
owner. Its generated typed events record corpus request/effect identities,
observed acknowledgements, independently verified readback, lifecycle writes
and reopen observations, offered-load intents/outcomes, resources and backup
receipts. The campaign result binds its exact byte count, record count and
SHA-256; offline verification streams one bounded record at a time and rejects
missing, substituted, truncated or changed evidence. This is evidence, not
engine state or replay authorization. Missing outcomes remain uncertain.

Corpus intents are flushed and synchronized in batches of at most 64 before
that batch's first effect. ACK/readback checkpoints bound evidence retention
without collecting the corpus in memory. Lifecycle intent and ACK checkpoints
precede each kill. Segment intents are checkpointed before offered load;
completed outcomes and resource samples are retained as they arrive. Each
completed segment and final report synchronizes progress. A process/host death
can still lose observations since the last checkpoint; those absent records
cannot become successes. Quota admission includes progress bytes and reserves
space for the segment and final result before append. Native I/O errors remain
failures; a failed disk write cannot promise a complete final receipt.

The installed driver catches setup/runtime panics and attempts to seal a failed
campaign with retained progress. A quota-exhausted failure marker does not
prevent sealing the failed result. Added regression definitions for quota
rejection without file changes and missing/truncated/modified progress. A short
ignored installed workload case now exercises actual seeded records, edges,
vectors, point/range/cancel operations, concurrent backup/write and restart
readback before exact replay. The native candidate workflow invokes that case
separately from its existing 100-cycle cases. It explicitly makes no capacity
or soak qualification claim.

Read: storage sustained writer/validator/CLI and schema generator, storage
contract golden assertions, installed sustained scheduling/collection,
corpus/lifecycle/workload/native adapters, installed reopen harness and
candidate workflow. Changed those owning paths plus `storage/progress.rs` and
the generated progress/result schemas. Research decision: existing Rust file
sync and bounded evidence contracts suffice; no new runtime algorithm or
third-party implementation was imported. Failure oracle is preservation and
rejection of incomplete evidence, not green compilation.

Commands: `cargo clippy -p rrflow-cli --test installed_reopen --locked -- -D
warnings` passed (2.60 seconds). Expanded strict Clippy first surfaced a test
module before a production item; moved the test module to the end, then
`cargo clippy -p rrflow-eval -p rrflow-cli --all-targets --locked -- -D warnings`
passed (1.92 seconds). `cargo run -p rrflow-eval --example
refresh_storage_contract --locked` regenerated schemas (3.75 seconds), with
no native workload. No local runtime regression, native smoke, volume fill or
soak executed. Required candidate CI must execute the definitions. Cold/pin
interference, fleet allocation, fixed-working-set update/delete histories,
physical disk reserve and complete native profile/24-hour acceptance remain
open; OBJ-02, C-07 and POAM-035 are unchanged.


## Bounded live working set and mixed-family histories

Continued from `b188e877463c008c32bcb348654dfceb9a9a15f7`. The offered mixed-write
path now reuses finite slots bounded by the frozen segment-attempt quota,
independent of immutable seeded point/range records. A slot appears at most
once per segment. Successful independently read-back generations advance
through insert, update, atomic record/relation/vector retirement, recreation
and repeated update/retirement cycles. Rejected work leaves the generation
unchanged; uncertain or mismatched work fails the segment/campaign. Unique
transaction identities and retained earlier receipts are not rewritten.

Typed slot/generation coordinates appear in retained intents and outcomes.
The evaluation owner checks slot bounds, forbids duplicate slot writers in a
segment and derives acknowledged transition counters. Full qualification now
also requires observations of every transition type. Retired state is checked
as exact absent vector identities and empty record/relation queries at the
same captured cursor; it is a present logical retirement effect, not an
unacknowledged absent transaction. Slot-generation memory is included in
prepared-request admission. Canonical version history may still grow; bounded
live key count does not promise bounded storage.

After the complete offered schedule the driver kills/reopens its owned engine
and independently regenerates/read-checks the entire seed corpus and each
latest acknowledged working-slot state before any retry. A bounded raw final
readback event binds identities and rolling digest. Both the initial lifecycle
lane and final readback use the frozen lifecycle deadline; the outer launcher
reserves both. The short installed candidate case uses the same generator for
three slots ending retired, recreated and updated, verifies every transition,
then reads and exactly replays the final receipts after actual restart.

Read: public transaction retirement/data/vector/query contracts, runtime
retirement target validation, existing vector version/retirement visibility,
workload generator and independent readback, sustained scheduler, aggregate
validator, progress protocol, launcher and workflow policy. The PR brief was
updated before implementation. No upstream algorithm or schema of thought was
introduced. New contract regression definitions check transition cycles and
deny write coordinates on read attempts; existing qualifications now require
all transition categories. The user-facing evaluation instructions describe
actual commands, bounded artifacts and still-unavailable qualification lanes.

Remote verify job 120229929066 exposed that check_workflow.py still required
the old unfiltered ignored-test command. Its policy now requires the explicit
100-cycle selector AND the added workload smoke command; both original native
100-cycle cases remain required. `python scripts/ci/check_workflow.py` passed.
`python -m py_compile scripts/ci/run_sustained_capacity.py` passed. Strict
Clippy for rrflow-eval/rrflow-cli all targets passed (16.29 seconds initially,
2.24 seconds after final readback/smoke changes). Storage schema generation
passed (21.20 seconds). No local runtime tests or campaigns were executed.
Required candidate CI is still pending. Physical cold/pin lanes, multi-estate
allocation, recovery disk reserve and actual full profile/24-hour acceptance
remain open. No owner completion status changed.


## Storage observation without graph-history reconstruction

Continued from `f53ddab45e955b894b318f62bde0c3ae7406c3c7`. Source inspection
confirmed that the sustained sampler called the full diagnostic assembly,
including graph/history traversal up to a million changes and cross-model
consistency retries. This would distort measurements or fail at the requested
large-corpus sizes. Recorded the deficiency and correction in POAM-035;
POAM-029's full diagnostic consistency issue remains independent.

Added `diagnostics-storage-read` at `/v1/diagnostics/storage/read` in the existing
executable endpoint catalogue. Its `ReadStorageDiagnostics` request contains
only bounded instance scope; its response reuses DiagnosticStorageSnapshot.
RrdEngine applies the existing diagnostics permission and exact scope check,
then calls the same physical projection directly. No graph, changefeed, audit
or catalogue history is assembled. The result explicitly remains process-local
observation, not a consistent authoritative snapshot. Existing full diagnostic
behavior is preserved. HTTP dispatch, Rust client, sampler, native smoke and
all five generated language projections use this operation.

Regression definitions cover request bounds/unknown fields, read-only bearer
route policy, denied permission and wrong scope, successful physical reads
inside the existing diagnostic-conflict window, and actual client/server
physical counters/permission denial. The native installed workload case also
reads this operation. Shared conformance's expected endpoint count is 43;
Rust's typed HTTP-method count is 38. Full SDK/product qualification is not
inferred from generated operations.

Reads: contract diagnostic and endpoint/schema owners, server dispatch and
existing authenticated diagnostic handler, Rust operation bindings, engine
physical projection/authorization and diagnostic assembly, security and real
client/server fixtures, generated-surface scripts and SDK generators, reference
records and sustained sampler. PR brief was updated before implementation.
Research decision: reuse current storage projection/authorization, with no
new external dependency or copied algorithm. No implementation was removed
or moved; the sampler's call site now selects the bounded observation path.

Strict Clippy for rrflow-eval, rrflow-cli, rrd-contract, rrd-engine, rrd-client
and rrd-server all targets passed (19.76 seconds; 1.77 seconds after regression
definitions). All five SDK generators completed. The first Python invocation
lacked Ruff on PATH; the pinned `uv run --no-project --with ruff==0.17.1 python
sdks/python/scripts/generate.py` succeeded. Go generation initially used the
repository root, then succeeded from sdks/go. The TypeScript generator warned
that local Node 24.21.0/pnpm 10.30.3 differ from declared Node >=24.22.0 and
pnpm 12.14.0; this is authoring output, not toolchain qualification. A Windows
legacy-codepage read introduced documentation mojibake; restored the affected
UTF-8 bytes before commit and retained only intended edits.

Generated-surface write/check passed: 43 HTTP operations, unchanged signal
catalogue `239df2ced5974d4351ca01565369646964cb65c63d0fbae01dddccb4c3ac5a07`,
OpenAPI `f6557460e7fb5c20d48bef62366d4c4a84ff47ba71973a039076bfb59ce65241`.
Synthetic storage schema/fixture regeneration passed (4.26 seconds). No local
runtime suite, native campaign or soak executed. Candidate CI, native capacity
and full profile/24-hour acceptance remain required and unclaimed. Parent
objective/roadmap/POAM status remains open.

## Owned Linux cold-start lane and SDK candidate correction

Continued from `9416494979e1ba1617fdf3b617a50c8368c68c32` in PR 72.
The pre-effect PR brief binds this extension to OBJ-02/C-07g/POAM-035.
Added support/sustained/cold.rs and wired the existing installed campaign:
kill/reap its owned engine, resolve its installed native root, admit sorted
inventory under sealed recovery file/metadata/byte limits, reject aliases,
foreign ownership and cross-device paths, sync each file, issue aligned
DONTNEED advice and observe page residency without touching mapped data.
Mappings are at most 64 MiB and residency vectors at most 16 KiB. The lane
fails on remaining residency; it never changes host-wide cache settings.

Raw bounded progress retains each file identity, size and residency. A typed
ColdStartEvidence binds aggregate inventory, zero observed residency,
restart/first-read timings and independently expected readback digest to the
first warmup segment. The validator requires bounded counts/bytes, plausible
page totals, ordered lifecycle timing and matching lane digest. Regression
definitions cover missing raw receipt, resident pages, exceeded file/byte
limits, invalid timing/page size, impossible page totals and changed digest.
The launcher reserves this additional lifecycle interval. It measures cold
startup plus first read; startup can warm pages and residency observations
are point-in-time, not a guarantee against external future reads.

Read in full: new cold helper; current progress protocol; relevant sustained
validator, installed driver and smoke, launcher, evaluation instructions,
SDK endpoint-count assertions, installed configuration storage policy, owning
C-07 plan and repository change-authoring procedure. Reviewed full patch before
commit. No implementation was deleted or moved. libc remains the locked
existing dependency, now enabled for Linux test targets as well as macOS.
Research: [Linux mincore](https://man7.org/linux/man-pages/man2/mincore.2.html)
and [posix_fadvise](https://man7.org/linux/man-pages/man2/posix_fadvise.2.html)
require page-aware observation and make advice alone insufficient. No copied
upstream engine algorithm. Existing finite progress and explicit unavailable
platform behavior preserve the trace/resource decisions in the PR brief.

Candidate SDK job 120234601470 failed at .NET's stale 42-operation assertion.
Inspected all five SDK tests and corrected their same stale literal to the
43-operation catalogue introduced by storage-only diagnostics. This changes
expected catalogue size, not authentication, deadlines or response assertions.

`cargo fmt --all` passed. Strict Clippy for rrflow-eval/rrflow-cli all targets
passed (9.69 seconds, then 1.50 seconds after regression definitions).
`cargo run -p rrflow-eval --example refresh_storage_contract --locked` passed
(3.70 seconds); it regenerated synthetic schemas without a native workload.
`python -m py_compile scripts/ci/run_sustained_capacity.py` passed. These are
Windows compile/authoring checks; Linux-only code and native eviction still
require candidate execution. No local runtime suite, capacity effect or soak
ran. Reader-pin interference, multi-estate allocation, full native damage and
backup matrix, profile campaigns and 24-hour acceptance remain open. No
objective, roadmap or POA&M completion status changed.

## Ordinary installed damage refusal and backup restoration

Continued from `0c1743b3`; PR 72 brief and backup refinement were posted
before implementation. Added `support/sustained/damage.rs` under the existing
installed harness. Fresh and existing estates acknowledge actual mixed-family
records, relationships and vectors, create a verified public backup, then
acknowledge another WAL-backed batch. Eight cases independently remove or
corrupt WAL, segment, manifest and CURRENT bytes. Ordinary full verification
and serving must refuse each damaged estate without changing its streamed
byte inventory. Only the fixture restores saved bytes; subsequent reopening
must independently read every acknowledged effect before exact receipt replay.
Inventory is bounded to 4096 entries, 128 MiB file content, 16 MiB path metadata,
depth eight, and a 64 KiB hashing buffer; aliases and nonregular files fail.

The same case stages and applies the original backup through existing public
and installed restore contracts. Two plans must agree without mutating project
bytes, publication must preserve quarantine, all backed-up families must read
back, and the later batch must be absent. Backup intentionally excludes session
authorization and operation receipts: the old session must receive NotFound,
then a newly authorized session commits mixed work whose exact receipt survives
another restart. No automatic salvage or old session resurrection is implied.
The existing workload absence assertion is exposed within this module rather
than duplicated. Native installed-reopen CI explicitly selects both cases on
all three operating systems; its policy checker requires that selection.

Read/reviewed: complete damage helper, sustained module declarations and
workload verification/retirement logic; lifecycle backup helper; installed
restore plan/apply fixtures; store backup exclusion policy; engine session
authentication and API error mapping; CI job and checker. Existing canonical
restore publication and absence characterization remain the destinations;
no implementation is deleted, moved or replaced. No new research, dependency,
runtime authority, logging of credentials or changed capacity threshold.

Initial compile surfaced the nonexistent `rrd_client::ClientError` name;
corrected to the existing `rrd_client::Error::Api` and exact NotFound code.
`cargo fmt --all` and `cargo clippy -p rrflow-cli --test installed_reopen
--locked -- -D warnings` passed after the final inventory bounds (10.96s).
No local runtime test, disk effect, campaign or soak ran. Remote candidate CI
must execute this new matrix; compilation is not damage or restore evidence.
Gate C, OBJ-02, C-07f/g and POAM-035 remain open. Reviewed the complete changed
helper and wiring before commit.
## Streamed progress reconciliation and completion enforcement

Continued from `8c1dabca4ab0457c5a697bd067345e6a0946e767`; updated PR 72's
brief before implementation. Exact progress bytes previously did not prove
that a completed result contained final post-reopen readback or that cold
aggregate receipts agreed with raw file events. The existing progress owner
now streams a bounded accumulator per declared point/seed (maximum 32 x 32),
checks page size/count and frozen inventory bounds, folds each raw identity,
and matches cold aggregate counts, residency and digest. Offline campaign
verification consumes each matching cold segment once; completed artifacts
cannot omit an observed cold receipt. Partial interrupted/failed observations
remain admissible without implying completion.

Completed results require exactly one bounded final readback for every
point/seed, complete requested seed count, bounded working slots, valid process
identities and lifecycle duration, no retained failure, and readback after all
finished offered-load segments. The writer enforces progress and temporal
completion before publishing a result; offline verification repeats these
checks and additionally binds raw cold observations to segment receipts.
No day-long list of operations or files is retained. File observations now
include native page size so their page counts can be checked exactly.

Read/reviewed: complete progress module, sustained writer/aggregate verifier
and fixtures, cold recorder, generated progress schema. Added negative
regression definitions for omitted raw/aggregate/final events, partial cold
advice, changed bytes/digests, nonzero residency, repeated final readback,
incomplete seed count, failure after readback, segment mismatch/repetition and
readback preceding load. This is evidence validation, not authentication of
an arbitrary third-party measurement. No runtime policy or authority changes;
no upstream algorithm, new dependency, source deletion or move.

Strict Clippy initially rejected an unnecessary clone in a regression fixture;
changed it to slice::from_ref. Strict Clippy for rrflow-eval/rrflow-cli all
targets then passed (1.43 seconds); schema refresh passed (3.57 seconds).
Final temporal-order additions were formatted and strictly linted again.
No local runtime tests, capacity effects or soak ran. Required candidate CI
owns execution. C-07g still lacks installed retained-reader interference,
multi-estate capacity execution and full profile/24-hour evidence; Gate C,
OBJ-02 and POAM-035 remain open. Full diff reviewed before commit.
## Configurable read distribution and release campaign selection

Continued from `8f66c55a2f20a9eb9869a61444768ee6a2639990`; PR 72 brief
updated before implementation. Replaced the installed driver's embedded
25%-of-sequences/64-key behavior with required frozen ReadKeyDistribution:
uniform, exact hot key, or bounded hot set and traffic basis points. The
existing deterministic SplitMix64 coordinate selector now belongs to the
sustained model and validates corpus/region bounds. Non-hot traffic selects
the disjoint cold region. Point and range-start selection and prepared
request/effect identities use this policy. Mixed-write finite slots and their
generation cycle remain unchanged; read hot-key support is not write-hot-key
qualification. Evaluation instructions describe the actual input shapes.

Source inspection found that the launcher selected a debug harness despite
J-04's release-build requirement. It now explicitly builds with --release
and accepts only the resulting optimized, no-debug-assertion test artifact.
The existing executable/configuration digests and ordinary-binary diagnostic
marker exclusion still apply. This does not run a campaign or qualify scale.

Read/reviewed: full workload selector/preparation/independent readback;
sustained allocation validation/fixture, session and smoke constructors;
launcher Cargo selection, current J-04 workload owner and evaluation
instructions. No new research, dependency, algorithm adaptation or runtime
authority; the same noncryptographic coordinate mixing is used for workload
selection only. Regression definitions cover full-corpus selection, exact
hot-key selection, hot-only and mixed regions, invalid key/fraction/corpus
bounds, unknown fields and changed campaign identity.

Strict Clippy for rrflow-eval/rrflow-cli all targets passed (21.64 seconds).
Synthetic storage schema generation passed (5.64 seconds); Python syntax
compilation passed. No local runtime test, native resource effect, campaign
or soak executed. Candidate CI is pending; full retained-reader/fleet/native
profile and 24-hour evidence remain open. Full changed diff reviewed; no
objective, roadmap or POA&M completion status changed.
Navigation and execution inventory regeneration, documentation policy and CI workflow policy passed; staged diff whitespace check passed.

## Frozen post-burst ownership and debt recovery

Continued from `dccbefeb`; the PR brief named this refinement before edits.
Added required DrainPolicy at the existing storage campaign owner: declared
mutable/projected growth tolerances, debt threshold and 2..64 consecutive
settled samples. The first warmup storage observation must precede every
attempted warmup effect and becomes the measured baseline. Drain summaries
retain only the final declared ownership window, including missing gauges.
Outstanding/queued requests and logical admission bytes must return to zero;
active or fenced admission, excessive retained growth or debt fail. Warmup
must precede drain. Mutable canonical history is allowed only within its
explicit tolerance, not mislabeled as automatically reclaimable memory.

The installed driver validates drain before entering soak, and writer
completion/offline verification repeat the same derived-summary check.
Failure remains visible in retained samples and the driver's failed result.
No new sampler, runtime authority, automatic flush or policy relaxation was
added; synchronous maintenance can legitimately expose an unsustainable
operating point. Existing resource sampling/caps and raw segment hash chain
remain authoritative. Schema projections and eval instructions were updated.

Read/reviewed: full new storage/drain.rs; sustained campaign validation,
resource series, bounded summary merge, writer/offline completion, fixtures;
installed initial sampling/scheduling and drain loop; DiagnosticStorageAdmission
and store retained-mutable accounting; evaluation instructions. Regression
definitions cover every retained/debt/admission/fence failure, missing and
insufficient observations, repeated time, missing baseline, finite tail merge,
invalid sample windows, changed policy identity and late baseline observation.
No upstream adaptation or new dependency; no source deletion or relocation.

Candidate `8f66c55a` engine-authority job 120238838388 exposed stale exported
OPENAPI_DOCUMENT_SHA256 in rrd-contract/src/lib.rs. The regression correctly
compared the generated `f6557460...65241` against old `58273fc2...bb163`.
After locating the actual owner and refining the PR brief, updated that public
constant to the reviewed diagnostics-storage-read projection. The exact
regression assertion remains unchanged. Initial patch aimed at a test-local
literal and made no change because that literal does not exist there.

Strict Clippy for rrd-contract/public_contract passed (13.17s); rrflow-eval
and rrflow-cli all targets passed after final drain ordering (1.53s).
Synthetic storage schema refresh passed (9.57s); generated surface parity
passed for 43 HTTP operations and all seven signal projections; whitespace
check passed. No local runtime suite, native capacity effects or soak ran.
Candidate CI must execute these regression definitions. Gate C, C-07f/g,
OBJ-02 and POAM-035 remain open; reader-pin/fleet execution, disk reserve and
full profile/24-hour qualification remain unfinished. Full changed diff
reviewed before commit.

## Candidate workflow parse repair

Candidate `a5084140` runs 39901929763 and 39901935907 failed before any jobs
were created. Reproduced the committed workflow's YAML parse error at line
502: the unquoted Rust filter ends in `::` followed by a space. Changed its
run value to a folded YAML block, preserving the exact command and both cases.
The existing workflow policy now requires that safe scalar form. Read the
failed run metadata, complete changed step and relevant command-policy list;
no execution or acceptance requirement changed. Local PyYAML parsed the fixed
workflow successfully and rejected the prior committed bytes at the reported
position; repository workflow policy passed. This was syntax validation only,
not a local engine test. Candidate CI must restart on the corrected head.

Dependency merge: incorporated C-07f `9f0f3bf3` native reserve admission after
its source/projection review. Resolved conflicts in generated SDKs, synthetic
receipts, current reference digests and inventory by regenerating from the
combined owners, preserving the storage-only endpoint and drain contract.
The combined OpenAPI digest is
`f675d37054cd59fbc001b8eb73c183dade10444b3168b81aac23f6d9c6339b3e`;
all 43 operations and seven signal projections pass parity. Storage campaign
and result schemas now include the installed reserve as well as drain policy.
Execution inventory and whitespace checks passed. Native C-07f receipts from
ebb702 remain tied to that historical candidate, not this combined revision.

## Owning capacity-profile binding

Starting revision `40f0f6b57be80d126abf6bc0f5edc2c56fb6736d`. Before implementation, PR #72's brief bound this work to OBJ-02/C-07g: qualification previously accepted an arbitrary owner digest and reduced CPU/memory/corpus under any profile label. The new `storage/capacity.rs` validates qualification against a mechanically generated `eval/capacity-profiles-v1.json`. Its sole owner is J-04's workload table. The renderer hashes the exact owning section, parses all four rows, resolves fleet/large-data inheritance and fails on unfamiliar grammar. No duplicate hand-maintained hardware table. `storage campaign profiles` exposes those coordinates offline. Qualification now binds owner digest, profile ID, exact allocation, minimum record/edge/vector/source corpus, dimensions and declared load coordinates. Fleet is explicitly refused pending per-estate interference evidence; individual admitted points do not prove the full sweep. Reduced characterization is unchanged.

Complete affected functions/modules read: capacity renderer, storage CLI/mod, campaign validation, qualification/result reducer and synthetic fixtures; owning C-07 capacity/failure sections and J-04 workload ladder; workflow verification block and its policy; evaluation usage record. Regression definitions cover every allocation/corpus dimension, changed owner digest, unknown profile, unsupported fleet and out-of-sweep rate/client coordinates. Python definitions cover owner edits, inherited allocation and changed grammar. Required CI runs projection drift and parser characterization. No private reasoning or model trace is an acceptance oracle. No new runtime telemetry or dependency.

Authoring failures: the initial strict renderer assumed both standard rows said `concurrent clients`; Single service says `clients`. Accepted precisely that optional word and regenerated all four coordinates. Initial `cargo fmt --all -- --check` reported formatting only; applied formatter. `cargo clippy -p rrflow-eval --all-targets --locked -- -D warnings` passed (15.02s). Renderer generation/check and workflow policy passed. No local runtime suite, native workload, resource-control effect, disk-fill or soak executed. Candidate `39903111939` also exposed execution-inventory drift: the untracked renderer had entered the previous inventory without being committed. This package includes the file and regenerates the complete inventory. C-07/OBJ-02/POAM-035 remain open. Next candidate must prove the unchanged runtime and qualification oracles.

## Per-open read-retention ceilings and ownership observations

Starting revision `b67914021c42920e72279a44a61f9e8c7d871fa6`. PR #72's refinement was posted before implementation. Inspection of `ActiveReadViews::acquire` showed that its aggregate admission used the incoming caller's budget, allowing a later caller to raise the aggregate limit. Native `DatabaseOptions` now owns positive `active_read_views` and `pinned_read_bytes` ceilings, retained identically on create/open; each request uses the stricter caller/owner ceiling. Sealed installed `StorageLimits`, bootstrap TOML, engine lowering and generated configuration projections bind these restart-only policies. No new HTTP operation or external dependency. Existing GC roots, stream release and independent output-buffer reservation remain the authority.

The registry exposes a single-lock observation of active streams and charged pinned bytes. `PhysicalStoreEvidence` and the existing authenticated storage diagnostic expose these measurements; absent backends report absence. Campaign samples reject counts beyond installed ceilings, and every settled drain sample must return at or below the pre-load active-view/pin baseline. Read views and copied Arrow output allocations have different lifetimes: stream drop releases its physical lease, while returned buffers keep their own output reservation until their last owner drops. Regression definitions enforce this distinction, caller-budget escalation denial, configured reopening, lower request limits, pinned-byte denial without admission leakage, positive/bounded/digest-sealed policy and lowering. No timeout-driven lifetime release was added.

Read/traceability: native DatabaseOptions/create/open/projected-read path, full ActiveReadViews acquisition/release and ProjectedReadBatch/output ownership; existing projected-lifetime tests; store physical evidence and logical-only defaults; installed StorageLimits validation/defaults/lowering; diagnostic admission projection; campaign sample and drain reduction; owning C-07 failure/retention requirements. Changes stay at those owners. The native retained-reader/installed maintenance lane and fleet orchestration still require separate completion; these ceilings do not claim those workloads ran.

Added the declared first-party `rrd-engine` example `refresh_sdk_installation_identity`, using the real read-only planner and checked binary anchor. It changes only the corpus instance, estate and matching four scopes, validates the typed corpus and requires that the project remain empty. `--check` compares the derived typed corpus without writing. This replaces manual identity transcription after default-policy changes; the unchanged existing engine and SDK candidate oracles verify it. Initial compilation found sha2 0.11's array does not implement LowerHex; fixed explicit byte encoding. The example then completed without installing or starting an engine. An initial batch-lifetime assertion was corrected during source review: physical streams and copied output buffers own separate charges, as the existing native output module documents.

Validation: strict `cargo clippy -p rrd-lsm -p rrd-store -p rrd-engine -p rrd-contract -p rrflow-eval -p rrflow-cli --all-targets --locked -- -D warnings` passed (20.75s initially, 8.30s after final definitions). Installation/storage fixtures and all five language projections were regenerated; a subsequent description correction exposed stale TypeScript parity, so endpoint/model generators were rerun against the final owner. Combined OpenAPI digest is `12334d8bad21d6326f49262111dcbbae9c6ec89f41f5a1615cc0049a7e50a1dc`, still 43 operations. No local runtime tests, resource-control effects, disk filling or soak were executed. Native runtime acceptance remains required in candidate CI. C-07/OBJ-02/POAM-035 remain open.

## Aggregate native transaction ownership and copy removal

Starting revision `e65608ff56ba1f2f2792a79d1b9df0b18e0265dc`. PR #72 recorded the brief and duplicate-owner traceability before implementation. Native transaction snapshots previously bounded neither simultaneous owners nor the sum of their pending buffers. Added `TransactionLimits` and sealed restart-only `native_transactions`/`native_transaction_bytes`, lowered through the installed options. The registry atomically admits count and aggregate owned key/value capacity plus map-entry/handoff representations before accepting a replacement. Rejection preserves the prior write set. A shrinking replacement releases charge only after its old buffers drop. Rollback/drop clear buffers before releasing; handoff to the WAL batch keeps its charge until commit returns. Conflict, wrong-owner and early-error drops use the same owner; poisoned-lock unwind cleanup retains exactly-once release. These are logical reservations, not allocator-exact RSS: caller inputs, allocator/tree metadata and transient decoded checkpoint handles remain separate.

Removed `RrflowKvTransaction.runtime_snapshot_writes` and unconditional key/value clones. `Transaction::pending_prefix` exposes borrowed final pending entries; the store validates snapshot handles from that canonical owner before checkpoint effects and retains the existing create/rollback/reconcile order. Current behavior/source: rrflow_kv.rs put/delete/commit and native transaction write map. Destination: borrowed native pending-prefix iterator; owning C-07 hard-buffer/retention boundary. Existing transaction ordering, checkpoint/snapshot and reopen characterization remain required, with a new ordered replacement/tombstone/prefix oracle. No parallel lifecycle or format.

Actual owner count/bytes flow through PhysicalStoreEvidence and the existing DiagnosticStorageAdmission. The sustained sampler includes native buffers in retained logical bytes; every sample checks installed ceilings and drain requires both counts to return to baseline. Typed `TransactionLimit` is preserved through the store. Generic engine conversion remains an uncertain storage error because exhaustion can happen while resolving an already-published commit; only an independently observed absent domain outcome plus cleared intent converts it to known admission rejection. This refinement avoids labeling a committed operation rejected merely because its receipt lookup ran out of slots.

Regression definitions cover aggregate count, aggregate buffer denial with unchanged prior reads, spare Vec capacity, smaller replacement, rollback/commit/conflict/drop release, reopen policy, poisoned registry cleanup, and borrowed pending-prefix order. Engine flow definitions preserve a prior ACK, reject a large native-buffer mutation with absent outcome/cleared intent, reopen and admit valid work. A second flow commits first, fills native slots, requires uncertain replay, releases owners and recovers the identical receipt without duplicate application. Read native transaction/database/error paths, store physical transaction/checkpoint publication and error translation, engine commit-intent/outcome handling, installed policy/default/lowering, diagnostics, sustained sampling/drain and existing write-admission history tests.

Surfaced errors: removing the duplicate map left an unused BTreeMap import, fixed after strict Clippy reported it. The first engine replay assertion used the workload wrapper's receipt_sha256 field on the public CommitReceipt; compiler rejected it, corrected to full receipt equality after normalizing only the replay flag, following the existing oracle. Candidate `39904619510` job `120253991453` failed schema goldens because a diagnostic field description had been refreshed in SDKs but not evaluation schemas; parsed both assertion documents and confirmed the sole difference was that description. Refreshed evaluation schemas from final typed owners. The previous `289b1152` push omitted the already-regenerated corpus from staging; follow-up `e65608ff` published it without rewriting history.

Validation: full six-package all-target strict Clippy passed (12.21s, then 4.63s after copy removal, final 2.39s after receipt-flow definitions); five SDKs, installation/storage fixtures and planner-derived SDK identity regenerated. Generated parity passes all 43 operations/seven signal projections at OpenAPI `e30eb4d3002df2664069336a7bd2933ecd058698e0f58a779fdbae7a7f464175`; formatting and whitespace checks pass. No local runtime suite, installed workload, native control, disk fill or soak was executed. Candidate CI must run every regression and installed lane. C-07, OBJ-02 and POAM-035 remain open. Full fleet, installed reader-pin interference and 24-hour qualification remain outstanding.

## Candidate strict uniform-distribution decoding correction

Candidate `69e415526fa9d5efc2a17e3d4111c1bc8906d435`, run `39905826617`, product-operations job `120257998649`, failed the existing negative decode oracle: Serde accepted `{"kind":"uniform","keys":1}` for the internally tagged unit variant despite enum-level deny_unknown_fields. The pre-edit PR brief records the correction. Changed only the Rust uniform representation to an empty struct variant, preserving `{"kind":"uniform"}` and its distribution algorithm, then updated all five construction/match sites and affected projections. The unchanged negative oracle must pass in the next candidate; no unknown field is reclassified as accepted. Read the full distribution enum/validation/selection/regression function and every call site. No local runtime suite executed; compile/projection checks and native candidate CI remain the acceptance path. No gate/status change.


Dependency correction: incorporated native ext4 inode admission commit 5627c07e.
The inventory generator refused the unmerged index; the shell then incorrectly
continued and published merge 03e83faf with inventory conflict markers. No CI
acceptance is claimed for that revision. Regenerated the mechanical inventory
after resolving the index, checked documentation and whitespace, and published
a normal corrective commit without rewriting history.

### Candidate inventory ordering correction

Candidate f8d204d8 verify job 120264174581 rejected inventory drift: the prior
journal amendment followed inventory generation. Regenerated after the final
journal content, and checked again before commit. This is a publication-order
error; no engine or acceptance threshold changed. New exact-candidate CI is
required.

## Candidate corpus schema correction

Candidate run 39910913330, Linux installed reopen job 120274396699, failed
its capacity workload setup with the explicit contract denial `schemaless
table capacity_document cannot declare a strict property contract` (2.79s).
Read Workload::initialize and rrd-contract::validate_data_schema. The existing
schemaless fixture incorrectly set the strict additional-properties flag.
Corrected that flag to false with empty properties for all three corpus tables;
engine validation remains unchanged. The existing installed capacity smoke is
the failure oracle. Pre-edit scope was recorded on PR 72 comment 6359454924.
No local installed workload ran; corrected native acceptance remains pending.
Navigation, inventory, documentation and workflow policy checks passed before
publication. C-07 and POAM-035 remain open.


### Native NTFS exhaustion correction

PR comment 6359986493 records the pre-edit refinement. Candidate
39915579945, Windows job 120288970973, passed fresh WAL/segment/manifest
native exhaustion but timed out awaiting CURRENT's native error. A filler
create failure in one directory was insufficient evidence of global NTFS
metadata exhaustion. Microsoft documents reusable MFT records and distinct
MFT/data allocation: https://learn.microsoft.com/en-us/windows/win32/fileio/master-file-table.
No upstream implementation was copied. The precise absence of a CURRENT
error is observed; residual MFT/index capacity is the working diagnosis.

Read capacity.rs, installed_crash.rs, filesystem.rs persist_rename, manifest.rs
CURRENT publication, and fault_io/crash.rs plus crash/installed.rs control.
Changed only the owned-volume filler: descend 64KiB/4KiB/256/16/1-byte
writes after capacity errors; on Windows, precreate 16 private directories
and retain entries in each to consume reusable MFT records independently of
one full directory index. Aggregate metadata stays at 8192 entries and the
existing 25-second/volume-byte bounds remain. Cleanup removes exact owned
files and then empty directories, never recursively or from an unverified host
path. The engine must still emit actual native capacity failure and the full
reopen/retry/readback oracle remains unchanged. No timeout increase.

`cargo clippy -p rrflow-cli --test installed_crash --features crash-test
--locked -- -D warnings` passed (18.67s); `cargo fmt -p rrflow-cli` passed.
No local VHD, disk fill or installed workload was run. Native candidate
rerun is required; this correction is not itself proof of real exhaustion.
All earlier failed evidence remains retained. C-07/POAM-035 remain open.
