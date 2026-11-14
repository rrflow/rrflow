# Split the kernel's runtime.rs by concept

**Status:** active change evidence

**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/repository/core-runtime-split-20261113`

**Owner:** rules R1 and R9 of the code structure and conventions standard (kernel package K13)

**Starting revision:** `development/main` at `501a16bc`.

**Change.** `crates/kernel/rrd-core/src/runtime.rs` goes from 2,825 lines to 40. Its items move unchanged, by exact syn spans, into `runtime/`:

| Module | Contents |
|---|---|
| `ids` | identifiers and record references |
| `model` | values, records, relations, events, mutations and commits |
| `stamps` | read stamps, snapshot handles and retention pins |
| `transaction` | data transactions and views |
| `projection` | projection state, families and work |
| `audit` | audit decisions and envelopes |
| `data_snapshot` | read validation, change pages and the data reducer |
| `graph` | the graph reducer and graph diffs |
| `validate` | text, digest, window and property validation |
| `encoding` | canonical encoding |
| `tests` | the 324-line inline test module, moved per R9 |

Private items become `pub(super)`.

**Rules that read source text** now read the whole module wherever it is split:
- `workspace_architecture.rs` gains `module_source`, which returns a file plus its child folder, or a crate root plus every source file of its crate.
- The `direct_read_source_closure` allowlist points the two kernel reducer self-tests at `runtime/tests.rs`.

**Characterization (unchanged behaviour):**
- `cargo test -p rrd-core`: 98 passed, the same as on main.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: clean.
- `cargo test -p rrd-engine --test workspace_architecture --test direct_read_source_closure`: 34 and 1 passed.
- `build_execution_inventory.py --check` and `test_execution_inventory.py`: OK.
