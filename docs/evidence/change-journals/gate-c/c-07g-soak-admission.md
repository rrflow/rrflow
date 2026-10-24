# C-07g: admit soak only after valid preflight and measure readback

**Status:** active implementation evidence; full capacity and soak qualification remain open
**Coordinate:** `rrflow://rrflow-instance/data/evidence/c-07g-soak-admission`
**Owner:** OBJ-02 / C-07g / POAM-035 / PR116

## Baseline, brief and traceability

Starting public main `0273f11bc9784681f9532ca06d912b7f0e11ddb7` is the merge of
[PR115](https://github.com/rrflow/rrflow-development/pull/115). Its exact head
`3c13f4558e93315b12206f658eaf08b8a4995aea` passed candidate pipeline
[40107347790](https://github.com/rrflow/rrflow-development/actions/runs/40107347790)
before merge. All 24 selected capacity cases passed across Linux, macOS and
Windows, with zero unresolved ledger entries or violations; native 100-cycle
reopen jobs also passed. These results apply to that candidate, not an arbitrary
later artifact. No full capacity profile or 24-hour soak was accepted.

[PR116](https://github.com/rrflow/rrflow-development/pull/116) carries the scoped
brief before implementation. Empty opening commit `095d2297250c18bd726083359e7ef2454d69cc36`
allowed the brief-first GitHub PR; it changes no planning file or authority.
The target is bounded qualification work: reject already-invalid operating
points before spending a soak, and expose independent readback cost without
weakening any database or verification oracle. Stop if this requires changing
transaction authority, profile/corpus/latency/error thresholds, historical
receipts, storage format, or removing an independent readback.

Read owners: README, AGENTS, alpha objective, Gate C/C-07, execution map and
change-authoring, CI operations, POAM-035, engine data flow and existing signal
contracts; for the requested demonstration, Gates G/H/J and POAM-026. Read
implementation and complete affected definitions in rrflow-eval's
`src/storage/sustained.rs`, `drain.rs`, `capacity.rs`, storage contract tests and
fixtures; rrflow-cli's `tests/support/sustained.rs`, sustained workload and
corpus/readback helpers; the bounded capacity launcher and operator-local
characterization inputs. Inspected legacy `rrflow-eval/src/main.rs::metrics`
for usage accounting. Reviewed retained campaign/result/segment/progress data
and the native CI receipts, not only compilation or schema existence.

The existing `satisfies_operating_requirements` predicate remains the final
acceptance owner. Its per-point checks move into a shared private helper used
by both final qualification and `CampaignWriter::admit_soak`. The installed
phase loop invokes admission before soak. Admission requires all four preceding
phases, their frozen duration/load/error/latency/metrics/corpus requirements,
lanes, transitions, forced reopens and the existing settled-ownership drain
oracle. Final acceptance still requires the actual soak and all other proofs.
Characterization intentionally retains unsuccessful points; it cannot qualify.
Fleet children remain characterization inputs to their existing fleet owner;
this change does not claim a new fleet-level admission gate.

`LoadAttempt` gains optional `verification_finished_ns`; workers capture it
while their client slot remains occupied. Response time retains its existing
meaning. Historical missing values remain unavailable, with no fabricated zero.
Validation rejects times preceding response completion or exceeding the segment.
The two storage JSON schemas are mechanically regenerated. No dispatch,
transaction, durability, maintenance schedule or independent oracle is removed.
The existing test suite characterizes the shared predicate before/after its
extraction, including missing/invalid prerequisite variants and incomplete soak.

## Retained operating failure and research decision

The ordinary optimized Linux characterization at `3c13f455` used 1,000 records,
5,000 edges, 1,000 384-dimensional vectors, four clients and 25 RPS with a 2-GiB
aggregate limit. Its WSL host is not the required isolated Desktop allocation.
Corpus creation/readback took 23.854 seconds; final restart/readback of 1,050
slots took 123.634 seconds. Of 3,001 attempts, 2,680 acknowledged and 321 rejected:
312 driver-concurrency-limit and nine storage conflicts. Fifty scheduled
latencies exceeded budget. Zero unresolved/mismatched readbacks and a completed
harness do not make the operating point pass. An initial operator description
mistook harness exit zero for success; it was corrected immediately and is not
accepted evidence. A subsequent 10,000-record attempt was interrupted at 25.78
seconds; its conservative cleanup receipt remains unchanged, while independent
inspection found its owned cgroup absent and systemd scope inactive.

Retained operator-local evidence:
`/home/devcontainers/rrflow-c07-native-79663cdd/integrated-growth-1000-3c13f455/evidence`.
Campaign digest `6507ffcc49a6e0d4a0d27af84ad9f0cf2253b2a6304d6158f00512cebe6670f5`;
final segment `c2f5d6611af433239f44d6cca580cd629ecd80ade680bfee512ca8b61779755e`;
progress `07caf2d5f4667b8e6d395b8a500648f8c32503205f74c814aead9c60b4537911`.
Engine executable `6207b0eecb222b418e8a27844678d3a1800e75f8bff0064dcae3672d7499f50c`.
Native platform receipts are retained in operator-local
`target/c07-pr115-native-evidence/validated-capacity-summary.json` and CI job logs.
The original failed 100,000-record Desktop campaign remains unchanged; inspection
confirmed it failed during setup, never starting the 24-hour soak.

The new timestamp is a measurement decision: response timings excluded the
additional independent readback occupying each driver slot. Existing scheduling,
resource, write-phase and maintenance counters remain intact. Correlation is
not causal attribution, and concurrent stage sums are not wall time. No new
runtime trace protocol, upstream storage code, provider dependency or competing
engine is introduced. FoundationDB's [engineering paper](https://www.foundationdb.org/files/fdb-paper.pdf)
and [snapshot-isolation semantics](https://jepsen.io/consistency/models/snapshot-isolation)
support retaining independent fault/atomicity oracles; they do not confer
serializability or distributed claims on RRFlow.

The user's local-evidence/frontier-synthesis demonstration is captured in the
existing [data-flow owner](../../../architecture/engine-data-flow.md#project-evidence-model-usage-and-reasoning-efficiency),
with H-05/G-06/J-04 links: distinguish deterministic local work, local-model
usage, remote usage, web/tool calls, source quality and amortized costs. Missing
usage is not zero; cache/reasoning subsets cannot be added again to totals.
Primary-source OpenTelemetry, peer-reviewed Adaptive-RAG and the explicitly
experimental VikingRAG preprint inform that evaluation contract. No token-saving
claim or completed UI is made. POAM-026 records the observed legacy missing-usage
and first-model-only parser limitations; their repair is outside this package.

## Acceptance and remaining work

Focused `cargo test --locked -p rrflow-eval --lib storage::sustained::tests:: --jobs 6 -- --nocapture`
passed 16 tests, zero failures. `RRFLOW_UPDATE_STORAGE_GOLDENS=1 cargo test --locked -p rrflow-eval --test storage_contract generated_schemas_and_synthetic_receipts_match_goldens --jobs 6`
passed and regenerated only the two expected schemas. `cargo test --locked -p rrflow-eval --jobs 6`
passed 48 tests (34 unit, three acceptance, 11 storage-contract), zero failures.
The initial format check exposed only formatting changes; `cargo fmt -p rrflow-eval -p rrflow-cli`
resolved them. Installed smoke, strict lint and documentation checks are recorded
below when complete. Operator-local log: `target/c07-soak-validation.log`.

Changed source is limited to the two sustained Rust files and their generated
schemas. Supporting changes update eval guidance, the existing C-07 and
POAM-035 result/next-step records, the data-flow/G/H/J/POAM-026 signal owners,
this journal and generated navigation/inventory. No implementation files are
moved/deleted. No version, roadmap checkbox, objective or POAM closure changes.

The package does not run or claim the 24-hour soak or full profile matrix.
Further bounded characterization must distinguish scheduling, readback and
maintenance costs before the unchanged full-profile preflight. Qualification
must bind candidate identities, actual native limits, complete corpus and
independent readbacks, lifecycle/load/drain and full-duration evidence.
PR116 candidate CI and its merge remain pending at this record's creation.


### Final local validation and checklist

The ordinary Windows installed workload command
`cargo test --locked -p rrflow-cli --test installed_reopen --jobs 6 -- sustained::installed_capacity_workload_smoke --exact --ignored --nocapture`
passed in 77.02 seconds: eight records, 24 edges, 16 vectors, all four write
transitions, 12 replayed transactions, backup under writes, retained reader
release and actual process termination/reopen. Projected ownership drained
from 225 bytes to zero. The receipt explicitly reports
`capacity_qualification=false`; executable SHA-256 is
`f92aca7c5362abbadbed916174379d0a69a925678de07d43e46bd4b9a2287354`.
`cargo clippy --locked -p rrflow-eval -p rrflow-cli --all-targets --jobs 6 -- -D warnings`
passed. `cargo fmt --all -- --check` and `git diff --check` passed.
`python scripts/ci/check_documentation.py` passed (501 statuses / 499 classified
coordinates), and `python scripts/knowledge/render_navigation.py --check`
passed (17 indexes, 500 nodes, 2,086 edges). Navigation was regenerated before
commit; the inventory projection is regenerated from the final staged source.

Checklist completed: owner/baseline and scope bound before edits; current and
target behavior plus failure oracle stated; implementation replacement remains
at its canonical owner; research/measurement decisions recorded; focused,
owning and installed integration checks executed; surfaced formatting and
historical operating failures retained; authored diff and affected functions
reread; linked evidence and generated navigation included. Full native/profile
CI and new Linux characterization are pending on this new source and are not
substituted by the previous candidate's results. No full workspace test or
24-hour run was performed locally for this bounded harness change.
