# C-07e bounded acquisition of large history pages

**Status:** active native repair evidence; installed capacity qualification open
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journals/gate-c/c-07e-large-page-acquisition`
**Owner:** [C-07e/g](../../../roadmap/rrflow-1.0/gate-c-07.md), OBJ-02 / POAM-035

Starting revision 07cc02c9f2958cadb07e68916e0828ec870e5f11, tree 8951511a614aa2da3e7d61f960dd345e9a14343b. PR72 pre-edit comment
6363377986 binds the exact source scope, failure oracle, research and stop
conditions. The combined installed candidate 8b301075 reached its first
input-admitted compaction but failed campaign10 with an Internal response.
That failure remains retained; its exact server cause is not inferred from
this independent source-level defect.

## Change brief and traceability

The v6 row-group contract retains a complete key/version chain even when it
exceeds a soft page target. File-backed `segment::read_page` allocated that
valid page and submitted it as one native read. `IoContext::read_exact_at`
correctly denied requests above the configured per-request bound. The native
regression constructs two 3000-byte versions in separate segments, then
compacts them under a 4096-byte request cap. Before the repair compaction
failed: `segment page requests 6000 bytes beyond the configured 4096 byte I/O bound`.

The same file-backed acquisition now reads its existing page buffer in chunks
no larger than the existing I/O cap, carries the actual backend selection
through fallback, checks offset arithmetic and verifies the full stored-page
checksum before returning any page. Empty-page behavior is preserved. No
format, digest, page allocation, logical-read budget, mmap semantics, policy
default or error classification changes. This replaces the single-request
assumption at its canonical owner without introducing another reader.

Reviewed owner records: README, C-07, POAM-035, the change-authoring routine
and tiered-persistence reference. Implementation reviewed: complete
`tests/tiered_io.rs`, `segment::read_page`, its page-source/context/checksum
branches, `IoContext::policy/read_exact_at/record_read`, v6 row-group and
compression bounds, database compaction and its native regression patterns.
Existing corruption, snapshot and raw/decoded lifetime tests remain the owning
oracles. No upstream adaptation or online research is needed for this local
contract mismatch. Native request counters and independent historical values
provide the resource/debug evidence; no payload logging is added.

## Acceptance and checklist

- Before implementation: the focused hot-key regression failed with the exact
  6000/4096-byte mismatch above.
- `cargo test --locked -p rrd-lsm --test tiered_io -- --nocapture`: three passed,
  including exact historical/current values across compaction and reopen,
  actual bytes read above 4096, peak request at most 4096 and no fallback/mmap.
- `cargo test --locked -p rrd-lsm`: 184 passed, zero failed, one
  intentionally ignored owned-child entry used by the native crash parents.
  Includes fault publication, checksums, history, ownership and native kills.
- `cargo clippy --locked -p rrd-lsm --all-targets -- -D warnings`: passed, 1.30s.
- `cargo fmt -p rrd-lsm` completed. Changed source and regression reread and
  full source diff reviewed; navigation/inventory/documentation checks are
  passed before commit. The first documentation check rejected a status line
  lacking the required active/historical classification; corrected to active.

Changed paths: `rrd-lsm/src/segment/mod.rs`, `tests/tiered_io.rs`, this journal,
POAM-035 and generated navigation/inventory. No files moved or deleted.
Stop conditions: no relaxed native cap, partial unauthenticated page exposure,
new allocation authority, changed format or unsupported qualification claim.
The exact installed follow-up, current-candidate native CI and complete
capacity/24-hour profiles are not yet run for this repair. C-07 and alpha remain
open; PR72 can merge only after its required candidate CI passes. No gate-status
change is made by this component result.
