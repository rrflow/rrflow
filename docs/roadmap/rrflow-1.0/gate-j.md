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
