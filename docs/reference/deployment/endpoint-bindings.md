# RRFlow endpoint bindings, profiles, and hardware inventory

**Status:** active implementation reference; configured experimental calls exist, release qualification remains open
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

Binding types live in `crates/transport/rrd-contract/src/endpoint_binding.rs`;
the optional selector policy lives in `endpoint_policy.rs`, and observation
delivery policy lives in `observation_delivery.rs`. These derive
`JsonSchema` and reject unknown fields.

| Type | Meaning | Rules |
|---|---|---|
| `WorkloadClass` | The eight classes code depends on: large-model serving, always-warm small models, embedding and reranking, vector store, canonical store, weight updates, behavior updates, control plane. | Closed enum. A class contract is the trait or operation set the engine already defines for that work, such as `EmbeddingBackend`; this contract names the class, not the shape. |
| `BindingKind` | `local`, `self_hosted`, `subscription`, `api`: how an endpoint is reached and paid for. | Closed enum. The kind never changes behavior. |
| `EndpointLocator` | `in_process { component }` or `url { url }`. | A URL carries a scheme and a host, no whitespace, and no user information; credentials never ride in a URL. In-process execution is allowed only for local bindings. |
| `CredentialReference` | `provider` and `name` of a secret held by a configured secret provider. | The value never appears in this contract, a store, a client, or a log. The name has no whitespace or control characters. The schema has no value field. |
| `DeclaredFootprint` | Memory a binding declares on its placement machine, and a GPU requirement with its memory. | Nonzero; replaced by measured values once observation exists. |
| `BindingLimits` | `max_concurrency`, `timeout_ms`, `max_input_bytes`. | Nonzero within the contract ceilings. Class-specific limits stay with their class contracts. |
| `EndpointBinding` | One configured endpoint serving one class: `id`, `class`, `kind`, `endpoint`, `service_id`, optional `manifest_sha256`, optional `credential`, optional `placement`, `footprint`, `limits`. | Local and self-hosted bindings name a placement machine; subscription and API bindings carry a credential reference, use a URL, and have no placement. `manifest_sha256`, when present, pins the manifest or artifact the service must present. |
| `EndpointProfileInput` | Operator-authored: `contract_version`, `name`, `bindings` in declaration order, and optional `selection_policy` and `observation_policy`. | Within one class, declaration order is fallback order; the first binding is primary. Ids are unique; a class has at most sixteen bindings. Absent policies configure neither selection nor observation delivery. |
| `EndpointProfile` | The effective profile: the input plus an engine-assigned `revision` and `profile_sha256`. | The digest binds version, name, revision, bindings and every present policy field; drift fails validation. `fallbacks(class)` and `primary(class)` read the order. |
| `EndpointSelectionPolicy` | Explicit controller binding, workload, candidates and declared effort evidence, selection limits, system prompt, output-token ceiling and temperature in thousandths. | Bounded and closed; candidate identities must match the profile. No provider, model, prompt or ranking defaults. Configuration grants no permission to invoke it. |
| `HardwareInventory` and `MachineCapacity` | Declared `memory_bytes`, optional GPU `memory_bytes`, and `disk_bytes` per machine. | Unique machine ids; nonzero values. On unified-memory hardware the GPU pool and machine memory are the same bytes, declared in both places. |
| `MachineLoad` | Declared load per machine under a profile. | Returned by `validate_placement` for display beside observed state. |

`EndpointProfile::validate_placement(&HardwareInventory)` checks that every
placed binding names a declared machine, that the declared memory of all
bindings placed on one machine fits its capacity, and that a binding requiring
a GPU finds one with enough declared memory. It returns the declared load per
machine.

## Governed selector configuration

The optional policy participates in the existing endpoint-configuration
preview/apply/read operations and their approval digest, revision CAS, replay
and revocation behavior. Every present field changes profile identity; absent
policies retain existing profile hashes and omit the wire field. A policy can
project an allowance only from a configuration containing that exact policy.
This verifies declared consistency, not capability evidence or current caller
eligibility. The engine must still verify evidence, intersect authorization,
bind the task/source and reserve budget before any external call.

The [policy journal](../../evidence/change-journals/gate-g/controller-policy-configuration.md)
records approval and embedded persistence/reopen tests using proposals derived
from retained actual settings. It does not establish installed-binary execution,
provider qualification or engine-admitted selector calls.

## Governed observation delivery

`observation_policy` selects a `behavior_updates` URL binding with a process-owned
credential reference, expected devspace and protocol-evidence digest. It explicitly
grants metadata export and network access for listed scopes. The policy bounds
retained records, retention, attempts, cooldown and response size; the binding
bounds request size and timeout. One dispatch may be admitted at a time.
HTTPS is required except for literal loopback HTTP. Redirects, URL credentials,
query/fragment-bearing endpoints and implicit proxy configuration are refused.

The public `observation-prepare`, `observation-deliver` and `observation-read`
operations use the existing endpoint profile and engine control transaction
boundary. Preparation reads an authorized terminal task attempt or installed job
and guards its durable source bytes alongside current authorization and endpoint
configuration. Job revision and plan digest identify a source independently of
the public read's observation timestamp. Later edits do not reinterpret retained
observations or execute the source task again.

The engine saves exact source-response bytes locally. Authorized readback returns
those bytes and their digest, including after the job advances. The RROFLOW capture
packet contains bounded references and digests only. Its configured receiver's
acknowledgement proves storage, not engine attestation, task success, model quality
or permission to use an observation for training.

Before I/O, delivery commits a lease and attempt fence. Only an exact event-ID and
devspace acknowledgement under unchanged authority/configuration can publish an
acknowledged receipt. Lost replies, expired leases and rejected completion fences
remain uncertain. A fresh authorized retry reuses the retained event and payload;
an acknowledged delivery never dispatches again. Credentials are resolved from the
existing immutable process registry and are neither stored nor returned.

Expired uncertain records remain visible by default and can fill the export
history. An operator may explicitly approve `retire_expired_uncertain: true` in
a new profile to permit audited retirement during subsequent preparation. This
does not resolve their outcomes. Retired IDs are outside the idempotency window;
a new preparation gets a fresh remote identity. Queue pressure affects export,
not the source project's lifecycle. Public integration and release acceptance
require the package's engine, transport and deployed-service evidence.

When an approved edit reduces the history cap, preparation can retire eligible
expired records in at most three control transactions. Each batch checks the
same source, configuration, authority and index identities. A concurrent change
may leave audited retirement progress while rejecting the new preparation;
retry requires current inputs. No uncertain record becomes acknowledged through
cleanup, and source tasks remain independent of export capacity.

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

- General runtime resolution. The engine reads and persists profiles through
  governed endpoint-configuration operations. Saving a profile does not dispatch
  work. Explicit experimental proposal and observation-delivery operations can
  call their configured bindings under current authorization and limits.
  The embedding registry remains process-local, and `rrflow install` seals
  [`EngramConfiguration`](../../../crates/transport/rrd-contract/src/deployment.rs)
  limits. Admitted runtime resolution on configuration changes remains separate
  from storing a profile.
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
  a revisioned record beside `EngramConfiguration`.
- H-07 owns endpoint resolution through a mesh; G owns the LFG and SWAG
  plug-in configurations that bind their models through this contract.
