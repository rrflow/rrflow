# D-01 independent installation identities

**Status:** active bounded runtime repair and qualification evidence
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-d/d-01-independent-installation-identities`
**Owner:** OBJ-07, D-01, POAM-020

## Baseline and change brief

[PR 44](https://github.com/rrflow/rrflow-development/pull/44) began from public
development main `79cced3c311b631ca58b5aff33beba0393f9d802`, tree
`9d1a008c7747e25ec543c748b9856f00bd16cbf0`, in an isolated clean worktree.
Empty anchor `60f91637cbda126c27f2a6e7e0f5160a874e639a` allowed the PR brief
to precede implementation. Its superseded workflow `39527677994` was cancelled.

The first regression reproduced the defect: distinct empty directories with
identical content received the same project, estate and instance IDs. The
old seed bound only mode/content/profile/executable/configuration. Target:
make independent identity an explicit input to deterministic installation,
bind new apply and interrupted recovery to the previewed root, and retain the
installed identity through relocation, reopen and completed replay.

## Implementation and traceability

`rrd-contract/src/attunement.rs` gives installation plans their own version 2,
mandatory `installation_identity` and `project_root_sha256`, and a new digest
domain covering both fields. `lib.rs` exports the version. Existing attunement
and result envelopes remain version 1. Closed schema/golden tests reject
missing fields, version-1 plans, tampering and stale digests. The existing
mixed install/attunement golden is regenerated from its owning Rust test.

`rrd-engine/src/engine/installation.rs` adds
`RrdEngine::generate_installation_identity`, using its existing OS-entropy
boundary to produce a non-secret 256-bit input without project effects.
`plan_installation` requires this explicit input and validates it before
reading the target. The identity seed includes it; the plan separately binds
the initial canonical root digest. New apply/recovery compares that root before
creating intent or managed state. A completed locator resolves persisted
authority before this initial-root check, preserving portable exact replay.
No live path becomes permanent identity, global registry is introduced, or
external identity service is called.

The primary CLI exposes `install identity`, advertises the installation-plan
contract version through JSON `version`, and requires `install plan
--identity`. Rust installation callers explicitly provide one input. Reusable
test preview helpers hold a fixed fixture input; real independent-project
cases allocate separate values. The shared SDK harness uses its existing
install-anchor digest as a fixed explicit fixture identity, and only the six
resulting instance/estate references change in
`fixtures/rrd-sdk-conformance-v1.json`. The corpus's operation and outcome
assertions remain unchanged. Source scope is the PR diff and generated file
inventory; no parallel initializer or compatibility fallback remains.

Engine tests exercise distinct empty and identical-content existing projects,
foreign apply with no filesystem effect, separate credentials, foreign-key
denial and own-key authentication, invalid/tampered input, rejected interrupted
relocation and successful recovery at the original root, completed relocation
and exact replay. The existing eight-stage recovery and restore corpus remains.
The real primary lifecycle allocates an explicit input, proves generation has
no project effect, retains it for repeated previews, and denies a plan applied
to an identical foreign root before installing the intended project.

Owning lifecycle and runnable SDK instructions include the new input. The
Connectome guide explains how to retain it across regenerated previews. The
pinned workbench keeps its older named engine/SDK pair and is explicitly not
new-contract UI qualification. POAM-020 links this bounded repair without
closing broader topology work. Journal indexes and execution inventory are
regenerated before commit.

## Inspection and decisions

Reviewed README/AGENTS, OBJ-07, Gate D/dependency spine, POAM-020, topology,
installed lifecycle and change-authoring/traceability owners. Inspected
installation plan/seed/validation/apply/open/replay, root inventory, locator,
credential entropy and recovery boundaries; the closed plan/result contract
and golden; CLI command/adapter/lifecycle; every `plan_installation` caller;
the SDK harness/corpus and real-server test entry points. Changed code and
fixture diffs are reread before commit. Mechanical caller updates do not
replace their owning authentication, mutation or recovery implementations.

No external research, third-party adaptation or dependency is needed for this
local identity defect. Existing `getrandom`-backed engine entropy is retained.
Random input allocation is explicit; planning never generates randomness.
Current plan/receipt traces bind the new fields through the plan digest. No
secret, full host path, performance claim or new trace authority is added.
Stop on silent identity reuse, a permanent live-path identity, a second
authority, weakened recovery/authentication, undeclared effects or failed CI.

## Executed evidence

Windows, pinned Rust 1.99.1, shared development target directory.
`cargo test --locked -p rrd-engine
independent_installations_do_not_reuse_content_identity -- --nocapture`
first failed with identical `InstalledEstateIdentity` values (one failure,
117 filtered library tests). After implementation the same focused oracle
passed one test, including both fresh/existing installations, foreign-plan
denial and cross-credential checks. Zero-test filtered integration targets are
not counted as additional proof.

`cargo test --locked -p rrd-engine --lib engine::tests::installation --
--nocapture` passed 24 tests, including eight-stage recovery, root binding,
relocation and restore publication. The contract golden was regenerated with
the test's explicit `RRFLOW_UPDATE_GOLDEN=1` mode, then independently checked
without that mode: all nine `attunement_contract` tests passed. The complete
`cargo test --locked -p rrd-contract --all-targets` suite passed.

`cargo test --locked -p rrd-engine --all-targets` passed 182 tests across
12 result groups. Strict all-target Clippy passed for contract/engine/CLI and
client/server/MCP. The first caller check surfaced an existing Windows unused
parameter in the MCP test's Unix permission helper; explicitly consuming that
parameter on non-Unix removed the warning without changing permission behavior.
`check_generated_surfaces.py` verified all 42 HTTP operations and seven signal
projections; the public OpenAPI digest is unchanged.

A real `sdk_conformance_server` was started with the updated corpus and an
actual temporary manifest passed as `RRD_SDK_CONFORMANCE_MANIFEST` to
`cargo test --locked -p rrd-client --test sdk_conformance -- --nocapture`.
The Rust conformance test passed; harness shutdown exited zero. Corpus SHA-256:
`fd6c477471d52c5829883a6b36d734c6531b028fd82f1605495a98362d3bf874`.
Other language execution remains candidate-CI evidence, not a local claim.
The first complete `cargo test --locked -p rrflow-cli` run passed all 16
tests; its real-primary fresh/existing lifecycle cases passed in 209.72
seconds. After adding installation-contract advertisement, the rebuilt actual
binary reported product `1.0.0` and installation plan contract `2`, and strict
CLI all-target Clippy passed again. Final merged-candidate process and CI
results attach to the PR; the earlier local run is not a claim that later
source was already tested. The SDK fixture identity mismatch was identified by
caller inspection before running conformance; the fixture uses an explicit
fixed input and preserves the single shared expected corpus. Exploratory
guesses for two test paths were absent and resolved from the tracked file list.

Local documentation/workflow policy, generated navigation/inventory, four
execution-inventory tests, 17 knowledge tests, formatting and diff checks
also passed before the implementation commit.

## Checklist and limits

Owner/brief, baseline, exact scope, replacement behavior, first failure,
research and trace decisions, acceptance commands and stop conditions are
recorded above and in PR 44. Candidate CI must pass before merging. Every push
records the development URL, feature ref and exact SHA; no direct main or
official promotion push is authorized by this package.

Do not substitute this repair for project-sharing consent, a global uniqueness
registry, filesystem-race/ACL qualification, model routing, attunement,
PostgreSQL compatibility, signed distribution or an alpha release. Callers
must allocate a fresh input for each independent estate; deliberately reusing
one is not shared-context authorization. Version-1 previews are rejected, and
existing digest-bound pre-alpha estates retain their original executable;
this package supplies no upgrade/migration path. All objective, roadmap,
POA&M lifecycle and product-version status remains unchanged. Exact candidate
CI and merge receipts attach to PR 44.
