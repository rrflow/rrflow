# Split rrd-vector's hnsw.rs by stage

**Status:** active change evidence

**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/repository/vector-hnsw-split-20261113`

**Owner:** rules R1 and R9 of the code structure and conventions standard (kernel package K13)

**Starting revision:** `development/main` at `501a16bc`.

**Change.** `crates/compute/rrd-vector/src/hnsw.rs` goes from 2,291 lines to 28. Its items move unchanged, by exact syn spans, into `hnsw/`:

| Module | Contents | Lines |
|---|---|---|
| `config` | format constants, configuration, descriptors and the persisted body | 207 |
| `index` | `HnswIndex`: build, advance, load, search and cost | 535 |
| `graph` | insertion, neighbour selection, pruning, reachability repair and layer search | 362 |
| `score` | dense scoring with NEON, AVX2 and a portable fallback | 269 |
| `validate` | deterministic levels, graph validation and encoding | 94 |
| `tests` | the inline test module, moved per R9 | 822 |

Three changes in visibility only:
- `SearchLayerOptions`' fields become `pub(super)`.
- The test-only `IDENTITIES_VISITED` thread-local becomes `pub(super)`.
- Private functions become `pub(super)`.

**Characterization (unchanged behaviour):**
- `cargo test -p rrd-vector`: 105 passed, the same as on main.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` and `cargo fmt --all -- --check`: clean.
- Architecture and read-closure tests: 34 and 1 passed.
- `build_execution_inventory.py --check`: OK.
