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

The owner's rrflowDB identity requires PostgreSQL compatibility. The new
[core-profile contract](../../reference/protocol/postgresql.md) defines the
first compatibility matrix and [H-08](../../roadmap/rrflow-1.0/gate-h.md#h-08-postgresql-compatibility-packages)
defines bounded implementation packages. Neither is implementation evidence.
The inspected server exposes RRD HTTP/WebSocket; `rrd-operator-knowledge`
contains an outbound PostgreSQL adapter. The native value/schema model and
snapshot transaction port still need SQL type, relational constraint,
statement-snapshot, savepoint and concurrency semantics. Wire/client conformance,
authorized catalogue discovery, bulk transfer, vector SQL, source-exit import
and installed-profile qualification remain open. No required profile case is
currently accepted as a PostgreSQL compatibility pass.

The requested DevPulse/shippin.cloud account synchronization path is absent:
repository inspection finds no account binding, consent activation or
cross-instance reconciliation implementation. Native changefeeds and local
persistence are prerequisites, not sync proof. The [integration contract](../../reference/client/devpulse-cloud.md)
defines local independence, opt-in, replay/conflict/revocation requirements and
real-account acceptance under D-06/H-04/H-07. This gap remains sequenced.

## Current-detail owners

- [rrflowDB identity and compatibility boundary](../../architecture/system-overview.md#persistence-and-memory-boundary)
- [PostgreSQL compatibility and independent database contract](../../reference/protocol/postgresql.md)
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

A-07, B-04, B-05, C-07, D-01/D-06/D-11, F, H-04 through H-08, J-01 through J-05

Roadmap owners: [Gate A](../../roadmap/rrflow-1.0/gate-a.md),
[Gate B](../../roadmap/rrflow-1.0/gate-b.md),
[Gate C](../../roadmap/rrflow-1.0/gate-c.md),
[Gate D](../../roadmap/rrflow-1.0/gate-d.md),
[Gate F](../../roadmap/rrflow-1.0/gate-f.md),
[Gate H](../../roadmap/rrflow-1.0/gate-h.md), and
[Gate J](../../roadmap/rrflow-1.0/gate-j.md).

## Closure evidence

H-08 additionally requires every required PostgreSQL core-profile case to pass
against pinned reference/client versions and installed native rrflowDB, with
shared-engine result/authorization/receipt equivalence, concurrency and crash
proof, explicit unsupported-feature denials, and a verified synthetic external
import/export and recovery rehearsal. A wire handshake or parser alone cannot
close this gap. H-04 and H-08 evidence are both required; neither substitutes
for the other.

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
