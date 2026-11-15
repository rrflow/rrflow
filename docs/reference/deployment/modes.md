# RRFlow deployment profiles

**Status:** active accepted deployment classification; typed discovery implemented, full conformance remains incomplete
**Coordinate:** `rrflow://rrflow-instance/data/reference/deployment/modes`
**Owner:** deployment-form, storage-profile, and endpoint-presentation combinations plus their conformance boundaries

RRFlow is one independently deployable engine. A deployment profile describes
how one installed RRD instance is composed and reached; it never selects a
different transaction coordinator, graph engine, vector database, query
planner, reasoning authority, or persistence product.

The [instance-topology architecture](../../architecture/instance-topology.md)
owns project, engram, instance, environment, and physical-placement identity.
The [engine data-flow architecture](../../architecture/engine-data-flow.md)
owns the common write, read, Arrow/DataFusion, reasoning, and recall paths. The
[distributed cluster contract](../distributed/cluster-contract.md) owns the
unavailable clustered target and the disposition of current cluster code. The
[local-process adapter](local-process-driver.md) owns host process effects, the
[Kubernetes adapter](kubernetes-operator.md) owns Kubernetes projection and
reconciliation for one installed instance, the
[server reference](../protocol/server.md) owns HTTP/WebSocket behavior, and the
[roadmap](../../roadmap/rrflow-1.0.md) owns implementation order and completion
evidence. This record owns only the profile classification and the proof needed
to claim semantic equivalence across its independent axes.

## Independent profile coordinates

The installed profile is the product of three orthogonal coordinates:

| Coordinate | Accepted values | What it selects | What it cannot select |
|---|---|---|---|
| Deployment form | `embedded`, `single_node_server`, `clustered_server` | Whether the same logical `RrdEngine` is called in-process, served by one RRD process, or served by a coordinated cluster. | Storage durability, endpoint security, a different operation catalogue, or another engine. |
| Storage profile | `rrflow_mx`, `rrflow_kv` | The volatile rrflowMX implementation or the durable rrflowKV implementation beneath the same storage and semantic transaction contracts. | Process placement, network reachability, query semantics, or an automatic volatile-to-durable migration. |
| Endpoint presentation | `in_process`, `loopback_http_websocket`, `network_http_websocket` | How an authenticated client invokes the installed instance. A server may expose more than one configured presentation. | Instance identity, authorization, durability, or mesh membership. |

Security posture is bound separately. Cleartext is restricted to an explicitly
configured loopback endpoint. A configured network endpoint requires the
accepted authenticated encrypted transport. TLS, mutual TLS, certificate
rotation, credential providers, and mesh carriage are security or adapter
capabilities—not deployment modes.

Artifact packaging is also separate. The current `rrflow-edge` mmap artifact,
browser bundles, mobile packages, SDKs, Connectome, and model packages are
clients or derived artifacts. None is a fourth deployment form and none owns
canonical engram state.

## Valid combinations and alpha posture

| Deployment form | Storage profile | Endpoint presentation | Target posture |
|---|---|---|---|
| `embedded` | `rrflow_mx` | `in_process` | Valid volatile composition. It must pass all semantics that do not require durability. |
| `embedded` | `rrflow_kv` | `in_process` | Valid durable composition. The embedding process exclusively owns the rrflowKV writer. |
| `single_node_server` | `rrflow_mx` | loopback or configured network HTTP/WebSocket | Valid volatile server composition. Durability-only operations fail before preparing authoritative state. |
| `single_node_server` | `rrflow_kv` | loopback or configured network HTTP/WebSocket | First-alpha persistent service target and Connectome bootstrap target. The server exclusively owns the rrflowKV writer. |
| `clustered_server` | `rrflow_kv` | configured network HTTP/WebSocket | Target only and unavailable. It is not a first-alpha exit requirement; the roadmap owner must add and accept a distributed implementation gate before this value can be installed or advertised. |
| `clustered_server` | `rrflow_mx` | any | Invalid: process-local volatile state cannot be the canonical replicated cluster substrate. |

The default first-alpha installation is one authenticated single-node RRD
server over rrflowKV with an explicit loopback HTTP/WebSocket endpoint. A
network or mesh-resolved endpoint is an explicit configuration addition. The
default is a release target, not a shortcut: embedded and rrflowMX compositions
must still use the same semantic contracts, and unsupported combinations must
fail explicitly rather than silently switching profiles.

rrflowMX is not a write buffer, cache, or automatic hot tier for rrflowDB.
rrflowKV's WAL, memtable, immutable pages, and bounded cache form the
hot-to-durable path of rrflowDB. Moving accepted semantic state between
rrflowMX and rrflowKV requires an explicit authorized operation with ordinary
validation,
stamps, evidence, and commit semantics.

## Installation and runtime discovery

Gate D-01 selects the complete profile during preview/apply and commits it in
the installed project-engram-instance binding. Startup resolves that binding
read-only. The current canonical install profile selects
`single_node_server` + `rrflow_kv` + `loopback_http_websocket`; engine
constructors expose only the storage profile they actually own. An embedded or
server composition supplies deployment form, and the server supplies endpoint
presentation from the actual bound listener address. No component reconstructs
storage or process form from incidental facts.

