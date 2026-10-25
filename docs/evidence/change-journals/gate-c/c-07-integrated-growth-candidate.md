# C-07: one integrated growth and native-failure candidate

**Status:** active implementation evidence; integrated candidate qualification pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/c-07-integrated-growth-candidate`
**Owner:** OBJ-02 / C-07c,f,g / POAM-035 / PR123

## Integration boundary

[PR123](https://github.com/rrflow/rrflow-development/pull/123) is the single
landing candidate for the related storage-growth repairs. It starts from public
main `94d0567fb865fb49a90aaeeef4b5861707c9f8a7`. Its reviewable brief was written
before merging implementation. The opening revision was
`79ecbb2e036008aaa9e4418c623cb10901b51d47`.

Normal merges preserve `a42491a4f0b1c9db825e8129aed00c1e3fc25982` (PR121 plus
its completed-characterization record, already including PR117/119/120) and
`94a0786710519c7e389098041c6c84cff7cde478` (PR122). No history was rewritten.
The integration introduces no additional runtime algorithm or second owner.
Relative to the first input, the only non-documentation difference is
`crates/adapters/rrflow-cli/tests/support/capacity.rs`; its Git blob equals
PR122's `774097e318e22fdd63a268e7a0e9c6c27649941c`. This is a source-preservation
check, not proof of correct behavior.

The PR's traceability table binds the existing kernel digest, archive writer,
LSM point-read selection, CLI sustained management/process/filesystem support,
evaluation backup/progress validators and native capacity fixture to their
existing owners and tests. Their complete source reviews, original failure
oracles and focused-suite results remain in the immutable
[digest](c-07g-canonical-digest-throughput.md),
[point-read](c-07g-point-read-selection.md),
[backup scheduling](c-07g-backup-scheduling.md),
[startup admission](c-07g-reopen-admission.md), and
[metadata pressure](c-07f-metadata-pressure.md) records. None is reclassified
as integrated-candidate acceptance.

## Delivery correction and retained CI

The actual main-branch rule requires strict up-to-date
`pipeline / ci-gate`. Separate landing of these overlapping candidates would
invalidate the next branch's pipeline repeatedly. This package replaces that
landing sequence with one current-main candidate whose complete pipeline must
pass. It does not waive any prerequisite, native case or branch rule.

Before cancellation, exact job status was retained in operator
`target/c07-integration-ci-transition.json`:

| Previous PR | Candidate run | Successful jobs before cancellation | Remaining work |
|---|---|---|---|
| 119 | 40125311629, `eb4d7091` | 21 | macOS 100-cycle lane |
| 120 | 40126878347, `037635af` | 20 | Windows/macOS 100-cycle lanes |
| 121 | 40128872160, `316569a3` | 18 | Three 100-cycle lanes and macOS domain crash |
| 122 | 40129500604, `94a07867` | 5 | Remaining candidate partitions, including native capacity |

These runs had no reported failing job at cancellation; they are incomplete,
not accepted. Opening-only PR123 run 40131208344 was also canceled before its
implementation candidate. PR117 run 40124044612 had already failed the macOS
8192-metadata-file bound; its remaining Windows job was canceled after retaining
the failed log. That failure remains open until genuine native evidence on the
combined candidate proves the repaired fixture. No predecessor green job is
rebound to the new candidate.

## Review, surfaced errors and verification

The only merge conflict was the generated file plan. Its generator initially
refused the unresolved index. A PowerShell command sequence incorrectly
continued after that nonzero exit and committed the unresolved generated text
locally in `e25ab019`. The error was retained and immediately corrected by
regenerating from the resolved index; `db314f0e8e32fbcc2bcbed51397d2ebf6a90c111`
contains the regenerated inventory. No conflicted candidate was pushed.
Subsequent dependent mutations use checked subprocess calls. No engine/test
source conflict or manual inventory ownership was introduced.

Read the README, C-07 owner, POAM035 links, change-authoring and structural
traceability procedures, CI owner, complete capacity fixture and complete
integration diff. Preserve parent source identities and immutable journals;
resolve additive owner links and regenerate discovery. Newly created tracked
paths are this record and the later native-result record; owner links,
navigation and inventory are the only new document effects.

Focused checks started only after the 100-reopen campaign confirmed cleanup.
On the integrated source at `db314f0e`, with only evidence-document edits in the
worktree:

- `cargo test -p rrflow-cli --test installed_crash --features crash-test
  --locked filler`: five passed; 121.141 seconds including rebuild, 0.08 seconds
  test execution. Aggregate-byte and original-deadline refill checks passed.
- `cargo test -p rrflow-cli --test installed_reopen --locked`: twelve passed,
  seven explicitly invoked native helpers ignored; 42.172 seconds including
  rebuild, 0.24 seconds test execution. Backup scheduling/deadline and original
  owned-state retention/cleanup checks passed.
- `cargo clippy -p rrflow-cli --test installed_crash --test installed_reopen
  --features crash-test --locked -- -D warnings`: passed in 26.062 seconds.
- `cargo fmt --all -- --check`: passed in 2.453 seconds.
- `python scripts/ci/check_documentation.py`,
  `python scripts/knowledge/render_navigation.py --check`,
  `python scripts/ci/build_execution_inventory.py --check`, and staged/unstaged
  `git diff --check`: passed. Navigation contains 512 coordinated nodes; the
  generated inventory contains 1608 records.

Exact commands and results are retained in
`target/c07-pr123-evidence/commands.json` and bounded per-command logs. Full
native crash/capacity, ordinary lifecycle, three-platform 100-cycle and engine
suites were not rerun locally: they are mandatory in this candidate's complete
pipeline, not replaced by the component or predecessor results. The native
100-reopen characterization has now completed as recorded
[separately](c-07g-integrated-hundred-reopens.md).

## Remaining acceptance and checklist

The completed short 10k result belongs to exact source `316569a3`, as recorded
in [its native characterization](c-07g-admission-native-characterization.md).
The additional 100-reopen campaign also keeps its own source and result; neither
is a receipt for this integrated revision. The full Desktop-corpus preflight
uses the combined candidate in the existing owned four-vCPU/eight-GiB VM with
a two-GiB engine cap and a frozen manifest. It is not the actual 24-hour soak.

Research: no new algorithm or source adaptation; retain the component primary
sources. Trace/resources: preserve native error codes, relative file admission,
private owned failure-state retention, original deadlines, hard caps, independent
ACK/effect/readback ledgers and bounded resource observations. No credential
contents are published. Stop conditions remain source conflicts requiring an
undeclared repair, changed durable identities or relaxed acceptance/security.

Checklist: one owning alpha prerequisite and bounded integration brief; mapped
source/test provenance; normal merges; reviewed source-preservation evidence;
surfaced merge/tooling failure retained; immutable linked journals; generated
navigation/inventory; focused and full-candidate checks distinguished. Native
APFS qualification, the intermittent startup cause, full profile/repetition
matrix and actual 24-hour soak remain open. C-07, POAM-035 and alpha status do
not change. Merge is permitted only after this candidate's complete CI passes.
