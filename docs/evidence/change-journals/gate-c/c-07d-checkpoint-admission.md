# C-07d bounded checkpoint admission

**Status:** active implementation candidate; native candidate acceptance pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-c/c-07d-checkpoint-admission`
**Owner:** OBJ-02 / [C-07d/e](../../../roadmap/rrflow-1.0/gate-c-07.md) / POAM-035

[PR 77](https://github.com/rrflow/rrflow-development/pull/77) carries the
pre-implementation brief. Starting candidate was
`d4a98d2ae55c41c1ef8f064273fd12808f21a4d9`, built on development main
`6de0c5be56a6b7c888586186babb3b1648eb55ae`. Dependency corrections advance to
`a654344f0ce87c1a90b61bb4a1009ddf62db71af`, tree
`e3851fa3da4b38f9be9b2f894904711116d8132c`. Version remains 1.0.0.

## Change brief and traceability

ManifestStore previously admitted arbitrarily many named checkpoints and its
ordinary listing collected directory entries and metadata without a finite
budget. Runtime snapshot handles depend on those physical pins. The canonical
manifest owner now serializes checkpoint admission/list/release and enforces a
restart-only count ceiling before a new publication. Every directory entry,
including an interrupted temporary file, counts. Existing identical requests
remain idempotent at capacity. No retained pin is evicted. Release allows new
admission; excess or oversized inventory returns typed pressure without deleting
bytes. Existing reclamation entry/byte/time budgets bound checkpoint metadata.

StorageLimits seals the configurable default of 1024 entries (1..65536),
DatabaseOptions lowers it at native open/create, and typed CheckpointLimit
survives store/engine admission translation. Runtime reconciliation releases
only pins absent from the authoritative live-handle set before admitting
missing pins. Logical snapshot denial leaves state unchanged. Physical
storage diagnostics expose actual count and configured ceiling, using the
existing authenticated capability and all five generated clients.

PR refinement 6360041952 binds the same counts into sustained qualification:
observed limit must equal sealed configuration, count cannot exceed it, and
post-burst pins must return to the measured pre-load baseline. The negative
oracle rejects a leaked checkpoint even when the final sample looks settled.
No new persistence format, scheduler, operation catalogue, provider authority,
or runtime dependency is introduced. Unknown publication outcomes, premature
reclamation, over-admission, missing propagation or failed native evidence
remain stop conditions; no complete C-07 claim follows from this prerequisite.

## Reviewed boundaries and projections

Read the README/OBJ-02/C-07/POAM-035 owner chain, change-authoring and
traceability procedures; manifest.rs publication/checkpoint/list/release,
database.rs options/create/open/checkpoint wrappers, reclamation.rs budgets,
LSM/store error translation, rrflow_kv.rs runtime checkpoint preparation and
reconciliation, engine.rs PhysicalStoreEvidence, engine core native options,
transaction known-denial handling, diagnostic projection and write_admission
regressions; deployment.rs and diagnostic.rs public contracts, manifest and
snapshot integration targets, deployment_configuration_contract.rs, sustained
resource validation, drain.rs and reader_interference.rs. Reviewed the full
changed source diff and relevant publication/reclamation callers. Generated
inventory records exact edited paths; no implementation was moved or deleted.

Generated installation and public-contract fixtures, the read-only installed
SDK identity corpus, storage schemas/goldens, TypeScript/Python/Go/Java/.NET
clients and signal projections from their existing owners. Updated six authored
current OpenAPI citations to
`03639d8aff602c966b800becaa5b8442d0e912904cb50383f1ecee86ad32fb9a` (44 operations).
Generated model order changes are replaceable generator output, not a new
contract authority.

Research decision: no new algorithm or upstream adaptation; enforce the
already-owned C-07 checkpoint quota using native publication and reclamation
semantics. Resource/debug decision: count/limit and typed rejection at the
existing storage boundary; no checkpoint names or private data in telemetry.
Actual native capacity and long-duration RSS qualification remain CI/profile
work, not inferred from these counters.

## Commands and evidence

- First focused native checkpoint oracle: 2 passed, including eight concurrent
  contenders for one remaining slot, exact retry, release/reuse, reopen and
  bounded metadata without pin deletion.
- `cargo test -p rrd-store --test snapshot native_checkpoint_quota --locked`:
  1 passed; denial preserves read stamp/catalogue, release admits replacement,
  reopened handle reads the original durable cursor.
- Focused deployment quota test: 1 passed; rejects zero/65537/u64::MAX and binds
  valid configuration to its digest.
- `cargo test -p rrd-lsm --test manifest -p rrd-store --test snapshot
  -p rrd-contract --test deployment_configuration_contract --locked`:
  11 manifest, 9 snapshot and 8 configuration tests passed (28 total).
- `cargo test -p rrd-engine --lib
  installed_disk_reserve_lowers_to_native_policy_and_rejection_class --locked`:
  1 passed; sealed native lowering and typed service admission.
- `cargo test -p rrflow-eval --locked`: 45 passed (31 library, 3 compatibility,
  11 storage contract), including the leaked-checkpoint drain rejection.
- `cargo clippy -p rrd-lsm -p rrd-store -p rrd-contract -p rrd-engine
  -p rrflow-eval --all-targets --locked -- -D warnings`: passed (21.93s)
  before the final drain refinement; final parity/lint results follow below.
- Existing refresh_installation_contract, refresh_storage_contract and
  refresh_sdk_installation_identity examples passed; the last is a read-only
  planner, not a native install. Python generator with ruff 0.17.1, Java,
  .NET, Go and TypeScript with pnpm 12.14.0 passed. Signal renderer passed.

A draft engine assertion named `options` instead of `native`; corrected before
compiling that target. Exploratory reads of nonexistent resources.rs/lanes.rs
were corrected by locating the owning modules; neither was an implementation
failure. No failure was hidden by changing limits or removing assertions.

## Checklist and remaining evidence

- [x] Pre-edit PR brief, one owning outcome, exact native/configuration boundary.
- [x] Serialize count admission and preserve publication uncertainty/pin lifetime.
- [x] Independent concurrency, durable snapshot and negative drain oracles.
- [x] Public configuration, diagnostic and five-client/schema propagation.
- [x] Trace/resource/research decisions and surfaced errors recorded.
- [ ] Exact candidate CI/native acceptance and complete C-07 profile/24-hour matrix.

No local installed workload, hard allocation, disk fill or soak ran in this
package. Full workspace and native SDK suites are delegated to required
candidate CI, not implied by focused component results. C-07, OBJ-02 and
POAM-035 remain open. PostgreSQL breadth, cloud sync, model quality and UI
readiness are separate requirements. No merge or release acceptance is claimed.


Final strict all-target Clippy passed again after drain/contract validation
(7.25s). Generated parity passed for all 44 operations and seven signal
projections. Documentation policy initially rejected the journal status label
because it lacked the required active/historical classification; corrected
the label to active. This was an evidence metadata error, not a runtime pass.


Final documentation, workflow, navigation and execution-inventory policy passed.
`git diff --check` then caught CRLF output from the Python-driven Python, Java
and .NET generators on Windows. Pre-edit comment 6360090382 binds the remedy:
the existing writers now specify UTF-8/LF. Reran all three exact generators;
whitespace check passed. This removes platform-dependent projection churn.
Their complete generator source and output diff were reviewed; no catalogue
semantics changed. Final inventory is regenerated after this evidence entry.


Current reference review corrected stale pre-cursor counts in public-contract,
TypeScript, Python and Rust SDK records from the exported catalogue: 44 HTTP
operations, three Rust diagnostic methods and 39 typed Rust methods. The same
five known missing Rust operations remain explicit. Historical evidence counts
were preserved. This reference correction does not qualify those SDKs.
