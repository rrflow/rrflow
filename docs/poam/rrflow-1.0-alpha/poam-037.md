# POAM-037 — external knowledge providers and turnkey cloud synchronization

**Status:** active parent-owned POA&M detail
**Coordinate:** `rrflow://rrflow-instance/data/poam/rrflow-1.0-alpha/poam-037`
**Owner:** [RRFlow 1.0 alpha POA&M](../rrflow-1.0-alpha.md)

The parent ledger owns priority and lifecycle status. This detail records a
verified capability gap; it cannot close a roadmap gate independently.

This unlanded record originally used POAM-032 on PR16. Current main already
assigns that identity to prompt-flight absorption; this integration assigns
POAM-037 without rewriting either historical journal.

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
bidirectional engram synchronization. No default hosted endpoint may be
fabricated from the website domain.

Current integration also verified a client gap: the authored knowledge editor
at Connectome revision `8a9eb5e52b8df91b888b75f6c98edabeb8d3199d` is not in
current engine pin `03b716b4dfe5623e6c82408c6f4df367c9795388` or PR114 pin
`38142b77110a8f1db77e306e7d1ad375fd918267`. The old native knowledge bridge,
renderer contract/editor and its tests are absent there. Connectome PR10 is
closed without merge, and its original branch is retained. Its behavior needs
adaptation into the current native session and project-workspace boundaries;
a newer pin or Git ancestry is not replacement evidence. Engine PR16 qualifies
the public configuration API, not a completed knowledge editor in that pin.
The [2026-11-01 pin](../../evidence/change-journals/gate-h/h-06-connectome-chain-pin.md)
does not contain it either. `8a9eb5e5` is not an ancestor of Connectome
`c1cbff203ecee89013878609d11aed04e89d0e71`, and that revision has no
knowledge-tree editor or native knowledge bridge.
The [earlier 2026-11-03 pin candidate](../../evidence/change-journals/gate-h/h-06-connectome-teal-pin.md)
did not contain it either: `8a9eb5e5` is not an ancestor of Connectome
`95ba9975c506886f79984ad956e5dba557369f93`.

### Nomen and first hosted profile

The 2026-10-25 context audit found no Nomen reference in the searched engine
`crates/` or `docs/` at revision `497b911f136c0418651dbf53b5890953ab386d0c`.
That is a missing product binding, not proof that Nomen does not exist elsewhere.
The [integration owner](../../reference/client/devpulse-cloud.md#nomen-identity-and-delivery-priority)
now captures Nomen as central login across products and local bidirectional
sync with shippin.cloud as the first hosted profile. The authoritative Nomen
contract and real account/engine integration remain unverified. H-07a must
resolve that contract before implementation, and H-07b/c/d/g must retain
service/project authorization, revocation, offline continuation and reconnect
proof. Central login is not a second engine authority or permission to share
project context. Custom/self-hosted profiles remain configurable targets.
The owner's follow-up corrects the manual-authorization framing: offer one
sharing/sync setup bundle, create its selected service bindings automatically,
reuse Nomen/provider sessions and resume previously approved work. Automatic
local attachment, saved-choice resumption and actual Claude/Codex subscription
client integration still require H-06/H-07 implementation evidence. A disabled
cloud toggle or documented permission boundary alone does not satisfy this UX.

## Impact

A configured path can otherwise be mistaken for an operational connector.
Users cannot yet provision a turnkey managed engram or prove two-way knowledge
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
Turnkey cloud acceptance also requires authenticated engram identity delivery,
conflict rules, cursors, tombstones, echo suppression, retries, offline recovery,
export and process-restart evidence. Configuration persistence alone cannot
satisfy these conditions. Release and roadmap status remain open.

The [project-directory knowledge path package](../../evidence/change-journals/gate-d/d-05-project-directory-path.md)
records the prior inability to represent a project-local folder in the existing
knowledge-tree catalogue. The candidate adds that configuration binding only;
folder observation, outside-project folders, connector execution and automatic
source-grounded relationship construction remain open.

The [device-name correction](../../evidence/change-journals/gate-d/d-05-project-directory-device-names.md)
records and repairs the first candidate's acceptance of Windows superscript
device aliases. This does not establish filesystem source access or close the
remaining local and external knowledge-source gaps.
