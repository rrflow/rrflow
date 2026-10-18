# POAM-032 — external knowledge providers and turnkey cloud synchronization

**Status:** active parent-owned POA&M detail
**Coordinate:** `rrflow://rrflow-instance/data/poam/rrflow-1.0-alpha/poam-032`
**Owner:** [RRFlow 1.0 alpha POA&M](../rrflow-1.0-alpha.md)

The parent ledger owns priority and lifecycle status. This detail records a
verified capability gap; it cannot close a roadmap gate independently.

## Observed deficiency

The 2026-10-18 D: source audit found native vector collections and a real Rust
PostgreSQL/pgvector adapter, but no HydraDB, MongoDB Atlas or Pinecone Nexus
adapter and no existing knowledge-tree/provider-path management. The new
catalogue package supplies typed project-bound configuration and is separately
qualified; it does not execute an external provider. A general document-tree
scan/chunk/embed/ingest pipeline is also unfinished under existing Gate D.

The canonical Shippin API catalogue exposes no RRFlow provisioning/sync offering,
and the current shippin-site cloud-control reservation explicitly has no
implemented client/service data. Existing outbound projection receipts are not
bidirectional estate synchronization. No default hosted endpoint may be
fabricated from the website domain.

## Impact

A configured path can otherwise be mistaken for an operational connector.
Users cannot yet provision a turnkey managed estate or prove two-way knowledge
synchronization with custom/native instances. HydraDB is the owner's first
selected connector; Pinecone Nexus requires an accepted deployment contract,
not an assumption that its published demo equals production.

## Owning gates

D-06 optional integrations, D-02/D-05 durable ingestion, H-04 through H-07 public
and client conformance, with Shippin owning customer-scoped provisioning/auth.
[Knowledge trees](../../reference/knowledge-trees.md) owns the capability map.

Roadmap owners: [Gate D](../../roadmap/rrflow-1.0/gate-d.md) and
[Gate H](../../roadmap/rrflow-1.0/gate-h.md).

## Closure evidence

Close only with typed, authorized native/adapter operations; explicit capability
negotiation; live configured provider qualification; provenance and exact source
scope; bounded I/O and cancellation/error behavior; and no secret exposure.
Turnkey cloud acceptance also requires authenticated estate identity delivery,
conflict rules, cursors, tombstones, echo suppression, retries, offline recovery,
export and process-restart evidence. Configuration persistence alone cannot
satisfy these conditions. Release and roadmap status remain open.
