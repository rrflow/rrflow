# C-07e overlapping snapshot and reclamation histories

**Status:** active local native qualification; exact-candidate CI and merge required
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-c/c-07e-interleaved-lifetimes`
**Owner:** OBJ-02 prerequisite, [C-07e](../../../roadmap/rrflow-1.0/gate-c-07.md), [POAM-035](../../../poam/rrflow-1.0-alpha/poam-035.md)

## Brief and baseline

[PR 64](https://github.com/rrflow/rrflow-development/pull/64) carried the
preimplementation brief. Accepted main was
`f4e404be38a0c21a4a3665b1907bad2226f5c13a`, tree
`e10ae21641df4eeea7ac647979d4f9268bce70ec`. Seed commit
`81425d0d95e9df1d6b8d82661c157fbf0ba2cada` contains the immutable
[baseline](c-07e-interleaved-baseline.md) and generated navigation/inventory.
Seed CI 39822662557 was canceled intentionally and is not acceptance.
Admission PR 63 remains independent pending CI; accepted-main reconciliation
precedes this package's final candidate.

The existing independent projected-read model already covered generated
histories and a single reader through maintenance. Separate fixtures covered
checkpoint roots and terminal ownership. This package joins overlapping
readers at distinct stamps to mutable/immutable sources, transaction pins,
checkpoint removal, compaction, actual file reclamation and durable reopen.
The first complete history passed against the accepted reader runtime; no
new runtime repair was necessary or made.

## Implemented history and traceability

The existing `tests/support/projected_read_model.rs::Model` remains the only
expected-state owner. A child support module under
`tests/support/projected_read_model/interleaved_lifetimes.rs` uses requested
mutations and independently predicted sequence intervals. The existing
`projected_read_adversarial.rs` integration target adds three physical cases:
mapped raw, bounded raw and bounded adaptive-LZ4, with fallback disabled and
cache capacity zero. Observed page counters must prove the chosen path ran.
These are fixture settings, not product defaults or an admitted capacity profile.

Each physical case runs completion, cancellation and drop: nine histories,
54 acknowledged atomic writes, 18 captured reader snapshots, nine checkpoint
releases, nine reopen checks and nine explicit directory cleanup checks.
Mixed-family physical keys contain updates and deletes; their labels do not
establish semantic graph traversal, ANN quality or reasoning execution.

Each history performs these joined checks:

1. Capture the older stream over an immutable segment and an unflushed mutable
   head; retain its first copied batch and individual Arrow data-buffer clones.
   Hold a separate native transaction at the same snapshot.
2. Write/delete, flush and compact; capture a different reader stamp and require
   the third reader to hit the active-view limit.
3. Drive a reader on an owned scoped thread while the writer commits three more
   mixed batches and performs compaction and GC. Capacity-one channels permit
   one command in flight; receives have a 30-second deadline. Compare every
   emitted prefix with its immutable expected rows, all held-transaction values
   with the old model and every current scan with the new model.
4. Remove the named checkpoint while the old reader remains live. Require both
   old manifests/segments to remain present and reported retained. They must
   already be absent from the current generation before reclamation is claimed.
5. Complete, cancel or drop the old stream. For completion/cancellation, keep
   the terminal stream object alive while requiring actual old-file removal
   and repeated admission reuse. Finish the newer reader exactly, then require
   its source reclamation and reuse of a one-view limit.
6. Join the reader, release the transaction, compact/GC and close. Reopen and
   compare the complete current state and sequence. Remove the disposable
   directory, then verify both copied snapshot batches and independently
   cloned Arrow buffers remain readable, including after the first batch drops.

Production source, formats, policy, dependency graph, authority and version
remain unchanged. Only test support, explicit native CI requirements, workflow
policy, CI operations, C-07/POAM links and generated projections change.
No source is deleted/moved and no second runtime or oracle is added.

Read/review: README/AGENTS and change-authoring owners; C-07, POAM-035 and CI
ownership; full model, adversarial/projected-lifetime fixtures and LSM Cargo
manifest; corresponding compaction/transaction/checkpoint fixtures and reader
lease/batch/finish plus database compaction/GC paths. The new child module,
changed integration/model files and full candidate diff were reread. Mechanical
paths/digests remain in the generated execution inventory.

Research/adaptation: no upstream algorithm or new external API. Existing native
snapshot, root and Arrow ownership contracts supply the oracle. Trace/debug:
preserve runtime instrumentation; bounded test phase labels and actual counters
identify physical/terminal cases. Resource decision: ownership and deletion
proof is distinct from aggregate allocation or native memory measurement.
Stop on lost acknowledged state, wrong snapshot, unsafe/stalled reclamation,
unjoined worker, changed runtime scope or failed exact-candidate CI.

## Verification and retained failures

Local host: Windows x86_64 with the pinned Rust toolchain; isolated
`CARGO_TARGET_DIR=D:/localdev/devlabs/rrflow/target/c07e-interleaved-build`.
Logs are in the original workspace's ignored `target/c07e-interleaved-*.log`.

| Command | Actual result |
|---|---|
| `cargo test -p rrd-lsm --test projected_read_adversarial interleaved --locked -- --nocapture` first characterization | 3 tests / all 9 histories passed in 0.41 seconds; no runtime edit preceded this result. |
| `cargo test -p rrd-lsm --test projected_read_adversarial --test compaction --test projected_lifetime --locked` | 18 passed, 0 failed. |
| `cargo test -p rrd-lsm --all-targets --locked` first run | 143 passed, 0 failed, 1 ignored across 20 groups; the stress example exposed an unused helper re-export warning. |

After the corrections below, the focused matrix passed all three tests/nine
histories again. Final `cargo test -p rrd-lsm --all-targets --locked` passed
143 tests with zero failures and one ignored owned crash-child fixture across
20 groups, with no warnings. The parent crash fixtures explicitly run that
child; Linux-only memory targets execute zero cases on this Windows host.
`cargo clippy -p rrd-lsm --all-targets --locked -- -D warnings` passed.
Formatting, frozen version, workflow/documentation policy, generated navigation
and execution inventory, and staged/unstaged diff checks passed. Final generated
counts are 17 indexes, 403 nodes/1,759 edges and 1,394 inventory records.

The first seed push failed with GitHub DNS resolution; the destination and
exact ref/revision were recorded again before the successful retry. This was
an external transport failure, not an accepted push or runtime result.
The helper re-export was replaced with a function entry point because the
stress example also compiles the shared model. Initial strict-lint capture
ended with exit 1 without a useful diagnostic; a direct rerun identified the
408-byte event variant. Its one large payload is now boxed, retaining a bounded
capacity-one channel without suppressing the lint. Focused tests passed after
both changes. No lifecycle invariant or test expectation was weakened.

## Checklist, limits and landing

The baseline, PR brief and this record satisfy the bounded change checklist:
owning outcome, source and expected-state authority, current/target/scope,
first characterization, research/trace/resource decisions, retained failures,
actual commands, full changed-file/diff review, navigation/inventory generation
and named stop conditions. Exact-candidate native CI and accepted-base merge
remain the landing boundary; earlier candidate artifacts cannot qualify this
new test matrix automatically.

C-07, POAM-035 and alpha stay open. This finite schedule does not exhaust all
thread interleavings, measure RSS or prove aggregate retained-output limits.
Linux-only memory fixtures do not run on this host. Installed GC composition,
configured queue/mutable/debt/recovery caps, native disk exhaustion, admitted
capacity profiles and a real 24-hour soak remain required and were not run.
No PostgreSQL, model-routing, knowledge-provider or full reasoning-flow
completion is inferred from physical storage histories.
