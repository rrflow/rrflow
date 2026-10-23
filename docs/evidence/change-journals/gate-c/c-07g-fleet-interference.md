# C-07g independent installed estate interference

**Status:** active implementation; native acceptance pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-c/c-07g-fleet-interference`
**Owner:** OBJ-02 / [C-07g](../../../roadmap/rrflow-1.0/gate-c-07.md) / POAM-035

[PR 75](https://github.com/rrflow/rrflow-development/pull/75) contains the
pre-implementation change brief. Starting revision:
`893c8e764719e9202b39d8e8e55c71bf1507578f`. Dependencies 71?74 remain
candidates. Product version stays 1.0.0; C-07 remains open.

## Change brief and implementation trace

Current single-estate sustained execution cannot qualify independent estate
interference. The target extends the existing installed Workload, PointRun,
segmented history and resource verification to quiet, equal and abusive
neighbour trials under the one J-04 fleet envelope. No engine scheduler,
context-sharing policy or distributed database is introduced.

The first implemented boundary is a two-estate ordinary-binary correctness
case. Each installed estate has independent process, session and differently
seeded identical record/relation/vector identities. Four concurrent requests
per estate exercise mixed writes, point reads, range reads and cancellation.
Four write generations insert, update, retire and recreate the working record.
Canonical Workload verification runs independently after acknowledgement.
A constant-size hash chain binds original seed receipts; kill/reopen performs
readback before retries and requires exact original receipts. Foreign sessions
must receive typed authorization denial before load, after load and after
restart; transport errors cannot count as isolation. Retained physical readers
cross the concurrent writes and are released using the canonical ownership
oracle. This is characterization, explicitly not fleet capacity qualification.

Read README and relevant C-07/J-04 owners, current sustained source, Workload,
PointRun, corpus/lifecycle helpers, native allocation verification/launcher,
capacity validation, progress and campaign schema, existing cursor journal,
CI installed native steps and workflow-policy command list. Reused their
boundaries directly. Changed sustained/fleet.rs, its parent module, required
native CI command and its mechanical policy check. New work does not replace
an engine implementation; current single-estate paths remain canonical.

Research: reuse established independent-history and native owned-allocation
contracts; no upstream code imported or new performance algorithm claimed.
Trace output contains bounded coordinates, receipts and ownership counters,
not secrets or raw content. Stop on cross-estate acceptance, receipt drift,
partial semantic effect, lost acknowledgement, failed native membership,
exceeded bounds or candidate CI failure.

## Authoring evidence

`cargo clippy -p rrflow-cli --test installed_reopen --locked -- -D warnings`
passed (19.30s) after adding the two-estate implementation and regression
entry. No runtime suite, installed workload, resource controller, disk fill or
soak ran locally. Candidate native CI must execute this new ignored entry on
Linux, macOS and Windows. Full ten-estate trial coordinator, aggregate
reducer and native qualification remain in progress.

## Auditable change checklist

- [x] Bind one objective, package, pre-implementation PR brief and starting SHA.
- [x] Inspect existing implementation, tests, worktree and dependency diff.
- [x] Preserve canonical workload, transaction, receipt and authorization owners.
- [x] Record scope, source traceability, research, trace and stop decisions.
- [x] Implement independent installed-estate correctness and compile its oracle.
- [ ] Complete fleet coordinator and bounded evidence projection/reduction.
- [ ] Record final static/generated checks and exact candidate runtime results.
- [ ] Qualify full native fleet profile and 24-hour soak; reconcile C-07h.

No gate, objective or POA&M item closes on compilation or this case alone.
