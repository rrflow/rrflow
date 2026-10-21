# C-07 execution-plan authoring

**Status:** active planning evidence; implementation and qualification remain open
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-c/c-07-execution-plan`
**Owner:** OBJ-02 prerequisite, C-07, POAM-035

Starting revision `b6052feef8df2aa86e5a866ff0c8da196bc8f2ab`, tree
`64e50acd608bc9a400808a25e9fad4d4894205da`.
[PR 54](https://github.com/rrflow/rrflow-development/pull/54) carried the change
brief before roadmap/POA&M edits. The factual
[baseline journal](c-07-planning-baseline.md) was the minimum ahead commit
needed to open that draft; it records the baseline tests and GitHub sequence.
That committed journal is unchanged.

## Change brief and scope

Current: C-07 has an acceptance row and useful component/installed evidence,
but no ordered implementation packages or joined growth/failure matrix.
Target: one [roadmap-owned C-07 chapter](../../../roadmap/rrflow-1.0/gate-c-07.md)
with eight ordered packages, falsifying tests, explicit resource and
durability semantics, native qualification, inherited J-04 workload targets,
and downstream integration/dogfooding handoffs. This is the user's requested
planning deliverable, not a claim that C-07 runtime work is implemented.

Created: `docs/roadmap/rrflow-1.0/gate-c-07.md`,
`docs/poam/rrflow-1.0-alpha/poam-035.md`, this journal, and the baseline journal.
Changed: Gate C's plan link, dependency-spine link/first-package pointer,
POA&M parent row, generated roadmap/POA&M/Gate C journal indexes, and generated
file inventory. No source, test, workflow, dependency, storage-format, public
operation, version, runtime configuration or implementation placement changed.
No implementation was deleted, moved or replaced. The existing catalogue,
configuration, evaluation and installed-harness boundaries remain owners.

The complete new chapter and POAM-035 were reread, and all handwritten link/
ledger diffs reviewed. Full existing document reads include Gate C, POA&M
parent/governance and 002/025/026/033/034, alpha objectives, knowledge-connectors,
change-authoring, implementation-navigation and Gate E. README product/status,
the dependency spine, architecture scale rationale, J-04 capacity/measurement
sections and evaluation runner were reviewed at their relevant sections.
The baseline journal records exact full fixture reads and targeted source
symbols; large production files are not falsely represented as full reads.

## Research, trace and resource decisions

The chapter retains primary-source links and specific adopted/deferred choices
for PostgreSQL, RocksDB, SQLite, Microsoft, Arrow/Parquet, Qdrant, HydraDB,
Pinecone Nexus and TurboQuant. No source is adapted. No vendor credential,
runtime, service or model was activated. The unavailable HydraDB documentation
index is explicitly recorded, and an available introduction cannot prove
an admitted API. Research and vendor performance claims are not RRFlow evidence.

Architecture-skill tradeoff analysis was applied within the existing roadmap
owner: synchronous maintenance first, measured bounded concurrency/group
commit only when justified, existing format/cache retained, and explicit
retention/resource admission. No parallel ADR/status authority was introduced.
Future work must extend the existing trace/evaluation owners, with finite
resource/diagnostic budgets and no raw content or private reasoning in metrics.
No runtime instrumentation was added by this documentation package.

## Oracles, verification and surfaced failures

The requirement coverage review maps WAL/manifest/checksum failures to b/c/f;
maintenance/backpressure to d/g; pinned snapshots/Arrow-buffer ownership to
e/g; real capacity errors to f; and acknowledged-write/reopen proof to c/f/g.
h reconciles all required results. a establishes the shared measurement and
policy-validation prerequisite without claiming the broader J-04 work complete.
All eight packages remain unaccepted. The native matrix distinguishes I/O
simulation, process kill, real disk exhaustion and later OS/power-loss proof.

Review tightened the capacity oracle: after space is restored, capacity-only
faults must reopen with exact acknowledged history and accept valid work;
permanent denial alone cannot pass. Deliberate authoritative corruption has a
different fail-closed oracle. The policy table covers batch/queue/maintenance/
pin/recovery/process limits with units and frozen pre-run thresholds.
J-04's hardware/SLA proposals are linked rather than copied into a second table.

Executed baseline commands/results are in the baseline journal: 15 focused
tests passed and full `cargo test -p rrd-lsm` exited 0 on Windows; the Linux-only
RSS target executed zero cases. No new runtime tests were authored or claimed.

Initial documentation checks rejected POAM-035's combined heading and missing
direct Gate C owner link. Separate required `Owning gates`/`Closure evidence`
headings and the direct owner link fixed those failures. The repeated checker
passed with 380 document statuses and 378 classified coordinates before this
final journal was added. `git diff --check` passed. Final navigation/inventory/
documentation checks are required after this journal and recorded below before
commit; full candidate CI still gates merge.

Not run: native exhaustion or power-cut experiments, new installed fault/scale
drivers, 24-hour soak, competitor/vendor/model/UI qualification, or release
assembly. None exists as new evidence in this plan. No new runtime error was
reproduced beyond the documented coverage/resource gaps. POAM-034 and the
remaining C-04/E/F/G/H/I/J work keep their existing owners and status.

Checklist: owner/baseline inspection; complete fixture/targeted code evidence;
PR brief before owner effects; bounded exact documentation scope; no source
replacement; research provenance; trace/resource decisions; falsifying oracle;
executed versus unrun checks; surfaced failures/corrections; full new-file and
diff review; generated projections; immutable linked journals; candidate CI
required. Only POAM-035 is added as Open. No gate, objective or alpha maturity
is advanced by writing the plan. C-07a is the next implementation package.

Final local verification: `python -X utf8 scripts/knowledge/render_navigation.py`
and `--check` passed (17 indexes, 380 nodes, 1,658 edges);
`python -X utf8 scripts/ci/build_execution_inventory.py` and `--check` passed
(1,341 records); `python -X utf8 scripts/ci/check_documentation.py` passed
(381 statuses, 379 coordinates); `git diff --check` passed. Inventory is
regenerated once more after recording these results. The final PR candidate's
CI/run/merge identity belongs to PR 54; no future result is claimed here.
