# C-07h installed evidence reconciliation

**Status:** active implementation; native acceptance pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-c/c-07h-acceptance-reconciliation`
**Owner:** OBJ-02 / [C-07h](../../../roadmap/rrflow-1.0/gate-c-07.md) / POAM-035

[PR 76](https://github.com/rrflow/rrflow-development/pull/76) carries the
pre-implementation brief, based on `a08a395f979225140f89fa1736a0f3c539fb97a8`.
Dependency correction `846cb695192fd86874c21f77df61eeb69fa1a763` brings the
canonical schemaless corpus declaration from PR 72. Product version stays
1.0.0; objective, C-07 and POAM-035 remain open.

## Change brief and boundary

Previously campaign/fleet reducers verified individual histories without a
closed reconciliation of exact candidate CI, required profile coordinates and
the owner's nominal thresholds. Implemented the offline acceptance boundary
inside rrflow-eval/storage, reusing those reducers. It requires complete
successful candidate jobs/required steps, exact source/tree/catalogue identity,
raw API records matching the normalized receipt, current owner projection,
explicit absence reasons, and unchanged campaign/result semantic digests.
Bounded coverage reports retain failed points and report missing coordinates
and measured qualified envelopes. No runtime authority or release/status
mutation is added. Unknown source, altered histories, missing native steps,
failed limits or missing qualification stop acceptance.

The primary-API collector retains complete bounded attempt-specific pagination,
checks the run again against rerun/source changes, and delegates acceptance
rules to Rust. Candidate checkout policy prevents ref/repository/path overrides.
Raw record integrity is not a signed server attestation. The PR head is distinct
from its actually tested merge revision; source substitution is denied.

Owner threshold review found a verified gap: a campaign's own permissive SLOs
could pass without meeting J-04. PR comment 6359294536 bound this correction
before implementation. J-04 workload and threshold sections now mechanically
produce projection `4d8b30e45b0c53d96b27b8bd1b1a4a1bad70f2050e9f72853b2895c0e98f2e45`.
The nominal reducer requires p95/p99 conservative upper bounds and exact error
basis points for all nominal clients and three seeds. It binds a second raw
segment pass to the verified final chain digest. Physical range is a storage
proxy, not full recall/context quality. POAM-035 records the gap/remediation.

## Implementation trace and reviewed sources

The package reviewed README, OBJ-02/C-07/J-04 and POAM-035 owning records,
change-authoring and traceability procedures, existing campaign/fleet/progress
reducers, native coordinator, current workflow/checker, schema refresh/golden
fixtures and evaluation CLI. Edited source review includes acceptance.rs,
acceptance_tests.rs, acceptance_latency.rs, latency.rs, capacity.rs, fleet.rs,
fleet_latency.rs, storage/mod.rs and cli.rs; refresh_storage_contract.rs and
storage_contract.rs; collect_storage_acceptance.py and its tests;
render_capacity_profiles.py and its tests; check_workflow.py and the native
candidate workflow. Generated schemas and profile JSON remain projections.
The diff and generated inventory identify exact changed paths.

PR comment 6359301587 records the pre-edit histogram move: the existing fleet
4096-bin integer histogram moves to storage/latency.rs, keeps its independently
sorted rank oracle, and exposes arbitrary 1..100 percentiles. Fleet projects
its existing p99 contract from this shared implementation; no parallel fallback
remains. Replacement tests preserve conservative isolation ratios and add
p95/invalid-percentile boundaries. Acceptance retains at most 128 timing cohorts
per campaign and 1536 reported kind observations, with bounded source reads.
Trace output is source/run/campaign identity and aggregate measurements, not
engine content or credentials. No new runtime diagnostic path is introduced.

Research decision: this is evidence composition using the repository's existing
storage and CI contracts, not an upstream algorithm adaptation. GitHub's
[checkout source](https://github.com/actions/checkout) and
[reusable-workflow documentation](https://docs.github.com/en/actions/sharing-automations/reusing-workflows)
were consulted for the triggering-source boundary. Actual API inspection of
run 39910177310 showed head 997ffb20 and referenced reusable revision 7ce47f44;
the collector must not equate them. No successful run is invented from that
in-progress observation.

## Checks and surfaced failures

Focused acceptance tests initially exposed a Rust partial move in the synthetic
fixture's source identity; cloning the retained CI strings repaired the fixture.
The subsequent five acceptance tests passed, including required-step omission,
merge/head substitution, source/result drift, forged normalized CI and missing
coverage. Two histogram tests passed against independently sorted ranks.
The owning `cargo test -p rrflow-eval --locked` then passed all 44 tests
(30 library, 3 provider-compatibility, 11 storage-contract; no provider invoked).
`python -X utf8 scripts/ci/test_collect_storage_acceptance.py` passed five pure
API/policy cases. `python -X utf8 scripts/knowledge/test_capacity_profiles.py`
passed four projection/grammar cases. Schema refresh completed without running
an engine. Strict lint and final mechanical checks are recorded below.

No local installed workload, hard resource controller, disk exhaustion,
large corpus or 24-hour soak ran for this package. Candidate CI and complete
native campaign artifacts remain required. The synthetic API fixtures are
negative/characterization oracles, not real candidate receipts. No complete
acceptance report or qualified native operating envelope is claimed.

## Auditable change checklist

- [x] Bind alpha outcome, owning package, pre-implementation brief and baseline.
- [x] Inspect current implementation, tests, worktree, existing diff and owners.
- [x] Map histogram replacement and preserve its independent characterization.
- [x] Record source/trace/resource/research and explicit stop decisions.
- [x] Implement bounded acceptance, raw evidence binding and nominal thresholds.
- [x] Run focused oracles, then the owning evaluation suite; retain failures.
- [x] Link verified threshold deficiency/remediation in existing POAM-035.
- [ ] Record final lint, generated/navigation/inventory checks and candidate CI.
- [ ] Qualify native profile/soak evidence and hand off accepted Gate C results.

No roadmap status change. No missing run is treated as passed.

## Final authoring verification

`cargo clippy -p rrflow-eval --all-targets --locked -- -D warnings` passed
(1.20s). `cargo clippy -p rrflow-cli --test installed_reopen --locked -- -D
warnings` passed (20.04s), covering the native harness compilation against the
shared latency refactor. Navigation, inventory generation, documentation,
workflow policy, generated-surface parity and `git diff --check` passed.
Generated parity retains 44 operations and OpenAPI digest
`6a8d3e490eeb0e13a10465b752dee165223c7097e18ccd08acb79b38934e4650`.
The journal was appended before final inventory regeneration. Candidate CI is
pending and the native profile/soak checklist remains unchecked.

## Outer native cleanup reconciliation

Read the existing sustained launcher, native cgroup/Windows Job ownership
helpers, campaign writer/reducer and fleet child/result binding. Inspection
found that the inner harness can seal a successful result before outer native
cleanup fails. Pre-edit PR comment 6359584213 binds the correction at this
acceptance boundary. Extended the existing launcher, not the engine, to retain
an exclusive 8 KiB `launch.json` after cleanup, including failed/interrupted/
unavailable outcomes. If no harness directory exists it retains only the
frozen campaign and outer receipt; it never fabricates an inner result. Existing
inner evidence cannot be overwritten. Reservation covers the receipt before
workload admission. The acceptance manifest binds the outer receipt separately;
all resource samples must match its allocation and harness across the exact
verified segment/trial chains. Missing/failed/unconfirmed cleanup cannot qualify,
even if the inner result passed. Reports expose both outcomes distinctly.

The first refresh command exposed a missing StorageLaunchReceipt import in
the schema generator; corrected the explicit import and reran successfully.
Six focused acceptance tests passed. `test_sustained_launch.py` passed three
pure filesystem cases (0.051s), including inner success followed by outer
failure and exclusive receipt preservation. The evaluation owner suite passed
45 tests (31 library, 3 provider-compatibility, 11 storage-contract), and strict
all-target evaluation Clippy passed (2.53s). No native process/resource fixture
or soak was launched by these regressions. Schema projections were refreshed.

PR 71 candidate run 39910177310 separately completed all three native domain
crash/capacity jobs (120272369258 Linux, 120272369290 Windows, 120272369301
macOS). Retrieved job logs each contain eight fresh/existing actual-capacity
receipts and a passed suite. This is evidence for that candidate only, not
this newer acceptance candidate or a 24-hour capacity qualification. Its
Windows ordinary reopen job was still running when inspected.

The launch oracle additionally read a two-segment chain and rejected a second
allocation even when the changed segment's hash chain was recomputed correctly.
The focused regression passed (0.03s); strict all-target lint passed (1.12s).
Navigation, inventory, documentation, workflow policy and diff checks passed
for the cleanup refinement before publication. Final inventory regeneration
includes this evidence paragraph.
