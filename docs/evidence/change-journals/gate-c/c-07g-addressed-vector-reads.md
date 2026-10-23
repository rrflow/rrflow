# C-07g: authenticated named vector reads

**Status:** active evidence record; implementation complete, native qualification pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/c-07g-addressed-vector-reads`
**Owner:** [C-07](../../../roadmap/rrflow-1.0/gate-c-07.md), OBJ-02 and POAM-035
**Pull request:** [80](https://github.com/rrflow/rrflow-development/pull/80)

## Baseline, brief and traceability

Starting revision `9d7212749ed78e0ab01d17fbc04fd73553e91a2e` integrates
PR 79 (`e8171409dd088e640ee9020c867747e8fe0a3900`) over development/main.
The PR description bound this package before implementation. Version stays
1.0.0. No release or objective status changes.

`RrdEngine::retrieve_vector_points` previously read every vector version in
the estate before selecting requested references. It also filtered collection
and name before temporal identity materialization, which could expose an old
address after the identity moved to another collection. Named reads now select
the existing authenticated `RuntimeVersionedSource::Identity` ranges at one
read stamp and caller key budget. Latest valid identity selection precedes
collection/name filtering. Request order, missing identities, exact values,
retirement and receipt evidence retain their existing contracts. Scroll and
search retain their own access paths.

The replacement stays in `engine/vector/points.rs`; it consumes the C-04
identity-read contract and E-04 vector visibility implementation. No storage
format, index, dependency, public operation or parallel authority was added.
Characterization lives in `engine/tests/vector_index/point_reads.rs`, registered
by the existing test module. Existing point-read and vector-index tests remain.

Read README and its OBJ-02, C-07, Gate E and POAM-035 owners, the change-authoring
routine, store `access/versioned_read.rs` (range admission, authentication and
budgets), core logical models, contract `RetrieveVectorPoints` validation,
engine `vector.rs`/`vector/points.rs`, vector `exact.rs`, and the complete
vector-index fixture helpers and new point-read tests. Reviewed the complete
production diff and both new histories. The public request admits at most
4,096 unique references; the existing source planner has no conflicting lower
source-count cap and rejects overlapping ranges. Exact changed paths remain
mechanically inventoried by repository tooling.

Research decision: reuse the already implemented authenticated identity path;
no upstream algorithm adaptation or external research is needed for this
repair. Debug/resource evidence consists of public key/value counters and
bounded failures, with no payload trace or private model reasoning. Stop
conditions: schema/index redesign, weakened authorization or temporal
semantics, hidden failed qualification, or unsupported C-07 completion.

## Native motivating failure

The Linux scale04 run at `79663cdd4d162379c82a2fab0ff3f87d5cbf907e`
used 1,000 records, 5,000 edges and 1,000 vectors of 512 dimensions under
four CPUs and a 2 GiB process allocation. It acknowledged all 1,000 setup
inserts but verified only 936 before the frozen 1,800,000 ms setup deadline.
The run failed after 1,801.89 seconds and confirmed cleanup. Retained evidence
is in `target/c07-linux-scale-evidence-04` and its launcher log. This establishes
a failed setup envelope, not throughput. The family scan was identified in
source; this package does not claim it accounts for all elapsed setup time.
Relation filtering and whole-scope write constraints require their own measured
evidence. The same native corpus must be rerun after combined integration.

## Commands, failures and results

Commands use dedicated `target/c07g-addressed-build`.

| Command | Result |
|---|---|
| `cargo test --locked -p rrd-engine --lib point_reads` before repair | Both regressions failed: unrelated growth exceeded the 128-key budget at key 129; collection move exposed its old address |
| Same focused command after repair | Two passed, 0.45 s |
| `cargo test --locked -p rrd-engine --lib` | 160 passed, two existing ignored, 24.06 s test execution |
| `cargo clippy --locked -p rrd-engine --all-targets -- -D warnings` | Passed, 40.62 s |
| `cargo fmt --all` | Passed |

The first fixture compile used correlation identifiers where canonical IDs
were required; corrected the fixture types. The first repaired test run caught
an incorrect expected source cursor after retirement. Retirement adds an
authoritative version even before its effective time; the oracle now derives
that cursor from the prior committed head plus the known retirement. No
production semantics or admission bounds were weakened to satisfy the test.

The first history adds 96 unrelated vector identities, then proves unchanged
requested points and vector-family examined/decoded counts under a fixed
128-key budget. It covers update, historical reads, retirement, exact reopen,
missing identities and explicit exhausted-budget classification. The second
history moves one identity between collections and proves both historical
visibility and absence at the old address, including persistent reopen.

Logs: `target/c07g-addressed-red-02.log`, `c07g-addressed-green.log`,
`c07g-addressed-green-02.log`, `c07g-addressed-suite.log` and
`c07g-addressed-clippy.log`. Failed intermediate results remain retained.

## Checklist and remaining acceptance

- Bound one outcome/package, source baseline, exact scope and failure oracle
  in the pre-implementation PR brief.
- Mapped replaced family scanning to canonical identity ranges and retained
  characterization; no modules were moved or deleted.
- Implemented before broad verification; ran focused regressions first, then
  the owning engine suite and strict lint.
- Recorded failures, resource/debug/research decisions, remaining limits and
  the POAM-035 follow-up. Navigation and execution inventory are regenerated
  with this journal; documentation and diff checks precede commit.
- Candidate CI and ordinary PR merge remain required. Installed native larger
  corpus, full capacity profiles and a completed 24-hour soak have not run for
  this source. They cannot be inferred from these component tests.

C-07, OBJ-02 and POAM-035 remain open. No competitive, latency, throughput,
full-alpha or full-profile qualification is claimed.
