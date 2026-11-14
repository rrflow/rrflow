# Split rrd-store's backup.rs by stage

**Status:** active change evidence

**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/repository/store-backup-split-20261113`

**Owner:** rules R1 and R9 of the code structure and conventions standard (kernel package K13)

**Starting revision:** `development/main` at `501a16bc`.

**Change.** `crates/persistence/rrd-store/src/backup.rs` goes from 2,468 lines to a module root. Its items move unchanged, by exact syn spans, into `backup/`:
- `types`;
- `create`;
- `catalogue` (load, verify, prune);
- `restore`;
- `control_journal`;
- `retention`;
- `catalogues` (manifest capture, validation and restore);
- `object_payloads`;
- `persistence` (catalogue file and prune validation);
- `validation`;
- `tests`, the inline test module moved per R9.

The largest module is 332 lines. The existing `capture_tests` module is unchanged.

Twelve private items used across the new modules become `pub(super)`; only those the compiler reported were raised. The `direct_read_source_closure` allowlist names `backup/retention.rs`, where the backup object-reference scan now lives.

**Characterization (unchanged behaviour):**
- `cargo test -p rrd-store`: 274 passed.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` and `cargo fmt --all -- --check`: clean.
- `cargo test -p rrd-engine --test workspace_architecture --test direct_read_source_closure`: 34 and 1 passed.
- `build_execution_inventory.py --check`: OK.
