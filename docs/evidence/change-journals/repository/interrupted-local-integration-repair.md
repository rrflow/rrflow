# Interrupted local integration repair

**Status:** active integration evidence; candidate qualification pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/repository/interrupted-local-integration-repair`
**Owner:** OBJ-07 / D-01 integration prerequisite, A-07 projection consistency

## Bound change and recovery

Starting HEAD `7c087d980c60dd45bc74ceb060fedc3b1024697e` had an interrupted merge
of `8e07399843d4dd55e847dc191e7bda59bc05614a`, with 27 unmerged paths.
Current development main was `089d05f162ebd907244a873928a45ea3cec0a77d`.
The owner explicitly requested fixing and merging this checkout. Before edits,
257 changed paths, Git merge/index state and both binary patches were saved in
`target/rrflow-merge-recovery-20261020.zip` (1,836,773 bytes).
The change brief was saved in `target/rrflow-merge-repair-pr.md` before resolution.
The existing interrupted merge was repaired in place before preparing the PR;
the ordinary branch-from-main/PR-before-edit ordering was therefore not followed.
This exception is recorded, not presented as prior PR approval.

The bounded target is a buildable integration preserving local clock/restart,
SDK identity and development-container work plus current main's installed
lifecycle, backup/restore, build provenance and learning references. No product
gate is completed by merging ancestry. No official-repository promotion,
force push or direct development-main push is authorized by this package.

## Traceability and checklist

- Owners read: README, AGENTS, change-authoring and execution-map procedures,
  CI operations and POAM-006. Existing clock, native lifecycle and invocation
  correction journals supply prior behavior and limits.
- Source inspection: complete clock module and lifecycle test; installation,
  recovery, estate-control and authority call sites; installed CLI dispatch,
  restore and clock helpers; contract export and SDK generation/test paths;
  workflow validator and development-container configuration. Large owner files
  were read in relevant sections; this is not a claim of whole-repository review.
- Local clock observation/policy remains at `RrdEngine` installation. Main's
  installed-record visibility, restore publication and estate catalogue behavior
  remain there. Remove obsolete timestamp arguments from newly merged callers.
- Main's bounded `RunningEngine` harness replaces the older child helper. Carry
  startup instance identity, authenticated restart readiness, closed sessions,
  abrupt owned-child termination, durable query/cursor/schema/manifest identity,
  clock verification and credential-redaction assertions into that harness.
  Preserve later graceful Unix shutdown and backup publication assertions.
- The canonical contract export regenerates SDK endpoints, models and signal
  identities. Authored digest citations follow that export. Keep current main's
  retired active-change record and CI topology unchanged.
- Existing historical journals remain evidence of their original executions;
  this journal records integration results. Navigation and file inventory are
  generated. No runtime authority or separate configuration lane is added.
- No external research or upstream adaptation is needed for this merge repair.
  Preserve existing clock traces and real-process failure output. No throughput,
  scale, model quality or container-build claim is made.
- Stop on unrelated worktree mutation, unknown credentials, loss of user state,
  weakened authority or failed candidate CI. Full diff review and exact-head
  candidate checks are required before merge.

## Verification in progress

The first installation run reported 19 passing tests but reused a stale binary
that lacked the clock tests present in source. A filtered listing returned zero
clock matches. These runs are excluded as evidence. Cleaning only generated
artifacts with `cargo clean -p rrd-engine -p rrd-contract -p rrflow-cli` exposed
three obsolete timestamp arguments; the same signature mismatch was corrected
in the additional client, CLI and authority test callers. CLI restore also
referenced the removed dispatch timestamp; it now samples the existing fallible
clock helper only for apply, leaving restore planning independent of time.

After rebuilding, `cargo test --locked -p rrd-engine --lib installation` passed
21 tests, zero failed, 87 filtered out, including rollback and interrupted
recovery. Workflow policy passed with 19 development-container pins.
SDK generation initially lacked the local required Python 3.15 environment and
Ruff executable; generation used the pinned Ruff 0.17.1 through an isolated
`uv run --no-project` environment. This is generation evidence, not qualification
of the Python SDK on its required interpreter. Generated Java/.NET/Python text
was normalized to repository LF after generation on Windows.

Remaining native/package/candidate results will be recorded before landing.
No objective, roadmap or POAM closure status changes. Broader signed distribution,
authenticated time, runtime-wide high-water marks, model execution and cloud sync
remain with their existing owners. Development-container image build and the
full crash/clock matrix are not part of this integration proof.
