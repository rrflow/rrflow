# Kernel contract: CLD as a columnar copy, borrow or build, rrflowMX into rrflowKV

**Status:** active planning evidence; no runtime behavior changed

**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/repository/kernel-decisions-20261114`

**Owner:** the [RRFlow kernel contract](../../../roadmap/kernel-contract.md)

**Starting revision:** `development/main` at `b0cbc4df` (#386 merged).

**Owner decisions, 2026-11-14.** The owner asked whether KV and MX are truly separate, whether the engine attempts too much, and whether LSM and CLD were understood. Three options were then proposed, and the owner returned them as answers:
1. CLD is a columnar copy first, not a demotion tier.
2. A borrow-or-build rule applies to each layer, starting with the vector index.
3. rrflowMX folds into rrflowKV, and its model stays as a test-only reference.

**Method:**
- **Code reading on `development/main`:**
  - `RrflowMxStore` (`rrd-store/src/engine.rs:216-566`) reimplements the per-key MVCC and first-writer conflict check that `rrd-lsm/src/transaction.rs` provides.
  - `RrdEngine::rrflow_mx` has no production caller, and installation refuses it (`installation/profile.rs:277`).
  - 53 test files use `RrflowMxStore::new()` as a reference oracle.
- **Research:**
  - TiDB's row store plus columnar replica, chosen per query at one timestamp (VLDB 2020).
  - SurrealDB's interchangeable storage engines, with SurrealMX as its in-memory engine.
  - turbopuffer's memory, NVMe and object-storage tiers (vendor and second-hand figures).
  - Stonebraker and Pavlo, "What Goes Around Comes Around… And Around" (SIGMOD Record, 2024), on vector search being absorbed by general systems through libraries.

**Changed paths:**
- `docs/roadmap/kernel-contract.md`:
  - S4 rows for LSM and CLD;
  - S9's decision paragraph, tier table and copy job;
  - new section 4.2 (borrow or build);
  - the LFG/SWAG proof renumbered to 4.3;
  - section 5, item 10 (rrflowMX);
  - the K13 row;
  - one open decision restated.
- `docs/architecture/system-overview.md`: the rrflowCLD and rrflowMX terminology rows.
- This record.

**Checks:** `render_navigation.py` and `check_documentation.py`.

**Not established:**
- No package is implemented.
- None of the floor checks has been run. The only measured gap is vectors, at 15.3 against 567 queries/s (`eval/lab/evidence/arm-20k-20261106/report.json`).
- The CLD copy lag and the row-or-column threshold are unmeasured.
