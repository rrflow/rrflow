# D-01.6 native acceptance receipt retention

**Status:** active measured evidence; owner reconciliation follows in this package
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-d/d-01-native-acceptance-receipts`
**Owner:** OBJ-07 prerequisite, D-01.6, POAM-006

Starting revision `fe0028c6e0fd616e38e68b59e90ff3ea9420de38`, tree
`e1a600d45b65da3c1e83e986963ef30f8f18ca39`, after
[PR 52](https://github.com/rrflow/rrflow-development/pull/52) merged at
2026-10-21T21:37:04Z. Its exact candidate was
`9b6c357186c495acd807b7efced61234fcdb324a`, with the same tree.

## Effect boundary and evidence

The D-01.6 brief was prepared before this retention step. GitHub cannot open a
zero-ahead pull request: this seed contains measured receipts and their journal,
not a planning-only commit. The draft PR is opened from this seed before any
product-status or roadmap-owner edit. No production code, runtime configuration,
format or first-party dependency changes. Existing worktrees remain untouched.

[Run 39674145683](https://github.com/rrflow/rrflow-development/actions/runs/39674145683)
has 17 successful required jobs at the exact candidate. Attempt 1 completed
fifteen jobs; macOS never acquired a hosted runner and was cancelled with an
explicit ARM64 capacity annotation, causing the aggregate gate to fail. Attempt
2 reran macOS and its dependent gate at the same source revision. No failing
test was waived. macOS's ordinary five-case lifecycle finished in 649.69 seconds;
the completed log reports success for every required job step.

The three `d01-native-installed-*.json` files under `docs/evidence/` preserve
the uploaded receipt bytes. `d01-native-qualification.json` retains candidate,
tree, run, all job conclusions, artifact IDs/digests, receipt SHA-256 values,
native ownership and mount observations, installation test names and recovery
stages. The scoped `.gitattributes` rule disables newline conversion for these
receipts. These are evidence inputs to the roadmap owner, not a new authority.

## Verification and review

- `gh run view 39674145683 --repo rrflow/rrflow-development --json attempt,status,conclusion,headSha,jobs`:
  successful, attempt 2, exact candidate, 17 distinct successful jobs.
- `gh run download 39674145683 --repo rrflow/rrflow-development --name offline-macos-latest --dir target/d01-native-final-macos`:
  passed; Linux and Windows receipts had already been downloaded from this run.
- `gh api repos/rrflow/rrflow-development/actions/runs/39674145683/artifacts`:
  all three artifact records bind the same candidate SHA.
- Local review command `python -X utf8 target/d01-reconcile-evidence.py`:
  validated all 17 jobs, three receipts, 15 actual executable cases with zero
  failures/skips, six closed fresh/existing mode receipts with cursors 2/4/3,
  privileged owner cases (one passed, zero ignored each), Linux bind mount,
  macOS mounted APFS, and installed WAL-damage preservation on all three OSes.
  It reuses `run_offline_lifecycle.lifecycle_receipts` for closed schema checks.
- Windows installation suite: 46 passed, one parent-invoked child fixture;
  macOS: 48 passed, one parent-invoked child fixture. Two extra Unix symlink
  cases account for the difference. Linux engine library: 148 passed, two
  explicit child/privilege fixtures; the privilege fixture separately passed.
  Eleven process-death stages are printed on both native platform logs; the
  same parent test passes in the Linux engine suite.
- Log extraction initially encountered Windows PowerShell's UTF-16 redirection;
  the local reader now detects its BOM. This was a review-tool decoding error,
  not a product or candidate failure. Missing root `.gitattributes` probes were
  harmless discovery misses; only the new evidence-scoped attribute is added.
- GitHub returned HTTP 404 for the still-running macOS job log; it was fetched
  and reviewed successfully after completion. No result was inferred from 404.

Full owner reads: README, alpha objectives, release-roadmap portal, Gate D and
its evidence chapter, executable dependency spine, POAM-006, CI operations,
evidence index, execution-map portal, change-authoring and implementation-
navigation procedures. Full helper reads: `run_private_owner.py`,
`run_filesystem_containment.py`, CLI `tests/support/process.rs`; receipt schema,
installation/bootstrap/concurrency/recovery symbols and final logs reviewed.
Production full-file reads and characterization belong to the linked D-01.5
journals. All three raw receipts and the selected summary were read and checked.

Research/source adaptation: none needed for measured-evidence retention; native
API research remains in the D-01.5 journals. Trace/resource/debug decision:
preserve existing bounds and only sanitized structural outcomes/digests; retain
no raw estate, credential or private reasoning. No implementation is moved,
deleted or replaced. Further local runtime suites are not repeated because the
exact candidate already passed and this package changes evidence/docs only.

Checklist: current main synchronized; precise owner/scope and failure oracle
prepared; exact CI/native evidence independently reconciled; byte-preserving
receipt retention; errors and unrun checks explicit; linked journal and generated
navigation/inventory required before commit. Stop on any missing/mismatched
receipt, failed required row, secret-bearing output or expanded runtime scope.
Owner status is unchanged by this seed. The final reconciliation journal records
mechanical validation, full diff review, PR linkage and intended owner changes;
this package still requires its own candidate CI before merge.
