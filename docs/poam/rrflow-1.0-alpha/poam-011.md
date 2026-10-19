# POAM-011 — public surfaces, SDKs, and Connectome

**Status:** active parent-owned POA&M detail
**Coordinate:** `rrflow://rrflow-instance/data/poam/rrflow-1.0-alpha/poam-011`
**Owner:** [RRFlow 1.0 alpha POA&M](../rrflow-1.0-alpha.md)

This detail record is part of the canonical RRFlow 1.0 alpha POA&M. The parent
ledger owns priority and lifecycle status; this file cannot change either
independently.

## Observed deficiency

The executable public contract, six language SDKs, CLI, MCP,
GraphQL carriage, mesh resolution, and Connectome have not passed one
D-01-installed, real-process conformance corpus across every supported operation
and transport. B-04 supplies the bounded Rust WebSocket protocol and B-05
supplies catalogue-derived GraphQL lowering, but outward GraphQL carriage,
generated WebSocket support, complete operation/model validation, safe session
and retry/cancellation semantics, installed-profile coverage, and adversarial
resource/error behavior remain incomplete across the client surfaces. The
language packages and Connectome also retain the runtime, packaging,
reproducibility, offline-install, platform, and real-engine gaps recorded by
their dedicated owners.

The generated-surface topology now names one executable operation/capability
catalogue as semantic owner, but the complete one-invocation compiler and
installed cross-surface corpus have not been implemented. Hand-maintained
operation/model/reference/conformance copies can therefore still drift.

## Current-detail owners

The Connectome source is now pinned through the optional `connectome/`
submodule, but source registration does not close the client-conformance gap.
The repository previously rejected all gitlinks and attempted to read a
gitlink directory as a file during inventory generation; the
[source-integration journal](../../evidence/change-journals/gate-h/h-06-connectome-submodule-and-delivery-positioning.md)
records the bounded correction and its checks. Current main subsequently
required recursive checkout in every CI job.
The [policy-integration receipt](../../evidence/change-journals/gate-h/h-06-connectome-submodule-policy-integration.md)
records reconciliation with that owner-approved source policy. The owner then
explicitly authorized making Connectome public. The
[publication receipt](../../evidence/change-journals/gate-h/h-06-connectome-public-association.md)
records anonymous source access and successful recursive candidate checkouts.
That acquisition blocker is resolved; full candidate CI and client qualification
remain separate requirements, and POAM-011 remains open.

The [delivery targets](../../architecture/system-overview.md#delivery-experiences)
also lack qualified end-to-end evidence here for shippin.cloud private
devspaces, Rust/Python/Go execution and UI previews, web deployment, and
DevPulse account creation and synchronization. Their contracts and acceptance
packages must be established before implementation or readiness claims; H-06
source registration alone cannot close those delivery gaps.

- [Executable public contract](../../reference/protocol/public-contract.md)
- [Generated-surface design](../../roadmap/rrflow-1.0-execution/generated-surfaces.md)
- [SDK index](../../reference/sdk/README.md):
  [Rust](../../reference/sdk/rust.md),
  [TypeScript](../../reference/sdk/typescript.md),
  [Python](../../reference/sdk/python.md),
  [Go](../../reference/sdk/go.md),
  [Java](../../reference/sdk/java.md), and
  [.NET](../../reference/sdk/dotnet.md)
- [Connectome client contract](../../reference/client/connectome.md)

These records own current surface behavior and language-specific deltas. This
POA&M item owns only the unresolved cross-surface gap and its closure condition.

## Impact

User-visible behavior can disagree with the engine; credentials, cancellation,
or retry state can cross the wrong boundary; clients can consume unintended
resources; and green generation, type, mock, or fixture output can falsely
qualify an incomplete or unusable installed surface.

## Owning gates

A-07, B-04, B-05, D-01, H-04 through H-07, J-01 through J-05

Roadmap owners: [Gate A](../../roadmap/rrflow-1.0/gate-a.md),
[Gate B](../../roadmap/rrflow-1.0/gate-b.md),
[Gate D](../../roadmap/rrflow-1.0/gate-d.md),
[Gate H](../../roadmap/rrflow-1.0/gate-h.md), and
[Gate J](../../roadmap/rrflow-1.0/gate-j.md).

## Closure evidence

One deterministic compiler projects the executable wire types and
operation/capability catalogue into internal dispatch metadata, OpenAPI,
internal Rust client bindings, language SDK operations/models, reference
coverage, and shared conformance
cases with byte-identical clean regeneration and no handwritten duplicate
registry. Every supported client binds each catalogue operation exactly once
and passes its owned request/response/status/media/identity, session secrecy,
retry uncertainty, correlated cancellation, bounded streaming, trace,
authenticated endpoint, fault, and resource contracts.

Each language package then passes the runtime, consumer, supported-toolchain and
platform, reproducibility, signature, and signed-offline-distribution evidence
named by its linked SDK owner. A D-01-installed rrflowMX/rrflowKV corpus must
compare result, denial, stamp, digest, receipt, trace, restart, and resource
evidence across HTTP, WebSocket, Rust, every generated SDK, CLI, MCP, GraphQL,
and Connectome; missing harness configuration fails or is explicitly skipped
and never reports conformance.

## Evidence history

- Structural SDK evidence:
  [Rust](../../evidence/change-journals/gate-a/a-07-1b-evidence-journal.md),
  [TypeScript](../../evidence/change-journals/gate-a/a-07-1c-evidence-journal.md),
  [Python](../../evidence/change-journals/gate-a/a-07-1d-evidence-journal.md),
  [Go](../../evidence/change-journals/gate-a/a-07-1e-evidence-journal.md),
  [Java](../../evidence/change-journals/gate-a/a-07-1f-evidence-journal.md), and
  [.NET](../../evidence/change-journals/gate-a/a-07-1g-dotnet-sdk-responsibility-boundary.md)
- [B-04 WebSocket evidence](../../evidence/change-journals/gate-b/b-04-evidence-journal.md)
  and [B-05 GraphQL evidence](../../evidence/change-journals/gate-b/b-05-evidence-journal.md)
- [Generated-surface and linked-record refactor evidence](../../evidence/change-journals/repository/poam-027c-linked-execution-records-and-generated-navigation.md)
