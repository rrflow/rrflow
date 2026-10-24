# C-07d mutable limits and accepted staging admission reconciliation

**Status:** active locally qualified reconciliation; reader-history reconciliation and candidate CI pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-c/c-07d-mutable-admission-reconciliation`
**Owner:** OBJ-02 prerequisite, [C-07](../../../roadmap/rrflow-1.0/gate-c-07.md)

## Bound scope and baseline

[PR 65](https://github.com/rrflow/rrflow-development/pull/65) retains its
preimplementation brief. The [native mutable-limit implementation record](c-07d-mutable-limits.md)
remains immutable behavior/initial-test evidence. This successor records
combination with independently accepted staging admission.

Local implementation `2586b9acd6d5bfa26f990c0d31c72b019b9a3b72`, tree
`67e442b90de48e20ca86ad6cf8aa0960da419d8a`, was merged with accepted main
`dde6ef86d278b8c59cf1f61ebc2d3923186396d5`, tree
`bc76b62d5a500395fb8e1193dcd4d94eeac79ada`. Main accepted PR 63 at
2026-10-22T18:31:32Z after all 23 exact-candidate checks passed in
[CI 39822132211](https://github.com/rrflow/rrflow-development/actions/runs/39822132211)
for `3b7f8995a6e6dd33376421b2a086d45f1f5f0665`. Its six validated ordinary
optimized native reopen receipts reconcile 600 kill/reopen cycles and 30,900
attempts with no unresolved outcomes or violations. Those predecessor results
are not acceptance of this new native policy.

## Reconciliation and review

`git merge --no-commit development/main` exposed four conflicted files:
CI workflow, C-07 chapter, generated journal index and generated execution
inventory. Workflow resolution retains the four accepted encoded-admission
steps and the new mutable-limit step independently in both native partitions.
C-07 retains both prerequisite descriptions and open qualification limits.
Generated records were marked resolved before regeneration from combined source.

Native error and export definitions merged automatically; both typed error
families and exports were reviewed. The new database, memtable, mutable policy,
inspection and mutable test target remain byte-identical to `2586b9ac`.
Engine/store runtime, native batch accountant and native transaction source are
byte-identical to accepted main. Those comparisons were executed with Git.
No runtime redesign, format, dependency, version, authority or scope change was
needed. This record and regenerated projections are the additional evidence
paths. No old evidence body is silently rewritten.

The prior authoring checklist still binds one outcome, boundary, source map,
characterization, acceptance, research/resource/debug decisions and stop
conditions. Source changes, merged error/export/CI definitions and the full
combined diff were reviewed. No additional external research or upstream code
is required. Preserve native write/failure diagnostics and the exact accounted
charge definition; do not infer aggregate heap, RSS or installed policy support.
Stop on partial effects, wrong uncertainty classification, lost tests, unsafe
reclamation, an undeclared owner or failed candidate qualification.

## Verification

Isolated build target `target/c07d-mutable-build`; retained parent-workspace
logs `target/c07d-mutable-reconciled-*.log`. Commands and actual outcomes:

- `cargo test -p rrd-lsm --test mutable_limits --test write_admission --locked`:
  **8 passed, 0 failed**, joining native active-state and encoded staging limits.
- `cargo test -p rrd-lsm -p rrd-store --all-targets --locked`:
  **360 passed, 0 failed, 1 ignored**, 61 result groups.
- `cargo test -p rrd-engine --all-targets --locked`:
  **209 passed, 0 failed, 2 ignored**, 12 result groups. This complete combined
  rerun also passes the tracked-source architecture check that failed before
  staging the initial implementation's new test file.
- `cargo clippy -p rrd-lsm -p rrd-store -p rrd-engine --all-targets --locked -- -D warnings`:
  **passed**.
- Formatting, frozen product version, workflow policy and staged/unstaged diff
  checks passed. Native/storage child fixtures are invoked by their parents;
  privileged foreign-owner evidence remains native CI work. Linux-only memory
  fixtures execute zero cases on this Windows host.
- Documentation, navigation and execution-inventory checks passed after
  generation: 17 indexes, 407 nodes, 1,775 edges and 1,402 file records.

No combined test or lint failure occurred. The merge conflicts are retained
above as surfaced integration work, not hidden failures. The later reader
history PR 64 is running its own exact-candidate CI at this point; its new
interleaving fixtures are not included in these counts. After it lands, its
accepted source must be reconciled and the relevant checks run before PR 65's
final candidate push and native pipeline.

## Remaining qualification

No C-07/POAM/alpha status closes. Configurable installed startup budgets,
aggregate live owners/queues, maintenance debt/work, installed GC, native disk
exhaustion, measured capacity and a real 24-hour soak remain open. No native
memory, provider/model, PostgreSQL or whole reasoning-flow completion is
inferred. No soak was started. This local source must pass exact-candidate CI
before landing; no official-repository promotion is authorized or performed.
