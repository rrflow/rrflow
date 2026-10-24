# C-07e interleaved-history accepted-base reconciliation

**Status:** active locally qualified reconciliation; exact-candidate CI pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-c/c-07e-interleaved-reconciliation`
**Owner:** OBJ-02 prerequisite, [C-07](../../../roadmap/rrflow-1.0/gate-c-07.md)

## Brief, baseline and acceptance predecessor

[PR 64](https://github.com/rrflow/rrflow-development/pull/64) and its
[implementation record](c-07e-interleaved-lifetimes.md) remain the bounded
change brief and behavior evidence. Starting local implementation
`902a959c055a5755068bb87bcec114a85f4450df` was reconciled with accepted
main `dde6ef86d278b8c59cf1f61ebc2d3923186396d5`, tree
`bc76b62d5a500395fb8e1193dcd4d94eeac79ada`.

That main revision merged [PR 63](https://github.com/rrflow/rrflow-development/pull/63)
at 2026-10-22T18:31:32Z after all 23 checks passed for exact candidate
`3b7f8995a6e6dd33376421b2a086d45f1f5f0665` in
[CI 39822132211](https://github.com/rrflow/rrflow-development/actions/runs/39822132211).
All four explicit admission steps passed on each native OS. The six downloaded
ordinary optimized reopen receipts were validated with the workflow collector:
600 acknowledged kill/reopen cycles, 30,900 reconciled attempts, zero unresolved
outcomes/violations and all owned process/directory cleanups complete. Binary
hashes: Linux x86_64 `7df8098cbdde2f1ed5dad55be38d139f10fd76b108f66720e52893664dacd360`;
macOS aarch64 `ce6ed90f3e8f582579766c943d46e9fb470afa6d147d4304632d2eab5ce3fe28`;
Windows x86_64 `b30add3d4629cd73ce0cfc2f953f719564d3493b853965464bed5f2eab2388e5`.
The PR 63 description retains all six receipt hashes. These predecessor receipts
cannot independently qualify this package's added tests.

## Reconciliation, review and change checklist

A non-mutating merge-tree preview found only generated-index conflicts. After
PR 63 was accepted, `git fetch development main` and `git merge --no-commit
development/main` produced those same two conflicts: the gate-C journal index
and execution file inventory. Both were marked resolved and regenerated from
the complete combined source. CI workflow/policy and C-07/POAM content merged
cleanly and were reviewed to preserve admission and reader-history checks.
No source conflict or behavioral edit was necessary. No old journal body is
rewritten; this record describes the additional verification.

Production native/store/engine source is byte-identical to accepted main.
The model, adversarial entry points and new child history module are unchanged
from `902a959c`; their complete history/source and new merged CI/documentation
diff were reread. The native ownership contracts remain at the same boundaries.
No move, deletion, second oracle, dependency, format or version change occurs.
Generated inventory owns mechanical identities. This package adds this linked
record and regenerated indexes/inventory to the previously declared scope.

The original authoring checklist remains satisfied: one outcome and owning
gate, current/target/scope, independent model, first characterization, precise
acceptance, research/debug/resource decisions and stop conditions. No new
external research or algorithm is needed for base reconciliation. Preserve
bounded phase labels and native counters; do not label a finite schedule as
exhaustive concurrency, installed reclamation or measured aggregate memory.
Stop on changed source authority, dropped tests, lost/partial state, unsafe
reclamation or a failed exact-candidate check.

## Commands and results

Isolated target `target/c07e-interleaved-build`; logs in the parent checkout's
ignored `target/c07e-interleaved-reconciled-*.log`.

- `cargo test -p rrd-lsm --test projected_read_adversarial interleaved --locked -- --nocapture`:
  **3 passed / all 9 physical-terminal histories passed**.
- `cargo test -p rrd-lsm --all-targets --locked`: **149 passed, 0 failed,
  1 ignored**, 21 result groups. The ignored owned crash child is invoked by
  parent fixtures; Linux-only RSS tests execute zero cases on Windows.
- `cargo test -p rrd-store --test write_admission --test semantic_commit_atomicity --locked`:
  **6 passed, 0 failed**, checking accepted typed admission and atomicity.
- `cargo clippy -p rrd-lsm --all-targets --locked -- -D warnings`: **passed**.
- Formatting, frozen version and workflow policy passed. Production-source
  comparison with accepted main and staged/unstaged diff checks were clean.
  Documentation, navigation and inventory checks also passed after generation:
  17 indexes, 407 nodes, 1,776 edges and 1,401 file records.

No combined test or lint failure occurred. The two generated merge conflicts
were exposed and resolved rather than hidden. Full engine tests were not
repeated locally: production code is unchanged from independently accepted
PR 63, and this package modifies only LSM fixtures and required CI projections.
The full candidate pipeline still must run before landing.

## Limits and landing

Native disk exhaustion, aggregate retained-output admission, installed GC and
configuration binding, capacity profiles and a real 24-hour soak were not run
or inferred. C-07, POAM-035 and alpha remain open. This candidate must pass its
own native CI after the final reconciliation commit; the seed-only canceled
run remains non-evidence. Push destination and exact revision are recorded
immediately before push; no direct-main or official-repository promotion.
