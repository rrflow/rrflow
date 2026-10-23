# C-07g shared fleet budget

**Status:** active implementation evidence; native fleet qualification pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journals/gate-c/c-07g-shared-fleet-budget`
**Owner:** [C-07g](../../../roadmap/rrflow-1.0/gate-c-07.md), OBJ-02 and POAM-035

Starting revision: b3bd96e5077a53b6da094baacc3354923737c367, PR75.
Pre-edit comment 6362199886 binds this correction. The canonical J-04 fleet
has ten estates under one 8 GiB aggregate engine ceiling on a 32 GiB host.
The existing validator equated process_bytes with that aggregate ceiling, then
multiplied it by ten, making the owning profile inadmissible as 80 GiB.
The actual projected-profile CLI input failed with
`estate reservations exceed host budget`; no workload was executed.

The existing Budgets owner now exposes optional aggregate_process_bytes.
Without it, ordinary per-estate reservation semantics are unchanged. With it,
the individual process ceiling and the sum of disjoint estate reservations
across all estates must fit the shared pool, and the pool must fit host_bytes.
Multi-estate campaigns must declare that pool and bind it exactly to the
native allocation engine_bytes. The existing process ceiling remains equal
to engine_bytes, matching the current Windows Job/cgroup binding. Mechanical
child campaigns inherit the same pool and limiter identity. This does not
multiply, enlarge or remove the native aggregate limit.

Reviewed the full Budgets model/validation, StorageCampaign allocation binding,
J-04 profile projection, fleet_child/validate_fleet and existing fleet tests,
native allocation receipt/membership validation and launcher bindings. The
single-estate synthetic campaign fixture now declares one estate; fleet tests
explicitly declare a shared pool. The old standalone manifest behavior remains
covered. No engine configuration, SDK, runtime policy, external dependency or
new authority is introduced. Research/source adaptation is unnecessary because
the owning profile and native limiter semantics establish the mismatch.

Changed: storage/model.rs, policy.rs, sustained.rs and fleet_tests.rs; three
generated storage manifest/campaign/segment schemas; this journal, POAM-035 and
navigation/inventory. The optional field preserves existing nonshared evidence
encoding by omitting None. Unsupported shared declarations fail closed; no
qualification status is inferred from an accepted manifest.

## Validation

The focused canonical ten-estate qualification declaration passed, including
mechanical child derivation and rejection of an omitted/mismatched pool,
reservation overcommit and insufficient host budget (one test, 0.01s).
The existing shared Cargo target initially exposed a stale contract artifact
from the checkpoint worktree; no source workaround was added. An isolated
evaluation target compiled the correct owning dependency. The full suite first
reported expected schema-golden drift. After the existing
refresh_storage_contract example regenerated projections, all 39 tests passed:
25 library, three provider compatibility and 11 storage-contract tests. Strict
all-target evaluation Clippy passed (16.46s). Formatting/diff checks passed.

Exact commands: `cargo test -p rrflow-eval --lib --locked canonical_ten_estate_profile_admits_one_shared_pool_and_denies_overcommit`;
`cargo run --quiet --locked -p rrflow-eval --example refresh_storage_contract`;
`cargo test -p rrflow-eval --locked`;
`cargo clippy -p rrflow-eval --all-targets --locked -- -D warnings`.

No native fleet or 24-hour campaign ran for this repair. Candidate CI and full
profile evidence remain required. Stop on any changed J-04 allocation, false
per-process enforcement claim, hidden overcommit or qualification shortcut.
Version, C-07 and alpha status remain unchanged.
