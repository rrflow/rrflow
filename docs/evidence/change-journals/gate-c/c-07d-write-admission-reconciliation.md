# C-07d accepted-reader reconciliation and combined verification

**Status:** active combined-tree qualification; exact-candidate native CI required
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-c/c-07d-write-admission-reconciliation`
**Owner:** OBJ-02 prerequisite, [C-07d](../../../roadmap/rrflow-1.0/gate-c-07.md), [POAM-035](../../../poam/rrflow-1.0-alpha/poam-035.md)

## Brief and accepted baseline

[PR 63](https://github.com/rrflow/rrflow-development/pull/63) binds this
accepted-main reconciliation as part of the original encoded-admission
brief. The [implementation journal](c-07d-write-admission.md) remains immutable
and owns its red/green results, source map and retained intermediate failures.
This record advances no additional runtime scope or release status.

The clean admission branch started at
`69efebfe6f98a7ad661cd6cfc5a253a5411f2504`, tree
`08d9f15b68d09cc44d8a705d8473e0fbc71bd553`. Development main advanced through
[PR 62](https://github.com/rrflow/rrflow-development/pull/62) to
`f4e404be38a0c21a4a3665b1907bad2226f5c13a`, tree
`e10ae21641df4eeea7ac647979d4f9268bce70ec`.
The accepted reader candidate was `948846abdfc0724534721ce1aa5656126e31f124`;
all 23 jobs in [CI 39811509553](https://github.com/rrflow/rrflow-development/actions/runs/39811509553)
passed before its merge at 2026-10-22 17:17:41 UTC. The unchanged collector
validated all six downloaded ordinary-primary receipts: 600 forced restarts,
30,900 reconciled attempts, 600 resolved logical operations, zero unresolved
operations/violations, and complete owned-child cleanup. Separate admission
and terminal-source reader tests passed on Linux, macOS and Windows. Exact
native binary and receipt digests are retained in that PR description. These
qualify the reader candidate; admission requires its own candidate CI.

Current-to-target: combine accepted reader lifetime repair with encoded write
admission and rejected-commit recovery, retaining both native test obligations.
The format, ceilings, version, configuration, dependencies, semantic authority
and implemented admission behavior remain as in the original brief. Stop on
lost checks, changed source scope, failed oracle or unrelated changes.

## Reconciliation, review and decisions

After verifying the development URL and clean branch, `git fetch development
main` and `git merge --no-commit development/main` preserved history. Four
conflicts appeared: the reusable workflow, C-07, generated journal index and
generated file inventory. Both workflow step groups and C-07 paragraphs were
retained. Both policy blocks merged automatically and were checked together.
The journal index and execution inventory were regenerated from the combined
owners; no generated inventory became a hand-maintained authority.

Reviewed: README, change-authoring procedure, C-07, POAM-035, CI operations;
full original admission and reader reconciliation journals; admission batch,
native/store transaction, semantic plan, error and engine recovery changes;
complete new admission tests and the extended semantic fixture; reader
ownership tests; merged workflow/policy and full combined diff. Earlier
journals own complete implementation-read lists. No implementation moved or
was deleted. Admission source remains identical to the starting commit;
reader source remains identical to accepted main.

Research/adaptation: no new external API, algorithm or source. Existing
format/transaction/ownership contracts supply the oracles. Trace/debug:
preserve existing runtime signals and bounded content-free admission errors.
Resource decision: encoded staging and actual source release remain separate
proofs; neither establishes aggregate process memory, installed recovery caps
or retained-output admission. The tests preserve independent expected values,
full physical fixture comparisons and authenticated stamped reads.

Exploratory reads with guessed paths and incorrectly quoted `gh --jq` queries
failed without repository effects; path discovery and PowerShell JSON parsing
corrected them. They are not runtime failures or acceptance evidence. The
four merge conflicts were resolved before testing; no gate was bypassed.

## Combined commands and results

Windows x86_64, repository-pinned Rust toolchain. Runtime and lint commands use
`CARGO_TARGET_DIR=D:/localdev/devlabs/rrflow/target/c07d-isolated-build`;
the previously observed shared-target anomaly is not reused as evidence.
Logs remain under the original workspace's ignored `target/c07d-reconciled-*`.

| Command | Actual result |
|---|---|
| `cargo test -p rrd-lsm --lib admission_tests --locked` | 4 passed, 0 failed. |
| `cargo test -p rrd-lsm --test write_admission --test projected_lifetime --locked` | 6 passed, 0 failed across both targets. |
| `cargo test -p rrd-store --test write_admission --test semantic_commit_atomicity --locked` | 6 passed, 0 failed across both targets. |
| `cargo test -p rrd-engine --lib engine::tests::write_admission --locked` | 1 passed, 0 failed. |
| `cargo test -p rrd-lsm -p rrd-store --all-targets --locked` | 353 passed, 0 failed, 1 ignored across 60 groups. Owned crash-child fixture runs through its parents; Linux-only memory targets execute zero cases on Windows. |
| `cargo test -p rrd-engine --all-targets --locked` | 209 passed, 0 failed, 2 ignored across 12 groups. Installed child runs through parents; privileged foreign-owner denial is required separately by native CI. |

`cargo clippy -p rrd-lsm -p rrd-store -p rrd-engine --all-targets --locked -- -D warnings`
passed. Formatting, frozen-version policy, workflow policy, documentation,
navigation, execution inventory and staged/unstaged diff checks passed after
final regeneration: 17 indexes, 404 nodes/1,764 edges and 1,397 inventory records.
The policy script retains executable mode 100755.

The first inventory check reported drift after this new journal was edited;
regeneration after the final evidence text resolved it. No runtime test failed during
this reconciliation. Native installed crash/reopen and all other candidate
partitions must independently build this combined candidate before merge.
The local optimized-primary corpus was not rebuilt here; the isolated affected
package suites and required candidate lane are the declared combined checks.

## Limits and checklist

The original brief and linked records bind the outcome, current/target scope,
source traceability, first failing oracle, acceptance commands, research and
resource decisions, stop conditions and surfaced failures. This successor
adds accepted-base identity, merge resolution, combined full-file/diff review,
verification and regenerated projections. Exact-candidate CI and the PR merge
record remain mandatory; the canceled seed run is not acceptance.

No C-07 subgate, POAM-035, objective or alpha status closes. Configurable
aggregate bounds, pre-lock queue admission, maintenance/debt/recovery limits,
installed reclamation, broader retained-owner interleavings, native memory,
real bounded-volume exhaustion, capacity profiles and the actual 24-hour soak
remain open. No such run is claimed here. Neither storage qualification nor
elapsed engineering hours proves full prompt/knowledge/model flow.
