# C-07e retained projected output admission baseline

**Status:** active bounded baseline; implementation and qualification pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-c/c-07e-output-budget-baseline`
**Owner:** OBJ-02 prerequisite, [C-07d/e](../../../roadmap/rrflow-1.0/gate-c-07.md), [POAM-035](../../../poam/rrflow-1.0-alpha/poam-035.md)

Accepted baseline is `dde6ef86d278b8c59cf1f61ebc2d3923186396d5`, tree
`bc76b62d5a500395fb8e1193dcd4d94eeac79ada`. PR 63 passed all 23 candidate
checks in CI 39822132211 before merge. PR 64's interleaved histories are in
CI; PR 65's mutable-state limit is locally qualified. Reconcile their accepted
changes before final candidate qualification.

Inspection of `segment/reader.rs` confirms per-batch and per-stream cumulative
limits, while `ProjectedReadBatch` exposes independently clonable Arrow buffers.
No aggregate output pool survives stream completion to constrain successive
streams' retained allocations. Correct source-pin release is a different
boundary. This is an observed missing bound, not a reproduced memory crash.

The preimplementation PR brief binds a native per-Database-open-instance pool,
preallocation batch reservation and immutable allocation-owned release. Its
scope is `segment/output.rs`, reader resource/build/construction, native options
and exports, focused native tests, required native CI partitions, and supporting
records/projections. No existing persistence or semantic model is replaced.
Caller copies, source pages, allocator metadata, retained memtables, independent
reopened instances and RSS remain outside this charge. Installed budget binding,
total-process limits and capacity qualification remain open.

Reviewed: README/AGENTS, authoring, C-07/POAM-035 and existing lifetime evidence;
reader batch/lease/stream/cursor/meter functions, database options/create/open/read
construction, exports, native custom mmap ownership and Arrow consumers. Exact
locked Arrow 59.2.0 mutable allocation, immutable owner, bytes/deallocation and
optional pool claim implementations were inspected. `from_len_zeroed` uses exact
layout sizes; unlike `with_capacity`, it does not round capacity to 64 bytes.
Do not infer a rounding bug from the latter API.

Primary references: [Arrow Buffer](https://docs.rs/arrow/latest/arrow/buffer/struct.Buffer.html),
[Allocation](https://docs.rs/arrow-buffer/latest/arrow_buffer/alloc/trait.Allocation.html)
and [DataFusion MemoryPool](https://docs.rs/datafusion/latest/datafusion/execution/memory_pool/trait.MemoryPool.html).
The latest docs orient the ownership decision; pinned source governs actual API
behavior. Exact-version web URLs were inaccessible. No upstream code tree or new
dependency is adopted. Reuse native custom-owner semantics at the projected reader
boundary; validate immutable pointer lifetime and last-owner drop directly.

First oracle: add the policy surface without enforcement, then require retained
successive streams to fail a tiny shared budget. Test exact byte boundaries,
key-only/key-value output, clone/slice release, all terminal paths, competing
streams and maintenance/reopen histories. No new runtime test has run at this
seed. Full results, complete changed-file reread, exact commands, failures and
remaining limitations belong to the successor immutable journal.

Preserve read traces and expose bounded native limit/current/peak/denial
observations. Stop on early release, overflow, uncharged output, source ownership
regression, expanded authority or failed CI. Regenerate navigation/inventory;
require all exact-candidate checks before merge. C-07, POAM-035 and alpha remain
open; this seed establishes no native capacity, exhaustion or soak result.
