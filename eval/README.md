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

The `storage campaign` contract freezes candidate identity, sealed engine
configuration, allocation, corpus, load schedule, operating points, thresholds
and finite evidence limits. The installed driver seeds and independently reads
all requested records, relationships and vectors before offered load. Immutable
seed data serves point/range reads. Mixed writes reuse at most
`evidence.segment_attempts` mutable slots and cycle through insert, update,
atomic retirement and recreation. Transaction identities remain unique;
canonical version history can still grow even though live key count is bounded.
After load the driver kills/reopens the engine and independently reads seed
and final slot state before any retry.

```text
cargo run --locked -p rrflow-eval -- storage schema campaign
cargo run --locked -p rrflow-eval -- storage schema progress-event
cargo run --locked -p rrflow-eval -- storage campaign validate path/to/campaign.json
python scripts/ci/run_sustained_capacity.py --campaign path/to/campaign.json --configuration path/to/configuration.toml --evidence path/to/new-evidence
cargo run --locked -p rrflow-eval -- storage campaign verify path/to/new-evidence
```

Launching is an explicit resource-consuming operation on a clean frozen
candidate. Linux additionally requires `--cgroup-parent` naming an already
delegated cgroup; Windows uses an owned Job Object. The driver verifies live
native limits and process membership rather than trusting environment labels.
No privileged host setup is implicit. The current runner rejects full
qualification, cold-cache/reader-pin lanes and multi-estate allocation until
those facilities exist. macOS constrained-capacity launch is unavailable.
Characterization results cannot close the profile or 24-hour requirement.

`progress.jsonl` retains typed intents, outcomes, samples and readback with
bounded record encoding and periodic durable checkpoints. Final reports bind
its exact bytes, count and digest. A crash may leave observations after the last
checkpoint uncertain; absent final reports and partial records are incomplete
evidence. Offline verification streams artifacts and rejects altered, missing
or truncated files. Reserve enough evidence disk for both successful and
failed runs. An evidence quota failure is a failed operating point, not a
reason to omit samples or increase the frozen limit mid-run.
