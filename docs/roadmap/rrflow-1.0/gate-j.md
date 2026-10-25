# Gate J — RRFlow 1.0 release proof

**Status:** active canonical RRFlow 1.0 roadmap chapter
**Coordinate:** `rrflow://rrflow-instance/data/roadmap/rrflow-1.0/gate-j`
**Owner:** [RRFlow 1.0 release roadmap](../rrflow-1.0.md)

This chapter is part of the canonical RRFlow 1.0 roadmap. The parent
record owns release order and completion status; this file cannot change
either independently.

Incremental pre-release source and evidence are published only to the public
`rrflow/rrflow-development` repository. The private `rrflow/rrflow`
repository receives no development branch, tag, binary, artifact, or release;
it is an explicit promotion destination only after every Gate J prerequisite
passes and the repository owner approves the exact revision and artifacts.
Promotion records the source and destination URLs, refs, commit and tree
digests, manifest digest, artifact digests, verification evidence, and owner
decision. A force push or history rewrite is never a release mechanism.

| Done | ID | Required change | Owning boundary | Acceptance evidence |
|---|---|---|---|---|
| [ ] | J-01 | Remove every superseded pre-release entrypoint, format/catalogue reader, missing-field fallback, backend selector, migration executor, editor/provider-owned automatic hook, duplicate source of truth, transitional alias, and stale generated artifact. | workspace | Strict warning/dependency/terminology searches, negative format fixtures, and all-target builds prove only the accepted RRFlow 1.0 surfaces remain and superseded RRFlow state fails closed. |
| [ ] | J-02 | Run unit, property, fuzz corpus, differential, recorded-seed state-machine, bounded concurrency-schedule, Miri/supported-sanitizer, crash/reopen, storage-full, security denial, resource-budget, exporter-failure, adapter, release/diagnostic-parity, and real-process suites. | workspace | Release evidence records exact build identity, commands/tool versions/targets, seeds and fault schedules, passed/failed/skipped counts, declared tool limitations, post-reopen verification, and retained failure artifacts. |
| [ ] | J-03 | Assemble complete manifest-verified native release-candidate bundles from tracked inputs; use only each bundle's primary `rrflow`/`rrflow.exe` to install and attune an empty fixture and this existing repository; then restart, verify, and repeat representative fast/heavy queries. | release harness | On native Linux, Windows, and macOS qualification runners, with outbound network denied, sibling repositories absent, and no compiler or external database/query/vector service, both estates reach authenticated readiness and verify with stable digests; unchanged rerun is incremental; the repair and restore rehearsal passes; and no checkout helper or manual database repair is needed. This qualifies contents and behavior before J-05 signs the reproducible default distribution. |
| [ ] | J-04 | Publish fixed-hardware rrflowKV, graph, BM25, exact/HNSW, DataFusion, context, LFG, end-to-end, and clean-rollout benchmarks against pinned declared baselines. | evaluation harness | Revision-bound raw histograms name the exact client/server/engine/stage timing boundary and retain failed samples; evidence records build identity, hardware/toolchain/filesystem/device/clock provenance, configuration, corpus/seed, offered-load model, warmup, cache state, concurrency, p50/p95/p99/p99.9, success/error separation, long-duration RSS/CPU/I/O/queue saturation, logical/apparent/allocated bytes, mixed-family interference, recall/quality metrics, and a SurrealDB/Qdrant deployment matrix with artifact and installed bytes, commands, elapsed time, services, ports, configuration, secrets, readiness, and persistent readback. Closed-loop tests correct coordinated omission. No ease or superiority claim is allowed until like-for-like correctness/quality evidence passes. |
| [ ] | J-05 | Produce reproducible signed default distributions whose one primary operator executable is `rrflow` on Linux/macOS and `rrflow.exe` on Windows, containing or manifest-binding every default engine capability, SDK, schema/golden, project/attunement template, configuration/profile, required local inference asset, SBOM/licence, artifact verifier, backup/restore tool, and runbook. Publish archive/executable checksums, a complete RRFlow byte manifest, build provenance/SBOM attestations, and the native platform signature required by the supported target, including Authenticode for Windows. | release tooling | After artifact acquisition, a clean native supported machine with no compiler, source checkout, sibling repository, repository-local cache, package registry, external database/query/vector service, or outbound network uses only the primary executable to run version, install plan/apply, serve, authenticated ready, commit/query/context, close/reopen, quick/full verify, repair rehearsal, backup/restore, and ownership-safe uninstall; every installed byte and runtime dependency resolves from the signed manifest. |

## Integrated delivery and competitive claims

