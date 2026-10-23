# C-07g candidate workload and metadata corrections

**Status:** active implementation evidence; native candidate qualification pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journals/gate-c/c-07g-candidate-workload-corrections`
**Owner:** [C-07](../../../roadmap/rrflow-1.0/gate-c-07.md), OBJ-02 and POAM-035

Starting source 775ae96103b0d38a3e3d639e1f279e529828bf0d; current main
c148465068f675ed70068b4b0aea60725e3718dc incorporated as
4599f5709b7a6b331a8f29dc29403d9b3f972ecf. PR 72 comments
6360663631 and 6360707755 record the pre-edit refinements.

Linux candidate 39919523350 job 120302344929 failed Workload::initialize:
expected schema revision 2, received 1. Installation already owns the initial
memory-estate schema. The driver now begins its transaction, reads that schema
through the authenticated public operation, checks the transaction/read cursor,
preserves installed definitions, rejects table collisions and increments the
revision with overflow checking. Failure aborts the lease without concealing
the original error. This follows the existing public local-database example.
No runtime schema rule or query behavior changed.

The first local compile exposed the u64 query versus u32 schema key budget;
clamp before narrowing. The next installed run reached readback and failed
because schemaless tables cannot bind typed rrflowQL record/relation fields.
The driver now declares body/string and revision/unsigned on its document
schema and revision/unsigned plus document endpoints on its relation schema.
The anchor allows absent body/revision; measured records retain their exact
independent expected values. No uniqueness or cardinality constraint is added.
Vector collection configuration and all retirement/recreation/reopen oracles
remain intact.

macOS candidate 39919513156 job 120301240754 exhausted the finite 8192-file
metadata bound without native failure. Filler::fill_metadata now syncs each
new file's metadata on macOS and retains its owned path before the sync, so
allocation failure cannot leak an untracked cleanup target. Rust's File owner
states that Drop ignores close errors and sync_all includes metadata:
https://doc.rust-lang.org/std/fs/struct.File.html. Delayed allocation is a
working diagnosis, not a proven APFS root cause. The engine must still report
its own actual native capacity error at the selected phase. Count, elapsed-time
and owned-volume limits are unchanged. Windows/ext4 retain existing behavior.
A cfg-only sync initially triggered Windows Clippy bind-instead-of-map;
using the constant platform predicate preserves Result error handling on all
builds. No upstream source was adapted.

Reviewed implementation boundaries: complete Workload::initialize and commit,
workload mutation/execute/verify/query flow, installed_capacity_workload_smoke,
Estate::install_configured, RrdClient::read_data_schema, DataSchemaSnapshot and
ReadDataSchema contracts/validation, validate_data_schema, semantic commit
schema revision validation, memory-estate planner, public local-database
initialize/extend_schema example, rrflowQL fields_for_source, and Filler
creation/sync/cleanup plus native-error handshake. These are test-driver
corrections; no source deletion, runtime replacement or parallel authority.
Changed paths are workload.rs, capacity.rs, this linked record and generated
navigation/inventory. Existing raw candidate failures and finite observations
are retained; no new runtime trace, payload log or telemetry sink is needed.

Acceptance results are recorded below before publication. No local native
APFS/NTFS/ext4 fill, full 100-cycle suite, capacity envelope or 24-hour soak is
claimed. Native macOS CI must resolve the metadata hypothesis. C-07, OBJ-02,
POAM-035 and alpha status remain open. Stop conditions remain unrelated engine,
format, authority or threshold changes; required candidate CI cannot be waived.

The next local run reached vector retirement and failed its zero-width validity
interval (39.76s). Pre-edit comment 6360740699 binds the correction: each
working generation advances checked base-valid-time + generation for writes
and retirement, and all three independent readbacks use that same time.
Immutable seeded reads keep the baseline. Runtime temporal validation stays
unchanged. Strict CLI installed-reopen/crash target Clippy passed after these
changes (0.77s); formatting and whitespace checks passed.

## Stable application backup during writes

The corrected executable flow then failed overlapping backup with `source
watermarks changed during logical export; retry from a stable cut` (42.15s).
PR comment 6360795668 binds the production repair under the package's existing
backup-under-write requirement. create_application_backup now captures one
native StorageTransaction and supplies every existing semantic repository with
a private read-only SnapshotStore. It preserves the physical snapshot pin and
transaction admission owner through archive publication, then releases them.
Repository get/scan reuse that cut; put/delete/commit are denied. Live writers
continue through the original store. Logical archive, immutable object closure,
catalogues and declared control journal therefore share one physical cut.
All existing digest and watermark checks remain, with no database-wide copy,
new public capability, format, scheduler or fallback. Direct live logical
export retains its prior concurrent-mutation rejection semantics.

Read the StorageEngine and StorageTransaction ports, native transaction
begin/get/scan and retained-pin lifetime, repository constructors, complete
create_application_backup, logical export passes/watermark checks and installed
backup operation preparation/publication. Current behavior was independent
live transactions for each repository read; canonical destination is the same
repositories over one admitted snapshot. C-07g owns this bounded repair.
Added src/snapshot.rs and its two-backend read/scan/write-denial/release
characterization; changed src/lib.rs and src/backup.rs. The focused native/MX
snapshot regression passed (1 test, 0.01s). Snapshot failures propagate and
all references drop on error. No new global serialization is introduced.

## Executed acceptance

- `cargo test -p rrd-store --lib snapshot::tests::captured_repositories_keep_one_cut_and_deny_effects --locked -- --exact`: 1 passed, 0.01s.
- `cargo test -p rrd-store --test backup_catalogue --locked`: 14 passed, 3.12s, including application object/catalogue/audit/control closure restoration and corruption denial.
- `cargo test -p rrflow-cli --test installed_reopen --locked -- sustained::installed_capacity_workload_smoke --exact --ignored --nocapture`: passed, 66.91s on native Windows x86_64. Eight seed documents, 24 edges, 16 vectors; insert/update/retire/recreate; backup overlapping a write; actual process kill/reopen; 12 exact transaction replays. Zero reported mismatch. Ordinary debug binary SHA256 033c037e210f882da77366f8a2b97a3977a7d36e786d3c1302fe286dda8715d5; backup SHA256 6b61a7935400aad2acfa703e1c97553a02d6fbe76431455be785034726c8e4d1. Build explicitly unqualified, provenance absent, capacity_qualification false. This is local changed-source characterization, not an exact committed candidate or capacity result.
- `cargo clippy -p rrd-store --all-targets -p rrflow-cli --test installed_reopen --test installed_crash --features crash-test --locked -- -D warnings`: passed, 5.75s.
- Full changed implementation and diff reviewed; no unrelated user changes. The repeated failing runs above remain part of this record. No acceptance threshold or native failure oracle was relaxed.

Navigation, mechanical inventory, documentation and workflow policy are regenerated
and checked before commit. Exact three-platform candidate CI and macOS native
metadata exhaustion remain required. No gate, version or alpha-status change.

Documentation policy initially rejected the new record status lacking the required
active/historical classification; corrected to active evidence and regenerated.
