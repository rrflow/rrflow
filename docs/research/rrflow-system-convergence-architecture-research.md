# RRFlow 1.0 system-convergence architecture research

**Status:** active supporting research; informs but does not override accepted architecture or roadmap status
**Coordinate:** `rrflow://rrflow-instance/data/research/rrflow-system-convergence`
**Owner:** primary-source evidence for the RRFlow 1.0 execution map
**Audience:** RRFlow owner and engineers executing the 1.0 pre-release gates
**Date:** 2026-09-12
**Scope:** the current RRFlow repository, the path from rrflowMX through rrflowKV and Arrow/DataFusion, native graph/lexical/vector access, deterministic project-tree inventory, installation/attunement, provider-neutral agent context, explicit automation, external project data adapters, and production observability/diagnostic/fault evidence
**Assumptions:** one RRFlow instance per project/environment; `RrdEngine` is the sole semantic, authorization, and mutation authority; version remains `1.0.0`; current code is inventory until the roadmap's behavioral evidence passes; the repository owner reports separate source-use rights for SurrealDB and Qdrant, whose legal scope is not adjudicated by this technical record

This is dated research, not a live implementation inventory or a release
order. Its original snapshot and source observations remain useful history;
current direction and status are owned by the [roadmap](../roadmap/rrflow-1.0.md)
and [indexing implementation plan](../roadmap/rrflow-1.0/project-indexing-execution-plan.md).
In particular, its earlier fixed-phase and full-snapshot assumptions are not
requirements for every request or effect.

## Direct answer

The repository is not a start-over. It contains substantial WAL, MVCC,
manifest, snapshot, query, DataFusion, BM25, HNSW, quantization, transport,
security, and runtime code. It is also not a true alpha yet. The central gaps
are integration and physical semantics: the Fjall selector and runtime are
already absent, although retained historical evidence still names its original
comparison profiles; C-05 has removed the executable pre-1.0 batch, manifest,
segment, and vector-catalogue readers; C-03 commits both graph directions and
the synchronous scalar/unique/BM25/vector source deltas atomically; and C-04
uses authenticated direct semantic reads rather than normal cursor-zero log
reconstruction. The remaining hot-path gaps are that graph, lexical, and
vector serving still rebuild broad in-memory projections; query execution
eagerly materializes `Vec<QueryRow>` before Arrow; DataFusion's provider wraps
that materialization instead of streaming rrflowKV pages; live query
evaluation reruns two snapshots. At the research baseline, installation and
attunement had contracts but no persisted executor; later main added persisted
jobs and Connect/Inventory execution while leaving other executors unavailable.

The correct convergence target is a hybrid storage engine, not a generic
row-only LSM and not “Arrow everywhere”:

1. `RrdEngine` authenticates, authorizes, binds one `ReadStamp`, validates
   semantic invariants, owns compare-and-swap, and is the only component that
   may request a commit.
2. rrflowMX and rrflowKV implement the same narrow transactional storage port.
   rrflowMX is volatile. rrflowKV owns WAL, MVCC, ordered point/range access,
   manifests, immutable segments, compaction, recovery, and durability.
3. rrflowKV keeps an ordered binary key/version spine. At flush and compaction,
   it also builds Arrow-compatible immutable column pages with explicit schema,
   encoding, compression, checksum, and lifetime metadata. Hot mutation state
   remains key-optimized; immutable scan state becomes columnar.
4. rrflowQL parses and binds semantic queries. Native graph, BM25, scalar,
   exact-vector, HNSW, and RRF operators choose bounded access paths at one
   stamp. DataFusion consumes streaming Arrow batches for analytical work; it
   never owns authorization, canonical state, transactions, or direct commits.
5. HNSW and quantization are replaceable projections over exact canonical
   vectors. Approximate candidates are filtered and exactly reranked; exact
   search remains the oracle and fallback.
6. LFG or another model adapter receives bounded, authorized inputs and may
   propose decisions supported by the versioned engine contract. The current
   three proposal types are an implementation baseline, not an exhaustive
   permanent vocabulary. The adapter cannot choose raw storage keys, physical
   indexes, authorization, or mutations.
7. PostgreSQL, Turso, Dragonfly, SQL stores, and project databases are
   discovered as governed external sources/adapters. They are never selected
   implicitly as rrflowDB persistence and never become a parallel RRFlow
   authority.
8. Project discovery is adaptive. For persisted source facts or reproducible
   effects, filesystem observations enter through an engine-authorized read
   plan and `RrdEngine` commits validated records with the relevant source
   identity. An effect whose complete dependency boundary is narrower does not
   require an unrelated full-project snapshot; a whole-project completeness
   claim does. Recheck dependencies and replan safely after relevant edits.
9. One causal operation graph follows work across ingress, authorization,
   storage, native indexes, Arrow/DataFusion, inference, attunement, routines,
   and delivery. Durable RRFlow evidence and process-local/exported telemetry
   describe that same work; neither becomes job, routine, or mutation state.

## Evidence reconciliation

### One executable dependency spine

The reviewed alphabetical `A -> B -> C -> D -> E -> F` ordering was not a
valid implementation dependency graph. A-06 listed database import and client
warp milestones that require later D/H/J behavior, and had blocked B until the
knowledge-bootstrap review completed. D-05 also names lexical, vector, and
graph attunement phases before E/F supply their accepted persistent access
paths. Executing that order would either deadlock the roadmap or build
temporary parallel paths.

The researched dependency spine is:

1. finish the checkout knowledge topology through KB-05, then freeze source,
   package, public, and trace vocabulary in A-07;
2. finish the provider-neutral contracts in B;
3. establish the one transactional rrflowKV/rrflowMX substrate in C;
4. implement install, durable jobs, deterministic inventory, and incremental
   parsing in D-01 through D-04;
5. implement native graph, scalar, BM25, exact-vector, HNSW, and planner access
   paths in E;
6. stream stamped rrflowKV batches into native Arrow operators and DataFusion
   with honest pushdown and one resource budget in F;
7. integrate the D-05 capabilities selected for each authorized project or
   request against the accepted C/E/F paths, then close the remaining
   deployment portions of D; this is not a mandatory phase sequence;
8. import and read back the deterministic knowledge package through those
   persisted attunement paths;
9. connect LFG, dynamic context, feedback, public delivery, and Connectome in
   G/H; then build explicit events, triggers, routines, skills, and host
   adapters in I; and
10. qualify the self-contained candidate through J-03, make rrflowDB the normal
    client warp-resolution path in KB-08, then complete comparative evidence
    and the signed distribution in J-04/J-05.

Trace vocabulary is frozen during A-07, but tracing is not postponed until H.
Each C-through-I work package must add the bounded spans, causal coordinates,
physical counters, denial/error outcome, and failure-path evidence for the
behavior it introduces. H-05 closes cross-surface completeness, propagation,
export, and redaction.

### Transaction and storage boundary

