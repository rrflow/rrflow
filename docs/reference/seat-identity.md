# RRFlow seat identity and memory warps

**Status:** active implemented reference; broader installed specialization remains incomplete
**Coordinate:** `rrflow://rrflow-instance/data/reference/seat-identity`
**Owner:** durable provider-neutral seat identity and `rrflow://` record-warp semantics

An RRFlow seat is the durable identity the engine resolves as self inside one
estate. A provider account may represent a seat, but never replaces or owns
it. Changing Claude, OpenAI, Gemini, Grok, LFG, or another provider therefore
does not change the RRFlow identity.

## Clyffy boundary

**Clyffy is this project's primary RRFlow seat specialization.** It is not a
second product, kernel, repository, daemon, model provider, storage engine,
query engine, or lifecycle authority. Generic RRFlow installation templates do
not hardcode Clyffy; the bundled versioned `clyffy` profile selects that seat
identity explicitly. Another installed project may select a
different seat without changing RRFlow semantics.

The executable Clyffy path must remain first-party Rust inside this repository.
`RrdEngine` owns authentication, authorization, routing orchestration,
deterministic predicates, context intent, compare-and-swap, audit, and commit.
The SWAG-selected `RouterBackend` may return only one of the three bounded router
proposals; [LFG supplies CPU observations](inference/lfg-swag.md).
Go can be installed as an outward SDK, generator, harness, or other
attuned capability; it is not a Clyffy or RRFlow routing, persistence, or
orchestration authority.

## Canonical records and relations

The implemented contract uses three canonical data kinds:

| Kind | Meaning |
|---|---|
| `rrflow-seat` | Provider-independent identity, display name, and purpose. |
| `rrflow-provider-identity` | Stable local provider-identity record containing an opaque provider name and subject digest. |
| `rrflow-represents` | Temporal relation from a provider identity to the seat it represents. |

A seat resolves as self only when at least one visible provider identity has a
valid `rrflow-represents` relation to it at the requested read coordinate.
Provider credentials and provider runtime sessions are never fields of these
records. The bind input hashes the provider subject and persists only its
SHA-256 digest.

Representation establishes durable attribution, not permission. The target
request path authenticates the calling provider or local principal, resolves
that exact identity's visible representation edge at the same `ReadStamp`, and
then evaluates ordinary RRFlow policy for the requested operation. A caller
cannot become Clyffy by supplying an actor string, a seat identifier, or another
provider's subject digest, and a valid representation cannot bypass a denied
operation.

The provider-neutral schemas are implemented in
[`memory_estate.rs`](../../crates/transport/rrd-contract/src/memory_estate.rs),
and their authorized planning and resolution are composed by
[`RrdEngine`](../../crates/authority/rrd-engine/src/engine/memory_estate.rs).

## Stable record warps

Every addressable canonical data record uses this path-safe form:

```text
rrflow://<instance>/data/<record-kind>/<record-id>
```

The URI identifies canonical RRFlow data; it is not a checkout file path and
does not copy record state into Markdown. During bootstrap, a README warp pairs
the durable URI with one local documentation fallback. Once the knowledge
import and persistence gates pass, clients resolve the URI through the engine
while the checkout link remains a recovery path.

The bootstrap coordinate for this project's primary seat is
[`rrflow://rrflow-instance/data/rrflow-seat/clyffy`](rrflow://rrflow-instance/data/rrflow-seat/clyffy).
An installed estate substitutes its actual instance identity in that URI.

Warp resolution validates the URI, converts its record kind and ID into a
canonical anchor, and invokes the same bounded temporal
`RrdEngine::assemble_context` path used by ordinary context requests. It does
not open physical keys or create a second retrieval mechanism.

## CLI operations

No seat-identity CLI operation is currently exposed. The implemented engine
boundary is `RrdEngine::plan_memory_estate`, followed by
the ordinary authenticated transaction commit,
`RrdEngine::resolve_seat_identity`, and `RrdEngine::resolve_memory_warp`. The
planner produces strict seat, provider-identity, and representation-relation
mutations; it owns no second write path. Resolve reads after reopen from the
selected RRFlow storage profile, and warp resolution enters the same bounded
context assembly path as other context requests. None bypasses mutation
authorization or creates provider-owned identity state.

The primary `rrflow` executable intentionally exposes only the implemented
installed lifecycle during D-01 convergence; its former raw `identity` and
`context --warp` commands were removed rather than retained as an alternate
database opener. A future public identity mutation/resolution operation must
begin in the executable operation catalogue and generate CLI/SDK/UI projections
from that owner. Documentation examples cannot make that operation available.

## Executable proof and remaining boundary

The focused engine corpus
[`seat_becomes_self_through_provider_neutral_edges_and_survives_reopen`](../../crates/authority/rrd-engine/src/engine/tests/memory_estate.rs)
binds an identity through an authenticated transaction, proves the persisted
provider value is a subject digest, reopens the persistent engine, resolves the
same seat URI, follows the warp through context assembly, and verifies the
representation-edge evidence. Canonical installation also creates the initial
seat in its one semantic install commit. The deleted raw CLI test is no longer
product evidence.

This is characterization of a useful persisted foundation, not proof of the
complete installed Clyffy path. The current `MemoryEstate*` type and module
names conflate seat identity with an estate; callers still commit the returned mutation plan through the
generic transaction API; resolution reconstructs a broad runtime
snapshot; the older claim path accepts an arbitrary producer actor string; and
the authenticated session, representation edge, authorization decision, route
packet, router proposal, and resulting mutation are not yet bound at one read
coordinate. No `RouterBackend` dispatch exists in the checkout.

The [accepted D-01 foundation](../roadmap/rrflow-1.0/gate-d-evidence.md#d-01-accepted-installed-foundation-2026-10-21)
now binds bundled generic/Clyffy specialization and the initial seat through
the versioned installed record. Full attuned specialization remains open.
The other convergence gaps above are not compatibility promises. A-07/C-03 own canonical
naming and effect-complete identity attribution; G-01 through G-05 own stamped
routing and persisted tree execution; H-04/H-05 own cross-surface identity and
causal evidence. The versioned specialization manifest, project attunement,
provider conformance, and normal rrflowDB-backed documentation resolution
remain owned by the
[agent-bootstrap reference](agent-bootstrap.md),
[roadmap](../roadmap/rrflow-1.0.md), and
[POA&M](../poam/rrflow-1.0-alpha.md).