In particular:

- a persistent root does not prove the deployment form;
- an absent root does not prove that the caller is embedded;
- TLS enabled or disabled does not distinguish a local from a remote engine;
- the client's network location cannot rename the server's profile;
- a loopback address is not installation or authentication authority; and
- compiled enum variants cannot advertise unavailable cluster or artifact
  behavior.

The public capability descriptor now reports these facts independently:

```text
installed instance/project/engram identity
deployment form
storage profile
active endpoint presentations
security/configuration/capability revisions and digests
available operation and resource-limit capabilities
```

The exact v1 Rust/wire fields are `DeploymentProfile { contract_version,
deployment_form, storage_profile, endpoint_presentation }`. Strict decoding
rejects the removed scalar `deployment_mode`; OpenAPI and the TypeScript
projection are generated from the same owner. This implemented vocabulary
preserves the accepted A-07 discovery baseline but does not complete the
profile conformance matrix, and unavailable combinations must not be emitted as
active capability values.

## One engine flow in every profile

Every valid combination enters the same authority and produces the same
non-durability-specific result:

```text
configured endpoint or in-process call
  -> authenticate instance and principal
  -> resolve installed project/engram binding
  -> RrdEngine authorize + capture stamp
  -> engine-selected native fast path or stamped rrflowQL plan
     -> rrflowMX or rrflowKV transaction/access primitives
     -> graph/scalar/BM25/vector operators when selected
     -> bounded Arrow/DataFusion analytical execution when selected
  -> same result, denial, stamp, digest, evidence, and commit impact
```

DataFusion remains compute-only in every form. It consumes stamped bounded
Arrow batches from the selected storage profile and returns results or a
proposal to `RrdEngine`; it never writes rrflowKV, mutates rrflowMX, commits an
index, or changes deployment state directly.

## Conformance is layered, not a label

A single two-document lookup cannot qualify these profiles. The completed
conformance suite is separated by responsibility so failures identify the
broken boundary.

### Storage-profile semantic differential

Run the same engine-owned corpus unchanged on rrflowMX and rrflowKV. Compare
exact results, denials, read/commit stamps, semantic digests, commit impacts,
and bounded resource counters for:

- authenticated allow and deny decisions, row/field filtering, and audit;
- transaction read-your-writes, rollback, conflicts, idempotent replay, and
  the declared snapshot-isolation behavior;
- current and bitemporal records, both graph adjacency directions, and bounded
  traversal;
- scalar and unique indexes, incremental BM25, exact vectors, filtered HNSW or
  quantized candidates, exact reranking, and deterministic RRF;
- direct fast reads and stamped rrflowQL execution over streamed Arrow batches
  through DataFusion with the same logical result;
- persisted reasoning-tree, context, evidence, feedback, event, routine, and
  attunement operations where those gates apply; and
- commit-impact changefeed/subscription behavior.

Durability assertions are not forced onto rrflowMX. rrflowKV alone must pass
WAL and manifest fault injection, crash/reopen, writer exclusion, backup,
recovery, compaction, mapped-buffer lifetime, ENOSPC, and acknowledged-commit
tests.

### Deployment-form differential

Execute the same public semantic operations through an embedded composition
and a real standalone single-node process. Compare instance/engram identity,
authorization, result, stamp, digest, cancellation, limits, and traces. The
standalone process must use the implemented bounded D-01 install boundary and
must prove authenticated readiness and bounded shutdown; a raw root, marker
file, or in-test server object does not qualify it.

Clustered-server conformance is a separate expansion of this corpus. It must
add independent-host quorum, replication, leader/follower failure, partition,
placement, snapshot-vector, reshard, and recovery evidence without weakening
the single-node semantics. The current roadmap does not schedule that
expansion; its exact entry conditions and current-code disposition are in the
[distributed cluster contract](../distributed/cluster-contract.md).

### Endpoint-presentation differential

Run equivalent operations over in-process, loopback HTTP/WebSocket, and
configured authenticated network HTTP/WebSocket presentations. Compare the
same typed operation, authorization decision, result/denial, stamp, digest,
deadline/cancellation behavior, backpressure, and correlated causal trace.
TLS and mesh tests qualify their security and carriage properties; they do not
create a `remote` database kind.

### Derived-artifact proof

Test `rrflow-edge` and other offline artifacts against their own immutable
source-cursor, checksum, bounded-read, and exact-result contracts. This proof
must not be counted as rrflowMX/rrflowKV, transaction, graph, DataFusion,
reasoning, or deployment-form conformance unless the artifact actually enters
those ordinary engine paths.

## Current implementation audit

The current classification now uses the accepted independent coordinates, but
its qualification corpus remains far below the preceding acceptance boundary.