SurrealDB's current architecture documentation describes one query layer over
transactional point/range storage, with document, graph, and index changes in
the same transaction and snapshot isolation plus write-conflict detection.
That supports RRFlow's single-engine direction, but it does not prove RRFlow's
implementation. RRFlow must retain its own exact differential, crash, and
reopen tests. [SurrealDB architecture](https://surrealdb.com/docs/learn/data-models/architecture)

FoundationDB's tuple layer demonstrates why an ordered byte encoding must
preserve tuple order, and its transaction documentation makes conflict ranges
and optimistic commit behavior explicit. RRFlow should borrow those contract
properties, not claim FoundationDB's strict serializability. The 1.0 target
remains the declared snapshot isolation plus write-conflict behavior until a
stronger contract is separately accepted and proven. [FoundationDB developer guide](https://apple.github.io/foundationdb/developer-guide.html)

### Source-level organization and integration lessons

SurrealDB's core is not one giant implementation file and it is not a set of
independent databases. Its current source tree separates database execution,
document mutation, physical execution, indexes, typed keys, and transactional
KV behavior into `dbs`, `doc`, `exec`, `idx`, `key`, and `kvs` modules under
one core. That supports RRFlow's grouped kernel/persistence/compute/authority
tree: modular source boundaries are healthy when one transaction coordinator
connects them. The present RRFlow problem is missing end-to-end semantics and
duplicate compatibility paths, not the mere existence of crates.
[SurrealDB core source](https://github.com/surrealdb/surrealdb/tree/93ab219d69f09d8f999851b0359c80ebe6726102/surrealdb/core/src)

SurrealDB's current graph-key module documents four adjacency keys for one
relation. The vertex-side pointer keys embed the opposite endpoint so a
directional range scan can resolve it without fetching the edge record, while
edge-side keys preserve endpoint adjacency. The useful lesson is typed,
ordered, direction-specific key families and atomic maintenance—not its exact
wire bytes or its superseded pre-release decoder. RRFlow is pre-release and must
freeze its own key codec, write the required directional entries in the same
semantic transaction, and retain no earlier-format branch.
[SurrealDB graph-key source](https://github.com/surrealdb/surrealdb/blob/93ab219d69f09d8f999851b0359c80ebe6726102/surrealdb/core/src/key/graph/mod.rs)

The SurrealDB transaction source keeps transaction-local caches, changefeed
and live-event buffers, and pending index-build state around the underlying
transactor. This demonstrates an important boundary: derived work and
notifications are coordinated with transaction outcome, rather than being
declared successful by an external hook. RRFlow's corresponding state belongs
behind `RrdEngine` and the rrflowKV commit receipt.
[SurrealDB transaction source](https://github.com/surrealdb/surrealdb/blob/93ab219d69f09d8f999851b0359c80ebe6726102/surrealdb/core/src/kvs/tx.rs)

SurrealDB's MCP implementation is described as a thin adapter over its
datastore, using the same authentication and query limits. That is directly
applicable: RRFlow MCP exposes or invokes bounded public capabilities, but
cannot become a context, routine, or persistence authority.
[SurrealDB MCP source](https://github.com/surrealdb/surrealdb/tree/93ab219d69f09d8f999851b0359c80ebe6726102/surrealdb/mcp)

Qdrant's source separates segment-local ID tracking, payload storage and
indexes, vector storage, vector indexes, quantization, and segment construction
inside its `segment` library, with collection/storage/WAL concerns above it.
Its appendable-versus-immutable segment operations and immutable index-build
tests are valuable implementation references for RRFlow's exact-vector source
plus replaceable HNSW/TurboQuant generations. They do not justify a second
Qdrant process, a second catalogue, or copying Qdrant compatibility paths into
RRFlow.
[Qdrant v1.19.1 libraries](https://github.com/qdrant/qdrant/tree/6ab21cac18ebb6f4ae29102c7f8f5cc11affd5de/lib),
[Qdrant v1.19.1 segment source](https://github.com/qdrant/qdrant/tree/6ab21cac18ebb6f4ae29102c7f8f5cc11affd5de/lib/segment/src),
[source-pinned RRFlow reference](qdrant-capability-inventory.md)

### Deterministic project discovery before parsing

Git's ignore specification is precise: ignore patterns target intentionally
untracked files, tracked files are unaffected, nested rules have defined
precedence, and later matching rules at the same level win. A generic walker
that merely reads `.gitignore` can therefore omit tracked files incorrectly.
RRFlow must merge the tracked set with Git-correct untracked eligibility when
Git is present, while retaining a complete non-Git path.
[Git ignore specification](https://git-scm.com/docs/gitignore)

Git's blob/tree object model is also a useful precedent for deterministic
project identity: content objects are addressed by digest and tree objects
bind names, modes, and child identities. RRFlow should use its own versioned
digest envelope and canonical path encoding, but the same Merkle property lets
an unchanged subtree retain identity and bounds incremental work.
[Git object model](https://git-scm.com/book/en/v2/Git-Internals-Git-Objects.html)

Rust's `ignore::WalkBuilder` provides Git-style ignore support, deterministic
sorting, depth and file-size controls, parallel walking, and disabled symlink
following by default. It is a candidate enumeration primitive, not the
semantic contract: RRFlow still needs tracked-file reconciliation, mount/root
safety, race detection, secret-before-open policy, resource accounting, and a
deterministic proposal independent of walker scheduling.
[`ignore::WalkBuilder`](https://docs.rs/ignore/latest/ignore/struct.WalkBuilder.html)

Tree-sitter documents incremental reparsing by editing the prior tree and
passing it with the new source, allowing unchanged structure to be reused. That
supports a parse phase driven by a committed source change set; it does not
replace inventory and cannot decide whether filesystem bytes are authoritative.
[Tree-sitter advanced parsing](https://tree-sitter.github.io/tree-sitter/using-parsers/3-advanced-parsing.html)

Filesystem notification libraries explicitly expose rescan-required events
because watches can lose fidelity. Notifications must therefore be scheduling
hints: explicit refresh remains sufficient in Gate D, and after Gate I they may
submit an engine event requesting another authoritative inventory diff. They
cannot directly patch the graph, syntax tree, or indexes.
[`notify` rescan flag](https://docs.rs/notify/latest/notify/event/enum.Flag.html)

SCIP offers a language-neutral schema for symbols, occurrences, definitions,
references, and documentation. It is useful interoperability evidence for the
later normalize/entity-link design, but RRFlow must not require a SCIP indexer
for the first inventory phase or let language tooling become canonical project
state.
[SCIP specification](https://github.com/scip-code/scip/blob/main/scip.proto)

The reviewed SurrealDB agent guidance now provides install instructions for
provider-specific skills and MCP, plus a coding-agent memory example that can
recursively ingest a documentation directory and recall/remember/reflect at a
repository scope. Those are useful deployment and user-flow references. They
do not specify a deterministic whole-project tree snapshot, Git tracked-file
reconciliation, filesystem error/race semantics, resource-bounded incremental
change set, or a transaction that gates parsing and index construction. RRFlow
must own that missing attunement boundary instead of mistaking MCP setup or
folder upload for project understanding.
[SurrealDB agent setup](https://surrealdb.com/docs/agents),
[SurrealDB coding-agent memory](https://surrealdb.com/docs/agent-memory/cookbooks/build/coding-agent-with-project-memory)

Qdrant's data-management guidance begins with caller-supplied points, vectors,
payloads, collections, and index configuration. Its LlamaIndex integration
explicitly delegates ingestion to another framework. This is strong evidence
for segment-local vector/index mechanics, but not a project discovery or
attunement contract. RRFlow uses those search mechanics inside its one engine
only after source inventory, parsing, grounding, and canonical vector commits.
[Qdrant data management](https://qdrant.tech/documentation/manage-data/),
[Qdrant LlamaIndex integration](https://qdrant.tech/documentation/frameworks/llama-index/)

Sourcegraph's auto-indexing is a useful operational comparison: it selects a
commit, runs language indexers in an executor sandbox, records job activity,
and publishes a code-graph index. RRFlow should borrow explicit job policy,
sandboxing, and observable failure, while replacing clone/upload authority with
the locally committed project-tree snapshot and phase checkpoints. An
optional SCIP-producing analyzer is a later attuned activity, never the first
inventory step.
[Sourcegraph auto-indexing](https://sourcegraph.com/docs/code-navigation/auto-indexing)

### Hybrid LSM and Arrow pages

Arrow specifies an in-memory columnar layout optimized for locality,
vectorization, and eligible zero-copy sharing, with alignment requirements.
It does not provide RRFlow's WAL, concurrency control, key ordering, manifest,
or mutable transaction semantics. [Arrow columnar format](https://arrow.apache.org/docs/format/Columnar.html)

Lance's file-format documentation provides useful page-level precedents:
independent per-column pages, random-access row ranges, explicit page metadata,
and aligned buffers, while leaving table semantics and search structures to
higher layers. RRFlow should use those principles as a comparison, not adopt
Lance as an authority or promise universal zero-copy. [Lance file format](https://github.com/lance-format/lance/blob/main/docs/src/format/file/index.md)

Published columnar-LSM work shows a sound transition point: use LSM flush and
compaction to transform mutable records into immutable columnar components.
This supports retaining a key/version-oriented WAL and memtable while producing
column pages in immutable rrflowKV segments. It also means schema evolution,
write amplification, point-read amplification, and compaction cost must be
measured rather than assumed away. [Columnar Formats for Schemaless LSM-based Document Stores](https://www.vldb.org/pvldb/vol15/p2085-alkowaileet.pdf)

“Zero-copy” is therefore conditional. An uncompressed, aligned, type-compatible
page with a pinned mapped lifetime may be borrowed. Compressed, encrypted,
misaligned, evolved-schema, dictionary-remapped, or otherwise incompatible
pages require decoding or copying into a budgeted Arrow pool. Every path must
report physical bytes read, decoded bytes, copied bytes, allocated bytes, and
borrowed bytes.

### DataFusion boundary

DataFusion's custom-provider guidance states that data must be fetched during
physical execution, not planning, or optimizer pushdown and resource controls
cannot reduce work. `TableProvider::scan` builds a lightweight
`ExecutionPlan`; `ExecutionPlan::execute` constructs each partition stream;
the polled `RecordBatchStream` performs storage I/O and produces batches. The
provider also declares each filter as exact, inexact, or unsupported. That
directly rejects the current eager `Vec<QueryRow>` snapshot as the 1.0
endpoint. [DataFusion custom table providers](https://datafusion.apache.org/library-user-guide/custom-table-providers.html)

DataFusion exposes bounded memory pools and spill behavior, but those controls
cover DataFusion reservations, not every native graph/vector/storage allocation.
RRFlow must enforce one cross-operator budget in `RrdEngine` and propagate it
to storage scans, native operators, and DataFusion. [DataFusion memory pools](https://docs.rs/datafusion/latest/datafusion/execution/memory_pool/trait.MemoryPool.html)

Moka is not required by Arrow or DataFusion and is not currently a workspace
dependency. Its weighted-capacity concurrent cache is a viable implementation
candidate only for Gate F-05 after measurement. The decision criterion is a
byte-bounded hit-rate/latency benchmark plus exact `ReadStamp`, schema, and
catalogue invalidation. A cache must never hold canonical state or conceal
stale results. [Moka crate documentation](https://docs.rs/moka/latest/moka/)

### Trace and diagnostic boundary

OpenTelemetry models a trace as low-cardinality named spans with parentage,
attributes, timestamped events, links, and status. W3C Trace Context defines
the interoperable `traceparent` and optional `tracestate` propagation fields.
Those are the correct outward diagnostic conventions; they do not define
RRFlow's durable state machine. [OpenTelemetry tracing API](https://opentelemetry.io/docs/specs/otel/trace/api/),
[W3C Trace Context](https://www.w3.org/TR/trace-context/)

RRFlow therefore needs two coordinated representations, not two authorities:

- the durable `RuntimeTraceEvent` record is bounded causal evidence linked to
  exact read, plan, projection, reasoning, source, and commit coordinates; it
  survives restart and may honestly retain an unmatched start after a crash;
- Rust `tracing` spans and an optional OpenTelemetry exporter are diagnostic
  projections of the same operation. They may be sampled or unavailable and
  can never establish that a mutation, attunement phase, or routine completed.

At ingress, RRD validates and continues an incoming W3C context or creates a
new one. `RrdEngine` binds that context to the authenticated request
correlation, actor, engram/scope, authorization decision, and `ReadStamp`.
Child work keeps the trace identity. Asynchronous work caused by a committed
event, projection delta, retry, or routine activity records a causal link
rather than inventing false synchronous parentage.

Durable operation names use one bounded, low-cardinality machine vocabulary:
`rrflow.<boundary>.<operation>`. Boundaries are `ingress`, `engine`, `kv`,
`ql`, `graph`, `lexical`, `vector`, `datafusion`, `inference`, `attunement`,
`routine`, `adapter`, and `delivery`. Dynamic scope, record, query, provider,
model, path, and error values never enter the operation name; they are typed
links, bounded attributes, or digests. Outward HTTP and database-client spans
also retain the applicable OpenTelemetry semantic attributes. OpenTelemetry's
database convention likewise requires low-cardinality operation names and
warns that query text can be high-cardinality and sensitive.
[OpenTelemetry database span conventions](https://opentelemetry.io/docs/specs/semconv/db/database-spans/)

The current trace foundation is useful but incomplete. It already has W3C-
width identifiers, parent IDs, bounded attributes, typed causal links,
start/annotation/finish phases, rrflowMX/rrflowKV equivalence, conflict retry,
and crash-visible incomplete starts. It does not parse or propagate
`traceparent`/`tracestate`, has no OpenTelemetry bridge, uses mixed unprefixed
operation names, exposes direct-store persistence helpers, and does not cover
the complete graph/BM25/vector/Arrow/DataFusion/context path. A-07 freezes the
mapping; each owning gate converges its names and instrumentation; H-05 proves
the final causal chain and redaction.

### Production observability, diagnostic builds, and latency evidence

The industry pattern is not a separate debug engine. Libraries instrument
work; the process binary installs subscribers and exporters; an optimized
diagnostic build retains the same product features and execution semantics as
the release build while preserving symbols and enabling runtime-selected
diagnostics. Rust `tracing` explicitly separates instrumentation from the
subscriber that records it, and Cargo custom profiles can inherit release
optimization while changing debug information and stripping independently.
Tokio Console is valuable for task/resource/waker diagnosis, but it requires
experimental Tokio instrumentation and therefore belongs in an explicitly
non-conformance diagnostic lane rather than the default release claim.
[^obs-1] [^obs-2] [^obs-3]

OpenTelemetry supplies a coherent signal model rather than three unrelated
logging systems. Trace and span IDs correlate logs with spans; metric exemplars
can carry the trace/span coordinates for a sampled observation; and resource
attributes identify the same service/build across signals. The database span
conventions deliberately keep `db.query.summary` low-cardinality, treat query
text as sensitive, and make parameter capture opt-in. RRFlow should apply the
same rule to rrflowQL, prompts, context, source paths, model output, credentials,
and external payloads: static operation names plus bounded attributes and
digests by default, with protected content absent rather than merely hidden in
the UI. [^obs-4] [^obs-5] [^obs-6]

Metrics need a separate contract from durable trace attributes. Google SRE's
four golden signals are latency, traffic, errors, and saturation, and its SLO
guidance warns that averages hide tail behavior. Prometheus histograms can be
aggregated across processes whereas client-computed summary quantiles cannot.
OpenTelemetry additionally requires an explicit cardinality limit and overflow
handling; its exponential histogram is appropriate for database latency that
spans microseconds through seconds. RRFlow therefore needs histograms, counters,
and gauges with a closed low-cardinality attribute set; it must report success
and failure latency separately and retain raw distributions for p50, p95, p99,
and p99.9 analysis. [^obs-7] [^obs-8] [^obs-9]

The useful database precedent is stage-level work accounting. DataFusion
already exposes each physical operator's `elapsed_compute`, output rows,
batches, and bytes through `ExecutionPlan::metrics` and `EXPLAIN ANALYZE`.
RocksDB's opt-in `PerfContext` and `IOStatsContext` split logical database work
from filesystem work and expose cache, block-read, key-comparison, seek, WAL,
flush, and compaction costs for the current operation. Qdrant exposes health,
readiness, Prometheus metrics, and a separate detailed telemetry endpoint;
SurrealDB exposes logs, metrics, and traces through OpenTelemetry-compatible
configuration. RRFlow should absorb these operational properties into one
native evidence path, not copy their endpoint or source topology and not let
DataFusion or an exporter become transaction authority. [^obs-10] [^obs-11]
[^obs-12] [^obs-13]

Latency evidence must define the clocked boundary before it reports a number.
For a public request the end-to-end boundary is accepted ingress through final
response byte or durable acknowledgement; each child span measures only its
own stage. Runs separate cold and warm cache, rrflowMX and rrflowKV, success and
failure, request class, input-size band, concurrency, and durability policy.
They bind the exact binary/build identity, corpus, seed, hardware, filesystem,
device, configuration digest, warm-up, sample count, and failed samples.
Coordinated-omission-safe load generation or correction is required whenever a
closed-loop client could otherwise hide pauses. HdrHistogram is a practical
reference representation because it covers a wide dynamic range and documents
coordinated-omission correction; it is not required as an RRFlow dependency.
[^obs-14]

Debuggability also requires destructive-boundary testing, not just spans.
FoundationDB demonstrates deterministic simulation with recorded seeds and
injected network/disk/process faults. RocksDB's stress practice combines
randomized operations, reopen verification, kill testing, white-box I/O
failure points, and sanitizer variants. In Rust, Loom can explore bounded
concurrent schedules, Miri detects classes of undefined behavior, and the
compiler supplies sanitizer builds; their documented limitations mean none is
a substitute for real-process crash/reopen, ENOSPC, corruption, and sustained
load tests. Every retained failure must identify its seed/input, exact build,
fault point, and post-reopen verification result. [^obs-15] [^obs-16]
[^obs-17] [^obs-18] [^obs-19]

Repository controls and runtime automation are different systems. Git states
that client hooks are not copied by clone, so a pre-commit hook cannot be the
project's authority. The portable pattern is a checked-in deterministic
presubmit command, executed by CI, reduced to one required status check, and
made non-bypassable with protected-branch/ruleset policy when the hosting tier
supports it. GitHub rulesets can require pull requests, reviews, status checks,
and block force pushes, but that external setting must be queried and recorded;
workflow YAML cannot prove it. This repository's 2026-09-11 API probe returned
HTTP 403 for both rulesets and branch protection on the private development
repository, so server-side enforcement is currently unproven. That gap cannot
be papered over with an editor/provider hook or confused with Gate I's future
engine events, triggers, routines, and skills. [^obs-20] [^obs-21]

Finally, diagnostic output is evidence only when it is reproducible and bound
to the artifact that emitted it. A capture bundle should include build commit
and tree, target, toolchain, profile, feature closure, schema/key/page format
identities, sanitized configuration digest, workload/seed/fault manifest,
traces, metric snapshot, DataFusion physical plan and operator metrics,
rrflowKV physical counters, process CPU/RSS/I/O, clock source, and explicit
omissions. Release artifacts later bind those identities to SBOM and SLSA
provenance. A bundle never contains plaintext secrets, prompts, source bodies,
raw query parameters, unrestricted paths, vectors, or hidden chain-of-thought.
[^obs-22]

[^obs-1]: [`tracing` crate documentation](https://docs.rs/tracing/latest/tracing/), instrumentation/subscriber separation and disabled-span behavior.
[^obs-2]: [Cargo profiles](https://doc.rust-lang.org/cargo/reference/profiles.html), inheritable custom profiles and independent debug/strip/optimization controls.
[^obs-3]: [Tokio tracing next steps](https://tokio.rs/tokio/topics/tracing-next-steps) and [`console-subscriber` builder](https://docs.rs/console-subscriber/latest/console_subscriber/struct.Builder.html), async-runtime diagnostics and prerequisites.
[^obs-4]: [OpenTelemetry logs data model](https://opentelemetry.io/docs/specs/otel/logs/), trace/log correlation and resource context.
[^obs-5]: [OpenTelemetry metrics data model](https://opentelemetry.io/docs/specs/otel/metrics/data-model/), exemplars and trace/span association.
[^obs-6]: [OpenTelemetry database client spans](https://opentelemetry.io/docs/specs/semconv/db/database-spans/), low-cardinality summaries and sensitive query-data rules.
[^obs-7]: [Google SRE: Monitoring Distributed Systems](https://sre.google/sre-book/monitoring-distributed-systems/), four golden signals and successful/failed latency separation.
[^obs-8]: [Google SRE: Service Level Objectives](https://sre.google/sre-book/service-level-objectives/), tail distributions and standardized indicator definitions.
[^obs-9]: [Prometheus histogram and summary practices](https://prometheus.io/docs/practices/histograms/) and [OpenTelemetry Metrics SDK](https://opentelemetry.io/docs/specs/otel/metrics/sdk/), aggregation, cardinality limits, and exponential histograms.
[^obs-10]: [DataFusion `EXPLAIN` and operator metrics](https://datafusion.apache.org/user-guide/explain-usage.html) and [metric definitions](https://datafusion.apache.org/user-guide/metrics.html), physical-plan evidence.
[^obs-11]: [RocksDB Perf Context and I/O Stats Context](https://github.com/facebook/rocksdb/wiki/Perf-Context-and-IO-Stats-Context), scoped logical and physical work counters.
[^obs-12]: [Qdrant monitoring](https://qdrant.tech/documentation/operations/monitoring/), health/readiness, metrics, and telemetry surfaces.
[^obs-13]: [SurrealDB observability](https://surrealdb.com/docs/manage/observability), coordinated logs, metrics, traces, Prometheus, and OTLP export.
[^obs-14]: [HdrHistogram](https://github.com/HdrHistogram/HdrHistogram), high-dynamic-range recording and coordinated-omission correction support.
[^obs-15]: [FoundationDB testing](https://apple.github.io/foundationdb/testing.html) and [client testing](https://apple.github.io/foundationdb/client-testing.html), deterministic simulation, seed replay, and fault injection.
[^obs-16]: [RocksDB stress tests](https://github.com/facebook/rocksdb/wiki/Stress-test), randomized, crash, white-box, verification, and sanitizer practices.
[^obs-17]: [Loom](https://github.com/tokio-rs/loom), bounded concurrency permutation testing and limitations.
[^obs-18]: [Miri](https://github.com/rust-lang/miri), undefined-behavior detection scope and limitations.
[^obs-19]: [Rust sanitizers](https://doc.rust-lang.org/beta/unstable-book/compiler-flags/sanitizer.html), compiler-supported instrumentation modes.
[^obs-20]: [Git hooks](https://git-scm.com/book/en/v2/Customizing-Git-Git-Hooks), including the non-propagation of client-side hooks.
[^obs-21]: [GitHub repository rulesets](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-rulesets/available-rules-for-rulesets), pull-request, review, status-check, and force-push controls.
[^obs-22]: [SLSA provenance v1.2](https://slsa.dev/spec/v1.2/provenance), artifact-to-build provenance model.

### Vector, filtering, and fusion

The HNSW paper establishes the approximate multi-layer navigable graph design;
it does not supply transactional freshness, filtered-query correctness, or an
exactness guarantee. [HNSW paper](https://arxiv.org/abs/1603.09320)

Qdrant's current indexing guidance explains why payload indexes feed
cardinality estimates, why full scan can outperform HNSW below a threshold,
why filter-aware HNSW depends on payload indexes, and why strict mode can reject
unsafe unindexed filters. RRFlow should retain exact fallback, require index
freshness/source-cursor evidence, and make the access decision inside the
planner. [Qdrant indexing](https://qdrant.tech/documentation/manage-data/indexing/)

BM25 and reciprocal rank fusion remain distinct operations: BM25 is an exact
lexical scoring model over persisted term/document statistics; RRF combines
ranked result lists and must be a pure query-time function. A verified outcome
may later update a versioned policy for future stamps, but a query must not
rewrite its own weights. [BM25 review](https://www.staff.city.ac.uk/~sbrp622/papers/foundations_bm25_review.pdf), [RRF paper](https://research.google/pubs/reciprocal-rank-fusion-outperforms-condorcet-and-individual-rank-learning-methods/)

### Attunement and provider-neutral context

Tree-sitter is explicitly incremental and robust to incomplete source, so it
fits a digest- and grammar-revision-bound parse phase. Parse errors are data to
record and ground, not a reason to pretend a project was fully attuned.
[Tree-sitter introduction](https://tree-sitter.github.io/)

Provider instructions must remain thin host adapters. Codex discovers scoped
`AGENTS.md` files; Claude documents importing `AGENTS.md` from `CLAUDE.md` to
avoid duplication; Gemini supports hierarchical context and `@file` imports.
These sources support the repository's forwarding stubs and reject provider
files as lifecycle or memory authorities. [OpenAI Codex agent loop](https://openai.com/index/unrolling-the-codex-agent-loop/), [Claude project memory](https://code.claude.com/docs/en/memory), [Gemini context files](https://geminicli.com/docs/cli/gemini-md/)

Dragonfly is a Redis/Memcached-compatible external in-memory datastore; Turso
and libSQL are application database technologies with their own replication
and consistency rules. Their presence in a discovered project should create a
governed source descriptor and optional adapter, never replace rrflowMX or
rrflowKV automatically. [Dragonfly documentation](https://www.dragonflydb.io/docs), [Turso embedded replicas](https://docs.turso.tech/features/embedded-replicas/introduction)

### DevForge placement and release evidence

Linux OverlayFS permits a shared read-only lower layer and a per-instance
writable upper/work directory, but the upper and work directories have
filesystem requirements and crash behavior still depends on explicit durable
writes. RRFlow's WAL, manifest, segments, and mutable catalogue must be wholly
inside one instance's upper layer; lower tools/models are immutable inputs
identified by digest. [Linux OverlayFS documentation](https://docs.kernel.org/filesystems/overlayfs.html)

Release artifacts need verifiable provenance describing where, when, and how
they were produced. That supports Gate J's signed artifact, SBOM, and clean
machine verification rather than a bare successful local build.
[SLSA provenance](https://slsa.dev/spec/v1.2/provenance)

### 2026-09-12 critical-path implementation audit

This audit is bound to commit
`b9c46c35febf81d548ec4b06ef9184df988ae693`, tree
`ae41e9acf71bcf60a68b35c63554b8ba8151376e`. It reviewed every source line in
`rrd-lsm` (8,125 lines), `rrd-query` (8,302 lines), and `rrd-vector` (10,480
lines), plus the complete graph/index/versioned-read/semantic-commit modules in
`rrd-store` and the complete context, retrieval, query, vector-search,
vector-catalogue, read-evidence, data-plane, and trace paths in `rrd-engine`.
The generated file plan remains the complete tracked-tree line/digest ledger;
this narrower audit does not falsely claim that unrelated engine, transport,
SDK, or operations files were manually reviewed.

The smallest existing package suites passed at that baseline: all 87
`rrd-lsm` tests, all `rrd-store`, `rrd-query`, and `rrd-vector` tests, and
strict package Clippy for the audited storage/query/vector packages. Those
results characterize existing behavior. They do not prove bounded graph
execution, streaming rrflowKV-to-Arrow reads, production vector indexing,
crash safety beyond the exercised faults, a turnkey database, or alpha
readiness.

| Boundary | What is real and worth retaining | What is not yet the claimed engine | Required package before rewrite |
|---|---|---|---|
| rrflowKV / `rrd-lsm` | framed WAL and fail-closed recovery; MVCC snapshots and write conflicts; manifest/CURRENT publication; immutable v4 segment spine plus six 64-byte-aligned Arrow-compatible pages; checksums, mapped-owner pinning, bounded/cache/io_uring modes, compaction and physical counters; C-06g owned/pinned generations, synchronous selective projected stream, exact global MVCC/tombstone merge, active-manifest GC retention, bounded Arrow-compatible output, and distinct segment-open/startup-reconciliation/query evidence | pages are uncompressed and the projected stream is not an asynchronous DataFusion `RecordBatch` provider; no measured key/value separation or family-specific placement; whole-segment hashing is on open; mutation-free inspection, ENOSPC/crash/lifetime/fuzz/long-run proof, and a fixed-hardware policy comparison remain incomplete | complete C-06h adversarial/property/fuzz qualification and C-06i measured physical-policy selection, then D-01 and C-07; do not replace the LSM with DataFusion or add another store |
| Temporal graph write path | typed current/history and incoming/outgoing key families in `keyspaces.rs`; `semantic_commit.rs` updates both adjacency directions in the same semantic batch and has reopen/parity characterization | `rrd-query::execute` reconstructs relation history and performs a linear scan of all relations for each visited vertex; `RrdEngine::context` rebuilds an adjacency map from a broad snapshot. That is correct small-corpus behavior, not native bounded graph execution | E-01 adds direction/range-specific cursor scans, temporal visibility, cancellation, work budgets, physical evidence, and MX/KV/reopen differential proof |
| Scalar and lexical indexes | transactional scalar/unique/BM25 source deltas, typed stamps, BM25 scoring code, and catalogue integrity exist | uniqueness/schema checks still scan broad state; BM25 materialization is whole-map JSON; so-called incremental reconciliation computes a whole old/new map diff; tokenizer stemming is simplistic and one offset error path is silently defaulted | E-02 and E-03 freeze typed segment/postings formats, analyzer identity, deletion/generation semantics, direct iterators, exact oracle, and malformed/reopen/resource tests |
| Vector index | exact scoring is a useful oracle; filtering, HNSW construction, immutable artifact digests, quantization lifecycle, mmap compact dense segments, exact rerank, catalogue integrity, and CPU-first accelerator admission are substantive prototypes | canonical candidates are rebuilt from history into process-local vectors; HNSW and several catalogues are monolithic JSON loaded or cloned wholesale; filtered HNSW traversal does not use payload indexes for pruning; compact rows are aligned f32 bytes rather than Arrow arrays; per-candidate allocations and query transforms remain; durable collection/catalogue code also exists inside the compute crate | E-04 moves durable vector truth and generations under the store/engine transaction while keeping `rrd-vector` compute-only; E-05 selects bounded filtered paths; F-03 streams candidates into native operators |
| `TurboQuant` | randomized orthogonal transform, Lloyd-Max scalar codes, deterministic artifacts, exact comparison, recall tests, and typed generation lifecycle are useful experimental mechanics | current code has no one-bit QJL residual estimator, so it does not implement the full published TurboQuant product estimator; padded one-bit coordinates are not QJL; the name and unbiasedness/superiority implications are therefore unqualified. Query rotation and code unpacking also repeat per candidate | E-04 must either implement and independently test the complete estimator or rename the current codec directly; benchmark exact, current codec, published TurboQuant, and a serious alternative before selection; no compatibility alias |
| rrflowQL / DataFusion | real DataFusion 55 execution, Arrow 59 batches, memory-pool/spill configuration, timeout/output limits, operator metrics, stamped planning, durable query spans, and a lower synchronous pinned projected-storage stream suitable for adaptation exist | `ArrowSnapshot` still first materializes all `QueryRow`s and `MemorySource` wraps them; rrflowQL does not consume the C-06g stream; storage work occurs before the provider stream; synchronous calls create a runtime or helper thread; storage/native allocations are outside the DataFusion pool; every selected access path still enters DataFusion | F-01 adapts the C-06g stream behind a stamped `TableProvider`/`ExecutionPlan`/`SendableRecordBatchStream`; F-02 proves exact/inexact/unsupported pushdown; F-03 composes native operators; F-04 owns one request resource ledger and cancellation |
| `RrdEngine` context and evidence | one composition boundary exists; reads carry stamps; vector catalogue objects and records are atomically bound; durable spans have typed causal links and crash-visible incomplete state | context rebuilds BM25, vectors, and relation adjacency per request and uses fixed RRF weights; vector runtime/catalogue reopening replays broad history; durable trace start/finish records cause their own commits and CAS rebasing, which can perturb hot-path cursors and must be budgeted rather than mistaken for free telemetry | E/F first make access paths real; H-01/H-02 then select eligible avenues and persist reasoning/feedback; H-05 proves one causal trace with bounded overhead and an outward OpenTelemetry projection |

The graph verdict is therefore precise: its transactional representation is a
useful base, but its current executor is not competitive. The LSM verdict is
also precise: it is serious pre-alpha storage code, not a fabricated wrapper,
but it is not yet a production database or a demonstrated advantage over
RocksDB, Lance, Qdrant, or another engine. Package tests and Clippy justify
retention while the named gates replace broad paths; they do not justify a
performance or readiness claim.

The current DataFusion guidance requires `TableProvider::scan` and
`ExecutionPlan::execute` to remain lightweight, with storage work performed as
the returned `SendableRecordBatchStream` is polled. It also makes projection,
filter, limit, partition, cancellation, and resource behavior explicit. This
directly determines F-01/F-02 and rejects merely wrapping an eager vector in a
`MemorySource` as the final design.
[DataFusion custom table providers](https://datafusion.apache.org/library-user-guide/custom-table-providers.html)

Arrow defines an interoperable physical memory layout and permits eligible
zero-copy relocation; it does not define a database, WAL, MVCC, mutation
coordinator, or index. RRFlow may borrow a mapped page only when alignment,
encoding, schema, and owner lifetime agree. Otherwise it must decode or copy
under the resource ledger and report that fact.
[Arrow columnar format](https://arrow.apache.org/docs/format/Columnar.html)

Current SurrealDB graph execution computes directional key ranges and decodes
adjacency entries inside a physical scan operator. The relevant lesson is not
its bytes or API: E-01 needs RRFlow-owned typed direction/range scans that
remain permission-aware and cancellation-aware rather than reconstructing the
whole graph.
[SurrealDB graph-key scan source](https://github.com/surrealdb/surrealdb/blob/main/surrealdb/core/src/exec/operators/scan/graph_keys.rs)

Tantivy provides a useful lexical-segment comparison: immutable snapshot-held
segments, separate deletion bitsets, a term dictionary pointing to posting
offsets, and delta/bit-packed blocks of 128 document IDs. RRFlow can adapt
those behavior and failure principles into E-03, but cannot outsource its
transaction, stamp, analyzer, or catalogue authority to Tantivy.
[Tantivy architecture](https://github.com/quickwit-oss/tantivy/blob/main/ARCHITECTURE.md)

The published TurboQuant algorithm combines an MSE-oriented quantizer with a
one-bit Quantized Johnson-Lindenstrauss residual estimator for the product
objective. Qdrant's current Turbo storage is additionally a fixed encoded file
with separate mutable deletion flags and mmap/io_uring access, not a cloned
JSON graph. These are research and differential references for E-04, not proof
that RRFlow's present similarly named module implements the same algorithm.
[TurboQuant paper](https://arxiv.org/abs/2504.19874),
[Qdrant Turbo vector storage](https://github.com/qdrant/qdrant/blob/master/lib/segment/src/vector_storage/turbo/turbo_vector_storage.rs)

WiscKey establishes that separating large values from the sorted LSM key spine
can reduce compaction write amplification, but it also introduces value-log
garbage collection and workload-dependent tradeoffs. LSM-VEC proposes an
LSM-distributed on-disk proximity graph with sampling and reordering. Both
remain C-06/E-04 benchmark candidates until RRFlow measures recovery, garbage
collection, read amplification, recall, memory, and device writes; neither is
an accepted format merely because it fits the narrative.
[WiscKey paper](https://www.usenix.org/system/files/conference/fast16/fast16-papers-lu.pdf),
[LSM-VEC paper](https://arxiv.org/abs/2505.17152)

The deleted `rrd-graph` and recall implementations were also reviewed from Git
history. Reusable requirements are deterministic project topology,
Tree-sitter-derived symbols, digest-based incremental freshness,
grounding/quarantine, provenance-preserving bitemporal recall sets, explicit
budget/truncation evidence, and A/B recall evaluation. Their direct filesystem
authority, hardcoded language/task/runner tables, FNV identity, monolithic JSON
projection, isolated PageRank router, fixed weights, and process-local
lifecycle are rejected. Those behaviors enter D-03/D-04, E-01, H-01/H-02, and
J-04 through new RRFlow-owned tests; the deleted crate is not restored or
renamed.

## Current-code gap matrix

The historical matrix below reflects the research baseline, not a live status
source. In particular, its original attunement row was superseded by the
[current indexing implementation plan](../roadmap/rrflow-1.0/project-indexing-execution-plan.md).

| Claim | Current evidence | Missing proof | Roadmap owner |
|---|---|---|---|
| rrflowKV is persistent | `rrd-lsm` has WAL, MVCC versions, manifest/CURRENT, immutable segments, recovery, snapshots, compaction, and failure injection | one final format, no earlier-format readers, transaction conflicts, hybrid column pages, crash matrix | C-01..C-07 |
| rrflowMX and rrflowKV share semantics | `RrflowMxStore`, `RrflowKvStore`, and the common `StorageEngine` trait exist | minimal transaction port and identical conformance corpus including conflict/rollback | C-02 |
| semantic writes are atomic | accepted C-03 batches runtime data, current/history graph plus both adjacency directions, scalar/unique/BM25/vector source deltas, log, outbox, cursor, audit, and outcome | keep this corpus green while E builds replaceable projection generations and native readers from the same committed source cursor | C-03 accepted; E-01..E-04 preserve it |
| reads are direct and stamped | accepted C-04 uses authenticated direct semantic-version reads with budgets and MX/KV/reopen equality; C-06g adds one bounded pinned selective physical stream with exact MVCC and operation-scoped evidence | specialized graph/postings/vector iterators and the asynchronous stamped DataFusion provider must replace remaining broad projection reconstruction | C-04/C-06g accepted evidence; E-01..E-05, F-01 |
| no compatibility backend | Fjall selection/dependency, migration runtime, and executable pre-1.0 batch/manifest/segment/vector-catalogue readers are absent; the native current formats are exclusive | retain all negative guards and complete J-01's final source/distribution closure without adding an alias or reader | C-05 accepted; J-01 preserves it |
| DataFusion is integrated | query execution uses DataFusion, `MemorySource`, spill pool, timeout, and output limits; rrflowKV separately has the C-06g projected physical stream | adapt that storage stream into a real stamped DataFusion provider; prove pushdown and cross-operator resource accounting | F-01, F-02, F-04 |
| native graph/BM25/vector are real | graph traversal, BM25 code, exact vector oracle, HNSW, catalogues, and planner exist | persistent incremental access paths and same-stamp native physical operators | E-01..E-05, F-03 |
| causal traces are durable | bounded trace contract, typed links, atomic runtime-log persistence, MX/KV equivalence, conflict retry, and crash-visible incomplete spans exist | one authorized engine emission path, W3C ingress/egress propagation, canonical low-cardinality names, per-gate physical evidence, export/redaction, and complete context-flow correlation | A-07, C..I, H-05, J-02 |
| production diagnostics are reproducible | selected binaries install Rust `tracing` subscribers; rrflowKV and DataFusion expose scattered local counters | optimized diagnostic profile with release-semantic parity; build identity; closed metric instruments/attributes; latency protocol; correlated log/trace/metric export; overhead/cardinality/self-telemetry limits; sanitized capture bundle; deterministic fault and profiler lanes | C-06/C-07, F-04, H-05, J-02, J-04, J-05 |
| dynamic context works | engine context/retrieval functions and RRF helpers exist | planner-selected eligible avenues with selected/skipped evidence, pure RRF, versioned feedback | H-01, H-02 |
| live delivery works | durable subscriptions and WebSocket delivery exist | commit-impact predicate deltas; current semantic live query reruns two snapshots | H-03 |
| install/attunement works | At the historical baseline, strict B-01 plan/job/checkpoint contracts and a fixed eleven-phase list existed. Later main has persisted jobs plus Connect and Inventory execution; the fixed list remains a defect, not a target. | Capability-selected plans, safe source-generation publication, installed Parse and index/query/restart proof remain open; consult the linked implementation plan for exact revision and current failure evidence. | D-01..D-10 |
| LFG is pluggable | embedding inference, the B-02 router wire contract, and B-03 manifest/handshake/byte admission exist | executable `RouterBackend`, LFG adapter, constrained decode, route execution | G-01..G-06 |
| automation is governed | bounded synchronous functions and proposed-transaction bindings exist | direct vocabulary convergence, canonical event envelope, persisted post-commit trigger conditions, resumable routine graphs, skill packages, explicit removable host-event adapters, and replacement of the false package-hook lifecycle | A-07, I-01..I-07 |

## Decisions and exclusions

- Keep and converge the real implementations; do not restore every deleted or
  superseded fragment and do not start a parallel engine.
- Do not import SurrealDB, Qdrant, Lance, Turso, Dragonfly, or DataFusion as a
  second source of truth. Use documented properties as acceptance references.
- Where repository-owner authorization permits source adaptation, map the
  exact upstream behavior and provenance into one canonical RRFlow boundary,
  replace its naming and authority assumptions, and require RRFlow-owned
  contract/differential/failure evidence before deleting the prior path. Do
  not vendor an upstream engine as a hidden second runtime.
- Do not claim universal zero-copy, sub-millisecond latency, superior
  performance, production readiness, or complete context flow before the named
  tests and fixed-hardware evidence exist.
- Do not add Moka during architecture cleanup. Gate F-05 owns a measured
  build-or-reject decision.
- Do not add GraphQL, SDK, Connectome, LFG, trigger, routine, skill, mesh, or
  provider-hook behavior before its preceding contract/storage/query gates.
- Do not change the `1.0.0` version during convergence.

## Claim-to-source ledger

| Claim family | Source | Publisher / author | Date or version | URL | Access note |
|---|---|---|---|---|---|
| Arrow layout and eligible zero-copy | Arrow Columnar Format | Apache Arrow | v25.0.1, accessed 2026-09-05 | https://arrow.apache.org/docs/format/Columnar.html | Official specification |
| Column pages and alignment precedent | Lance File Format | Lance project | main, accessed 2026-09-05 | https://github.com/lance-format/lance/blob/main/docs/src/format/file/index.md | Official format documentation |
| Columnar transformation at LSM events | Columnar Formats for Schemaless LSM-based Document Stores | Alkowaileet et al., PVLDB | 2022 | https://www.vldb.org/pvldb/vol15/p2085-alkowaileet.pdf | Peer-reviewed paper |
| Streaming provider and pushdown | Custom Table Provider | Apache DataFusion | accessed 2026-09-05 | https://datafusion.apache.org/library-user-guide/custom-table-providers.html | Official documentation |
| Memory-pool and spill semantics | `MemoryPool` | Apache DataFusion docs.rs build | 55.0.0, accessed 2026-09-05 | https://docs.rs/datafusion/latest/datafusion/execution/memory_pool/trait.MemoryPool.html | Official crate API documentation; matches the pinned workspace release |
| Trace spans, parentage, events, links, attributes, and status | Tracing API | OpenTelemetry | stable API, accessed 2026-09-07 | https://opentelemetry.io/docs/specs/otel/trace/api/ | Official specification |
| Low-cardinality database operation names and bounded query evidence | Semantic conventions for database client spans | OpenTelemetry | stable unless noted, accessed 2026-09-07 | https://opentelemetry.io/docs/specs/semconv/db/database-spans/ | Official specification |
| Cross-process trace propagation | Trace Context | W3C | Recommendation, accessed 2026-09-07 | https://www.w3.org/TR/trace-context/ | Web standard |
| Trace/log/resource correlation | Logs Data Model | OpenTelemetry | stable specification, accessed 2026-09-11 | https://opentelemetry.io/docs/specs/otel/logs/ | Official specification |
| Metric exemplars and stream identity | Metrics Data Model | OpenTelemetry | stable specification, accessed 2026-09-11 | https://opentelemetry.io/docs/specs/otel/metrics/data-model/ | Official specification |
| Metric cardinality limits and exponential histograms | Metrics SDK | OpenTelemetry | stable specification, accessed 2026-09-11 | https://opentelemetry.io/docs/specs/otel/metrics/sdk/ | Official specification |
| Rust instrumentation/subscriber boundary | `tracing` | Tokio contributors / docs.rs | 0.1.41, accessed 2026-09-11 | https://docs.rs/tracing/latest/tracing/ | Official crate documentation |
| Release-semantic diagnostic profiles | Profiles | Rust Cargo | 1.98.0 documentation, accessed 2026-09-11 | https://doc.rust-lang.org/cargo/reference/profiles.html | Official documentation |
| Asynchronous runtime diagnostics | Tracing next steps | Tokio project | accessed 2026-09-11 | https://tokio.rs/tokio/topics/tracing-next-steps | Official documentation; Tokio Console is a non-conformance diagnostic lane |
| Golden signals and latency separation | Monitoring Distributed Systems | Google SRE | accessed 2026-09-11 | https://sre.google/sre-book/monitoring-distributed-systems/ | First-party SRE guidance |
| Tail-latency/SLO measurement | Service Level Objectives | Google SRE | accessed 2026-09-11 | https://sre.google/sre-book/service-level-objectives/ | First-party SRE guidance |
| Aggregatable latency histograms | Histograms and summaries | Prometheus project | accessed 2026-09-11 | https://prometheus.io/docs/practices/histograms/ | Official documentation |
| DataFusion physical operator evidence | `EXPLAIN` and metrics | Apache DataFusion | 55.0.0 user guide, accessed 2026-09-11 | https://datafusion.apache.org/user-guide/explain-usage.html | Official documentation; matches the pinned workspace release |
| Per-operation storage/IO diagnostics | Perf Context and IO Stats Context | RocksDB project | accessed 2026-09-11 | https://github.com/facebook/rocksdb/wiki/Perf-Context-and-IO-Stats-Context | Official project documentation |
| Vector-service monitoring precedent | Monitoring | Qdrant | accessed 2026-09-11 | https://qdrant.tech/documentation/operations/monitoring/ | Official documentation |
| Multi-signal database observability precedent | Observability | SurrealDB | accessed 2026-09-11 | https://surrealdb.com/docs/manage/observability | Official documentation |
| Deterministic database failure simulation | Testing and client testing | FoundationDB | accessed 2026-09-11 | https://apple.github.io/foundationdb/testing.html | Official documentation |
| Randomized/crash/white-box database testing | Stress test | RocksDB project | accessed 2026-09-11 | https://github.com/facebook/rocksdb/wiki/Stress-test | Official project documentation |
| Bounded Rust concurrency exploration | Loom | Tokio contributors | main, accessed 2026-09-11 | https://github.com/tokio-rs/loom | Primary source/documentation |
| Rust undefined-behavior detection | Miri | Rust project | main, accessed 2026-09-11 | https://github.com/rust-lang/miri | Primary source/documentation |
| Rust sanitizer builds | Sanitizer compiler flag | Rust project | beta unstable book, accessed 2026-09-11 | https://doc.rust-lang.org/beta/unstable-book/compiler-flags/sanitizer.html | Official documentation; verification-only lane |
| Non-portability of client hooks | Git Hooks | Git project | 2nd edition, accessed 2026-09-11 | https://git-scm.com/book/en/v2/Customizing-Git-Git-Hooks | Official project book |
| Server-side pull-request/status enforcement | Available rules for rulesets | GitHub | accessed 2026-09-11 | https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-rulesets/available-rules-for-rulesets | Official documentation; unavailable for the current private-repository tier during this audit |
| Unified transactional data models | Architecture | SurrealDB | accessed 2026-09-05 | https://surrealdb.com/docs/learn/data-models/architecture | Official documentation |
| One modular database core | Core source tree | SurrealDB | v3.2.4, commit `93ab219d69f09d8f999851b0359c80ebe6726102`, repinned 2026-09-08 | https://github.com/surrealdb/surrealdb/tree/93ab219d69f09d8f999851b0359c80ebe6726102/surrealdb/core/src | Primary source |
| Directional graph adjacency keys | Graph-key module | SurrealDB | v3.2.4, commit `93ab219d69f09d8f999851b0359c80ebe6726102`, repinned 2026-09-08 | https://github.com/surrealdb/surrealdb/blob/93ab219d69f09d8f999851b0359c80ebe6726102/surrealdb/core/src/key/graph/mod.rs | Primary source; RRFlow does not adopt upstream historical decoding |
| Transaction-local cache/event/index coordination | Transaction module | SurrealDB | v3.2.4, commit `93ab219d69f09d8f999851b0359c80ebe6726102`, repinned 2026-09-08 | https://github.com/surrealdb/surrealdb/blob/93ab219d69f09d8f999851b0359c80ebe6726102/surrealdb/core/src/kvs/tx.rs | Primary source |
| MCP as a thin engine adapter | MCP crate | SurrealDB | v3.2.4, commit `93ab219d69f09d8f999851b0359c80ebe6726102`, repinned 2026-09-08 | https://github.com/surrealdb/surrealdb/tree/93ab219d69f09d8f999851b0359c80ebe6726102/surrealdb/mcp | Primary source |
| Ordered tuples and conflict ranges | Developer Guide | FoundationDB | 7.4.7, accessed 2026-09-05 | https://apple.github.io/foundationdb/developer-guide.html | Official documentation |
| HNSW design | Efficient and robust approximate nearest neighbor search using HNSW graphs | Malkov and Yashunin | 2016/2018 | https://arxiv.org/abs/1603.09320 | Original paper |
| Filtered vector planning | Indexing | Qdrant | accessed 2026-09-08 | https://qdrant.tech/documentation/manage-data/indexing/ | Official documentation |
| Vector segment source boundaries | Libraries and segment source | Qdrant | v1.19.1, commit `6ab21cac18ebb6f4ae29102c7f8f5cc11affd5de`, accessed 2026-09-08 | https://github.com/qdrant/qdrant/tree/6ab21cac18ebb6f4ae29102c7f8f5cc11affd5de/lib/segment/src | Primary source |
| BM25 semantics | The Probabilistic Relevance Framework: BM25 and Beyond | Robertson and Zaragoza | 2009 | https://www.staff.city.ac.uk/~sbrp622/papers/foundations_bm25_review.pdf | Primary technical review |
| RRF semantics | Reciprocal Rank Fusion Outperforms Condorcet and Individual Rank Learning Methods | Cormack, Clarke, Buettcher | 2009 | https://research.google/pubs/reciprocal-rank-fusion-outperforms-condorcet-and-individual-rank-learning-methods/ | Publisher record for original paper |
| Incremental resilient parsing | Tree-sitter Introduction | Tree-sitter project | accessed 2026-09-05 | https://tree-sitter.github.io/ | Official documentation |
| Git ignore precedence and tracked-file behavior | `gitignore` documentation | Git project | 2.51.0, accessed 2026-09-06 | https://git-scm.com/docs/gitignore | Official documentation |
| Content-addressed project trees | Git Internals: Git Objects | Git project | 2nd edition, accessed 2026-09-06 | https://git-scm.com/book/en/v2/Git-Internals-Git-Objects.html | Official project book |
| Bounded Git-aware Rust walking | `ignore::WalkBuilder` | BurntSushi / docs.rs | 0.4.24, accessed 2026-09-06 | https://docs.rs/ignore/latest/ignore/struct.WalkBuilder.html | Official crate API documentation |
| Incremental parser-tree reuse | Advanced Parsing | Tree-sitter project | accessed 2026-09-06 | https://tree-sitter.github.io/tree-sitter/using-parsers/3-advanced-parsing.html | Official documentation |
| Filesystem notification rescan | `notify::event::Flag` | notify-rs / docs.rs | 8.2.0, accessed 2026-09-06 | https://docs.rs/notify/latest/notify/event/enum.Flag.html | Official crate API documentation |
| Language-neutral code-index interchange | SCIP specification | Sourcegraph | main, accessed 2026-09-06 | https://github.com/scip-code/scip/blob/main/scip.proto | Primary schema; later-phase reference only |
| Provider skill and MCP installation | Agent setup | SurrealDB | accessed 2026-09-06 | https://surrealdb.com/docs/agents | Official product guidance; not a project inventory contract |
| Repository-scoped agent memory and folder ingest | Coding agent with project memory | SurrealDB | accessed 2026-09-06 | https://surrealdb.com/docs/agent-memory/cookbooks/build/coding-agent-with-project-memory | Official cookbook; reviewed boundary is documentation ingest and memory workflow |
| Vector/payload ingestion ownership | Manage Data | Qdrant | accessed 2026-09-08 | https://qdrant.tech/documentation/manage-data/ | Official documentation; begins at caller-supplied points |
| External ingestion integration | LlamaIndex | Qdrant | accessed 2026-09-08 | https://qdrant.tech/documentation/frameworks/llama-index/ | Official integration documentation |
| Sandboxed policy-driven code indexing | Auto-indexing | Sourcegraph | accessed 2026-09-06 | https://sourcegraph.com/docs/code-navigation/auto-indexing | Official documentation; operational comparison only |
| Codex instruction discovery | Unrolling the Codex agent loop | OpenAI | 2026, accessed 2026-09-05 | https://openai.com/index/unrolling-the-codex-agent-loop/ | First-party engineering article |
| Claude instruction import | How Claude remembers your project | Anthropic | accessed 2026-09-05 | https://code.claude.com/docs/en/memory | Official documentation |
| Gemini instruction hierarchy/import | Provide context with GEMINI.md files | Google | updated 2026-06-18 | https://geminicli.com/docs/cli/gemini-md/ | Official documentation |
| Optional weighted cache | Moka crate | Moka project | 0.12.16, accessed 2026-09-05 | https://docs.rs/moka/latest/moka/ | Official crate API documentation |
| External cache classification | Dragonfly Docs | Dragonfly | updated 2026-08-04 | https://www.dragonflydb.io/docs | Official documentation |
| External application DB classification | Embedded Replicas | Turso | accessed 2026-09-05 | https://docs.turso.tech/features/embedded-replicas/introduction | Official documentation; publisher marks this feature unsuitable for new designs |
| CoW upper/lower behavior | Overlay Filesystem | Linux kernel | accessed 2026-09-05 | https://docs.kernel.org/filesystems/overlayfs.html | Official kernel documentation |
| Release provenance | SLSA Provenance | Linux Foundation / SLSA | v1.2, accessed 2026-09-05 | https://slsa.dev/spec/v1.2/provenance | Approved specification |

## Research limitations and stop condition

The research determines architecture and testable boundaries; it does not
benchmark RRFlow, prove the present code, decide distributed consensus, or
adjudicate the scope of private source-use agreements. Zuul Zero/shippin.ai
protocol details were not publicly verifiable in this pass, so Gate H-07 must
treat the mesh as an adapter contract supplied by its operator, not invent
protocol behavior.

Discovery stopped after every consequential design slot had a primary or
first-party source, current code had been reconciled against each slot, and
additional searches were returning duplicate implementation patterns rather
than changing a boundary decision.
