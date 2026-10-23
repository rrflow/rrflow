# rrflowQL multi-model reads

**Status:** active implementation reference; native persistent access-path convergence remains incomplete
**Coordinate:** `rrflow://rrflow-instance/data/reference/query/multi-model`
**Owner:** implemented rrflowQL source families, temporal coordinates, traversal, and typed-predicate behavior

This record describes current query behavior. It does not declare the native
multi-model, Arrow/DataFusion, or persistence objectives complete. The
[system overview](../../architecture/system-overview.md#native-multi-model-boundary)
owns the accepted boundary, the
[engine data-flow record](../../architecture/engine-data-flow.md#current-implementation-boundary)
owns the present-versus-target distinction, and roadmap
[Gates C, E, and F](../../roadmap/rrflow-1.0.md#gate-c--establish-the-sole-hybrid-persistent-rrflowkv-substrate)
own convergence and evidence.

## Implemented source families

rrflowQL currently accepts:

- `record:<kind>` for bitemporal records;
- `relation:<kind>` for directed typed relations;
- `event:<kind>` for immutable cursor-addressed events;
- `claim` or `claim:<predicate>` for resolved bitemporal claims;
- `series:<kind>` where `<kind>` is the referenced series-record kind;
- `geo:<kind>` where `<kind>` is the geospatial value's reference kind; and
- `traverse:<relation> START <kind>:<id> DIRECTION <OUTGOING|INCOMING|BOTH>
  DEPTH <1..32>` for bounded recursive graph expansion.

The six data families and traversal syntax use explicit `AT VALID` and
`KNOWN` coordinates. Binding uses a captured schema and read stamp, planning
is content-addressed, and execution enforces scan, row, output, and batch
budgets. Series rows expose sample identity, series identity, observation
time, and a typed scalar value. Geospatial rows expose identity, subject,
field, validity, geometry kind, and point or bounding-box coordinates as
canonical decimal values. `PROJECT *` can expose custom properties, while
only the current built-in series and geospatial fields are bindable for
explicit projection and filtering.

Traversal validates the start and relation types against the captured schema,
uses the bitemporal graph snapshot visible at the requested coordinates,
orders relations canonically, returns the first deterministic shortest path to
each reached node, and suppresses cycles with a visited set. Direction and a
maximum depth are mandatory. Each row exposes its depth, reached node, edge,
endpoints, and complete node path.

Filters preserve a typed comparison operator through parsing, binding, plan
digests, and execution. The grammar supports `=`, `!=`, `<`, `<=`, `>`, and
`>=`. Equality and inequality work for every type accepted by the selected
field. Ordering currently fails closed unless both values have the same
integer, unsigned-integer, or string type. Decimal and mixed-type ordering are
rejected during binding. Only `cursor = <unsigned>` selects the exact event
cursor path; other cursor comparisons retain the current authoritative scan.

Exact record/relation ID equality and ordered ID intervals select complete
authenticated identity histories. A uniquely typed relation endpoint selects
its adjacency range and expands complete relation histories before temporal
filtering, preserving moves and retirement. These addressed paths capture
schema metadata without scanning the full source family. Their plan's source
cursor is the requested known prefix, a conservative freshness coordinate like
traversal, rather than the last mutation in an unrelated family. They do not
reuse a family index artifact. Other index/corpus paths retain their actual
family watermarks; MATCH still uses corpus-wide lexical statistics.

The combined `StampedQueryPipeline::run` used by the public query operation
charges schema/source capture and execution's semantic version/proof reads
against one `max_storage_keys` budget and reports their combined evidence.
Exhaustion is reported as resource exhaustion. Separately invoked bind and
execute operations retain their per-call budgets; their caller must compose
those operations explicitly. This semantic-read budget is not a process RSS
ceiling or a claim that unrelated control/index-catalogue work is metered by it.

## Verified implementation boundary

The focused query corpus commits records, relations, events, claims, series,
and geospatial values and proves equivalent results through rrflowMX and
rrflowKV. It covers valid-time exclusion, typed predicates, deterministic
bounded traversal, an exact event-cursor lookup, and rrflowKV close/reopen.
The secured server corpus exercises representative typed rows through the
public query operation.

That evidence proves query semantics, not final physical execution. Current
normal execution can reconstruct runtime state into materialized `QueryRow`
values and allocate new Arrow arrays before DataFusion. Graph, BM25, scalar,
and vector structures are not yet one atomically maintained set of bounded
native rrflowKV access paths. Gates C and E must establish those persistent
paths before Gate F replaces eager snapshots with stamped streams from the
hybrid rrflowKV segment layout.

The current index-catalogue behavior is described by the
[query index catalogue](index-catalogue.md), and the current join and bounded
mutation-program behavior is described by
[transactions and joins](transactions-and-joins.md). Current snapshot-diff
semantics are described by [live queries](live-query.md), and public
subscription delivery is described by the
[durable subscription reference](../protocol/subscriptions.md).
