# D-01.5 installed WAL damage preservation

**Status:** active locally verified characterization; final native candidate pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-d/d-01-installed-wal-denial`
**Owner:** OBJ-07 prerequisite, D-01.5 installed-authority row, POAM-006

Starting revision `d0c5fccf055ff83c3a90bff6ae8967bb0a62d380`, tree
`4b57c79ded9068ddc7aff5a054acf1c33244b1e3`;
[PR 52](https://github.com/rrflow/rrflow-development/pull/52) brief amended before
implementation. Acceptance reconciliation found lower WAL-writer/inspector
torn-tail denial and separate create/open tests, but no native installation
case directly asserting that installed open and verification both preserve a
torn or missing active WAL. Existing implementation already rejects these
states; the gap was installed-composition acceptance coverage.

## Exact scope and oracle

`engine/tests/installation.rs::installed_open_and_verify_never_repair_a_torn_or_missing_active_wal`
installs two independent project fixtures through the existing engine entry,
asserts each has one active committed WAL, then appends an incomplete tail or
removes that fixture's active WAL. Installed open and installed verification
must both reject, with the entire project file/byte inventory unchanged after
each attempt. This observes no truncation, recreation or unrelated mutation.
The existing small inert executable-identity fixture is retained; this test is
component composition evidence, distinct from the five actual executable cases.

Only this Rust test file, this journal, a POAM-006 note and generated projections
change. Production constructors, recovery, storage format, APIs and authority
are unchanged. No upstream implementation or new online research is needed:
the owning acceptance row and current installed/store/LSM code determine the
oracle. Existing bounded native runner and redacted receipt decisions remain.

Complete changed-file reread: `crates/authority/rrd-engine/src/engine/tests/installation.rs`.
Additional full test read: `crates/persistence/rrd-store/tests/installed_lifecycle.rs`.
Relevant sections reviewed: LSM inspection/WAL tests, Database::open_anchored,
RrflowKvStore::open_existing_anchored and from_explicit_database, README status,
OBJ-07, Gate D acceptance matrix and existing D-01 package records. The broader
PR retains its baseline owner/procedure read set. Complete diff review confirms
one added test, with no rewritten or removed production behavior.

## Commands and results

- First focused compilation surfaced E0433: InstallationManagedPathKind was not
  imported in the test module. The test now uses the canonical rrd_contract
  qualified type; no type duplication or production change was introduced.
- `cargo test --locked -p rrd-engine --lib installed_open_and_verify_never_repair_a_torn_or_missing_active_wal -- --nocapture`:
  one passed, zero ignored, 0.19 seconds (both damage cases).
- `cargo test --locked -p rrd-engine --lib installation -- --nocapture`:
  46 passed, zero failed, one explicitly parent-driven ignored child fixture,
  101 filtered, 23.82 seconds. Retained log:
  `target/d01-native-no-repair-installation.log`.
- `cargo clippy --locked -p rrd-engine --tests -- -D warnings`: passed, 4.78 seconds.
- `cargo fmt --all --check` and `git diff --check`: passed.
- Navigation: 17 indexes, 373 nodes, 1611 edges; inventory: 1329 records;
  workflow policy passed. The documentation check first rejected this journal
  because its status lacked the required active/historical classification;
  the status now explicitly says active. Documentation validation is rerun.

No unrelated local package suites are repeated: the change adds an installation
characterization only. Linux/macOS execution of this new case and the complete
exact-candidate pipeline remain required. The previous candidate's isolated
Windows IPv6/product result is still pending when this record is prepared.

Checklist: baseline/brief/owner and precise failure oracle established before
edits; current behavior mapped to the existing installed composition; small
real fixture and complete byte inventory selected; compile failure recorded and
corrected; focused then owning tests passed; full changed-file/diff review;
linked evidence/projections and unrun checks explicit. Stop on any mutation of
the damaged estate or unsupported native qualification. No D-01, OBJ-07, alpha
or POAM-006 status closes with this additional characterization.