The [DevPulse integration contract](../../reference/client/devpulse-cloud.md)
and [H-07 packages](gate-h.md#h-07-devpulse-managed-cloud-and-homelab-packages)
add explicit managed-service and self-hosted delivery profiles. The same native
engine remains independently operable. Do not label a development desktop ZIP,
container launch or local process test as a qualified cloud service or a
complete DevPulse/homelab integration. Those profile claims require their
actual provisioning, transfer, isolation, recovery and artifact evidence.

J-04 also includes the owner's Pinecone comparison target. Pin tested service
API/index configuration, region, data set, embedding model/dimension,
filter/selectivity distribution, recall target, offered load and cost period.
Report network-inclusive client latency separately from measured engine time;
never compare a local in-process call with an unexplained managed round trip.
Measure filtered dense/sparse/hybrid retrieval, update/delete freshness,
concurrent ingestion, tenant isolation and sustained resource/cost behavior.
The SurrealDB functional matrix includes the admitted model/transaction/query,
subscription, recovery and self-hosted deployment profiles. Record exclusions;
a vector-only benchmark cannot establish database-wide parity.

The owner's **Pinecone Nexus** target is its governed knowledge engine, not
only Pinecone's vector database. Its official product description names source
curation into reusable knowledge and typed, cited answers through KnowQL.
Treat vendor measurements as hypotheses to test, not RRFlow evidence. Compare
RRFlow's canonical attunement, graph/claim provenance, bounded context and
reasoning paths without importing a second engine or copying KnowQL:

- Pin the same source corpus/revisions, questions, expected answers and task
  end states, model versions, permissions and budgets. Score correctness,
  abstention, conflicting/stale sources, citation support and forbidden-data
  leakage using independent expected results and reviewable grading.
- Measure initial ingestion/curation and incremental update/delete costs
  separately from repeated query costs. Include model/tool calls, tokens,
  latency distributions, resource use, storage and amortized total cost;
  precomputation cannot disappear from the comparison.
- Change and revoke source content, then verify derived knowledge freshness,
  provenance and access enforcement after restart. Include relationships and
  questions requiring several sources rather than only nearest-neighbor recall.
- Run the admitted native binary/service profile with a real Nexus access path
  only when configured and authorized. An unavailable comparator is reported
  as not run. Pin its tested version/configuration and retain raw responses
  and error samples; published marketing numbers are not the baseline run.

Declare workloads and pass/fail thresholds before running the comparison,
retain failures, and compare at matched correctness/quality. No competitor
parity, superiority or ease-of-deployment claim is accepted from feature names,
code inventory, a screenshot, a build or ordinary candidate CI. Current primary
reference inputs (reviewed 2026-10-19; not benchmark results):

- [SurrealDB model and deployment overview](https://surrealdb.com/docs/what-is-surrealdb)
- [Pinecone architecture](https://www.pinecone.io/how-pinecone-works/)
- [Pinecone Nexus product and knowledge interface](https://www.pinecone.io/product/nexus/)
- [Pinecone Nexus GA architecture and benchmark account](https://www.pinecone.io/blog/pinecone-nexus-generally-available/)
- [Pinecone production checklist](https://docs.pinecone.io/guides/production/production-checklist)

J-04's mature-project demonstration must follow the [project evidence and
model-usage comparison contract](../../architecture/engine-data-flow.md#project-evidence-model-usage-and-reasoning-efficiency).
Compare baseline and locally grounded arms at matched quality and account for
initial construction and updates. A context-size reduction, provider cache hit,
or successful storage soak alone is not proof of remote-token savings.

## Capacity and optimization qualification

The [researched design rationale](../../architecture/engine-scale-and-contracts.md)
supplies tradeoffs; this gate owns workload acceptance. The following values
are **proposed engineering targets**, not measured capacity, default runtime
configuration, supported customer limits or an SLA. Freeze the exact manifest
and thresholds before a qualification run. A failing run remains a failure;
changing a target requires a new reviewed manifest and preserves old results.

### Initial workload ladder

Use release builds, local SSD/NVMe with filesystem/durability details recorded,
1 KiB deterministic record payloads, bounded degree distributions and separate
raw vector/index/storage accounting. All counts below are corpus targets.
Model-process/device memory is reported separately and also included in the
host total; it cannot disappear from the budget.

| Profile | Fixed allocation and corpus | Load sweep and purpose |
|---|---|---|
| Desktop | 4 vCPU, 8 GiB host RAM; engine RSS ceiling 2 GiB; one estate with 100,000 records, 500,000 edges, 100,000 dense 384-dimensional vectors | 1/4/8 concurrent clients; offered 5/25/50 requests/s; establish local operation, incremental ingestion and bounded interference with model work |
| Single service estate | 8 vCPU, 32 GiB host RAM; engine RSS ceiling 8 GiB; 1,000,000 records, 5,000,000 edges, 1,000,000 dense 768-dimensional vectors | 8/32/64 clients; offered 25/100/250 requests/s; find the sustainable rate and overload knee without silently reducing quality |
| Fleet isolation | 16 vCPU, 32 GiB host RAM; 8 GiB aggregate engine RSS ceiling; ten desktop-sized estates, aggregate corpus ten times the desktop row | 4 clients per estate; measure quiet, equal-load and one abusive estate at 2x its admitted rate; capture idle per-estate overhead and placement limits |
| Data larger than memory | Same single-estate allocation; at least 32 GiB measured logical source data and 2,000,000 dense 1536-dimensional vectors; source bytes, on-disk bytes and index bytes reported independently | Repeat selective and broad queries, ingestion, compaction, backup and cancellation under the 8 GiB engine ceiling; this is a streaming/denial/resource test, not a promise that every ANN configuration fits |

These are explicit laboratory resource limits. Record actual CPU model/count,
OS/target, memory limiter, storage/device, filesystem, available disk and model
placement. Equivalent profile names on unlike machines do not establish
comparable results. Unsupported limiting or measurement facilities are a
reported qualification gap.

The base offered-load mix is 50% point/scalar reads, 20% bounded graph/BM25
recall, 10% filtered vector search, 15% atomic record/relation writes and 5%
bounded context assembly without a new model call. Seed the operation sequence
and declare 1%/10%/100% filter-selectivity lanes, uniform and hot-key lanes,
fixed top-k, graph depth and result-byte ceilings. Measure real model routing
and full reasoning tasks in separate, identified lanes; a stub-model load
test cannot qualify model quality or end-to-end reasoning latency.

### Proposed pass/fail thresholds

At desktop 25 requests/s and single-estate 100 requests/s, propose point-read
p95 <= 50 ms, durable-commit p95 <= 100 ms, and bounded recall/context p95 <=
250 ms, all client end-to-end on loopback, with corresponding p99 ceilings
of 200/500/1000 ms. Use 1 KiB point/write payloads, top-k 20, at most two graph
hops/1,000 visited edges and at most 64 KiB context output for those thresholds.
Report broader queries separately. The acceptance envelope is the largest
tested rate that satisfies every quality, resource and latency condition.

At declared nominal load, unexpected terminal errors must be <= 0.1%;
intentional denial/fault cases are separate and never discarded from raw
evidence. Exact operations require zero oracle mismatches, lost acknowledged
writes, partial atomic groups, cross-estate disclosures and accepted stale
fences. Approximate search proposes recall@20 >= 0.95 against exact search on
the same authorized filtered corpus; record the actual recall curve and filter
cost. Full reasoning reports correctness, supported citations, abstention and
task end-state separately; its model/corpus-specific quality threshold must be
frozen before the run, not replaced by valid-JSON percentage.

Under a 2x nominal-load burst, require bounded queues/memory, explicit overload
outcomes, no safety violation and return to nominal latency/backlog within
60 seconds after load falls. In the fleet case, the abusive estate must not
drive compliant estates above 2x their isolated p99; report admission and
placement costs. This ratio is a proposed isolation threshold, not a feature
claim. Cancellation tests prove permits and bytes are reclaimed after actual
termination; a response timeout alone is insufficient.

Run 5 minutes of warmup plus 30 minutes measured at each accepted operating
point, at least three independent seeded repetitions, and a 24-hour sustained
mixed-write/maintenance soak. Retain cold start/cache experiments separately.
Every finished cohort must reconcile admitted, completed, rejected, cancelled,
failed and unresolved requests. Record resource time series and distinguish
RSS, mapped/pinned, cache, allocator and model bytes. After warmup and drain,
retained memory must remain within the configured caps and explainable live
owners; do not excuse an upward leak because the short average is small.
All these runs remain unexecuted requirements of J-04.

### Workload and evidence structure

Extend the existing evaluation boundary with one typed, versioned workload
manifest and result model; generate its validation schema and reject unknown
fields. Do not hand-author an unrelated JSON schema or copy runtime operation
definitions into the harness. The current paired-provider `Trial` is retained
until its behavior/tests are mapped; it is not this future engine manifest.

| Field group | Required structure and validation |
|---|---|
| Identity | Positive schema version, bounded case/profile IDs, source commit/tree, executable/bundle, operation/schema/configuration digests; fixed-length lowercase SHA-256 values, clean-source qualification |
| Environment | Target triple, toolchain, profile/features, CPU/OS/filesystem/device, memory/disk limits, model artifact/runtime/device identity, measurement-tool versions; no credentials |
| Corpus | Generator/version and seed, source digest, family counts, encoded/decoded bytes, vector dimension/metric, filter/degree/hot-key distributions, expected-result digest |
| Operation mix | Catalogue-derived operation references, weights summing to 100%, bounded payload/response shapes, concurrency, arrival schedule, retry policy, cancellation/fault schedule |
| Budgets | Positive finite byte/time/queue/worker/token/cost limits with units and scope; cumulative model cost and retries charged; no unlimited sentinel |
| Duration and thresholds | Warmup/measurement/soak durations, sample minima, quantiles, error/quality/resource/recovery thresholds; cross-field checks for valid durations/rates and declared profile |
| Results | Attempt and logical-call outcomes, start/end/duration boundary, receipt/stamp references, failure/uncertainty counts, quality oracle result, raw histogram and resource-series artifact digests |
| Coverage | Required/passed/failed/skipped/unimplemented counts and case IDs, seeds, fault locations, tool limitations, comparator exclusions and retained failure artifacts |

State invariants, replay schedules and resource assertions belong in ordinary
package tests before expensive runs. Extend fixtures through the installed
binary for lost ACK, kill/reopen, full disk, corrupt/truncated input, provider
stall, stale output, dropped/reordered/duplicated sync messages, revoked
permissions and exporter loss. Use an independent history/oracle checker;
the engine must not grade its own success.

### Qualification packages and optimization decisions

| Package | Dependency and concrete deliverable | Exit |
|---|---|---|
| J-04a: executable measurement contract | H-05 build/metric identity plus source/test mapping of `rrflow-eval` and existing component benchmarks; implement the typed manifest/schema and raw result writer at that evaluation boundary | Negative/golden validation, deterministic corpus identities, exact sample accounting, histogram boundary tests and retained failures; no model/service invocation without explicit binding |
| J-04b: installed workload driver | D-01/C-07 and the exercised E/F/G/H operations; catalogue-derived public client, open-loop schedule or documented coordinated-omission correction | Actual installed binaries complete the declared ladder; unavailable capability is unimplemented/failed, not a skipped success; logical-call and attempt timing stay separate |
| J-04c: sustained interference and recovery | J-04b; accepted maintenance, cancellation and transfer paths required by each case | Soak, overload/drain, noisy neighbor, disk-full, pinned-buffer and restart histories satisfy declared invariants and ceilings |
| J-04d: optimization and comparison | J-04a through J-04c and matched quality; existing SurrealDB/Qdrant/Pinecone/Nexus comparison requirements | One change at a time, same corpus/hardware/quality, raw before/after distributions and storage/cost accounting; reject regressions or explicitly review the tradeoff before acceptance |

J-04a specification and case preparation may begin now; runtime qualification
obeys the dependency spine. These packages do not postpone early instrumentation
or component performance checks. Every C-through-I package must carry the
physical counters and failure oracle for its own behavior.

Before tuning, identify the dominant measured cost: repeated log/catalogue
replay, eager materialization, physical reads/decode, allocation, registry or
commit lock wait, queue contention, model compute or transport. Preserve
the accepted C-06 cache/compression decisions until same-corpus evidence
supports revisiting them. Threshold regressions, changed output quality,
unbounded resources or incomplete failure accounting block an optimization
claim even when headline throughput rises.

## Release condition

### rrflowDB compatibility and independent-release evidence

The owner-directed [PostgreSQL core-profile contract](../../reference/protocol/postgresql.md)
adds the following evidence to the existing release items; it creates no
separate release authority or version train:

- J-02 includes H-08's locked-reference SQL/type/transaction/wire/client corpus,
  native-operation equivalence, denial/fuzz/resource/concurrency/crash cases,
  and synthetic Turso-exit import/export rehearsal. Required skips block release.
- J-03 exercises PostgreSQL clients against each supported installed native
  server and reopens/restores its native data with vendor services unavailable.
  Reference PostgreSQL and source-export tools are isolated test inputs, never
  default runtime dependencies. The exact supported client/profile report ships
  with the candidate; unsupported extensions/tools are stated explicitly.
- J-04 measures mixed SQL and AI workloads, including data larger than memory,
  alongside the existing correctness/quality and fixed-hardware evidence. There
  is no throughput or PostgreSQL superiority claim before those results exist.
- J-05 requires complete first-party licence texts, dependency notices, SBOM,
  retained reproducible source inputs, provenance, signatures, independent
  logical export/native recovery documentation, and a verified rebuild without
  a vendor account. A Cargo licence expression alone does not qualify packaging.

RRFlow 1.0 is releasable only when every Gate J item and every prerequisite is
checked. Until then the repository may describe implemented and measured
behavior, but it must not claim the complete target system is production-ready.
