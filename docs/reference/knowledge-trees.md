# Knowledge trees and provider paths

**Status:** active implementation boundary; external provider execution and cloud sync unqualified
**Coordinate:** `rrflow://rrflow-instance/data/reference/knowledge-trees`
**Owner:** logical knowledge-tree configuration, provider-path meaning and qualification boundary

A knowledge tree is a named hierarchy of grouping nodes and knowledge paths.
Each source path binds either an existing native RRFlow vector collection and
named vector, or an explicitly configured external knowledge service. The
public definitions in `rrd-contract/src/knowledge_tree.rs` own their exact
serialized shape and limits. They do not replace bootstrap KnowledgePackageV1,
reasoning trees, filesystem attunement or the endpoint-binding contract.

## One engine authority

The installed project's RrdEngine authenticates and authorizes catalogue reads
and replacements. Project and instance scope must match the installed estate.
A replacement carries the last observed revision and an idempotency identity;
the catalogue and exact accepted response receipt commit atomically through
the existing control journal. Stale writes fail rather than overwrite another
operator's changes. Retrying an accepted identity returns the original response
without rolling back later edits. Labels may change while tree/path identities
remain stable. Parents must be groups inside the same tree; duplicate, orphaned,
cyclic, excessively deep or oversized definitions are rejected.

Native collection and named-vector identities must exist when configuration
is saved. Runtime execution must revalidate their current availability; saving
a path does not protect its collection from a later independent deletion.
External paths reuse EndpointBinding and its admission limits, placement and
CredentialReference. Secret values and URL credentials are excluded. Saving
configuration invokes no provider, creates no external account/index, and is
not a successful search, ingestion, synchronization or capability-parity claim.

The public operation catalogue owns `knowledge-tree-list` and
`knowledge-tree-replace`; the negotiated capability is
`knowledge-tree-catalogue` version 1. Connectome uses that capability before
issuing a new operation, derives project/scope in the native session and keeps
unsaved edits on errors. Estates without the new capability retain their
existing Records, Vectors and Storage controls. An engine update and any new
policy grants require their own deployment evidence.

## Provider capability mapping

The owner selected HydraDB as the first external connector and Pinecone Nexus
for subsequent configuration. MongoDB Atlas remains another intended external
knowledge path. All capabilities must be mapped to one native engine capability
or an explicit optional adapter; a vendor label never establishes support.

| Capability family | Current RRFlow implementation | Remaining acceptance |
|---|---|---|
| Durable records, relations and native vectors | Native rrflowDB/rrflowKV, vector catalogues/search, public transactions and diagnostic lenses | Existing release gates still own full persistence, corpus and recovery qualification |
| Knowledge-tree and path configuration | Versioned contract, engine CAS/journal, public operation/SDK and authored UI package under qualification | Installed native client roundtrip, restart/concurrency/security and candidate CI |
| External PostgreSQL/pgvector | Real optional Rust adapter in rrd-operator-knowledge | Product-facing binding resolution and live deployment qualification; this is a client adapter, not an embedded PostgreSQL server |
| HydraDB databases, collections, metadata, context ingestion/query and connectors | No HydraDB adapter yet | Typed v2 protocol mapping, scoped auth, bounded I/O, asynchronous ingestion/status/deletion, provenance and live account evidence |
| MongoDB Atlas vector search | No Atlas adapter yet | Supported Rust driver, explicit database/collection/index/field binding, query/filter isolation, errors and live evidence |
| Pinecone Nexus curated context and retrieval | No Nexus adapter yet; separate from raw Pinecone vector indexes | Accepted deployment/API contract, typed async query/poll/cancel and source/citation mapping; published demo documentation is not a production default |
| Turnkey shippin.cloud | Existing custom saved runtimes; no catalogue provisioning/sync service found | Customer-scoped provisioning/auth, RRD endpoint/identity delivery and operational lifecycle |
| Bidirectional sync | Existing internal outbound operator-knowledge work/receipts only | Two-way cursors, conflict policy, tombstones, provenance, echo suppression, retry/offline recovery, export and restart tests |
| Recursive learning, LFG and SWAG | Existing reasoning/feedback contracts and endpoint profiles; extensions remain unimplemented | [Inference extensions and recursive learning](inference-extensions.md) and their owning roadmap gates |

Official protocol research: [HydraDB v2](https://docs.hydradb.com/api-reference/v2),
[MongoDB Vector Search](https://www.mongodb.com/docs/vector-search/), and
[Pinecone Nexus](https://www.pinecone.io/product/nexus/) with its
[published demo API](https://q2-demo.nexus.pinecone.io/docs/api). HydraDB's text/
hybrid context query and Nexus's scoped retrieval cannot be mislabeled as the
existing raw-vector OperatorSearchRequest. Adapt behavior to the canonical
provider-neutral knowledge boundary rather than inserting a hidden external
engine or aliasing one provider to another.

The verified gaps are tracked in [POAM-032](../poam/rrflow-1.0-alpha/poam-032.md).
The [Connectome contract](client/connectome.md) and
[Gate D](../roadmap/rrflow-1.0/gate-d.md) own client and integration acceptance.
