# D-01.3 exclusive installation and recovery

**Status:** active bounded implementation candidate; native candidate CI pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-d/d-01-exclusive-installation`
**Owner:** OBJ-07 prerequisite, D-01.3, POAM-006

Starting revision `0a5078b58c48331f878332ead8e533ea60776ef3`, tree
`9fa25202c611ef7fa6a78c294bb546e9cc2a71d6`, was synchronized with
development/main. [PR 50](https://github.com/rrflow/rrflow-development/pull/50)
owns this package. Its first regression commit is
`e45633c832e07ba7d7bd1bbb66bce46391e54c83`. The
[first oracle receipt](d-01-install-exclusion-oracle.md) records GitHub's
no-ahead-commit refusal and the explicit PR-creation sequencing exception.
The remote change brief preceded production edits.

## Change brief and traceability

The original create-new intent and later storage writer lock allowed another
process to finish recovery while its creator was paused before storage open.
Installation now acquires one nonblocking native project lease before intent
effects and retains its exact handle through final acknowledgement. A losing
contender returns a bounded conflict. Process termination releases ownership;
neither a PID, file age nor elapsed time grants it.

The lease file is private, empty, regular and single-link. It is never unlinked
on release, which avoids splitting admission across two file objects. Windows
uses native read-access handles with read sharing, denying write/delete access;
Unix uses the existing no-follow directory-relative private-file helper.
Both use `File::try_lock`. Creation retains the original admitted handle.
Read-only preview can validate an existing empty file without acquiring or
creating one. Invalid input is refused before creating a missing lease; every
mutable precondition is rechecked under ownership.

The installation-owned read branches in
`installation.rs::apply_installation_with_observer` move to
`installation/admission.rs::inspect`: completed replay, existing intent with
clock/inventory checks, and new-install preconditions. They keep the same
engine lifecycle and durable records. `EstateFilesystem` discovers missing
private descendants after acquisition while validating and preserving every
previously retained directory capability. Storage writer locks, secrets,
initial policy, checkpoint, audit and locator publication remain at their
existing canonical boundaries. No fallback or migration lane is introduced.

Plan contract 3 binds the ninth managed path `.rrflow-install.lock`, the
absent-or-private-empty precondition, acquisition action and retained removal
rule. The result includes the lease descriptor digest. Prior plan formats fail
closed; product version stays `1.0.0`. Only a validated exact lease is excluded
from fresh-target emptiness and input inventory. The complete bootstrap effect
description remains D-01.4.

Replacement evidence is the retained installation/recovery corpus and new real
process tests: same/different plans at nine admission/effect barriers, killed
owners at five stages, prepared-secret identity, live recovery exclusion,
completed byte-identical replay, independent projects, malformed/hard-linked
lease denial, and deterministic preview after interrupted admission. The first
oracle binds the actual test executable. The wider engine-process matrix uses
a small fixed distribution identity to avoid repeatedly hashing debug symbols;
it is not primary executable or offline distribution qualification. Existing
CLI journeys bind the real primary executable.

## Source review and effect scope

Owner reads: README, alpha objective, dependency spine, Gate D, POAM-006,
agent bootstrap, installed lifecycle, change-authoring procedure and CI record.
Complete changed-file reads and final diff review cover:

- `crates/authority/rrd-engine/src/engine/installation.rs`,
  `installation/admission.rs`, `installation/lease.rs`,
  `installation/inventory.rs`, `installation/recovery.rs`, `estate_layout.rs`;
- `crates/authority/rrd-engine/src/engine/token_key.rs`,
  `token_key/private_file.rs`, `token_key/private_file/unix.rs`,
  `token_key/private_file/windows.rs`;
- `crates/authority/rrd-engine/src/engine/tests/installation.rs`,
  `tests/installation_concurrency.rs`; test registration was reviewed in the
  first oracle commit;
- `crates/transport/rrd-contract/src/attunement.rs`, `src/lib.rs`,
  `tests/attunement_contract.rs`, `fixtures/install-attunement-v1.json`;
- `crates/adapters/rrflow-cli/tests/installed_lifecycle.rs`,
  `.github/workflows/ci-reusable.yml`,
  `docs/reference/deployment/installed-lifecycle.md`,
  `docs/roadmap/rrflow-1.0/gate-d.md`,
  `docs/poam/rrflow-1.0-alpha/poam-006.md`, and this receipt.

The two installation helper modules and this journal are new. The remaining
paths above are modified; no source file is deleted. The admission extraction
replaces the original branches directly. Repository tooling regenerates the
nearest journal index and execution inventory. The contract golden is
regenerated with its explicit update environment variable; generated HTTP,
OpenAPI and signal projections remain unchanged because this plan is not an
HTTP operation.

## Research and diagnostics

Primary API references are [Rust File::try_lock](https://doc.rust-lang.org/stable/std/fs/struct.File.html#method.try_lock),
[Microsoft LockFileEx](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-lockfileex),
and [Linux flock](https://man7.org/linux/man-pages/man2/flock.2.html).
No upstream database source is adapted. The native semantics require actual
platform CI; this package makes no network-filesystem claim.

Preserve redacted lifecycle signals and add typed lease admission evidence.
Process barriers are test-only, outside the project, with finite deadlines and
owned child cleanup. No production environment failpoint, private reasoning
trace or new telemetry store exists. Resource decisions are nonblocking native
admission, no lease payload read/write, and bounded existing inventory. No
throughput, memory or scale optimization claim is made.

## Commands, results and surfaced failures

Windows local evidence:

| Command | Observed result |
|---|---|
| `cargo test --locked -p rrd-engine --lib same_plan_contender_cannot_advance_an_active_installation_intent -- --nocapture` before repair | Exit 101; 0 passed, 1 failed: contender returned success while the owner was paused. |
| Same focused oracle after repair | Exit 0; 1 passed. |
| `cargo test --locked -p rrd-engine --lib installation` final | Exit 0; 39 passed, 0 failed, 1 ignored helper. |
| `cargo test --locked -p rrd-engine --all-targets` | Exit 0; 201 passed, 0 failed, 1 ignored helper across targets. |
| `cargo test --locked -p rrd-contract --test attunement_contract` | Exit 0; 10 passed, 0 failed. |
| `cargo test --locked -p rrd-contract --all-targets` | Exit 0; 105 passed, 0 failed. |
| `cargo test --locked -p rrflow-cli --test installed_lifecycle -- --nocapture` repeat | Exit 0; 5 passed, 0 failed, 0 ignored; 217.35 seconds. |
| `cargo clippy --locked -p rrd-engine -p rrd-contract -p rrflow-cli --all-targets --all-features -- -D warnings` | Exit 0. |
| `python scripts/ci/check_generated_surfaces.py --write` | Exit 0; 42 HTTP operations; generated tracked projections unchanged. |
| `cargo fmt --all --check` | Exit 0. |
| `python scripts/ci/check_documentation.py` | Exit 0 after correcting the journal status and regenerating its index. |
| `python scripts/knowledge/render_navigation.py --check` | Exit 0; 17 indexes, 361 nodes, 1574 edges. |
| `python scripts/ci/build_execution_inventory.py --check` | Exit 0; 1311 records. |
| `python scripts/ci/check_workflow.py` | Exit 0; existing substantive lanes and added native concurrency step accepted. |
| `python scripts/check_version.py` | Exit 0; product remains 1.0.0. |
| `python scripts/ci/check_generated_surfaces.py` | Exit 0; 42 operations and seven signal projections match their owners. |

The ignored helper is explicitly launched with an exact filter by the real
process parent tests, not an omitted acceptance scenario. Local logs are under
ignored `target/d01-exclusive-*.log`; CI supplies retained candidate evidence.
The first regression-only candidate also failed in
[CI run 39644461908](https://github.com/rrflow/rrflow-development/actions/runs/39644461908):
both the engine-authority and optional-engine Linux lanes observed the same
`success` versus expected `conflict` assertion (134 passed, one failed and one
helper ignored in each library run). The full gate correctly failed. Other
green jobs on that unrepaired revision are not acceptance of this repair.

Intermediate failures were retained: the first repair rejected the new lease
as outside `.rrflow`; the validator now permits only its exact typed path.
Windows lock error 33 exposed the test digest helper reading an empty locked
file; zero metadata length proves its complete empty content, while nonempty
files are still fully hashed. Strict clippy rejected a large admission enum;
the retained optional intent is boxed without suppressing the lint. Review
also replaced filesystem re-admission with discovery that preserves existing
directory capabilities. None of these intermediate failures counts as success.
The first documentation check rejected the journal's status because it lacked
the required active/historical classification; its status is now explicitly
active.
The first CLI invocation returned exit 1 without a final test summary or
failure diagnostic after its three argument tests passed. Its cause is not
established; that attempt supplies no lifecycle pass. A repeat with live output
passed all five tests in 217.35 seconds, as recorded above. The unexplained first termination is retained as a local harness limitation; candidate native journeys must still pass.

## Checklist, remaining evidence and status

- [x] Bound one prerequisite, current/target behavior, exact scope and owners.
- [x] Recorded the pre-production PR brief and first-edit sequencing exception.
- [x] Mapped extracted behavior to the existing authority and replacement tests.
- [x] Recorded primary research, no source adaptation, diagnostics and resources.
- [x] Ran failure-first, focused, owning-package and cross-boundary checks.
- [x] Reread changed sources and reviewed the diff; retained unrelated state.
- [x] Finished local CLI and mechanical checks; regenerated navigation/inventory.
- [ ] Full exact-candidate CI and merge; these are PR evidence, not local passes.

Stop conditions remain unprotected filesystem handoff, unknown effect,
unsupported native enforcement, secret diagnostic, duplicate authority,
unbounded retry or failed acceptance. Native Linux/macOS and Linux offline
execution are not run on this Windows host; the existing candidate lanes run
them. The wider privileged foreign-owner, hostile-path and offline platform
matrix remains D-01.5. Full workspace/SDK/platform coverage is delegated to
the required candidate pipeline, not inferred from the local package suites.

D-01.1/.2 bounded accepted evidence is reconciled to their actual PRs and CI;
D-01.3 is an implementation candidate. POAM-006 records this repair candidate
without closing the original observed defect ahead of CI. D-01.4-.6, D-01,
OBJ-07 and alpha stay open. This package supplies no claim of qualified graph
or vector scale, PostgreSQL compatibility, cloud synchronization or complete
reasoning execution. PR 50 must pass the full candidate pipeline before merge;
later acceptance reconciliation records its exact revision, run and merge.
