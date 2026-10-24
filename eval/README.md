# RRFlow evaluation

**Status:** active evaluation-tool instructions; no alpha or capacity claim
**Coordinate:** `rrflow://rrflow-instance/data/reference/evaluation/storage-measurement`
**Owner:** [C-07](../docs/roadmap/rrflow-1.0/gate-c-07.md) storage subset of [J-04](../docs/roadmap/rrflow-1.0/gate-j.md#workload-and-evidence-structure)

`rrflow-eval` retains its paired-provider `run`, `summarize` and `verify`
commands. The additional `storage` commands work offline. They validate
measurement evidence and emit a bounded report; they do not run a workload,
configure an engine, authenticate an observation, or call a provider.

## Storage contract

The Rust types in the evaluation crate own the version-1 storage manifest,
result and report. JSON schemas under [schemas](schemas/) are generated
projections. Rust validation also enforces cross-field sums, catalogue and
configuration identity, native profile consistency, required-case coverage,
and independent history invariants that JSON Schema cannot express alone.

The manifest freezes a clean candidate's source/executable identity, public
operation catalogue, installed configuration, physical diagnostics, profile,
corpus and independent expected-ledger digests, driver availability, cases and
seeds before execution. Operations resolve against `rrd-contract`'s catalogue;
availability is an explicit driver declaration, not an inference that the
catalogue entry has qualified runtime support. Validation is not activation.
Synthetic fixtures remain labeled synthetic even when every assertion passes.
Native crash/capacity histories require authoritative durability. A process
crash or I/O model cannot be submitted as an OS or physical power-loss class.

Each expected logical operation binds the request and exact effect digest and
requires presence, absence, or either result after an intentional uncertain
boundary. Every attempt retains client monotonic timing and one outcome:
acknowledged receipt, definite rejection, or uncertainty. Readback explicitly
records presence (including receipt and application count) or observed absence.
Missing readback remains unresolved. A required positive effect cannot pass by
rejecting every request; a lost ACK, wrong/duplicate effect, changed retry,
unknown identity or effect after only rejected attempts cannot pass. The
expected ledger is digested before execution and cannot be filled from the
engine's observed answer. This is a bounded history oracle; the external
collector must establish completeness of requests, recovery and readback.

Every required case must have one result, including failed, skipped and
unimplemented cases. Those statuses cannot become successful coverage. Fault
phase, occurrence, code and retained-artifact digest remain separate from the
case outcome: an injected fault followed by correct recovery may pass.
Malformed structure is rejected; well-formed failing histories produce a
failed report before the command returns a nonzero status. Unknown-field
errors report position/category without echoing rejected contents.

## Policy observations and bounds

The manifest embeds the existing public `EstateConfiguration` and
`DiagnosticStorageSnapshot` types. Its six effective settings are mechanical
projections to their `rrd-lsm` owners: WAL/version soft flush targets, cache
capacity, maximum I/O request bytes, L0 trigger and compaction output target.
No private storage dependency or duplicate operation model is introduced.
The installed engine has no storage reconfiguration capability yet; output
states that explicitly rather than promising live application or restart.

Laboratory budgets use explicit bytes/counts/milliseconds and are frozen per
run. Disjoint memory reservations must fit the process cap; process allocation
times estate count must fit the host cap. Atomic work fits mutable/queue
reservations; disk retains temporary and recovery reserves; wait and recovery
deadlines fit the run. Soft flush targets must be below proposed hard ceilings.
These checks do **not** enforce those ceilings inside the engine; C-07d owns
that implementation. Native RSS and disk measurements remain C-07g work.

Limits are positive JSON-exact integers. Inputs/outputs are limited to 8 MiB,
required cases/bindings to 256, and attempts to 10,000 across a run. This is
the per-segment fault/history contract. Sustained campaigns compose bounded
segments under an explicit total artifact quota; no large run is truncated
to fit a single document. J-04's full product workload remains a separate gate.

## Commands and fixture maintenance

Run from the repository root. The checked-in fixtures below are **synthetic
oracle tests**, not installed-engine or benchmark receipts.

```text
cargo run --locked -p rrflow-eval -- storage validate crates/evaluation/rrflow-eval/tests/fixtures/storage/manifest.json
cargo run --locked -p rrflow-eval -- storage verify crates/evaluation/rrflow-eval/tests/fixtures/storage/manifest.json crates/evaluation/rrflow-eval/tests/fixtures/storage/result.json target/storage-report.json
cargo run --locked -p rrflow-eval -- storage schema manifest
cargo run --locked -p rrflow-eval -- storage schema result
cargo run --locked -p rrflow-eval -- storage schema report
cargo test --locked -p rrflow-eval
```

`verify` requires a **new** report path and never overwrites a retained result.
Read the emitted `evidence_class`, `durability`, coverage and oracle counters;
`contract_passed` does not close a roadmap gate or authenticate external data.
Result and manifest digests use domain-separated typed JSON, so whitespace is
irrelevant and case/attempt ordering is preserved. Rejected raw input is not
copied into reports. Retained artifacts must be independently available and
authorized; a digest alone cannot prove an artifact exists or is correct.

After an intentional contract change, the documented test generator updates
schemas and synthetic fixtures from those Rust owners. In PowerShell:

```powershell
$env:RRFLOW_UPDATE_STORAGE_GOLDENS = '1'
cargo test -p rrflow-eval --test storage_contract generated_schemas_and_synthetic_receipts_match_goldens
Remove-Item Env:RRFLOW_UPDATE_STORAGE_GOLDENS
cargo test --locked -p rrflow-eval
```

Ordinary tests compare generated bytes and never update fixtures. Review
schema, fixture and protocol-version consequences before committing changes.

## Refresh synthetic projections

After an intentional installed-configuration or evaluation-contract change,
`cargo run --locked -p rrflow-eval --example refresh_storage_contract` regenerates
the storage and sustained campaign schemas and existing synthetic manifest/result/report fixtures
with canonical Rust serializers and digests. It preserves the independent
ledger inputs, refuses a non-synthetic manifest, and does not execute a native
workload. Review the diff; refreshed synthetic data is never acceptance evidence.


## Installed sustained campaigns

Each campaign also freezes `drain`: `maximum_mutable_growth_bytes`,
`maximum_projected_growth_bytes`, `maximum_debt_segments`, and `stable_samples`
(2 through 64). Growth is compared with the first pre-load warmup observation;
the final consecutive drain samples must meet those thresholds with zero
outstanding requests, queued requests and admitted bytes, and no active or
fenced writer. Missing ownership gauges fail this check. The driver checks
drain before soak; completion and offline verification repeat it. Current
maintenance is synchronous: a failed debt threshold is retained as a failed
operating point, without silently forcing extra maintenance or relaxing policy.

The `storage campaign` contract freezes candidate identity, sealed engine
configuration, allocation, corpus, load schedule, operating points, thresholds
and finite evidence limits. The installed driver seeds and independently reads
all requested records, relationships and vectors before offered load. Immutable
seed data serves point/range reads. Mixed writes reuse at most
`evidence.segment_attempts` mutable slots and cycle through insert, update,
atomic retirement and recreation. Transaction identities remain unique;
canonical version history can still grow even though live key count is bounded.
The required `read_distribution` is frozen in each campaign: `{"kind":"uniform"}`,
`{"kind":"hot_key","ordinal":0}`, or
`{"kind":"hot_set","keys":64,"traffic_basis_points":2500}`. The hot-set
fraction selects its first `keys` records; the remainder selects the disjoint
cold region. Keys and fractions must fit the declared corpus. Seeded selection
is reproducible, and changing it changes campaign/request identity. This
configures point-read keys and bounded-range starting keys; write-slot
generation and finite live key count keep their separate contract.
After load the driver kills/reopens the engine and independently reads seed
and final slot state before any retry.

```text
cargo run --locked -p rrflow-eval -- storage campaign profiles
cargo run --locked -p rrflow-eval -- storage schema campaign
cargo run --locked -p rrflow-eval -- storage schema progress-event
cargo run --locked -p rrflow-eval -- storage campaign validate path/to/campaign.json
python scripts/ci/run_sustained_capacity.py --campaign path/to/campaign.json --configuration path/to/configuration.toml --evidence path/to/new-evidence
cargo run --locked -p rrflow-eval -- storage campaign verify path/to/new-evidence
```

Launching is an explicit resource-consuming operation on a clean frozen
candidate using the ordinary release binary and harness. Linux additionally requires `--cgroup-parent` naming an already
delegated cgroup; Windows uses an owned Job Object. The driver verifies live
native limits and process membership rather than trusting environment labels.
No privileged host setup is implicit. Qualification requires verified native
allocation and every frozen acceptance condition. Actual retained physical
readers and multi-estate orchestration are implemented; their presence does
not qualify a profile. macOS constrained-capacity launch is unavailable.
Characterization results cannot close the profile or 24-hour requirement.

The Linux cold lane stops the owned engine, synchronizes and advises eviction
only for regular files inside its installed native root, and checks actual
page residency through bounded, untouched mappings. Nonzero residency fails
the lane. File inventory, size, memory and elapsed time have frozen bounds;
per-file observations and the restart/readback receipt remain in the evidence.
This measures cold startup plus the first independent read, since startup can
warm pages. It does not assert that later queries are cold. No host-wide cache
setting changes. The lane is unavailable on other operating systems.

`progress.jsonl` retains typed intents, outcomes, samples and readback with
bounded record encoding and periodic durable checkpoints. Final reports bind
its exact bytes, count and digest. A crash may leave observations after the last
checkpoint uncertain; absent final reports and partial records are incomplete
evidence. Offline verification streams artifacts and rejects altered, missing
or truncated files. Reserve enough evidence disk for both successful and
failed runs. An evidence quota failure is a failed operating point, not a
reason to omit samples or increase the frozen limit mid-run.

Capacity qualification binds the exact current J-04 projection digest, fixed
CPU/memory/estate allocation, minimum corpus, vector dimensions and declared
load coordinates. `campaign profiles` exposes the bundled projection without
running a workload. `scripts/knowledge/render_capacity_profiles.py` derives it
from the owning workload table; CI rejects drift. A qualifying operating point
is not proof of the entire load sweep. Reduced characterization remains
explicitly unqualified. Fleet manifests derive independent child campaigns
from the same frozen allocation and corpus; they require four clients per
estate and an exactly divisible aggregate request rate. Quiet rotates the measured interval through every estate, one at a time,
while neighbours retain their engines and sampled drain without offered load;
equal loads every estate; abusive offers estate zero twice its admitted rate.
Each trial retains the full warmup/measured/burst/drain/soak durations. All
members warm up before the quiet trial begins. Phase barriers share one clock
and wait for every member; joined sessions renew while waiting.

The aggregate cgroup/Windows Job bounds the harness and every child together
to `engine_bytes`; the host envelope remains independently observed. Native
membership is checked before ingestion and at resource sampling/reopen.
Pre-phase boundaries capture idle overhead, unique instance/PID identities
and explicit cross-estate session denial. Per-estate child histories retain
canonical record/relation/vector verification, lifecycle lanes, final restart
and readback. The aggregate reducer checks overlapping concurrent phases, non-overlapping
isolated measured intervals, quiet-neighbour drain, shared allocation, and a
conservative sum of member RSS peaks. It derives bounded p99 intervals from
raw measured attempt timings using 4,096 integer bins (at most 1/64 relative
bin width above 64 ns). Every compliant estate's abusive p99 upper bound must
be no greater than twice its own quiet p99 lower bound; ambiguous boundary
bins fail conservatively. `isolated_latency_passed` reports this separately
from complete qualification.
That sum is explicitly not a simultaneous RSS sample.

`storage-fleet-trial-v1.json` and `storage-fleet-result-v1.json` are generated
projections. `campaign verify` dispatches to the fleet reducer for multiple
estates and streams each bounded child campaign. Failed/aborted members retain
raw progress; missing members or trials cannot become completed evidence.
Root metadata and child evidence quotas are reserved before engine installation.
The coordinator uses at most twelve worker threads for ten estates. Fleet
execution is implemented but the complete native ten-estate matrix and 24-hour
trial receipts remain required; no capacity result is claimed here.

Installed read retention has per-open active-view and pinned-byte ceilings.
Requests can lower these limits but cannot raise them. Storage observations
report actual physical views separately from retained output-buffer bytes;
campaign drain checks require both view and pin counts to return to baseline.

Native transactions separately enforce an aggregate owner count and owned-buffer
reservation. These restart-only installed ceilings are visible in configuration;
actual counts/bytes are observed in storage diagnostics and must drain to
baseline. The reservation charges vector capacities and explicit record/handoff
representations; allocator metadata and caller input are not process-RSS proof.

## C-07 acceptance reconciliation

The offline acceptance boundary binds one clean tested source revision/tree,
the current public catalogue and J-04 profile/threshold projection, retained
candidate CI records, and explicitly enumerated capacity campaigns. It reuses
the campaign/fleet reducers, including their raw histories, native allocation,
readback, lifetime, failure and soak checks. It does not launch an engine or
change the roadmap. Generated contracts are `storage-acceptance-manifest-v1`,
`storage-acceptance-report-v1` and `storage-candidate-ci-v1` under `eval/schemas`.

```text
python scripts/ci/collect_storage_acceptance.py --run RUN_ID --output evidence/ci
cargo run --locked -p rrflow-eval -- storage schema acceptance-manifest
cargo run --locked -p rrflow-eval -- storage acceptance verify evidence acceptance.json new-report.json
```

The collector reads the development repository through authenticated GitHub
API calls, retains bounded run/job/commit records, and asks the Rust owner to
validate mandatory successful native jobs and steps. It preserves the PR head
separately from the actual tested reusable-workflow revision. Candidate checkout
policy prohibits alternate ref/repository/path inputs so that default checkout
continues to test that triggering source. The reusable workflow and PR merge
revision may differ from the branch head; campaigns must bind the tested
revision, never silently substitute the head. Reruns/attempt changes during
collection fail. Offline raw-record digests establish integrity and internal
provenance, not a cryptographic GitHub attestation; collect from the API again
when establishing current candidate status. No official promotion is implied.

The manifest names direct, non-symlink evidence directories and semantic hashes.
A missing result requires explicit `not_run` or `interrupted` status and a reason;
it cannot count as attempted coverage. Failed results remain visible alongside
missing coordinates and the largest actually qualified rate for each client
count. The report requires every declared point to qualify before indicating
readiness for owner review; a lower successful point cannot erase a failed
higher point. This conservative flag does not automatically accept a reduced
operating envelope or close any gate.

Nominal desktop/service observations are separately recomputed from raw measured
scheduled-to-finished timings for each seed/client count. The bounded integer
histogram is shared with fleet isolation; p95/p99 upper bounds must meet J-04's
owning limits and unexpected terminal errors must meet its exact basis-point
limit. Author-supplied campaign thresholds cannot substitute for those limits.
Three distinct seeds are required at every nominal client count. Physical range
timing is explicitly a C-07 proxy; graph/context/model quality remains under its
own later gates. Unknown owner grammar fails projection generation. Full
capacity and 24-hour native evidence remain required and are not supplied by
these commands or synthetic regression fixtures.


Outer launch cleanup is part of acceptance. The existing sustained launcher
now seals a bounded `launch.json` only after its owned cgroup/Job cleanup has
returned. An unavailable facility, interrupted launcher or failed cleanup
retains the frozen campaign and explicit outer outcome even if the harness
never created a result. Existing inner results are preserved. Each manifest
member binds this receipt with `launch_sha256` (semantic domain
`rrflow-storage-launch-v1`); absent launch evidence cannot qualify. Acceptance
compares its allocation/harness identity against every bound raw resource
sample across all segments and fleet children, then requires successful exit
and confirmed cleanup. A successful inner result cannot override an outer
failure. Campaign/fleet evidence admission reserves 8 KiB for this receipt.
The generated `storage-launch-v1.json` describes its bounded structure. No
native resource limit is changed by offline receipt verification.
