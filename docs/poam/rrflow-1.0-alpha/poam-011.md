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
DevPulse account creation and synchronization. The linked DevPulse contract now defines the account/sync boundary; its
implementation and acceptance evidence remain open. H-06 source registration
alone cannot close those delivery gaps.

The native Connectome workbench now has real local installation/query/reopen
evidence, linked in its [package receipt](../../evidence/change-journals/gate-h/h-06-native-turnkey-workbench.md).
That bounded result does not close H-06. Its process fixture exposed a concrete
query gap: a schemaless key/value table can persist a record, but querying it
as `record:settings` fails with an unregistered record-type error. The binder
in `crates/compute/rrd-query/src/plan.rs` (`fields_for_source`) requires the
registered record schema, while table metadata alone supplies none. The UI
now disables unsupported Browse and explains the limitation. Gate F/public
query qualification owns a canonical schemaless binding decision and positive,
negative, authorization, budget and reopen tests; H-06 then consumes that
qualified public behavior. A strict-schema positive fixture is not proof that
the schemaless gap is fixed.

The owner's complete delivery target also requires DevPulse project VM/container
provisioning, independent project-file/environment transfer and bidirectional
native data synchronization with an operator-controlled homelab, without a
hosted account. The earlier cloud-only framing was incomplete. The
[H-07 package sequence](../../roadmap/rrflow-1.0/gate-h.md#h-07-devpulse-managed-cloud-and-homelab-packages)
now identifies those implementation and real-service acceptance dependencies.
Current RRFlow transaction idempotency derives its transaction identity from
the session plus request key; it supplies no admitted cross-instance import
binding or source-origin deduplication. Native import/export coverage, durable
origin receipts and snapshot-to-changefeed handoff must be qualified before a
client may claim bidirectional sync. Existing DevPulse/Shippin application
code must be integrated at its actual canonical boundary, not replaced with a
new scaffold. The Windows development bundle does not close these gaps or
J-04's competitive parity requirement.

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
