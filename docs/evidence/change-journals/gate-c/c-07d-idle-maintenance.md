# C-07d: bounded maintenance after offered load falls

**Status:** active evidence record; native comparison and candidate CI pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/c-07d-idle-maintenance`
**Owner:** [C-07](../../../roadmap/rrflow-1.0/gate-c-07.md), OBJ-02 and POAM-035
**Pull request:** [93](https://github.com/rrflow/rrflow-development/pull/93)

Starting revision `05e0002320d14718b2b70317af95fd96cbd0d558` integrates PR92
from fetched main. The pre-edit PR brief records the scope and replacement map.
Linux10's actual failed drain retained 43 L0 debt segments after foreground
load stopped. Existing automatic compaction ran only after a write-triggered
flush or hard-cap pressure. That measured failure requires an idle progress
path; it does not authorize relaxing the zero-debt drain criterion.

## Implementation and traceability

`rrd-lsm/src/database.rs::maintenance_step` performs at most one bounded
pending reclamation or immutable compaction quantum. It never flushes the
active mutable tail. `compact_segments_inner` factors the existing publisher
out of `compact_inner`; explicit and foreground compaction retain their prior
flush behavior. There is one candidate selection, input admission, merge,
publication and reader-protection implementation. Automatic accounting is
shared with foreground maintenance. Idle compaction retains every MVCC version;
logical sequence, active WAL generation and the mutable tail remain unchanged.

`rrd-store/src/rrflow_kv.rs::try_maintenance_step` uses the existing database
mutex with nonblocking admission. A busy owner skips that offer; poisoning is
an error. `rrd-engine/src/engine/maintenance.rs` composes this physical capability
and returns no work for a volatile estate. Neither method opens another store,
creates a semantic transaction nor invokes an external capability.

`rrd-server/src/http/maintenance.rs` owns one joined thread, driven only by the
existing server sweep. At most one offer is pending or executing. Offers during
work are skipped, not queued. Drop closes admission, wakes an idle worker and
joins any admitted quantum, including when the serving future is cancelled.
Worker panic is visible to the next sweep. A failed storage quantum retains
native failure counters and emits a payload-free warning; it is not a successful
receipt. `http/server.rs::serve` now uses the same sweep as `serve_until` for
both local and TLS serving. No detached worker, new timer authority, runtime
configuration field, cap increase or logical/public operation was introduced.

The existing installed compaction/reclamation time limits are cooperative.
Joining can wait for a native filesystem call; this does not claim forceful
interruption of blocked kernel I/O. One admitted quantum still holds the native
writer mutex and can delay foreground requests. Native latency comparison is
required before claiming that this is a sustainable operating point.

## Inspection and decisions

Read README, alpha objective, C-07, POAM-035, authoring/traceability, installed
storage limits, complete replaced prepare-write/compaction/candidate-selection
and reclamation functions, rrflowKV lock/open/flush/compaction ownership,
StorageProfile, RrdEngine composition, cursor sweep, HTTP serving/shutdown,
and existing maintenance/compaction/native publication fixtures. Reviewed the
new modules, tests, complete changed functions and final diff. No external
source was adapted; the retained native failure and existing contracts provide
the research basis. Trace/resource decisions preserve native limits, counters,
publication boundaries, source identities and failed samples. Stop on changed
logical cursors, another writer, lost protected readers, unbounded queued work,
extra timer flushes, detached shutdown or relaxed native acceptance.

Changed files are the native database and maintenance tests, rrflowKV, engine
maintenance module/declaration, HTTP worker/module/serve loop, installed
lifecycle reference, this journal, POAM-035 and generated projections.
No implementation was abandoned or retained as a competing fallback.

## Evidence

Two native histories prove idle debt convergence while an acknowledged mutable
tail and historical transaction remain readable, exact reopen, and hard-budget
denial without flush/publication. Store contention proves nonblocking offers.
The engine fixture preserves claim/runtime/control cursors and reopens its
acknowledged tail. Worker histories prove skipped busy offers, no queued replay,
joined in-flight shutdown, idle wakeup on drop and observable panic.

The first multi-crate test build exposed a test-only constructor spelling
(`CanonicalId::parse` instead of `new`). After correction, the store fixture
exposed legitimate pending reclamation left by opening a store; it now permits
that single admitted quantum before checking idle. Neither correction relaxed
production admission. Focused native tests passed 2; focused engine/store/worker
tests passed 4. The owning results are:

- `cargo test --locked -p rrd-lsm -p rrd-store`: 429 passed, 2 existing ignored.
  This includes native fault/publication/reopen and compaction histories.
- `cargo test --locked -p rrd-engine -p rrd-server --lib`: 170 passed,
  2 existing ignored.
- `cargo test --locked -p rrd-server --test http_process`: 17 passed.
- `cargo clippy --locked -p rrd-lsm -p rrd-store -p rrd-engine -p rrd-server --all-targets -- -D warnings`:
  passed.
- Formatting, docs, navigation, inventory and diff checks passed before
  commit. Logs are retained locally under `target/c07d-idle-*`.

The next unchanged native Linux workload is required to measure actual debt
drain and contention. No full capacity profile or 24-hour pass is claimed.

Windows16 independently tested PR92 source
`7bf996cdcc6e92c3e28506ad5687f3cfd035432a`, binary
`97cc3bb81d6fe76b78cdaed0659f3381be02ae23a77a8113c902f24eaf68e890`, campaign
`18d9b7947a9cdadbb2cab0933666977f75dce170c91364300bb69ceb37d6c353`, harness
`510e980fc5505afb4f70f85d7b60559dbad7cf9dddf05e7b4ef804b4b0c0f91b`.
It failed after 18 segments/202358 ms on an actual 2048290100 ns resource gap
with PID 63088. Failed segment digest:
`7945ee72f1526b1a5d5e4ea64c2ff15dc6db650e9f47fed0544a3ed2358c6363`.
Cleanup is confirmed. The repaired cadence alone is insufficient for a soak
claim; no synthetic sample or gap-limit relaxation is permitted.

Checklist: pre-edit brief, measured failure, mapped shared implementation,
focused and owning checks, surfaced test corrections, final reread, linked
journal and mechanical projections are recorded. Native comparison, macOS's
missing selected I/O-failure receipt, sampling coverage, full profiles, soak
and exact candidate CI remain open. No objective/roadmap/POAM closure or
promotion change.
