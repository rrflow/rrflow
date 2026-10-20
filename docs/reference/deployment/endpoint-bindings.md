# RRFlow endpoint bindings, profiles, and hardware inventory

**Status:** active implementation reference; contract and validation only, runtime resolution and governed activation remain open
**Coordinate:** `rrflow://rrflow-instance/data/reference/deployment/endpoint-bindings`
**Owner:** the endpoint binding, endpoint profile, and hardware inventory contracts in `rrd-contract`

Every model, store, and service that a class of work reaches is a configured
endpoint binding. Which machine, provider, or model serves a class is
configuration, never code: a local, self-hosted, subscription, or API binding
changes nothing about the class contract the engine depends on. This record
owns the shape of that configuration. The
[deployment profiles](modes.md) own the engine's own form, storage, and
endpoint presentation; the [installed lifecycle](installed-lifecycle.md) owns
how configuration is sealed at install; the
[instance topology](../../architecture/instance-topology.md) owns which
machine identities exist.

## Contract

All types live in `crates/transport/rrd-contract/src/endpoint_binding.rs`,
derive `JsonSchema`, and reject unknown fields.

| Type | Meaning | Rules |
|---|---|---|
| `WorkloadClass` | The eight classes code depends on: large-model serving, always-warm small models, embedding and reranking, vector store, canonical store, weight updates, behavior updates, control plane. | Closed enum. A class contract is the trait or operation set the engine already defines for that work, such as `EmbeddingBackend`; this contract names the class, not the shape. |
| `BindingKind` | `local`, `self_hosted`, `subscription`, `api`: how an endpoint is reached and paid for. | Closed enum. The kind never changes behavior. |
| `EndpointLocator` | `in_process { component }` or `url { url }`. | A URL carries a scheme and a host, no whitespace, and no user information; credentials never ride in a URL. In-process execution is allowed only for local bindings. |
| `CredentialReference` | `provider` and `name` of a secret held by a configured secret provider. | The value never appears in this contract, a store, a client, or a log. The name has no whitespace or control characters. The schema has no value field. |
| `DeclaredFootprint` | Memory a binding declares on its placement machine, and a GPU requirement with its memory. | Nonzero; replaced by measured values once observation exists. |
| `BindingLimits` | `max_concurrency`, `timeout_ms`, `max_input_bytes`. | Nonzero within the contract ceilings. Class-specific limits stay with their class contracts. |
| `EndpointBinding` | One configured endpoint serving one class: `id`, `class`, `kind`, `endpoint`, `service_id`, optional `manifest_sha256`, optional `credential`, optional `placement`, `footprint`, `limits`. | Local and self-hosted bindings name a placement machine; subscription and API bindings carry a credential reference, use a URL, and have no placement. `manifest_sha256`, when present, pins the manifest or artifact the service must present. |
| `EndpointProfileInput` | Operator-authored: `contract_version`, `name`, `bindings` in declaration order. | Within one class, declaration order is fallback order; the first binding is primary. Ids are unique; a class has at most sixteen bindings. |
| `EndpointProfile` | The effective profile: the input plus an engine-assigned `revision` and `profile_sha256`. | The digest binds version, name, revision, and bindings; drift fails validation. `fallbacks(class)` and `primary(class)` read the order. |
| `HardwareInventory` and `MachineCapacity` | Declared `memory_bytes`, optional GPU `memory_bytes`, and `disk_bytes` per machine. | Unique machine ids; nonzero values. On unified-memory hardware the GPU pool and machine memory are the same bytes, declared in both places. |
| `MachineLoad` | Declared load per machine under a profile. | Returned by `validate_placement` for display beside observed state. |

`EndpointProfile::validate_placement(&HardwareInventory)` checks that every
placed binding names a declared machine, that the declared memory of all
bindings placed on one machine fits its capacity, and that a binding requiring
a GPU finds one with enough declared memory. It returns the declared load per
machine.

## Fixture

`crates/transport/rrd-contract/tests/fixtures/endpoint-profile-v1.json` is the
first profile, `ring-3`: large-model serving on two machines with a
subscription and an API fallback behind them, the brain classes (small models,
embedding, vectors, the canonical store) on a third, weight updates beside the
second serving machine, behavior updates through an API, and the control plane
on a fourth machine. It carries one binding of every kind. Its companion,
`hardware-inventory-v1.json`, declares the four machines. The contract test
pins the fixture's canonical digest
(`611144dbc4056fa8a4df87e5c18c8ae26342d288319f41c727183c08dc3da4d2`), so a
change to the canonical encoding fails the test until the fixture record and
the digest move together. Hosts in the fixture use reserved example names;
no value in it is a secret.

## What this record does not cover

- Runtime resolution. Nothing reads a profile yet: the embedding registry is
  process-local, and `rrflow install` seals only
  [`EstateConfiguration`](../../../crates/transport/rrd-contract/src/deployment.rs)
  limits. Resolving the active profile at request time or on change
  notification, so that an activation takes effect without a rebuild, is the
  governed configuration work that extends `EstateConfiguration` revisions
  after install.
- Activation. A binding is configuration. Discovery creates an inactive typed
  candidate, and only an exact previewed immutable binding plus explicit
  configuration, policy, and authorization may activate it (D-06).
- Model identity. A local model's bytes are identified by its
  [router model manifest](../../../crates/transport/rrd-contract/src/model_manifest.rs)
  or embedding backend snapshot; a binding points at that identity through
  `service_id` and may pin it through `manifest_sha256`.
- A profile guideline, not a rule: on unified-memory hardware an inference
  server that exhausts memory can take co-located processes down with it, so
  a profile should place the canonical store on a different machine from
  large-model serving. The `ring-3` fixture follows it.

## Owning gates

- D-06 owns the previewed immutable binding and its activation rules; this
  contract is its binding shape.
- Governed configuration revisions after install consume `EndpointProfile` as
  a revisioned record beside `EstateConfiguration`.
- H-07 owns endpoint resolution through a mesh; G owns the LFG and SWAG
  plug-in configurations that bind their models through this contract.
