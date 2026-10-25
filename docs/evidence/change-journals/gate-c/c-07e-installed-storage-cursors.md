# C-07e installed physical reader and output ownership

**Status:** active implemented candidate; runtime acceptance pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-c/c-07e-installed-storage-cursors`
**Owner:** OBJ-02, [C-07e/g](../../../roadmap/rrflow-1.0/gate-c-07.md), POAM-035

[PR 74](https://github.com/rrflow/rrflow-development/pull/74) contains the
pre-implementation brief. Starting revision: `5c4ed21d245c2f725b342688c7462cf8c5fe2164`.
PRs 71?73 remain dependency candidates. Version stays 1.0.0; no gate is closed.

## Brief, scope and traceability

Installed clients previously could observe counters but could not retain an
actual projected reader across requests. A logical transaction is not that
physical owner. The canonical replacement adds one instance-scoped,
session-authenticated storage inspection capability through the executable
catalogue, HTTP and client bindings. RrdEngine owns bounded ephemeral cursor
state; rrflowKV owns the actual ProjectedReadStream and ProjectedReadBatch.
It exposes page coordinates, length-framed SHA-256 and allocation counters;
physical keys/values never leave the store wrapper. Observe recomputes the
hash from retained buffers. Native storage still owns snapshots, admission,
retention and reclamation; no parallel durable authority or fallback exists.

Open declares rows, bytes and lifetime. Native installed ceilings remain
binding, with at most 64 cursor metadata entries, 4 MiB per batch, 4,096 rows
and 60 seconds per cursor. Advance includes expected ordinal; retrying the
last advance returns the current page and cannot skip. Open retries compare
parameters. CancelSource drops actual source ownership while preserving the
copied batch. Release, session close, expiry and engine disposal drop the
owned batch. Terminal reads release source owners without invalidating the
last copied batch. State is deliberately process-local, lost on restart;
no replayable durable cursor claim is made.

The server polls one owned one-second sweep future with both plain and mTLS
serve futures. No per-cursor thread or detached worker is created. Embedded
hosts can call the same reap capability. Every cursor call also sweeps;
monotonic deadlines are capped by session lifetime and checked after capture.
Close rechecks authorization under registry ownership and releases after the
canonical session closure. Foreign sessions cannot address another cursor.

The installed sustained driver holds a real reader while measured writes
run, recomputes the unchanged page hash, cancels the source and checks that
copied output remains, then releases and checks owner drain. Ten-second
segments and a maximum 30-second operation deadline fit the 60-second lease.
Raw evidence includes all four cursor snapshots, owner counters, maintenance
counters and times. A lane only qualifies with an overlapping acknowledged
mixed write and observed automatic flush/compaction. Receipt hashes are bound
to bounded raw-progress hash chains; declarations without these observations
cannot qualify the lane. This does not manufacture GC or process RSS proof.
Single-estate qualification execution now requires a validated native allocation
receipt and verified child/harness membership before corpus seeding. Unsupported
platform cold control and fleet execution remain refused. The strict 24-hour,
profile, seed, reopen, lane, metrics and drain acceptance reducers remain binding.
Execution admission is not qualification; fleet and C-07h acceptance remain open.

## Grounded read and change map

Read complete native inspection wrapper and cursor contract; projected range,
request, budget, batch access/allocation and stream next/cancel/terminal code;
RrflowKvStore lock/snapshot/maintenance/evidence; engine composition and
session authorize/close/expiry; HTTP dispatch/authenticated envelope/server
plain and mTLS lifecycle; Rust client session call pattern; sustained segment
scheduling, load joins, baseline drain, lifecycle smoke and progress binding;
existing store admission, engine security and real-server test patterns.
Read C-07 owner and package authoring requirements and CI native partitions.

Changed boundaries: store rrflow_kv/inspection_cursor.rs and reexports;
contract storage_cursor.rs and executable endpoint/schema catalogue; engine
storage_cursor.rs, core/session composition; existing server handlers/router
and serve lifetime; client operation; evaluation reader_interference.rs,
segment/progress reducer; sustained/reader.rs and existing installed driver.
Generators own SDK/OpenAPI/schema projections. Focused regression definitions
live with native store, engine, contract, public client and evaluation owners.
Native CI runs storage_inspection on supported OSes; existing installed smoke
now includes actual retained ownership. No first-party external dependency;
sha2 0.11 is already locked and now an explicit store build dependency.

Research decision: reuse accepted C-06 projected read and C-07 owner-lifetime
contracts. No upstream source or algorithm imported. Debug evidence is bounded
coordinates, hashes and native accounting; no raw payload logging. Stop on
wrong snapshot, retry advancement, unauthorized access, unbounded ownership,
premature release or required CI failure. RSS remains independently measured.

## Authoring evidence and surfaced failures

Initial manifest generation failed because sha2 was mistakenly declared as a
workspace dependency where none exists; corrected to the existing locked
0.11 dependency. First store/engine/server/client all-target Clippy passed
(19.92s). Native store and engine regression definitions then passed Clippy
(16.86s). Between worktrees the shared target reused stale contract artifacts;
missing already-defined fields/types exposed that mismatch. Cleaned only
rrd-contract/rrd-store package artifacts and rebuilt successfully. This was
build-cache contamination, not removed source requirements.

The installed reader helper initially named nonexistent rrd_contract::Error;
changed to explicit CorrelationId construction with inferred concrete error.
All affected CLI/eval/contract/client/engine/store targets passed strict
Clippy (13.70s) before the final evidence-negative definitions. `cargo fmt`
completed. Generator/publication results are appended below. No local runtime
suite, installed workload, resource control, filesystem fill or soak ran.
Candidate CI must execute the new oracles and all owning suites.

Native oracle compares an independently length-framed expected historical
page digest through update/delete, flush/compaction/GC, advance/retry, source
cancellation, store disposal and exact reopen readback. Admission denial must
leave one live owner unchanged; final drop returns output/views to zero.
Engine definitions enforce session isolation, close and deterministic expiry.
Public client definitions exercise permission denial and HTTP ownership.
Evaluation negative definitions reject changed page hashes, premature output
release, undrained pins and impossible lifetime. These are not yet pass receipts.

## Auditable checklist and remaining work

- [x] One owning outcome/package, pre-implementation PR brief and scoped map.
- [x] Reuse canonical physical readers and engine authorization/composition.
- [x] Bound source/output/metadata owners and preserve separate lifetimes.
- [x] Include retry, denial, expiry, session and process failure semantics.
- [x] Record research, telemetry, dependency and surfaced-error decisions.
- [x] Add independent regression definitions and compile changed boundaries.
- [ ] Complete projection/CI checks and exact-candidate runtime acceptance.
- [ ] Complete C-07 remaining profile/fleet/soak/reconciliation acceptance.

This record does not establish full C-07, alpha, PostgreSQL compatibility,
model execution, cloud sync or UI readiness. Full implementation reread,
diff review, projections and candidate publication remain to be recorded.


### Projection and review results

Public catalogue now projects 44 HTTP operations, OpenAPI
`6a8d3e490eeb0e13a10465b752dee165223c7097e18ccd08acb79b38934e4650`.
Regenerated Python (ruff 0.17.1), Java, .NET, Go and TypeScript (pnpm 12.14.0)
models/operations, storage and installation schemas, seven signal projections,
and current authored digest citations. The planner identity generator first
rejected the stale 43-endpoint SDK corpus expectation. Derived 44 from the
exported catalogue, then reran the real read-only planner successfully; no
installation or native workload executed. Both generated-surface render and
verification passed. A new negative fixture used unsupported CorrelationId
TryFrom<&str>; fixed to its canonical constructor. Final affected seven-package
all-target strict Clippy passed (1.94s). Navigation and inventory regenerated.

Full source and diff review covered inspection capture/advance/drop/hash,
engine registry/authorization/retry/expiry, server-owned sweep, copied-buffer
release, new regression definitions, segment overlap/maintenance qualification
and constant-space progress hash binding. The public contract documents
process-local identity and expiry semantics. No local runtime acceptance is
claimed; native candidate CI remains required. Full C-07 is still open.

Final publication checks: generated-surface parity (44 operations), navigation,
inventory, documentation policy, workflow policy and diff whitespace passed.
Qualification admission refinement passed installed_reopen Clippy (2.56s).
No runtime acceptance or merge is claimed.


A follow-up authoring review found that Completed-but-unqualified campaigns
could return command success. The pre-edit PR refinement now requires a true
qualification verdict for Qualification-purpose installed and offline CLI
success; completed insufficient evidence stays on disk. The CLI binds its
purpose check to the verified campaign digest. Characterization keeps its
explicit unqualified success semantics. No acceptance threshold changed.


### Candidate SDK count correction

Candidate run 39915649527, SDK job 120289418658, exposed the authored .NET
catalogue assertion still expecting 43 operations. Pre-edit PR comment
6359934150 binds this refinement. Updated RrdClientTests to expect 44 and
explicitly retain session authentication and read-only mutation semantics
for DiagnosticsStorageCursor. Generated catalogue remains the sole owner.
The focused local dotnet run could not execute: pinned SDK 10.0.402 is absent
on this workstation (the attempted --filter-method argument was also rejected).
This is recorded as not run, not a passing SDK test. Candidate CI installs
the pinned SDK and must rerun the full SDK conformance job. No gate closes.
