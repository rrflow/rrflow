# POAM-081 — task and flight lists read every version before paging

**Status:** active parent-owned POA&M detail
**Coordinate:** `rrflow://rrflow-instance/data/poam/rrflow-1.0-alpha/poam-081`
**Owner:** [RRFlow 1.0 alpha POA&M](../rrflow-1.0-alpha.md)

This detail is governed by the parent ledger; it cannot independently change
priority or closure status.

## Observed deficiency

An independent review on 2026-10-26 found this, and it was never recorded. The review was archived uncommitted in the worktree `c07-independent-review` and absorbed on 2026-11-14 ([ledger](../../evidence/change-journals/repository/archive-absorption-20261114.md)). A code read of `main` at `2b14c3ff` confirms it still holds:

- **`task-list`** (`crates/authority/rrd-engine/src/engine/task.rs:245-251`) calls `read_versioned` for every version of every record of the task kind. It then groups them by record, drops those at or before `after_cursor`, sorts, and truncates to `limit` (`task.rs:252-270`).
- **`flight-list`** (`crates/authority/rrd-engine/src/engine/flight/launch.rs:317-323`) does the same for the prompt-flight kind.

The page cursor and page size therefore do not bound the read. Listing the newest ten tasks reads the history of all of them. The only bound is `max_storage_keys`, which refuses the request once the kind grows past it; it does not make the read selective.

## Impact

A project that has run for a while cannot list its tasks or prompt flights cheaply, and past the key budget it cannot list them at all. Connectome's Tasks, Runs, Prompt flights and Causal traces views all page through these two operations.

## Owning gates

E-02 owns persisted scalar indexes; these two lists need a cursor-ordered index on their record kind. The kernel contract's planner package asks for keys examined in proportion to the result, and these lists bypass the planner.

Roadmap owner: [Gate E](../../roadmap/rrflow-1.0/gate-e.md).

## Closure evidence

Closed when all of the following hold:
- `task-list` and `flight-list` read through a cursor-ordered path, and their `read_evidence` shows keys examined proportional to the page, not to the kind.
- A test with at least 10,000 tasks lists a page of ten within the default key budget, and returns the same page as the full read does.
- A second test proves the same for `flight-list`.
