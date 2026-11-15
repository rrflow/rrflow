# POAM-040 — background jobs and memory receipts are not observable

**Status:** active parent-owned POA&M detail
**Coordinate:** `rrflow://rrflow-instance/data/poam/rrflow-1.0-alpha/poam-040`
**Owner:** [RRFlow 1.0 alpha POA&M](../rrflow-1.0-alpha.md)

The parent ledger owns priority and lifecycle status. This detail records a
verified gap against [OBJ-14](../../objectives/rrflow-1.0-alpha.md#required-outcomes);
it cannot close a roadmap gate independently.

## Observed deficiency

The [installed-job discovery candidate](../../evidence/change-journals/gate-d/d-02-installed-job-discovery.md)
adds a bounded authenticated public read over the installed initial job and
committed successors. It is a prerequisite for client recovery without derived
IDs, not completion of job journals, memory receipts, current-source verification
or Connectome workflow acceptance. The parent gap remains open.

The [discovery package on accepted main](../../evidence/change-journals/gate-d/d-02-job-discovery-on-main.md)
adapts the existing bounded installed-job list independently of unmerged worker
and task-budget ancestors. It remains a candidate for public initial/successor
discovery, with no current-source verification, job-journal, memory-receipt or
Connectome acceptance claim. This gap remains open.

The [draft replan receipt readback](../../evidence/change-journals/gate-d/d-02-replan-receipt-readback.md)
adds principal-scoped embedded inspection of a committed successor receipt
without replaying acceptance, and validates its lineage on both read and replay.
It does not expose a public operation or close job-journal, memory-receipt,
source-snapshot or candidate-CI gaps. This POA&M remains open.

The [embedded attunement receipt readback candidate](../../evidence/change-journals/gate-d/d-02-attunement-receipt-readback.md)
adds an authenticated, principal-scoped read of existing control receipts
without replaying a mutation. This is not a public HTTP/SDK operation, source
freshness check, memory receipt, or completion of this gap; exact candidate CI
and the public readback and installed-process requirements remain open.

Verified at development `main` `04fae276`:

- The operation catalogue (`endpoint_catalogue` in
  [`rrd-contract`](../../../crates/transport/rrd-contract/src/lib.rs)) advertises
  no attunement, job, routine, backup-job, function-receipt or control-journal
  operation. `read_attunement` and `control_attunement` are embedded Rust calls
  only, and the CLI offers no job or receipt inspection.
- The public audit read keeps only `security.audit` entries from the control
  journal (`audit_since` in [`rrd-security`](../../../crates/authority/rrd-security/src/lib.rs)),
  so attunement and engram control transitions cannot be read through it.
- `CommitReceipt` carries no principal or read stamp and has no read-by-id
  operation; it is returned again only on a retried commit.
- A memory read leaves no durable receipt: `context-assemble` packets are not
  persisted, and prompt flights, the closest durable record, carry no principal
  ([POAM-032](poam-032.md)).
- The `rrflow.attunement.job` span is declared in the trace vocabulary and the
  generated signal catalogue, and nothing emits it.
- [Connectome](../../reference/client/connectome.md) requires lifecycle state to
  come from public engine job records, which do not exist.

[POAM-008](poam-008.md) owns the generic effect-receipt identity and
[POAM-030](poam-030.md) the bounded audit-read defect; this record covers
observability.

## Impact

Neither the owner nor a client can see what background work is running, what it
bound, or how it ended, and nothing proves which memory was written or read on
whose authority. Job and memory claims therefore rest on internal state that
only the engine's own tests inspect.

## Owner decision

On 2026-10-26 the repository owner decided that the missing planned job
operations may be added: job list, read and status, resume and cancel, receipt
read and a job-transition journal read enter the operation catalogue under
existing grants. This answers the D-02a admission question for job control
recorded in [POAM-006](poam-006.md). Global configuration operations remain
undecided.

## Owning gates

D-02a/D-02b job operations; H-01 avenue evidence, H-05 signals and H-06
lenses; I-01 events and I-03 routines.

Roadmap owners: [Gate D](../../roadmap/rrflow-1.0/gate-d.md#project-workflow-build-packages),
[Gate H](../../roadmap/rrflow-1.0/gate-h.md) and
[Gate I](../../roadmap/rrflow-1.0/gate-i.md).

## Closure evidence

Catalogue-generated operations, with dispatch, OpenAPI, SDK and CLI projections
from the one catalogue, list, read and control jobs under existing grants. Every
job transition and every memory write and read leaves a durable receipt that
binds the actor, authorization, plan, bound sources, budget and outcome, and is
readable by identity after restart. Connectome shows job state and receipts
only from those operations. Tests read each receipt after killing and
restarting the shipped `rrflow`, at an exact revision.
