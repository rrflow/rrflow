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
