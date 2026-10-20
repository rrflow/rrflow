# rrflowDB capability record

**Status:** active target contract; the estate, temporal graph, claims, adjacency index, native traversal, prompt flights, and task records exist, while prompt graph families, the memory contract gap, and project links are planned
**Coordinate:** `rrflow://rrflow-instance/data/reference/data/rrflowdb-capability`
**Owner:** the planned AI memory capabilities of rrflowDB as the per-project estate: prompt graph families, the memory model, and opt-in links between projects; PostgreSQL compatibility is owned by the [PostgreSQL contract](../protocol/postgresql.md)

rrflowDB is the per-project AI estate: one project's governed knowledge,
memory, temporal graph, reasoning state, evidence, vectors, and index
definitions, composed by `RrdEngine` and made durable by rrflowKV
([system overview](../../architecture/system-overview.md#canonical-component-terminology),
[instance topology](../../architecture/instance-topology.md)). This record
fixes what "a Postgres-like database built in Rust and dedicated to AI
memory" means for RRFlow, which pieces exist, and the order and proof by which
the rest is built. It owns no delivery status. The
[roadmap](../../roadmap/rrflow-1.0.md) owns order and completion; items
H-09 and H-10 in [Gate H](../../roadmap/rrflow-1.0/gate-h.md) carry the work
this record describes. SQL access is not planned here: the
[PostgreSQL compatibility contract](../protocol/postgresql.md) owns its scope,
semantics, and qualification, and H-08 owns its packages.

## Owner decisions this record carries

| Date | Decision | Effect here |
|---|---|---|
| 2026-10-17 | One project ↔ one estate (rrflowDB) ↔ one RRD instance; an account owns many estates; a workspace groups estates above the engine. | Every capability below is per project. Nothing merges two estates. |
| 2026-10-18 | "The pure Rust Postgres DB for AI" is rrflowDB with a Postgres-wire SQL face: one adapter over the same estate, not a second database and not Postgres as persistence. | Owned by the [PostgreSQL compatibility contract](../protocol/postgresql.md) and H-08; see [SQL access](#sql-access). |
| 2026-10-19 | Knowledge and memory are the same graph family. | [Memory model](#memory-model). |
| 2026-10-19 | The graphs a prompt forms are configurable so they can be tested and experimented with. | [Prompt graph families](#prompt-graph-families). |
| 2026-10-19 | Relationships between projects are opt-in links: each project stays its own rrflowDB, a link is explicitly allowed, queries may follow it read-only, and every use is audited. | [Project links](#project-links). |

## What "Postgres-like" means, and what it does not

rrflowDB borrows Postgres's operator experience, not its storage or process
model. It is one Rust engine with one mutation authority
([ADR-0001](../../decisions/0001-single-engine-authority.md)).

| Postgres property | rrflowDB equivalent | State |
|---|---|---|
| ACID transactions | One `RuntimeCommit` per transaction, CAS against the authenticated read stamp's cursor, all-or-none across every family | Implemented (C-02, C-03) |
| MVCC snapshot reads | Every read is bound to one authenticated `ReadStamp`; historical stamps are proved against the RFC 9162 accumulator | Implemented (C-04) |
| Time travel | Bi-temporal: valid time and known cursor on every read (`AT VALID … KNOWN …`) | Implemented; Postgres has no equivalent |
| Typed schema and DDL | `RuntimeSchemaRegistry`: record, relation, event, vector, series, geo, and object tables with logical models, revisioned per scope ([schema catalogue](schema-catalogue.md)) | Implemented |
| B-tree and unique indexes | Scalar and unique entries written with each record mutation | Write side implemented; native reader is E-02 |
| pgvector | Native collections, named dense/sparse/multi vectors, payload filters, exact and HNSW search, quantization | Implemented in `rrd-vector`; native generations are E-04 |
| Apache AGE (graph) | Typed temporal relations with outgoing and incoming adjacency; rrflowQL `traverse:` reads native adjacency ranges | Adjacency written (C-03); rrflowQL traversal reads it natively (E-01a, pull request #21); context expansion still reads a snapshot |
| Full-text search | BM25 with highlights | Implemented over artifacts; incremental postings are E-03 |
| SQL over the wire | The rrflowDB PostgreSQL core profile | Planned, H-08 ([PostgreSQL contract](../protocol/postgresql.md)) |
| Roles and grants | `RrdEngine` sessions, policies, and audited authorization | Implemented |
| Extensions, PL/pgSQL, server-side triggers, logical replication | Not offered. Triggers, routines, and functions are explicit engine capabilities (Gate I); sync is per estate | Out of scope |

## Memory model

Knowledge and memory are one family: memory is what the estate knows,
with the time it became valid and the time the estate learned it. There is no
separate memory store
([system overview](../../architecture/system-overview.md#canonical-component-terminology)).

| Memory kind | Stored as | State |
|---|---|---|
| Semantic (what is known) | Records, typed relations, and bi-temporal claims | Implemented |
| Episodic (what happened) | Immutable events, prompt flights and their edges | Events and prompt flights implemented (H-06a–H-06c, pull requests #17–#19) |
| Procedural (how to act) | LFG category tree and revisions, rules, exemplars, recipes, skills, verdicts, and model outcomes per address, each as governed records | Planned with Gate G and Gate I |
| Consolidation | Claim tiers `local` → `primary` → `tenant`; promotion `unpromoted` → `pending` → `promoted` or `denied`; corrections supersede by digest | Kernel fields exist; nothing runs promotion (RROFLOW) |

Gap to close in H-09: the public `AssertClaim` mutation carries subject,
predicate, object, `valid_from`, `tx_time`, producer, and confidence only.
It cannot state `valid_to`, tier, promotion, or the claim it supersedes, so a
caller cannot write a correction or a promotion that the kernel already
models.

## Prompt graph families

A prompt graph family is a named, versioned set of record kinds and relation
kinds that one producer writes when a prompt is submitted. Families are
typed relation families inside the one temporal graph, never separate graph
databases, and rrflowQL traverses them with the same native adjacency reader.

### Default set

Eight families ship as the default configuration. The owner's notes name
lexicon, evidence, source, knowledge, and governance; classification,
reasoning, and plan/task come from the same notes (LFG notes v2, v2.1, v3;
the Notion RRO and LFG records). Knowledge includes memory.

| Family | What a prompt adds | Producer | Producer state |
|---|---|---|---|
| Lexicon | Prompt phrasing → canonical label; alias → label; unresolved phrasing → the inconclusive bucket | LFG lexicon | Absent; authored outside the repo |
| Classification | Prompt → address path: one node per Gate, Choice, Score, Extract, and Compose answer with value, probability, confidence, tier, and provenance (`rule@rN`, encoder, or model) | LFG kernel and extension (G-01, G-03) | Absent |
| Reasoning | Prompt → reasoning-tree run: branches taken, recipes, conditions, verification nodes | Persisted tree execution (G-04) | Contract only (A-02) |
| Plan / task | Named task → its prompts; RRO goal → tasks → parallel branches → output contract | Task records and plan compilation | Task records implemented (H-06c, pull request #19); plan compilation planned |
| Evidence | Prompt → each record its context used, with rank, fused score, and retrieval avenues; results, citations, and artifact references | Prompt flights | Flight-used edges with rank, fused score, and avenues implemented (H-06b, pull request #18); the registry-owned family is planned (M3) |
| Source | Record or answer → where it came from: file and revision, document and date, commit, operator correction, or model | Project inventory and ingestion (D-03, D-05) and commit provenance | Commits carry actor and digest; no source-node vocabulary |
| Knowledge (memory) | Records, relations, and claims the prompt read, asserted, corrected, or promoted | Ingestion, claim writes, consolidation | Records, relations, and claims exist; see the claim gap above |
| Governance | Checks run, verdicts (pass, fail, inconclusive), permission-scope decisions, and the actions they allowed or refused | Governance verification and permission gate | Absent |

### Configuration and experiments

- **Definitions** live in the schema registry. A family definition names its
  record and relation kinds, its producer capability, its retention, and its
  revision. Changing a definition is a schema revision through `RrdEngine`.
- **Activation** lives in the estate configuration revision (the
  UI-managed revision of build-order step 2). It selects which families a
  flight forms, per project, with no hardcoded list: the default set is the
  bundled initial revision.
- **Experiments** compare configurations without changing either. An
  experiment declares two or more arms (each an activation revision), an
  assignment rule, a rubric, a minimum sample, and a threshold. Each flight
  records assignment, exposure, and outcome as separate events, so "assigned
  but never formed" stays distinguishable from "formed". A configuration is
  promoted only when its grade meets the threshold **and** its sample meets
  the minimum; promotion is an authorized, reversible configuration revision.
- **Formation is atomic.** One prompt submission forms every active family
  whose producer has output in the flight's own commit, at the flight's read
  stamp. A family whose producer is absent or disabled is recorded as skipped
  with the reason, never silently missing.
- **Every edge carries provenance**: producer, revision, tier, and
  probability where the producer is probabilistic. A family is replayable
  from its committed inputs.

## SQL access

PostgreSQL clients reach rrflowDB only through the
[PostgreSQL compatibility contract](../protocol/postgresql.md), which owns
scope, type and transaction semantics, the wire adapter inside `rrd-server`,
the AI SQL surface, and qualification; H-08a to H-08h own its order. This
record adds no SQL behavior. Prompt graph families and project links appear to
SQL only through the relational and function surface that contract admits.

## Project links

Each project keeps its own rrflowDB, instance, credentials, and authority. A
link lets one project read another, never write it.

- **A link is two governed records**: an offer in the source estate naming
  the target estate's instance and estate identities, the families and kinds
  it exposes, direction (read-only), expiry, and approver; and an acceptance
  in the target estate. Either side revokes by retiring its record.
- **Following a link is a federated read** at the workspace layer
  ([instance topology](../../architecture/instance-topology.md)): two stamped
  reads, one per estate, each authorized by its own engine. The result
  carries both read stamps and both evidence sets. There is no cross-estate
  transaction and no shared cursor.
- **Cross-project edges are foreign references**: a record in the source
  estate naming a target estate and a record identity there. Resolving it
  requires an active link; without one it resolves to "unavailable", never to
  a guess.
- **Every use is audited** in both estates.
- **Placement is independent.** Linked projects may share a Shippin.cloud
  group or run in separate VMs; links travel over H-07 endpoint resolution.

## Current facts

| Fact | Evidence |
|---|---|
| One estate per project, rrflowKV storage, one authority | [instance topology](../../architecture/instance-topology.md), ADR-0001 |
| Typed temporal relations with both adjacency directions written atomically | C-03; [multi-model contract](multi-model-object-contract.md) |
| rrflowQL traversal reads native adjacency ranges and matches an independent full-replay oracle | Implemented: E-01a, pull request #21 |
| Claims are bi-temporal with tier, promotion, confidence, and supersession in the kernel | `rrd-core/src/claim.rs`; `ClaimChangeSnapshot` exposes them on reads |
| The public claim mutation omits `valid_to`, tier, promotion, and supersession | `TransactionMutation::AssertClaim` in `rrd-contract` |
| No project-link record and no graph-family registry; PostgreSQL compatibility is planned under H-08 | Repository search at this record's revision |
| DataFusion 55 is embedded in `rrd-query` and evaluates stamped Arrow batches | `crates/compute/rrd-query/Cargo.toml`; OBJ-05 |

## Build order

Each step is one pull request with its own oracle. Steps inside a column can
start once their dependency lands; the columns run in parallel. SQL access
builds separately as H-08a to H-08h.

| Step | Work | Depends on | Proof |
|---|---|---|---|
| M1 | This record and roadmap items H-09 and H-10 | — | Documentation, navigation, and inventory checks |
| M2 | Graph-family definitions in the schema registry, activation in the configuration revision, the bundled default set | Build-order step 2 (configuration revisions) | Golden registry fixture; activation revision round trip; unknown family rejected |
| M3 | Flight commits form every active family atomically with per-edge provenance; evidence and plan/task move onto the registry | M2, H-06a–H-06c (pull requests #17–#19) | One commit per flight; skipped families recorded with reason; rrflowQL walks each family; replay is identical |
| M4 | Memory contract: `AssertClaim` gains `valid_to`, tier, promotion, and supersession; memory reads by subject, predicate, and time | — | Correction supersedes without loss; promotion is reversible; old stamps unchanged |
| M5 | Experiment arms: assignment, exposure, and outcome events; promotion by grade and sample | M2, M3 | A seeded two-arm fixture promotes only past both thresholds |
| M6 | Source family from the committed project tree and commit provenance | D-03, D-05 | Every ingested record resolves to its file and revision |
| M7 | Lexicon and classification producers | G-01, G-03 | Golden address and lexicon fixtures per prompt |
| M8 | Reasoning and governance producers | G-04, Governance verification | Verdicts and branches visible per prompt; inconclusive gates as fail |
| L1 | Link offer and acceptance records, revocation, audit | H-04 | Two-party opt-in; revocation stops resolution immediately |
| L2 | Federated read across a link at the workspace layer | L1, H-07 | Result carries both stamps; the target's denial surfaces as unavailable |
| L3 | Connectome shows links and drills across them | L2, H-06 | Owner can open a linked project's graph read-only |

## Proof standards

- Every family and link read reproduces the same answer at the same read
  stamp after close and reopen, on rrflowMX and rrflowKV.
- Physical evidence follows the work requested: a traversal follows visited
  edges.
- No performance or superiority claim against Postgres, pgvector, AGE,
  SurrealDB, Pinecone, Qdrant, or HydraDB is made until J-04's comparative evidence
  passes ([OBJ-10](../../objectives/rrflow-1.0-alpha.md)).

## Open decisions

| Decision | Options | Blocks |
|---|---|---|
| Who may accept a link when the owner of both projects is the same account | Explicit acceptance every time; or account-level standing approval | L1 |
| Whether prompt graph families may be added at runtime by an operator, or only by a schema revision shipped with a release | Operator-defined families; or bundled families only | M2 |
| Default retention per family (for example, keep lexicon and classification nodes forever but compact evidence after N days) | Per-family policy values | M2 |

## Sources

- Owner statements in the Rrflow project, 2026-10-17 to 2026-10-19.
- LFG_NOTES_v3 (2026-09-26), Notes v2 and v2.1 (2026-09-25), Session Notes (2026-09-24).
- Notion: RRO, LFG, and "RRO → LFG routing pipeline" records (2026-08-16/17).
