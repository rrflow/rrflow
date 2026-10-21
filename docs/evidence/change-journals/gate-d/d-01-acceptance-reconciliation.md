# D-01.6 installed-foundation acceptance reconciliation

**Status:** active acceptance reconciliation; this documentation candidate requires CI before merge
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-d/d-01-acceptance-reconciliation`
**Owner:** OBJ-07 prerequisite, D-01.6, POAM-006

Starting main revision `fe0028c6e0fd616e38e68b59e90ff3ea9420de38`, tree
`e1a600d45b65da3c1e83e986963ef30f8f18ca39`.
[PR 53](https://github.com/rrflow/rrflow-development/pull/53) carries the
pre-edit change brief. Its evidence seed `97324bb00d02ddd566572bca6c8fcc5c3fc05234`
retained actual measurements and the [retention journal](d-01-native-acceptance-receipts.md)
so GitHub could open the draft before owner edits. No planning-only commit or
retired active-change record was introduced.

## Brief, scope and acceptance decision

Current behavior already passed D-01.5's exact candidate; README/objective still
described eight recovery stages and the roadmap still named D-01 as active.
This package reconciles the eight D-01 acceptance rows against the observed
native results, retains their provenance and advances only that prerequisite.
The [accepted matrix](../../../roadmap/rrflow-1.0/gate-d-evidence.md#d-01-accepted-installed-foundation-2026-10-21)
is the roadmap-owned result. C-07 becomes the active executable item. Product
version stays 1.0.0, maturity stays pre-alpha, OBJ-07 stays partial and POAM-006
stays open. No graph/vector scale, full PostgreSQL compatibility, model execution,
cloud synchronization, signed distribution or complete alpha claim is made.

Changed owners: README current-status passage, OBJ-07 state, release-roadmap
D-01 projection, Gate D package table/accepted matrix, dependency spine,
POAM-006 current assessment, CI accepted-evidence link and evidence index.
Created evidence: three exact native receipt JSON files, one selected CI/native
observation JSON, scoped `.gitattributes`, two immutable journals. Generated
journal index/navigation and exhaustive file inventory are refreshed. No runtime
implementation, public schema, dependency, executable operation, test, workflow,
format or installation behavior is changed, moved, deleted or replaced.

Research/source adaptation: no new research is necessary to reconcile measured
results. Native API choices and prior failing candidates remain in D-01.5's
immutable journals. Trace/resource/debug decision: preserve existing production
and qualification bounds; retain selected structural evidence and digests only.
No new telemetry, diagnostic collector or semantic authority is introduced.

## Review and oracles

Full reads: README; alpha objective; release-roadmap portal; Gate D requirements
and evidence; dependency spine; POAM-006; CI operations; evidence index;
execution-map portal and linked change-authoring/implementation-navigation
procedures. Full native-owner/mount wrapper and CLI process-helper reads plus
receipt admission, installation/bootstrap/concurrency/recovery symbols, final
native receipts and logs underpin the acceptance review. The source/test full
reads and implementation failure oracles remain bound by D-01.5 journals.

First acceptance oracle: reject any mismatched candidate, failed/missing job,
missing lifecycle case, skipped required native fixture, invalid closed durable
receipt, changed artifact bytes or unsupported owner-status claim. The local
review script reused the checked-in receipt admission and independently joined
17 job results, three artifact identities and native fixture/test observations.
All required rows passed before owner status changed. Read-only exploration of
C-07 maintenance/failure tests made no C-07 changes or completion claim.

The seed's `git diff --cached --check` flagged CRLF in the original Windows
uploaded receipt. Byte comparison simultaneously proved all three staged blobs
matched their uploads exactly. The seed retained those measured bytes; the
pre-edit PR brief declared the correction. A file-specific `whitespace=cr-at-eol`
attribute now recognizes its native endings; `-text` prevents checkout
conversion. No evidence is normalized or silently rewritten. The final diff
check and staged blob comparison must both pass.

## Commands and results

- Final native acceptance: [run 39674145683](https://github.com/rrflow/rrflow-development/actions/runs/39674145683),
  exact candidate `9b6c357186c495acd807b7efced61234fcdb324a`, 17 successful jobs,
  with only macOS/gate retried after runner-capacity cancellation. PR 52 merged
  as the starting main revision, preserving the exact candidate tree.
- `python -X utf8 target/d01-reconcile-evidence.py`: passed; 17 successful jobs,
  three native receipts, fifteen actual isolated cases, six closed mode receipts,
  explicit ownership/mount results and installed WAL denial. The retained
  observation JSON contains exact names/hashes rather than a prose inventory.
- Seed navigation: 17 indexes, 374 nodes, 1,612 edges; inventory: 1,335 records;
  documentation: 375 statuses and 373 coordinated records; all passed.
- Three staged Git blobs compared byte-for-byte with the original downloads:
  passed. Git reports `text: unset` for each retained native receipt.
- Final owner-edit script preflight checked every replacement exactly once in
  seven current owner files before mutation; passed. Eight acceptance rows and
  remaining owner limits were then reviewed in the final evidence chapter.

Final mechanical results:

- `python -X utf8 scripts/knowledge/render_navigation.py` and `--check`: passed,
  17 indexes, 375 nodes, 1,625 edges; no remaining index drift.
- `python -X utf8 scripts/ci/build_execution_inventory.py` and `--check`: 1,336
  current/generated/planned records; final regeneration follows this journal edit.
- `python -X utf8 scripts/ci/check_documentation.py`: passed, 376 statuses,
  374 coordinated records, owner/index/link checks.
- `python -X utf8 scripts/ci/check_workflow.py`: passed, three workflows,
  seven substantive job families, five suites, twenty default-feature packages,
  six optional-feature packages and nineteen container pins.
- `git diff --check fe0028c6e0fd616e38e68b59e90ff3ea9420de38`: passed, including
  the byte-preserved CRLF receipt under its file-specific attribute.
- Owner files, the complete change diff, both new journal bodies, receipt
  fields/test names and generated navigation/inventory were reviewed. Corrected
  the old Gate D prose that still called its parent open; historical baseline
  paragraphs now explicitly identify their earlier revision.

This documentation/evidence candidate must still pass its own full PR pipeline;
a future CI or merge result is not claimed inside an immutable pre-merge journal.
The PR retains those exact results. The seed run is superseded by the final
owner/evidence candidate; its partial results do not count as acceptance.

No local Rust suite is repeated: the accepted source tree passed all native and
package lanes, and no runtime/test source changed here. This is not permission
to reuse that runtime result for a different implementation revision. Remaining
errors are the already-open C-07, later D/E/F/G/H/I/J and POA&M work.

Checklist: current main and tree bound; PR brief before owner edits; one coherent
prerequisite; existing implementation/tests preserved; independent acceptance
oracle; research/debug/resource decisions explicit; failed checks retained and
corrected; full-file/diff review and generated projections required; unrun tests
and future CI explicit. Stop on any missing row, evidence drift, unexpected
runtime change, secret-bearing receipt or false alpha-status closure.
