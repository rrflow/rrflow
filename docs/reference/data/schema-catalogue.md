# RRFlow logical schema catalogue

**Status:** active implementation reference; physical catalogue convergence remains incomplete
**Coordinate:** `rrflow://rrflow-instance/data/reference/data/schema-catalogue`
**Owner:** canonical logical model, schema mode, and governed mutation validation

`rrd_core::RuntimeSchemaRegistry` is the canonical logical schema catalogue for
one runtime scope. It is installed as `RuntimeMutation::Schema` through the
normal cursor-CAS commit path. `RrdEngine` remains the only transaction and
authorization authority; index, vector-collection, and other physical
projection catalogues may describe access structures but cannot redefine a
table's logical identity or independently publish canonical data.

## Logical models and table identity

Each registry names one namespace and database; both default to `default`, and
the [instance-topology record](../../architecture/instance-topology.md#canonical-identities-and-cardinality)
reserves any other value for a later schema contract. A table kind maps to one
tagged `RuntimeLogicalModel`: document, relational, graph node, graph
relation, key-value, vector, event, time-series, geo, object, reasoning claim,
reasoning record, reasoning event, lifecycle record, or lifecycle event.

Record-like models share `RuntimeRecord` identity, graph relations use
`RuntimeRelation`, events use the commit cursor for immutable identity, and
the vector, series, geo, object, and claim families retain their typed value
contracts. The logical-model tag determines the permitted mutation family; it
does not create a separate engine per data model.

## Strict and schemaless modes

`RuntimeSchemaMode` is explicit:

- `strict` validates declared property types and required fields. Specialized
  record, relation, and event schemas additionally govern uniqueness,
  endpoints, cardinality, subjects, and additional-property policy.
- `schemaless` accepts an already bounded property bag and cannot also declare
  a strict property contract or specialized strict schema.

A table kind occurs once in the effective catalogue. A model/family mismatch,
ambiguous strict/schemaless definition, missing governed table, invalid
property, dangling target, uniqueness conflict, or cardinality violation fails
before the store advances the runtime cursor.

## Commit, snapshot, and reopen behavior

Schema and data mutations can share one `RuntimeCommit`. Schema revisions are
positive and consecutive, and writes are validated against the resulting
catalogue before publication. The rrflowMX and rrflowKV conformance fixtures
prove equal mixed-model create, update, retire, recreate, and conflict
behavior. rrflowKV fixtures also prove the registry and resulting data
snapshot survive close and reopen.

`RuntimeDataSnapshot` reduces records, relations, events, vectors, series, geo
values, and objects selected from authenticated semantic-version point/range
reads at one valid time and `ReadStamp`. Each value obtains its model only from
the registry's explicit table map. A missing type fails closed; the mutation
family or caller cannot supply a fallback model.

## Required 1.0 convergence

The kernel `RuntimeSchemaRegistry` and public `DataSchemaRegistry` now require
an explicit nonempty `tables` map. Specialized record, relation, and event maps
add strict family constraints but cannot create a table or infer a model. The
kernel's paired `define_record_table`, `define_relation_table`, and
`define_event_table` methods author those two pieces together and reject a
cross-family collision. Persisted or public JSON with an omitted table map,
an empty table map, a specialized schema without its matching strict table, or
a runtime value whose kind is absent from the map is rejected.

This closes the schema-authority portion of C-05. C-05g closes the
collectionless vector representation, and C-05h removes generic quantized
publication plus the duplicate TurboQuant request and suppression branch.
Gate C-03 already commits records, both graph adjacency directions,
synchronous index changes, the runtime entry, and durable projection deltas as
one physical batch. Gate C-04 already replaced normal whole-log snapshot
reconstruction with versioned keys at one `ReadStamp`. Gates E and F must make native graph,
scalar, BM25, vector, and Arrow/DataFusion paths consume that same catalogue
and stamp. Until those gates pass, the logical catalogue is real and
persistent, but it is not proof that RRFlow's multimodal storage and recall
paths are physically unified or optimized.

## Structure and input contract plan

The following is the design contract for completing existing owners, not a
claim that the missing operations or fields are implemented. The
[research rationale](../../architecture/engine-scale-and-contracts.md) explains
the choices. Stable effect boundaries are closed and versioned; unfamiliar
semantic intent remains representable as bounded inert data until a capability
explicitly admits it. No schema governs private model reasoning.

| Contract family | Canonical producer and structure | Required semantic checks beyond JSON shape | Owning implementation |
|---|---|---|---|
| Public request and response | Existing `rrd-contract` operation types and executable catalogue; version, operation identity, typed payload, error/outcome and receipt projection | Authenticate the actual caller; resource scope, catalogue/profile negotiation, idempotency digest and deadlines; reject unknown effects before dispatch | H-04 |
| Logical data | Kernel `RuntimeSchemaRegistry`, projected public `DataSchemaRegistry`; explicit table/model/mode and specialized property/relation/event constraints | Consecutive revision, atomic schema/data commit, required/extra fields, uniqueness, relation endpoints, historical stamp and prepared-plan invalidation | C/E/F, H-08b |
| Model and recipe | Existing `RouterBackendDescriptor`, model manifest and `ReasoningRecipe`; immutable revisions and artifact/input/output-schema digests | Resolve exact verified bytes, runtime/device capability, network permission and executable limits before loading; a schema digest must resolve to its authorized content | G-01/G-03 |
| Route input | Existing `RouteStepRequest`; descriptor identity, active cursor, bounded intent/signals, eligible recipes/branches, context allowance, allowed decisions and deadline | Extend the same packet with same-stamp principal/seat/representation/policy binding; verify candidate membership, read/source freshness, budgets and sorted canonical collections | G-02 |
| Route output | Existing tagged `RouteStepDecision`: `select_recipe`, `advance_branch`, `request_context` | Validate response/input/descriptor binding, allowed recipe/edge/parameters and deterministic predicates; the model cannot select physical storage or authorize mutation | G-03/G-04/G-05 |
| Durable reasoning step | Existing kernel tree/cursor and engine transaction/job boundaries; expected cursor, bounded public input/result/evidence references, dispatch identity, budget usage, fence and outcome receipt | CAS/fencing, atomic state/audit/outbox, exact replay and uncertain outcomes; extend current types rather than creating a client-owned scheduler or generic JSON state blob | G-02/G-04; generic external activities remain I-03 |
| Multimodal source | Existing source/object/vector references plus admitted extractor/model capability; media type, digest, source revision, encoded and expanded size/duration/dimension limits | Content digest and permission before decode; bounded decompression/tensors, declared modality, finite values and model-compatible vector dimension/normalization | D-05/E, G-01/H-01 |
| Sync transfer | Future admitted H-07 public operation types; binding/consent revision, source/origin commit, schema profile, full atomic group, snapshot/cursor, digest and destination receipt | Reauthorize at destination; atomic origin deduplication and writes; detect conflict/retention gap; tombstones and revocation; no foreign raw KV import | H-07a/H-07b/H-07c |
| Diagnostic and benchmark evidence | Kernel signal catalogue projected outward; evaluation-owned manifest and result types described in Gate J | Closed dimensions, sanitized fields, build/corpus/config identity, complete sample accounting; evidence is not lifecycle state | H-05/J-04 |
| LFG observations and SWAG policy | [Inference contract](../inference/lfg-swag.md); extend existing endpoint/configuration and route boundaries with separately versioned analysis, selection and cache-scope values | Typed units/uncertainty, effective-policy digest, candidate membership, hard locality/cost/resource limits, authorization at dispatch and cross-scope cache denial | D-06, G-01/G-03/G-06 |
| Connector requirements and availability | [Knowledge connector contract](knowledge-connectors.md); extend executable capability catalogue and binding types, generate all discovery surfaces | Distinguish implemented/installed/configured/authorized/healthy/admitted; required secrets by reference, exact effects, freshness, source ACL and revocation; recheck at invocation | D-06, H-04 |
| Knowledge source and derivation | Existing source/evidence/knowledge family schemas with source revision, chunk locator, transformation and model identity, references and publication receipt | ACL before retrieval/expansion; bounded decode; atomic publication, replay deduplication, deletion invalidation and compatible vector space; Parquet is explicit exchange only | D-05/E/F, H-09/H-10 |

Existing route ceilings are already executable in `rrd-contract/src/router.rs`
(512 KiB request, 128 KiB response, 32 KiB intent and 60,000 ms maximum declared
execution). These are contract ceilings, not recommended defaults or proof of
runtime enforcement. Generate their schema projection from that owner and
intersect installed estate, backend and request limits at admission. Byte
limits apply to encoded UTF-8/decoded resources as appropriate; JSON string
length alone cannot enforce them.

For every completed family, the owning PR must:

1. Identify the one authoritative Rust type/validator and any necessary
   kernel/public mapping; generate OpenAPI/JSON Schema, SDK models and UI
   discovery from the established catalogue. Do not manually maintain another
   version of the same model. Keep generated output replaceable.
2. Pin contract version, canonical encoding/digest rules, required/optional/null
   distinctions, finite numeric ranges, units and unknown-field behavior.
   Enforce recursive depth, item count, encoded bytes and aggregate allocation
   before unbounded decoding or hashing.
3. Test golden acceptance and malformed/oversized/unknown-version/unknown-tag
   denial through both the generated surface and semantic validator. Include
   wrong digest, stale stamp, schema change, cross-estate references, unauthorized
   fields and required-but-missing data. JSON Schema `format` annotations do not
   establish these properties.
4. Prove schema-change atomicity, historical reads and cache/prepared-plan
   invalidation. Pre-release convergence removes conflicting representations
   directly under repository traceability rules. Future released-version
   evolution needs an explicit compatibility decision and export/restore
   rehearsal; it cannot silently reinterpret persisted bytes.

The [Gate G package table](../../roadmap/rrflow-1.0/gate-g.md#bounded-reasoning-implementation-packages)
owns reasoning implementation order. H-07/H-08 retain their existing package
DAGs. This table introduces no executable operation, API endpoint or new
catalogue authority by itself.
