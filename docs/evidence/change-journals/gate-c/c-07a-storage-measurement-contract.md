# C-07a storage measurement contract

**Status:** active implementation evidence; synthetic contract qualification only
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-c/c-07a-storage-measurement-contract`
**Owner:** OBJ-02 prerequisite, [C-07a](../../../roadmap/rrflow-1.0/gate-c-07.md), early [J-04a](../../../roadmap/rrflow-1.0/gate-j.md#workload-and-evidence-structure) subset

## Baseline and change brief

Baseline `6d63bea79094bea13c0487090d889dde315a8fbd`, tree
`e7be3f22e6a49d81130604ab25a925f501177fdc`, was synchronized from public
development main. [PR 55](https://github.com/rrflow/rrflow-development/pull/55)
carried the exact brief before implementation. The immutable
[baseline journal](c-07a-measurement-baseline.md) enabled PR creation and
records the prior zero-test evaluator and retained eight-trial verification.

Current-to-target: extend the existing paired-provider evaluator with the
versioned storage history manifest/result/report, generated schemas, bounded
admission, effective public-diagnostic policy projection and independent
acknowledgement ledger. This does not implement native fault collection,
runtime hard caps, a workload driver or a second configuration authority.

Exact code scope: evaluation `Cargo.toml`, `src/main.rs`, new `src/lib.rs`,
`src/storage/{mod,model,policy,ledger,validation,cli}.rs`,
`tests/storage_contract.rs`, `tests/provider_compatibility.rs`, and three
synthetic JSON fixtures under `tests/fixtures/storage/`. `Cargo.lock` adds
only dependency edges to already locked `rrd-contract`, schemars, sha2 and
tempfile; no third-party version changes. Three schemas under `eval/schemas/`
are generated from Rust. `eval/README.md` documents the executable interface.
The C-07 chapter and dependency spine link this evidence; journal navigation
and the file inventory are regenerated. No code moved or deleted, no runtime
policy/format/public operation changed, no workflow edited, and version stays
1.0.0. Stop conditions were runtime scope expansion, a new authority or
unsupported qualification claims; none was needed to complete this subset.

## Read and traceability record

Read in full for this package: evaluation manifest/main, all new handwritten
evaluation modules/tests, the new tool instructions and this journal;
C-07 plan, POAM-035, change-authoring and implementation-navigation records.
README/AGENTS were read as bootstrap. Targeted reads covered OBJ-02,
dependency-spine current order, J-04 measurement structure, CI topology and
verification commands, LSM `MaintenancePolicy`, `CompactionPolicy`,
`DatabaseOptions`, `Durability` and WAL payload limit; store
`physical_store_evidence`; engine `rrflow_kv_snapshot`; public
`DiagnosticStorageSnapshot` and nested policy types, `EstateConfiguration`
validation/default, endpoint catalogue and capability types; workspace
architecture dependency checks and the existing provider evidence header.
These larger engine/contract/test files were not read in full or changed.

Preservation map: `Trial`, `run_trial`, `metrics`, `summarize`, `verify` and
their provider dispatch remain at their original boundary; the only main
change is storage subcommand dispatch. Three new compatibility tests verify
retained provider evidence, paired regression/token summaries and invalid
run-argument rejection without launching a model. Public `rrd-contract`
types/catalogue remain semantic owners, with no private store dependency.
Synthetic expected effects are independent fixture inputs, frozen into the
manifest before observed readback is considered. Schema/fixture generation
is a documented explicit test command, not a runtime fetch or hidden step.

## Behavior and oracle

All structures reject unknown fields. Identifiers/digests and finite positive
limits have validated scalar domains; generated schema assertions retain
those constraints. Input/output is at most 8 MiB; cases/bindings at most 256;
attempts at most 10,000 across the result. Disjoint memory reservations fit
the process allocation, estate allocations fit host memory, atomic work fits
its reservations, disk preserves temporary/recovery space, and deadlines fit
the run. Timing uses JSON-exact bounded client monotonic nanosecond offsets.

The manifest binds source, executable, catalogue/configuration/corpus and
expected-ledger identities, platform/device/flush/tool identity, declared
driver support and required cases/seeds. Neither declarations nor digests
authenticate execution or qualify a provider. Six effective settings derive
from the existing diagnostic owner and name their actual LSM type. Soft
targets remain soft; the absence of installed storage reconfiguration is
explicit. C-07d owns hard-cap enforcement, C-07b/c/f own native fault collection,
and C-07g/J-04 own sustained workload/resource series and performance evidence.

The ledger distinguishes ACK, definite rejection and uncertainty; it checks
expected presence/absence, exact request/effect/receipt and one application.
Missing readback stays unresolved. Lost ACKs, wrong/duplicate effects,
changed retries, unknown/duplicate identities, changed expected data and
all-rejected workloads with a required positive effect cannot pass. Missing
cases reject; failed/skipped/unimplemented cases retain coverage/provenance
without passing. Well-formed failing histories write a bounded failure report
before exit failure; existing reports cannot be overwritten. Parse diagnostics
retain location/category without echoing rejected input.

## Commands and results

- First executable history oracle:
  `cargo test -p rrflow-eval --test storage_contract admits_closed_manifest_and_independent_acknowledgement_history`
  passed one test after the compiler corrections below.
- `cargo test -p rrflow-eval --test storage_contract` passed 11 tests:
  admission, scalar/nested rejection, budget overflow/conflicts, exact
  histories, lost/duplicate/rejected effects, uncertainty, false-success
  rejection, coverage, digest/duplicate-key handling, generated parity and
  bounded/no-overwrite CLI failure retention.
- `cargo test --locked -p rrflow-eval` passed 14 integration tests (11 storage,
  three provider compatibility), zero failures/ignored. Library, binary and
  doc-test targets contain zero tests; they are not counted as coverage.
- `cargo clippy --locked -p rrflow-eval --all-targets -- -D warnings` passed.
  `cargo fmt --all -- --check` passed.
- `cargo test --locked -p rrd-lsm --test maintenance --test failure_matrix`
  passed three maintenance and four failure-matrix tests. Injected logical
  StorageFull/Crash cases remain component evidence, not native exhaustion.
- The documented `storage validate` and `storage verify` commands passed on
  the synthetic fixture. The latter created a new ignored report in `target/`.
  It reported one case, three logical calls/four attempts, two ACKs, one
  rejection, one uncertain response, and explicit two-present/one-absent
  resolution. The result visibly remains `Synthetic`.
- Actual CLI `storage schema manifest/result/report` stdout matched the three
  generated files byte for byte. Golden generation used the documented
  `RRFLOW_UPDATE_STORAGE_GOLDENS` switch; ordinary package tests then passed
  with it unset. No raw result was replaced by a generated success.
- Documentation policy passed with 383 statuses and 381 classified
  coordinates; navigation passed with 17 indexes, 382 nodes and 1,666 edges;
  the generated inventory passed with 1,359 path records. Diff checks passed.

## Failures, research and limits

Intermediate compiler failures exposed SHA-256 output formatting (the locked
digest array has no LowerHex implementation), a fixture's overlapping borrow,
and ambiguous `TryFrom` input inference. These were corrected directly.
Schema review then found serde `try_from` erased field validation in generated
schemas. An attempted container validation annotation failed compilation;
the supported `#[schemars(!try_from, !into)]` override preserves field
constraints. Explicit schema assertions now cover minimum/maximum, digest
pattern and identifier bound. Primary reference:
[Schemars attribute documentation](https://graham.cool/schemars/deriving/attributes/).
The prior C-07 primary-source research informs acknowledgement/uncertainty
semantics; no upstream algorithm or implementation was copied. Exploratory
missing paths/PowerShell glob misses were resolved by repository discovery.

No runtime spans or metrics were added: this is offline evidence validation,
with bounded diagnostics and retained failures. No real provider/network
call, native power cut/disk exhaustion, new installed crash matrix, memory
measurement, throughput/soak, competitor evaluation, signed bundle or full
alpha test ran locally. No new runtime storage defect was reproduced.
The early history format is intentionally finite; J-04's full measurement
collector and versioned large-run artifacts remain future implementation.
Input validity cannot prove external collector completeness or truthful data.

## Review, checklist and landing

The handwritten source/tests, command instructions, owner changes and complete
diff were reviewed. Generated schemas/fixtures were checked through deterministic
parity and explicit constraint assertions, rather than manually maintained.
The checklist is satisfied by the PR's prior brief, owner/baseline inspection,
preservation map, independent negative oracle, research/trace/resource decisions,
focused then package checks, recorded errors, generated checks and this linked
journal. The final PR records exact tested revision, candidate CI and merge.
Only the public development remote receives this branch; no official promotion.

C-07a now has bounded implementation evidence. C-07, J-04, OBJ-02 and
POAM-035 remain open; there is no alpha maturity change. After exact-candidate
CI and merge, C-07b is the next ordered implementation package.
