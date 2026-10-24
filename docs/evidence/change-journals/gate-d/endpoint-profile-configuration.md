# Durable endpoint profile configuration

**Status:** implementation in progress; no gate completion claim
**Coordinate:** `rrflow://rrflow-instance/data/evidence/endpoint-profile-configuration`
**Owner:** D-06 configuration prerequisite; alpha configurable engine

Starting revision: `335c9b67d8913f548de119378929a7aba8062a5a`.

## Research and implementation decision

The existing endpoint profile has bounded, revisioned types but no engine-owned
configuration publication. Use the existing control compare-and-swap boundary;
keep one current head and a revocation revision. Preview and read cannot probe
providers or resolve credentials. Configuration alone cannot authorize ingestion.

[Qdrant v1.20 scroll](https://api.qdrant.tech/v-1-20-x/api-reference/points/scroll-points)
requires a collection and offers filters, shard selection, pagination and read
consistency. [Pinecone 2025-04 list](https://docs.pinecone.io/reference/api/2025-04/data-plane/list)
uses an index host, namespace and pagination token. These different source
identities must be preserved by a future qualified adapter; a generic URL is not
proof of source isolation or snapshot consistency. Neither provider's remote
cursor can be substituted for an RRFlow read stamp. Reviewed 2026-10-24; no
upstream implementation was copied and no live vendor qualification is claimed.

This prerequisite makes existing profiles configurable through the canonical
engine before later source-specific knowledge-path activation. It does not
complete D-06, change C-07 status, or add a client-owned registry. The current
[knowledge connector owner](../../../reference/data/knowledge-connectors.md)
continues to govern activation, provenance and ingestion.

The change brief was written before this research deliverable. GitHub needs a
branch diff to open the draft PR; runtime implementation follows that PR.
Implementation results and exact acceptance evidence will be recorded here.
