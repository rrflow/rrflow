# C-07e accepted-base reconciliation and installed regression

**Status:** active local combined-tree qualification; exact-candidate native CI pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-c/c-07e-read-view-reconciliation`
**Owner:** OBJ-02 prerequisite, [C-07](../../../roadmap/rrflow-1.0/gate-c-07.md), [POAM-035](../../../poam/rrflow-1.0-alpha/poam-035.md)

## Brief and accepted baseline

[PR 62](https://github.com/rrflow/rrflow-development/pull/62) retains the
[reader ownership repair](c-07e-read-view-release.md) and its original
red/green evidence. This successor records reconciliation and joined installed
regression; it does not rewrite either immutable earlier journal.

The repair branch started this reconciliation at
`934f40cc360157bccbcc16eb7c2183c34417ffa3`, tree
`2d789906fbce3b3eb6ea637c573751bc0fe1ab06`. Development main advanced through
[PR 61](https://github.com/rrflow/rrflow-development/pull/61) to
`952ec21b03a9e701967f2f15c0a37e3d34051fed`, tree
`12955e6e4d097f59401f5974eadad80e6bec8292`.
Its candidate `c6b6ac5d37a96a01bbbd6b54be297b8f4ed33f31` passed all 23 jobs in
[CI 39798467016](https://github.com/rrflow/rrflow-development/actions/runs/39798467016).
The six downloaded native receipts passed the exact collector: 600 forced
restarts, 30,900 reconciled attempts, 600 resolved logical operations and no
unresolved operations or violations. These qualify the prior binary; this
repair must earn its own candidate evidence.

Current-to-target: combine accepted installed recovery qualification with the
canonical reader repair, preserving both CI requirements and all source-flow
handoff requirements. No additional runtime, format, schema, default,
dependency, release-version or authority change. Stop on lost policy checks,
changed runtime scope, failed oracle, unrelated edits or a failing candidate.
The preimplementation PR already required accepted-main reconciliation; its
brief now binds this exact merge and the combined acceptance commands.

## Reconciliation and traceability

`git fetch development main` resolved the development remote above. The
worktree was clean. `git merge --no-commit development/main` preserved history
and surfaced two conflicts: workflow policy and the generated file inventory.
Both policy blocks were retained: reader tests in each native estate job and
the ordinary-primary 100-cycle job requirements. No gate was removed.
The inventory was regenerated from the combined source. Other overlapping CI,
roadmap, POA&M and journal-index changes merged without conflict and were
reviewed against accepted main. Existing source changes and tests are byte
identical to the prior repair commit; no source was moved or deleted.

The first inventory regeneration correctly refused an unmerged index entry.
After marking that generated path resolved, regeneration succeeded. A Python
write introduced CRLF-only policy noise; it was corrected to the original LF
bytes and executable mode before review. An unquoted PowerShell revision
expression failed; the quoted tree expression above succeeded. These were
local reconciliation/tooling errors, not accepted test results. Documentation
policy also rejected an unqualified status label on this new record; the
required active status was supplied and the check rerun.

Read/review record: README and the change-authoring procedure; full prior
repair journal and the relevant C-07, CI and POA&M owners; both changed reader
test files and reader diff; resolved workflow policy, candidate workflow,
CI operations, POAM-035, C-07 and generated-index changes. The prior journal
owns full source-read and implementation traceability. The complete resolved
diff was reviewed; generated file identities come from the inventory tool.

Research/adaptation: no new algorithm or external API; accepted repository
contracts suffice. Trace/debug: preserve runtime instrumentation and use the
existing test receipts/weak-owner oracles. Resource decision: this checks
actual source release and retained copied output, not aggregate process caps,
caller-held output admission or native RSS. Ordinary installed recovery tests
use the accepted fixed workload without smaller counts or weaker deadlines.

## Commands and results

Windows x86_64 / Rust 1.99.1, shared build-output directory only (all first-party
source remains within this repository):

- `cargo test -p rrd-lsm --test projected_lifetime --lib reader::lifetime_tests --locked`:
  three unit cases passed, covering twelve physical/outcome combinations; the
  name filter selected zero public integration cases. That zero was not counted
  as evidence, and the public target was immediately rerun without the filter.
- `cargo test -p rrd-lsm --test projected_lifetime --locked`: four passed.
- `cargo test -p rrd-lsm --locked`: 138 passed, zero failed, one ignored owned
  child selected by its parent crash tests; 20 result groups. Linux-only memory
  cases execute zero tests on Windows and are not native Windows memory proof.
- `cargo clippy -p rrd-lsm --all-targets --locked -- -D warnings`: passed.
- `cargo fmt --all -- --check`, workflow policy and `git diff --check`: passed.
- `cargo test -p rrflow-cli --test installed_reopen --release --locked -- --ignored --nocapture --test-threads=2`:
  two passed in 109.03 seconds after a 3m16s optimized rebuild. Both local
  receipts passed the unchanged candidate collector: 200 forced restarts,
  10,300 reconciled acknowledged attempts, 200 resolved logical operations,
  zero unresolved operations/violations and complete owned-child cleanup.
  This local combined-tree binary is separate from subsequent clean-candidate CI.

The local full-suite log is `target/c07e-reconciled-lsm.log` in the parent
workspace. Local installed output is bounded separately in
`target/c07e-reconciled-reopen.log` and `target/c07e-reconciled-reopen/`.
No local build-output path is a source or runtime dependency.

Local executable SHA-256:
`d5cb63f31835b23425e18fd428a40c502a07043a1992d97ad2efa3aa9019e995`.
Fresh receipt SHA-256:
`932775d71adbfdcb0d492d2de66f9877a5c1f5ed53cd514de24052c36c176f2b`;
existing receipt SHA-256:
`49a6dd02c134e0b0304d2a7c2dc863ff3543c32cb897161f62efb553f1175e09`.

`python -X utf8 scripts/ci/check_documentation.py`,
`python -X utf8 scripts/knowledge/render_navigation.py --check`,
`python -X utf8 scripts/ci/build_execution_inventory.py --check` and diff
checks passed. The combined inventory contains 1,391 records. Navigation
contains 401 nodes and 1,750 edges. Final source comparison confirms the reader
and both regression files are unchanged from the prior repair revision.
Exact-candidate CI is pending; PR 62 binds the final revision, run and merge.
Its earlier canceled runs `39799044813` and `39801623826` supply no acceptance.
No whole-alpha result, hard resource limits, installed reclamation, real disk
exhaustion, capacity profile or 24-hour soak is claimed. C-07 and POAM-035 stay
open. The change checklist, bounded scope, oracle, research/resource decisions,
traceability, surfaced failures and not-run qualifications are recorded here
and in the linked original repair; no second planning authority is added.
