# F-06: cold history hardening recovered from the archived worktree

**Status:** active implementation evidence; F-06 remains open

**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-f/f-06-cold-history-hardening-20261114`

**Owner:** F-06 in [Gate F](../../../roadmap/rrflow-1.0/gate-f.md); kernel schema S9 (memory tiers)

**Starting revision:** `main` at `84c50a3f4`.

**Where this came from.** The archived F-06 worktree, `wip/f06-main-integration-20261107`. `main` took its committed head on 2026-11-09 (`051921e2d`). A further round of hardening sat uncommitted in that worktree and never reached `main`. It is ported item by item, by hand; the [ledger](../repository/archive-absorption-20261114.md) tracks which items are done.

This record is extended as each item lands.

## 1. A timeout keeps its identity (landed)

**Defect on `main`.** Cold encode, decode and Parquet query export decided after the fact whether a failure was a timeout: if the error was a budget error and the clock had passed the deadline, it was reported as a deadline (`engine/cold_history/files.rs`, `cold_query_error`; `engine/query_export.rs`). The query crate reported its own timeouts as budget errors. So:

- a memory or byte-limit failure that surfaced after the deadline was reported as a timeout;
- a real timeout inside the decoder was reported as a configuration limit.

A planner that will choose between the cold copy and the hot store (S9) has to tell "ran out of time" from "over budget" to fall back correctly.

**Change:**

- `rrd-query`: `Error::DeadlineExceeded`. The cold decoder returns `ColdHistoryReadError { DeadlineExceeded, Query }` and checks its clock at entry, per batch, per row and before returning. The Parquet sink records its first failure (deadline, byte limit or allocation) beside the writer, because the Parquet footer writer turns sink errors into strings.
- `rrd-engine`: `rrd_query::Error::DeadlineExceeded` maps to `ServiceError::DeadlineExceeded`; the two clock-after-failure heuristics are removed, in cold history and in ordinary Parquet query export; a query trace classifies it as `deadline`, outcome denied.

**Public effect.** Error classification changes for clients: a timeout that used to read as a configuration limit now reads as a deadline, and a limit hit after the deadline now reads as a limit.

## 2. The artifact is read in bounded requests (landed)

**Before.** A cold generation's artifact was read with one `read_to_end`. **After.** `read_cold_artifact` reads at most 64 KiB per request, rechecks the deadline between requests, retries an interrupted read, and enforces the exact length both ways. One blocking filesystem read can still exceed the deadline; the function's comment says so.

## Oracles for items 1 and 2

- Query crate: `cold_json_preserves_deadline_and_resource_identity`, `empty_artifact_cannot_finish_after_deadline`, `buffered_parquet_footer_preserves_sink_timeout`, `buffered_parquet_footer_preserves_sink_byte_limit`, `writer_and_json_preserve_timeout_and_resource_identity`.
- Engine: `encoder_errors_keep_their_cause_after_deadline`, `decoder_deadline_and_resource_failures_are_distinct` (a resource failure stays a configuration limit even after its deadline has passed), `artifact_read_checks_deadline_and_exact_length` (short, long, chunk size, interrupted and slow readers).
- These tests could not run against `main` before the change: they use the new error types. The defect was established by reading `cold_query_error` and the query crate's error sites, not by a failing run.

**Traceability.** The query crate's `error.rs` and `cold_history.rs` had not changed on `main` since the archived head, so `main` plus the reviewed delta equals the archive's files. `parquet.rs`, `lib.rs`, `engine/error.rs` and `engine/query_export.rs` took the delta as a patch that applied cleanly. The engine's cold history file was split on `main`, so its delta was written by hand into `cold_history/files.rs` and `cold_history/maintain.rs`, and the runtime query delta into `runtime/query/execution.rs` and `attributes.rs`. Every delta was read in full first.

**Commands and results, Windows, 2026-11-14:**

- `cargo check --workspace --all-targets --locked`: passed.
- `cargo test -p rrd-query --locked`: 87 passed, 0 failed.
- `cargo test -p rrd-engine --lib --locked`: 484 passed, 0 failed, 13 ignored.
- `cargo test -p rrd-server --locked`: 37 passed.
- `cargo clippy -p rrd-query -p rrd-engine --all-targets --locked -- -D warnings`: passed.

## 3. Native upkeep cannot starve the cold copy (landed)

**Defect on `main`.** Two layers offered native maintenance first and cold history only when native had nothing to do:

- `RrdEngine::maintain_storage` tried native, and returned if it did work.
- The server's maintenance worker held one pending slot and dropped every offer made while a quantum ran.

A store with standing compaction debt therefore never advanced its cold copy. Under S9 the planner may read the cold copy only when its watermark covers the read, so a copy that falls behind whenever native work exists is a copy that is rarely usable.

**Reproduced.** `embedded_maintenance_publishes_cold_history_while_native_debt_remains` keeps compaction debt above zero and calls `maintain_storage`. On `main` it failed with "cold work must be offered while native debt remains" (committed failing as `d9c80c921` before the fix).

**Change:**

- `rrd-engine/src/engine/maintenance.rs`, `core.rs`: when cold history is configured, the preferred kind alternates between calls; if it has no work the other is tried once.
- `rrd-server/src/http/maintenance.rs`: one pending flag per kind; repeated offers coalesce; when both are pending the worker alternates after each started quantum.
- `docs/reference/storage/tiered-persistence.md` describes the scheduling.

**Oracles:** the engine test above, now passing, with a generation published while debt remains; `pending_kinds_alternate_and_duplicate_offers_stay_bounded` and `idle_maintenance_coalesces_offers_and_joins_inflight_work` for the worker; `publication_waits_for_a_foreground_transaction` still passes, so alternation did not break the transaction-gate wait.

**Limits.** This is ordering, not a wall-clock guarantee: a long native quantum still delays the cold one behind it.

## Not established

- Not run on Linux or macOS here.
- No test drives a real timeout through the installed binary and reads the error over HTTP.

**Research decision:** none. **Trace decision:** query traces gain the class `deadline`. **Roadmap/POA&M change:** none for these two items.
