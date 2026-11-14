# Split the kernel's trace contract by concept

**Status:** active change evidence

**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/repository/core-trace-split-20261113`

**Owner:** rules R1 and R9 of the code structure and conventions standard (kernel package K13)

**Starting revision:** `development/main` at `501a16bc`.

**Change.** `crates/kernel/rrd-core/src/trace.rs` goes from 2,537 lines to 31. Its items move unchanged, by exact syn spans, into `trace/`:

| Module | Contents | Lines |
|---|---|---|
| `ids` | contract constants and trace and span identifiers | 63 |
| `vocabulary` | phases, outcomes, data classes, boundaries, operations and attributes | 539 |
| `event` | causal links and `RuntimeTraceEvent` | 454 |
| `validation` | identity, link and value validation and canonical names | 436 |
| `convergence` | the direct-convergence vocabulary and its validation | 408 |
| `tests` | the inline test module, moved per R9 | 632 |

Private items become `pub(super)`. No other item needed raising.

**Characterization (unchanged behaviour):**
- `cargo test -p rrd-core`: 98 passed.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` and `cargo fmt --all -- --check`: clean.
- `cargo test -p rrd-engine --test workspace_architecture --test direct_read_source_closure`: 34 and 1 passed.
- `build_execution_inventory.py --check`: OK.
