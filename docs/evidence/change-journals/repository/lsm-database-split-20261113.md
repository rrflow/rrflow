# Split rrflowLSM's database.rs by responsibility

**Status:** active change evidence

**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/repository/lsm-database-split-20261113`

**Owner:** rule R1 of the code structure and conventions standard (kernel package K13)

**Starting revision:** `development/main` at `501a16bc`.

**Change.** `crates/persistence/rrd-lsm/src/database.rs` goes from 3,056 lines to 454. Its top-level items and the 2,333-line `impl Database`, method by method using exact syn spans, move into `database/`:

| Module | Contents | Lines |
|---|---|---|
| `options` | options, maintenance and compaction policies, and the reports they produce | 126 |
| `failpoints` | failure modes, write, flush, compaction and snapshot-install boundaries, and injection | 162 |
| `open` | `create*` and `open*`, layout validation, and the manifest and WAL path | 356 |
| `transactions` | projected reads, transactions and the single-writer write path | 619 |
| `flush` | flush, sync and named checkpoints | 154 |
| `snapshot` | snapshot bundle export and install | 269 |
| `compaction` | candidate selection, merge and reclamation | 623 |
| `gc` | garbage collection | 184 |
| `reads` | get, get_many, scan, scan_ranges, scan_each | 201 |

Accessors and the `Database` struct stay in `database.rs`. Moved text is unchanged except for visibility: private items and methods become `pub(super)`, so sibling modules that import the parent keep resolving them.

**Characterization (unchanged behaviour):**
- `cargo test -p rrd-lsm`: 253 passed, 0 failed.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: clean.
- `cargo fmt --all -- --check`: clean.
- `cargo test -p rrd-engine --test workspace_architecture`: 34 passed.