| Current implementation | Characterized behavior worth retaining | Required direct convergence |
|---|---|---|
| `DeploymentProfile`, `InstalledEngramIdentity`, and `ServiceCapabilities::{deployment,installed_engram,configuration}` | Strict independent axes, identity/configuration validation, generated OpenAPI/TypeScript projection, and rejection of the removed scalar now pass. A-07 is accepted. | Retain this as the sole contract while H-04/J qualify every generated client and deployment form against release-equivalent installed processes. |
| `RrdEngine::storage_profile_kind` and `open_installed` | MX and KV constructors report only the explicit storage axis and never infer process form from root presence. Installed open binds identity/configuration and verifies project-relative locator plus distribution executable; outward composition owns form and endpoint. Pre-canonical manifest/private-binding product openers are absent. | Keep raw construction confined to explicit component tests and complete the full storage/deployment semantic differential without making it a product startup mode. |
| `rrd-server::http::capabilities` | Reports single-node form, engine-selected storage, endpoint presentation from the bound address, installed identity when present, and effective ceilings. TLS no longer selects a database mode. | Qualify configured network listeners and every active presentation; do not count a raw-root standalone fixture as the installed product proof. |
| `rrd-deployment-conformance-v1.json` and contract validation | Strictly validates a versioned fixture with unique non-empty document IDs, bounds, and expected output membership. | Replace the overclaimed shared-deployment meaning with the layered semantic, form, transport, durability, and artifact corpora above. |
| engine deployment tests | Exercise one schema plus two records on rrflowMX and rrflowKV, run one exact-ID rrflowQL query through the current DataFusion executor, reject a durability operation on rrflowMX, deny a second rrflowKV writer, and reopen at cursor three. | Retain this as a seed characterization only. It does not prove streaming/pushdown, native graph/index/vector access, complete semantic parity, failure atomicity, reasoning, or resource bounds. |
| Rust client loopback and mutual-TLS tests | Exercise real sockets and verify selected TLS identity behavior without using TLS to choose the database or deployment profile. Installed-identity fixtures use plan/apply; unrelated transport component fixtures construct their engine explicitly. | Run the complete cross-surface corpus against a release-equivalent installed process. |
| installed SDK conformance daemon | Canonically installs an empty temporary project from a checked inert distribution anchor, extends its schema/security through the installed engine, requires the derived instance/engram identities to match the shared corpus, and drives Rust, TypeScript, Python, Go, Java, and .NET against one real daemon. | The shared corpus remains narrow and does not prove the full layered semantic, deployment, transport, durability, failure, or resource matrices. |
| standalone server-process and process-driver tests | The internal server process now requires project plus distribution executable and opens only installed state; process-driver fixtures preinstall the engram and retain real child, writer, stop, and fault behavior. | Replace marker/private controller authority and qualify authenticated launch/readiness, resources, distribution identity, and cross-platform behavior before counting it as deployment proof. |
| `rrflow-edge` offline test | Builds, mmap-opens, and queries a deterministic feature-hash artifact from the two fixture documents. | Keep it as derived-artifact evidence. It parses the fixture as untyped JSON and does not enter `RrdEngine`, rrflowQL, transactions, graph/index semantics, or the shared storage profile. |

The control journal and global runtime cursor are not two ordering authorities.
Accepted C-03 evidence commits authoritative semantic state, index and
projection deltas, function results, audit, outbox, and cursor through one
engine-owned semantic transaction. Control recovery reconciles around that
commit; logs and traces observe it and never manufacture it.

## Direct-convergence sequence

1. **Accepted foundation:** retain A-07's three-coordinate contract, C-02's
   shared transaction port, accepted C-03 atomic semantic commit, and accepted
   C-04 direct stamped reads without a compatibility shim.
2. **D-01:** retain the implemented preview/apply identity, profile, sealed
   configuration, installed-only product startup, and eight-stage exact-plan
   retry. Complete hostile-path/native-platform lifecycle qualification without
   reintroducing the removed pre-canonical initialization authority.
3. **B-04/H-04:** retain the closed WebSocket vocabulary and run one operation
   catalogue across HTTP, WebSocket, SDK, MCP, CLI, and Connectome.
4. **C-07/E/F/G/H/I:** complete durability/maintenance qualification and fill
   the shared corpus with native graph, lexical, vector,
   streamed Arrow/DataFusion, reasoning, context, feedback, delivery, and
   automation behavior as each owning gate lands.
5. **H-07:** add configured mesh endpoint resolution as an optional transport
   adapter, never as identity, authentication, storage, or deployment mode.
6. **J:** reject every superseded value/shape, qualify the complete matrix on
   clean installed artifacts, and publish raw comparative deployment evidence.

## Acceptance

The typed classification contract is implemented: installation binds one
explicit valid combination, capability discovery reports independent
coordinates, and no current caller may infer a mode from root, TLS, or network
location. Full deployment-profile acceptance still requires rrflowMX and
rrflowKV to pass the complete non-durability semantic differential, rrflowKV to
pass the durability matrix, embedded and installed single-node forms plus every
active endpoint to pass cross-surface equivalence, cluster values to remain
unavailable until independent-host proof, and derived edge artifacts to remain
outside deployment semantics. No `DeploymentMode` alias, speculative value, or
two-document fixture is substitute evidence.
