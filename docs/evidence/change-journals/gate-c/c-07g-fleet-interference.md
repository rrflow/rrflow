# C-07g independent installed estate interference

**Status:** active implementation; native acceptance pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-c/c-07g-fleet-interference`
**Owner:** OBJ-02 / [C-07g](../../../roadmap/rrflow-1.0/gate-c-07.md) / POAM-035

[PR 75](https://github.com/rrflow/rrflow-development/pull/75) contains the
pre-implementation change brief. Starting revision:
`893c8e764719e9202b39d8e8e55c71bf1507578f`. Dependencies 71?74 remain
candidates. Product version stays 1.0.0; C-07 remains open.

## Change brief and implementation trace

Current single-estate sustained execution cannot qualify independent estate
interference. The target extends the existing installed Workload, PointRun,
segmented history and resource verification to quiet, equal and abusive
neighbour trials under the one J-04 fleet envelope. No engine scheduler,
context-sharing policy or distributed database is introduced.

The first implemented boundary is a two-estate ordinary-binary correctness
case. Each installed estate has independent process, session and differently
seeded identical record/relation/vector identities. Four concurrent requests
per estate exercise mixed writes, point reads, range reads and cancellation.
Four write generations insert, update, retire and recreate the working record.
Canonical Workload verification runs independently after acknowledgement.
A constant-size hash chain binds original seed receipts; kill/reopen performs
readback before retries and requires exact original receipts. Foreign sessions
must receive typed authorization denial before load, after load and after
restart; transport errors cannot count as isolation. Retained physical readers
cross the concurrent writes and are released using the canonical ownership
oracle. This is characterization, explicitly not fleet capacity qualification.

Read README and relevant C-07/J-04 owners, current sustained source, Workload,
PointRun, corpus/lifecycle helpers, native allocation verification/launcher,
capacity validation, progress and campaign schema, existing cursor journal,
CI installed native steps and workflow-policy command list. Reused their
boundaries directly. Changed sustained/fleet.rs, its parent module, required
native CI command and its mechanical policy check. New work does not replace
an engine implementation; current single-estate paths remain canonical.

Research: reuse established independent-history and native owned-allocation
contracts; no upstream code imported or new performance algorithm claimed.
Trace output contains bounded coordinates, receipts and ownership counters,
not secrets or raw content. Stop on cross-estate acceptance, receipt drift,
partial semantic effect, lost acknowledgement, failed native membership,
exceeded bounds or candidate CI failure.

## Authoring evidence

`cargo clippy -p rrflow-cli --test installed_reopen --locked -- -D warnings`
passed (19.30s) after adding the two-estate implementation and regression
entry. No runtime suite, installed workload, resource controller, disk fill or
soak ran locally. Candidate native CI must execute this new ignored entry on
Linux, macOS and Windows. Full ten-estate trial coordinator, aggregate
reducer and native qualification remain in progress.

## Auditable change checklist

- [x] Bind one objective, package, pre-implementation PR brief and starting SHA.
- [x] Inspect existing implementation, tests, worktree and dependency diff.
- [x] Preserve canonical workload, transaction, receipt and authorization owners.
- [x] Record scope, source traceability, research, trace and stop decisions.
- [x] Implement independent installed-estate correctness and compile its oracle.
- [x] Complete fleet coordinator and bounded evidence projection/reduction.
- [ ] Record final static/generated checks and exact candidate runtime results.
- [ ] Qualify full native fleet profile and 24-hour soak; reconcile C-07h.

No gate, objective or POA&M item closes on compilation or this case alone.

## Fleet coordinator and aggregate verifier implementation

Implemented `storage/fleet.rs` at the existing evaluation boundary and
`sustained/fleet_campaign.rs` as orchestration of the existing `run_estate`,
PointRun, Workload, corpus, lifecycle and CampaignWriter. There is no second
load generator or database executor. StorageCampaign remains the frozen owner;
derived child campaigns split exact corpus/rate/client coordinates, preserve
the common aggregate allocation, and reserve all evidence before installation.
Quiet, equal and abusive trials run their complete phase durations. Quiet
neighbours warm up first and then retain live engines and sampled drain without
offered load. Estate zero is the explicitly named active/abusive member.

Phase barriers share a monotonic clock, preserve live leases through slow
neighbours, and capture distinct engine/PID and foreign-session denial evidence.
The coordinator uses bounded 2..10 member state, at most 70 boundary snapshots,
and at most twelve worker threads. Separate worker capacity avoids synchronous
installed startup blocking every peer's async requests. Child failures abort
remaining owned tasks and leave raw progress; no absent child can qualify.
The existing final reopen updates the sampled PID and rechecks native membership.

Windows Job aggregate memory now equals the same declared engine allocation
as Linux cgroup memory.max, including the harness and all children; it no longer
uses the larger host observation envelope. Per-process and aggregate limits,
CPU allocation and membership are checked against the retained native receipt.
No local native controller was activated. Full profile validation now admits
fleet execution only through this coordinator; a single-estate CampaignResult
cannot qualify multiple estates. The root reducer requires every exact child,
all seven boundary stages, distinct live processes, a changed final PID,
unchanged estate identity, explicit foreign-session denial, common native
allocation, bound child resource PIDs, overlapping active phases, canonical
per-estate acceptance and quiet-neighbour drain. Summed member RSS peaks are
labelled conservative, never simultaneous. Frozen root/child byte reservations
bound evidence for all trials and seeds, with at most 32 point/seed coordinates.

Changed the existing public offline CLI to verify the appropriate evidence
layout and added generated fleet trial/result schema projections and goldens.
Updated eval/README instructions that still claimed these controls were absent.
This is harness/evidence implementation, not proof of the complete profile.

## Verification and surfaced errors

The initial compile exposed AllocationEnforcement lacking value equality;
added its mechanical PartialEq/Eq derives. Strict Clippy then rejected manual
modulo divisibility checks; changed to is_multiple_of without changing admission.
Strict all-target eval and installed_reopen Clippy passed (3.18s, then 1.90s
after barrier lease maintenance). Source review caught the need for bounded
multi-thread scheduling and live lease renewal during neighbour barriers; both
were implemented before publication. No test failure was suppressed.

Following the replacement repository instructions to verify the smallest oracle
and then its owner suite, `cargo test -p rrflow-eval --lib storage::fleet::tests
--locked -- --nocapture` passed all five focused checks (0.02s runtime).
`cargo test -p rrflow-eval --locked` then passed 22 library, 3 provider-compatibility
and 11 storage-contract tests, including generated schema goldens (36 total).
`cargo run -p rrflow-eval --example refresh_storage_contract --locked` regenerated
synthetic contract/schema projections only. No provider invocation, native
installed workload, resource control, filesystem fill or long soak ran locally.
The native two-estate case is required in candidate CI; the full coordinator
and ten-estate qualification still need actual native execution receipts.

The initial stage command named a nonexistent generated directory; it failed
without staging that command's paths. Replaced it with the observed exact file
list. Dependency merge f866ab98 conflicted only in the generated inventory;
resolved the index first, regenerated from the merged tree, and committed.
Candidate 71's Linux direct-dependency and candidate 72's journal/inventory-order
corrections are being carried through the dependency chain, without replacing
required acceptance checks or rewriting history.

Publication checks: final eval/CLI all-target strict Clippy passed (15.00s);
navigation, documentation, workflow policy, generated public-surface parity
(44 operations), and diff whitespace passed. Python launcher syntax compiled.
Generated inventory is rerendered after this final journal amendment and must
pass --check before commit. Full native capacity/soak remains unexecuted.

### Per-estate isolated p99 acceptance refinement

Review of J-04's owning fleet threshold exposed a missing relative-latency
oracle. The pre-implementation refinement is recorded in PR 75 comment
6358992786. Quiet measurement now rotates through every estate rather than
using estate zero as a proxy for its neighbours. Waiting estates retain live
engines and emit canonical drain/resource segments; a monotonic atomic turn
admits exactly one measured baseline at a time. Phase/lifetime and launcher
bounds include the additional isolated intervals. Full equal/abusive phases
and soak requirements remain unchanged.

The evaluation boundary derives p99 lower/upper bands from retained raw
scheduled-to-finished attempt latency, using fixed 4,096 integer bins and
no interpolation. Below 64 ns bins are exact; higher bins have at most 1/64
relative width. Each compliant estate's abusive upper bound must fit twice
its own isolated lower bound, so ambiguous bins fail conservatively. Root
verification recomputes bands and the separate isolated_latency_passed field;
changed claims, missing members or overlapping isolated baselines fail.
This is mechanical bounded measurement, not an imported upstream algorithm
or superiority claim. Added fleet_latency.rs under the same evaluation owner.

All seven focused fleet checks passed (0.02s), including an independently
sorted percentile oracle and conservative boundary rejection. After regenerating
schemas, the owning eval suite passed all 38 tests: 24 library, 3 provider
compatibility and 11 storage contract. Strict eval/installed_reopen Clippy
passed (3.22s) before final projections. No installed workload or native
resource effect ran locally; full profile acceptance remains pending.
