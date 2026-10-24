# C-07d bounded recovery and inspection admission

**Status:** active implemented candidate; native acceptance pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-c/c-07d-recovery-admission`
**Owner:** OBJ-02, C-07d, POAM-035

Starting revision `4f871e00333ba1a286f21682c17d16951bc17076`, tree
`e08e5772aed8a16c565a92a9570eb8989a514448`.
[PR 65](https://github.com/rrflow/rrflow-development/pull/65) bound this
recovery continuation before implementation.

Current-to-target behavior: active mutable replay was capped, but normal open
and read-only inspection could read unbounded metadata/lineage and WAL frames
first. Native RecoveryLimits now admit metadata bytes, file count, conservative
whole-file read bytes, WAL bytes/frames and cooperative elapsed work. Metadata
reads use bounded native fault-observed reads before decoding; segment work is
admitted before constructing opened segment vectors. The existing WAL loop
checks the admitted length and frame count before payload allocation and still
validates every processed frame. A changing WAL length fails closed. Native
writer open and ReadOnlyDatabase share the policy; inspection additionally
bounds lineage reads and its full-file digest pass. No failure repairs,
truncates, creates a replacement authority, or retries with larger defaults.

Six required installed configuration fields reach the established
install/reopen/inspection/restore lowering and generated SDK contracts.
Work accounting is conservative (a repeated file read is charged again), not
allocator/RSS accounting or disk-space reservation. Cooperative time checks
cannot interrupt native filesystem calls. Explicit repair/configuration-change
UX and capacity headroom are not established by these limits.

Implementation traceability: DatabaseOptions/open plus native RecoveryBudget;
ManifestStore's existing current/load verification is shared through a bounded
reader callback, retaining its public unbounded component primitives; the WAL
replay loop has one internal implementation with optional admission; inspector
lineage, segment and hashing loops consume one budget. Existing corruption,
publication and authoritative-byte semantics remain. No old recovery engine or
fallback was retained. Typed RecoveryLimit maps through native/store/engine.

Reviewed boundaries: complete new recovery_budget module and recovery_limits
definitions; DatabaseOptions/create/open and segment loading; ManifestStore
current/load/checkpoint readers; WAL replay/checksum/torn-tail classifications;
ReadOnlyDatabase open/lineage/load_manifest/inspect_file; native/store/engine
error mappings; installed policy validation/default/lowering; typed fixture
regeneration and language capability fixtures. README, C-07, POAM-035,
change-authoring and existing native exhaustion investigation remain the owners.
No external algorithm/source was adapted. Diagnostics preserve content-free
resource/limit/observed errors; no new metric labels contain source content.

Characterization definitions check metadata/file/read/WAL-byte/frame pressure
for both writer and inspector, byte-for-byte preservation after denial,
exact-limit adequate reopen of every acknowledged value, and corruption still
being rejected when an initially insufficient budget is raised. They compile
but were not executed locally under the owner's instruction.

Prior candidate failure: run 39885478641, kernel-storage job 120192753421 and
optional-native job 120192753357 failed inspection.rs's two-parent assertion:
automatic GC had legally reclaimed an unpinned ancestor. The case now explicitly
checkpoints that initial ancestor and asserts its digest is authenticated in the
lineage, preserving the test's original multi-generation purpose.

Validation:
- `cargo check -p rrd-lsm -p rrd-store -p rrd-engine -p rrflow-cli --all-targets --locked`:
  passed, 12.51 seconds.
- `cargo clippy -p rrd-lsm -p rrd-store -p rrd-contract -p rrd-engine -p rrflow-cli -p rrflow-eval --all-targets --locked -- -D warnings`:
  passed, 12.83 seconds. Intermediate findings were an obsolete import and two
  unnecessary clones of Copy DatabaseOptions; both were corrected.
- Offline installation/synthetic-storage projection refresh, five SDK generators,
  generated parity, Rust/Go formatting completed. OpenAPI digest:
  `ad757ac1ddfe818f0b0b32bb06ec4f5d7f24171762decad07df7b33abc708bae`.

Local runtime suites, native disk exhaustion, capacity runs and 24-hour soak
were not run. Candidate CI remains required before merge. Synthetic generation
and compilation are not native evidence. Review/checklist: pre-effect PR scope,
canonical source/replacement map, failure/resource/trace decisions, regression
definitions and surfaced failures recorded; source and generated diffs reviewed;
linked navigation and mechanical execution inventory regenerated before commit.
No roadmap or POAM closure. Stop on lost/corrupted authoritative bytes, budget
bypass, silent repair or failed candidate CI. Recovery-space reserve, installed
repair controls, native exhaustion, capacity/host qualification and actual soak
remain owned C-07 work.
