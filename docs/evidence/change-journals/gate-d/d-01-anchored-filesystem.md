# D-01.2 retained filesystem authority

**Status:** active implementation candidate; native candidate acceptance pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-d/d-01-anchored-filesystem`
**Owner:** OBJ-07 prerequisite, D-01.2, POAM-006

## Baseline and change brief

[PR 49](https://github.com/rrflow/rrflow-development/pull/49) recorded the
scope, replacement map, first oracle and stop conditions before implementation.
Starting main was `ff028bc8a92f0e6d83cef0b0bafdc7dc5231fd1d`; empty anchor
`8f830cd2559561f1bbf2b0a5e1d5f6ca04f9d8d0`. Its green anchor CI
`39622712517` is not implementation evidence. The prerequisite PR 48 was
merged normally after candidate `096272357c02371c8f8bc912017d9af9dbe63d90`
passed run `39621697700`, attempt 2. Main merge `2d7167c9` was synchronized
into this branch as `5c14c655`; unrelated user stashes were preserved.

Replace pathname validation followed by ambient reopening with retained
directory authority through the existing engine and storage owners. Include
read, create, publication, cleanup, recovery and verified-object handoffs.
Keep physical formats, semantic transaction authority, project identity,
credential policy and product version `1.0.0`. No alternate storage lane,
provider lifecycle, runtime dependency download or project migration is added.
Stop on an unprotected handoff, undeclared owner, insecure fallback, changed
durability/transaction semantics, unbounded retry, secret-bearing diagnostics
or failed acceptance. Whole-install exclusion and complete bootstrap remain
the following packages.

## Replacement and effect map

| Prior behavior | Canonical replacement | Evidence boundary |
|---|---|---|
| Managed paths are checked and returned as names | `estate_layout::EstateFilesystem` retains project and private-directory capabilities; `safe_join` returns `AnchoredPath` | Real installation root replacement at intent, directory and locator stages |
| Manifest, WAL, segment and inspection owners reopen ambient paths | `rrd-lsm::filesystem` supplies relative traversal, native mount/object admission, publication and cleanup; owners retain the capabilities | Manifest escape regression, filesystem fixtures and physical storage suites |
| WAL recovery and segment authentication reopen their validated files | Replay/writer and header/digest/decode use the same file; successor WAL admission requires an empty valid stream | Existing crash, torn-tail, flush and snapshot suites |
| Object verification returns a pathname | `verified_file` returns the authenticated handle; vector mapping and cluster transport consume it | Object leaf replacement preserves original bytes; vector and OpenRaft conformance |
| Backup/archive/restore convert authority back into names | Explicit `AnchorInput` captures only public path inputs; internal operations and staging callbacks carry capabilities | Logical export/resume, backup closure, restore publication and restart suites |
| Windows vector publication syncs a directory opened as a regular file | Anchored atomic no-replace publication preserves the concurrent winner and platform durability semantics | All five compact-dense tests pass on Windows |

New implementation files are `crates/persistence/rrd-lsm/src/filesystem.rs`
and `src/filesystem/native.rs`; the new integration target is
`crates/persistence/rrd-lsm/tests/filesystem.rs`. Locked third-party inputs are
`cap-primitives = 4.0.3` and `cap-fs-ext = 4.0.3` without default features.
They supply native primitives, while RRFlow owns admission, mount policy,
types, error handling, tests and authority. No upstream database tree is copied.
The public source diff and generated file inventory bind exact changed paths.

Affected existing files: LSM database, manifest, WAL, inspection, segment,
snapshot and exports; store rrflowKV, object, backup and five archive modules;
engine core, estate layout/control, installation/inventory/recovery, private
objects, backup/restore, operator and vector catalogue; vector compact,
quantized, TurboQuant and runtime; cluster adapter and transport. Existing
tests adapt explicit capability inputs without removing their assertions.
CI adds `scripts/ci/run_filesystem_containment.py` and native fixture steps.
Supporting documentation, this linked journal and generated indexes/inventory
change together. No source path is moved and no product surface is generated
from a second authority.

## Research, diagnostics and limits

Primary references reviewed: [Linux resolution controls](https://man7.org/linux/man-pages/man2/openat2.2.html),
[exclusive rename](https://www.man7.org/linux/man-pages/man2/renameat2.2.html),
[NtCreateFile](https://learn.microsoft.com/en-us/windows/win32/api/winternl/nf-winternl-ntcreatefile),
[volume ACL capabilities](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-getvolumeinformationbyhandlew),
and [cap-fs-ext](https://docs.rs/cap-fs-ext/4.0.3/cap_fs_ext/trait.DirExt.html),
with the locked cap-primitives source inspected for relative open, sharing and
reopen semantics. Native directory-relative traversal is not equivalent to
unrestricted generic sandboxing. Linux requires `statx` mount IDs; macOS uses
native filesystem/mount identity; Windows rejects reparses, requires persistent
ACL support, retains ancestors without delete sharing and publishes with
write-through rename. Unknown or unavailable enforcement fails explicitly.

Private native helpers receive one validated component and a retained parent.
Secret creation still installs security before payload writes. Ordinary payload
opens deny multiple hard links before reading or truncation. Diagnostic paths
remain labels. Standalone storage APIs capture explicit input authority once;
already anchored inputs cannot regain ambient authority.

Existing redacted lifecycle and storage diagnostics remain. Tests assert
outside sentinel bytes and observable durable state, not private reasoning.
Enumeration remains streaming at project inventory boundaries. No latency,
scale, atomic project snapshot, same-owner attacker immunity or universal
filesystem support is claimed. The Linux mount fixture has mandatory isolated
setup and finite timeouts; its ordinary ignored entry does not count as a pass.
Network and arbitrary filesystems, full privileged ownership and offline native
distribution qualification remain D-01.5/J work.

## Failure oracles and local evidence

The first command, `cargo test --locked -p rrd-lsm --test manifest retained_root
-- --nocapture`, failed before production edits: publication through a substituted
Windows junction created a second outside entry. The corrected fixture denies
publication and preserves the outside sentinel. The first failure log is kept
separately from subsequent runs under ignored `target/d01-containment-*.log`.

Local Windows results observed during implementation:

| Command | Observed result |
|---|---|
| `cargo test --locked -p rrd-engine -p rrd-lsm -p rrd-vector --all-targets` | 366 passed across 45 targets |
| `cargo test --locked -p rrd-lsm --all-targets` after successor-WAL repair | 118 passed across 19 targets |
| `cargo test --locked -p rrd-store --all-targets` including verified-handle replacement | 202 passed across 38 targets |
| `cargo test --locked -p rrd-cluster --all-features --all-targets` | 53 passed across 12 targets |
| `cargo test --locked -p rrd-engine --all-targets` after existing-only object composition | 194 passed across 12 targets |
| `cargo test --locked -p rrd-lsm --test filesystem --test manifest` including failed-publication cleanup | 15 passed: 6 filesystem, 9 manifest |
| `cargo clippy --locked -p rrd-engine -p rrd-lsm -p rrd-store -p rrd-vector -p rrflow-cli -p rrd-cluster --all-features --all-targets -- -D warnings` | Passed |
| `python scripts/ci/check_workflow.py` | Passed |
| `python scripts/check_version.py` | Passed; 1.0.0 |
| `cargo test --locked -p rrd-engine --lib engine::tests::installation` after locator capability retention | 27 passed |
| `cargo test --locked -p rrflow-cli --test installed_lifecycle -- --nocapture` | 5 passed in 270.63 seconds; actual staged binary 155,658,752 bytes |
| `cargo test --locked -p rrd-store --test logical_archive` after receipt cleanup review | 4 passed |
| `cargo test --locked -p rrd-store --test logical_archive_resilience` after receipt cleanup review | 12 passed |
| `cargo test --locked -p rrd-vector --test compact_dense` after publication cleanup review | 5 passed |
| `cargo test --locked -p rrd-store -p rrd-vector --all-targets` after receipt/vector cleanup | 256 passed across 52 targets |
| `cargo test --locked -p rrd-store --lib oversized_manifest_is_refused_without_reading_its_payload` | 1 passed |
| `cargo test --locked -p rrd-store --all-targets` after final backup review | 203 passed across 38 targets |
| `cargo fmt --all --check` and `git diff HEAD --check` | Passed |
| `python scripts/ci/check_documentation.py` | Passed: 359 document statuses, 357 classified coordinates |
| `python scripts/knowledge/render_navigation.py --check` | Passed: 17 indexes, 358 nodes, 1,564 edges |
| `python scripts/ci/build_execution_inventory.py --check` | Passed: 1,304 records |
| `python scripts/ci/test_execution_inventory.py` | 4 passed |

The CLI binary was built before the final existing-only object composition,
locator capability and temporary-cleanup refinements; targeted owning tests
cover those refinements locally. Strict Clippy was repeated after final backup
review and passed. Native Linux/macOS, privileged bind mount, full candidate partitions
and exact final revision require candidate CI. Local earlier passes do not
replace that exact-candidate gate.

Surfaced failures: native Windows generic open initially rejected inappropriate
security flags with error 87; the private relative native helper now supplies
its security-quality structure directly. Existing Windows inspection fixtures
could not read the locked empty manifest lease using `std::fs::read`; their byte
inventory records a metadata-proven empty file and reads nonempty files normally.
An existing durability fixture flushed a read-only handle; it now opens its
test-owned file read/write. Vector publication's directory flush failed with
access denied and was replaced at the already declared publication boundary.
The first broad engine run rejected the untracked new test source; staging its
declared path fixed that architecture check. OpenRaft conformance then exposed
flushing read-only published snapshot handles on Windows; writable receiving
spools still flush, while already durable published snapshots reject writes and
do not request a writable flush. The exact failed conformance case and the full
cluster suite subsequently passed. No failed case was removed or counted as a pass.

Full-file review found error cleanup in receipt, backup and compact-vector
publication could remove a pre-existing temporary file when exclusive creation
failed. Creation now precedes cleanup ownership, with receipt/catalogue writes
using the shared durable publication helper. Backup manifest size admission and
bounded reading now use the same opened file; the oversized-file oracle passes.

## Review and acceptance state

Owner reads cover README/AGENTS, alpha OBJ-07, D-01 closure packages,
POAM-006, change-authoring/traceability, independent bootstrap, installed
lifecycle and candidate CI. Full source rereads and affected-call-chain review
cover all changed Rust implementation and test files, both changed Cargo
manifests, the containment script and native workflow. This includes complete
backup/archive, segment/database, vector catalogue/runtime and OpenRaft/transport
modules, not only edited functions. Changed documentation and the complete source
diff were reviewed; generated inventory and lockfile changes are checked as
mechanical projections. The exact source path set is recorded below.

Checklist state: pre-edit brief and first failure recorded; replacement mapped
to existing authorities; research, trace/resource decisions and intermediate
failures disclosed; owning suites exercised; full-file/diff review complete.
Implementation candidate CI remains required. This record does not
declare acceptance. D-01.1's bounded merge is reconciled in the roadmap;
D-01.2 is an implementation candidate and D-01/OBJ-07/POAM-006 stay open.

D-01.1 candidate CI attempt 1 failed Linux staged launches with `ETXTBUSY`;
attempt 2 passed all jobs. That intermittent staging/launch behavior is retained
as an explicit D-01.5 qualification obligation. Its retry is not a repair claim.
Every push must record the resolved public development URL, feature ref and
exact revision. PR 49 may merge only after its implementation candidate passes
the complete pipeline; neither the empty anchor nor PR 48's CI qualifies it.

## Complete source-file review set

- `.github/workflows/ci-reusable.yml`
- `crates/authority/rrd-engine/Cargo.toml`
- `crates/authority/rrd-engine/src/engine/backup.rs`
- `crates/authority/rrd-engine/src/engine/core.rs`
- `crates/authority/rrd-engine/src/engine/estate_control.rs`
- `crates/authority/rrd-engine/src/engine/estate_layout.rs`
- `crates/authority/rrd-engine/src/engine/installation.rs`
- `crates/authority/rrd-engine/src/engine/installation/inventory.rs`
- `crates/authority/rrd-engine/src/engine/installation/recovery.rs`
- `crates/authority/rrd-engine/src/engine/restore_publication.rs`
- `crates/authority/rrd-engine/src/engine/tests/installation.rs`
- `crates/authority/rrd-engine/src/engine/tests/installation_recovery.rs`
- `crates/authority/rrd-engine/src/engine/token_key.rs`
- `crates/authority/rrd-engine/src/engine/token_key/private_file.rs`
- `crates/authority/rrd-engine/src/engine/token_key/private_file/unix.rs`
- `crates/authority/rrd-engine/src/engine/token_key/private_file/windows.rs`
- `crates/authority/rrd-engine/src/operator.rs`
- `crates/authority/rrd-engine/src/runtime/vector_catalog.rs`
- `crates/compute/rrd-vector/src/compact.rs`
- `crates/compute/rrd-vector/src/quantized_segment.rs`
- `crates/compute/rrd-vector/src/runtime.rs`
- `crates/compute/rrd-vector/src/turbo_segment.rs`
- `crates/operations/rrd-cluster/src/openraft_adapter.rs`
- `crates/operations/rrd-cluster/src/transport.rs`
- `crates/persistence/rrd-lsm/Cargo.toml`
- `crates/persistence/rrd-lsm/src/database.rs`
- `crates/persistence/rrd-lsm/src/filesystem.rs`
- `crates/persistence/rrd-lsm/src/filesystem/native.rs`
- `crates/persistence/rrd-lsm/src/inspection.rs`
- `crates/persistence/rrd-lsm/src/lib.rs`
- `crates/persistence/rrd-lsm/src/manifest.rs`
- `crates/persistence/rrd-lsm/src/segment/mod.rs`
- `crates/persistence/rrd-lsm/src/snapshot_bundle.rs`
- `crates/persistence/rrd-lsm/src/wal.rs`
- `crates/persistence/rrd-lsm/tests/filesystem.rs`
- `crates/persistence/rrd-lsm/tests/inspection.rs`
- `crates/persistence/rrd-lsm/tests/manifest.rs`
- `crates/persistence/rrd-store/src/archive/control.rs`
- `crates/persistence/rrd-store/src/archive/export.rs`
- `crates/persistence/rrd-store/src/archive/format.rs`
- `crates/persistence/rrd-store/src/archive/receipt.rs`
- `crates/persistence/rrd-store/src/archive/restore.rs`
- `crates/persistence/rrd-store/src/backup.rs`
- `crates/persistence/rrd-store/src/lib.rs`
- `crates/persistence/rrd-store/src/object.rs`
- `crates/persistence/rrd-store/src/rrflow_kv.rs`
- `crates/persistence/rrd-store/src/s3.rs`
- `scripts/ci/run_filesystem_containment.py`
