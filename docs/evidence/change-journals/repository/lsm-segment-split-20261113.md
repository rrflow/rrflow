# Split rrflowLSM's segment module and retire its mod.rs

**Status:** active change evidence

**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/repository/lsm-segment-split-20261113`

**Owner:** rules R1, R3 and R9 of the code structure and conventions standard (kernel package K13)

**Starting revision:** `development/main` at `501a16bc`.

**Change.** `crates/persistence/rrd-lsm/src/segment/mod.rs` (2,802 lines) becomes the file-named module `segment.rs` (256 lines). Its items, and the 858-line `impl Segment` method by method, move by exact syn spans into child modules:

| Module | Contents | Lines |
|---|---|---|
| `evidence` | segment open evidence | 281 |
| `page_cache` | policy, regions, admission, eviction and statistics | 596 |
| `page` | page sources, reads, verification and decompression | 294 |
| `decode` | column decoding and whole-file verification | 183 |
| `write` | writing segments | 174 |
| `open` | opening, validation and install | 290 |
| `read` | point, version, range and cursor reads | 421 |
| `tests` | the inline test module, moved per R9 | 380 |

Fields and methods used across the new modules become `pub(super)`. They were found by compiling with all targets and all features, and only the items the compiler reported were raised.

The storage-closure architecture rule and the execution inventory now name `segment.rs`.

**Characterization (unchanged behaviour):**
- `cargo test -p rrd-lsm`: 253 passed.
- `cargo test -p rrd-lsm --all-features`, including the physical-policy lab: 260 passed.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` and `cargo fmt --all -- --check`: clean.
- `cargo test -p rrd-engine --test workspace_architecture --test direct_read_source_closure`: 34 and 1 passed.
- `test_execution_inventory.py` and `build_execution_inventory.py --check`: OK.

**Not done here:**
- `segment/physical_policy_lab.rs` (2,026 lines) should move to the Lab (R11).
- `segment/reader.rs` (1,840) and `segment/format.rs` (1,513) remain on the ratchet list.
