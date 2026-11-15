# RRFlow build, debug, trace, and optimization research

**Status:** active supporting research; not an implementation, benchmark, or release claim
**Coordinate:** `rrflow://rrflow-instance/data/research/build-debug-trace-optimize`
**Owner:** primary-source engineering evidence for future RRFlow build, diagnostic, observability, failure-analysis, and optimization packages
**Reviewed:** 2026-09-14
**Scope:** reusable engineering substrate for C-07, D-01, H-05, J-02, J-04, and J-05

This record answers one bounded question: what engineering substrate should
RRFlow establish so future code can be built reproducibly, debugged from a
retained failure, traced across one governed causal path, and optimized from
valid measurements without constraining adaptive reasoning or creating a
second engine authority?

The accepted behavior remains owned by the
[engine data-flow architecture](../architecture/engine-data-flow.md#trace-and-observability-flow),
delivery order and acceptance remain owned by the
[RRFlow 1.0 roadmap](../roadmap/rrflow-1.0.md), verified gaps remain owned by
the [POA&M](../poam/rrflow-1.0-alpha.md), and generated surfaces remain owned
by the [single-source generation design](../roadmap/rrflow-1.0-execution/generated-surfaces.md).
This research explains implementation choices and rejected shortcuts. It does
not override those records or move their status.

## Research method and confidence

The review combined three forms of evidence:

1. complete reads of the product, objective, roadmap, Gate H, Gate J, engine
   data-flow, CI, change-authoring, and prior H-05 planning owners;
2. inspection of the current workspace profile/dependency surface, tracing
   call sites, fuzz package, generated-surface policy, planned H-05 and Gate J
   paths, and absent release tooling; and
3. current primary specifications and first-party project documentation for
   Rust builds and diagnostics, OpenTelemetry and W3C correlation, deterministic
   testing, fuzzing and sanitizers, native profilers, benchmark statistics,
   tail-latency measurement, and artifact provenance.

Sources were reviewed on 2026-09-14. Version-sensitive advice must be checked
again against the repository's pinned Rust toolchain, target runners, and
locked dependency versions when its implementation package begins. The
recommendations below are high confidence where multiple specifications agree;
tool availability and overhead remain target-specific and require measurement.

No runtime suite, profiler, benchmark, reproducible-build comparison, native
platform qualification, or release assembly was run for this research. Any
numbers in this record describe the checkout or an external specification,
not RRFlow performance.

## Executive findings

1. RRFlow needs one semantic program with several evidence modes, not separate
   debug and release implementations. A diagnostic build should inherit the
   release profile and exact feature closure; only symbol/debug information and
   explicitly scoped diagnostic collection may differ.
2. Build identity is the first debugging feature. Every result must say which
   source tree, lockfile, toolchain, target, profile, features, catalogue,
   formats, configuration, executable, and distribution produced it.
3. Durable trace and diagnostic telemetry have different failure semantics.
   `RrdEngine` owns accepted state and receipts; logs, spans, metrics, captures,
   and profiler files are lossy projections that may help explain an outcome
   but may never create one.
4. Types belong at semantic and effect boundaries, not around thought. Stable
   operation, stage, outcome, error-class, resource, and receipt identities
   should be generated and validated. Exploratory hypotheses and unfamiliar
   concepts stay representable as narrative or bounded diagnostic extensions
   until they need cross-surface semantics.
5. Debugging must begin with a reproducible failure artifact, not a larger log.
   Inputs or corpus digest, seed, schedule, fault point, build identity,
   configuration, last authoritative coordinate, terminal result, and reopen
   result are the minimum useful unit.
6. No single verification technique is enough. Unit/property/differential
   oracles, deterministic faults, process crash/reopen, fuzz corpora, Loom,
   Miri, sanitizers, native stress, and offline release tests cover different
   failure classes and must report their limitations.
7. Tracing and metrics must be designed together but stored differently.
   Traces carry bounded per-operation causality; metrics aggregate low-cardinality
   work, latency, error, and saturation; exemplars correlate selected samples.
8. Profilers are hypothesis tools, not always-on product behavior. Use the
   least invasive tool that can distinguish the suspected CPU, allocation,
   async scheduling, lock, filesystem, or platform cause, and bind its output
   to the same build and workload manifest.
9. Optimization starts only after a correctness/quality oracle and measurement
   boundary exist. It ends with an accept-or-reject decision based on raw
   distributions, resource effects, failure behavior, and practical magnitude,
   not a single faster mean or a profiler screenshot.
10. The operation/capability catalogue should drive dispatch, OpenAPI, internal
    clients, language SDKs, observability descriptors, documentation, and
    conformance. Generation removes mechanical duplication; it must not turn
    generated output into authority or freeze adaptive reasoning into a schema.

## Current checkout baseline

The repository has important foundations but not the complete substrate:

| Area | Present | Material gap |
|---|---|---|
| Toolchain and workspace | Rust `1.98.0` is pinned; product version remains `1.0.0`; dependencies are locked. | The root `Cargo.toml` declares no custom profiles, so the architecture's `diagnostic`, `runtime-analysis`, and `benchmark` modes are not executable contracts. |
| Causal vocabulary | `rrd-core::trace` has a closed `TraceOperation` catalogue, boundaries, outcomes, links, and durable event types. | Existing direct-store and adapter instrumentation has not converged on one complete engine causal path. |
| Rust diagnostics | `tracing`/`tracing-subscriber` are workspace dependencies and selected binaries install subscribers. Thirteen Rust source files currently contain direct `tracing` call sites. | There is no provider-neutral observability contract, OpenTelemetry bridge, implemented metric catalogue, W3C client/server propagation module, or diagnostic capture bundle. |
| Failure testing | rrflowKV has retained corpora and two current libFuzzer targets: projected-read state machine and segment-v6 open. | Fuzzing is not yet a shared engine/release lane; whole-path deterministic faults, exporter failure, concurrency schedules, Miri/sanitizer matrices, and release/diagnostic parity remain open. |
| Query evidence | DataFusion `55.0.0` is exactly pinned and current code imports selected physical metrics. | RRFlow has no unified query latency/resource evidence implementation, retained release workload runner, or fixed-hardware baseline fixture. |
| Release proof | CI runs substantial correctness and characterization suites; roadmap paths already name future release-evidence and assembly tooling. | The release-evidence binary, qualification/assembly/verification scripts, deployment baselines, and distribution manifest are absent. |

These gaps agree with open H-05 and Gate J requirements. They do not imply
that the existing code is valueless, and they are not closed by documenting
them.

## The engineering evidence model

RRFlow should treat engineering evidence as four connected planes:

| Plane | Owns | May influence | Must never do |
|---|---|---|---|
| Authority | `RrdEngine`, canonical records, authorization, budgets, validation, stamps, receipts | Every governed durable or external effect | Infer success from a log, trace, benchmark, or tool exit alone |
| Signal | Structured events, spans, metrics, bounded logs, sanitized captures | Debugging, monitoring, comparison, incident reconstruction | Mutate canonical state, decide readiness, retry work, or become a parallel lifecycle |
| Experiment | Unit/property/differential tests, deterministic faults, fuzzing, concurrency models, sanitizers, profilers, benchmarks | Evidence for a bounded implementation decision | Rewrite the production semantic path merely to make the harness convenient |
| Release proof | Native artifacts, manifests, SBOM, provenance, signatures, offline qualification | Promotion decision and consumer verification | Substitute artifact identity for correctness, performance, or owner authorization |

The separation is deliberate. OpenTelemetry explicitly treats telemetry as
nonessential to application business behavior and requires runtime telemetry
failures not to escape into the application.[^12] SLSA provenance establishes
where and how an artifact was produced, not whether the artifact is correct or
safe.[^4] RRFlow therefore gains useful correlation without allowing the
observer to become the system being observed.

### Where strict typing stops

The closed contract should cover what another component must interpret
consistently: operation and capability identity, effect class, authorization,
source/read stamp, budgets, terminal outcome, stable error class, format and
schema identity, and receipt. Unknown effects fail closed.

Diagnostic enrichment can remain progressively typed:

- a small stable core identifies operation, boundary, stage, outcome, build,
  and causal coordinates;
- versioned, namespaced attributes add a bounded measurement or explanation;
- consumers ignore an unfamiliar diagnostic attribute without interpreting it
  as authorization, status, retry policy, or lifecycle state; and
- a concept enters the closed catalogue only when dispatch, persistence,
  cross-surface behavior, or shared aggregation requires stable semantics.

This keeps new ideas expressible without creating a schema of thought. It also
matches OpenTelemetry's distinction between stable semantic conventions and
versioned schema transformations.[^14] The pressure to type a field should come
from a durable or interoperable effect, not from a desire to record every
intermediate hypothesis.

## Build architecture

### One semantic feature closure

Cargo profiles control code-generation properties such as optimization,
debug information, stripping, assertions, overflow checks, LTO, incremental
compilation, and codegen units. Custom profiles inherit from another profile
and are defined at the workspace root.[^1] That makes the root manifest the
correct future owner of build modes, but profile names alone do not establish
semantic parity.

The architecture-owned mode names should translate into the following
implementation behavior:

| Mode | Build relationship | Valid use | Invalid claim |
|---|---|---|---|
| `dev` / `test` | Fast local defaults with assertions and test-only harnesses | Editing, unit/property/integration correctness | Release behavior or latency |
| `release` | Optimized, shippable semantic baseline | Native qualification and shipped behavior | Debuggability without retained symbols/build identity |
| `diagnostic` | Inherit `release`; retain line tables or symbols; exact feature closure | Reproduce a release result with bounded extra signal after parity passes | Alternate algorithms, formats, authorization, limits, or adapters |
| `runtime-analysis` | Diagnostic binary plus explicitly enabled scheduler or profiler instrumentation | Locate a performance or concurrency cause | Conformance or timing equivalence |
| Loom/Miri/sanitizer | Tool-specific verification build | Find bounded concurrency, undefined-behavior, race, leak, or memory faults | Native behavior, exhaustive state space, or release timing |
| `benchmark` | Release-derived executable plus fixed workload harness and declared signal level | Comparison under J-04 provenance | Product superiority or correctness by itself |

An illustrative future Cargo fragment is intentionally small:

```toml
[profile.diagnostic]
inherits = "release"
debug = "line-tables-only"
strip = "none"

[profile.benchmark]
inherits = "release"
```

This is not a committed configuration recommendation. Native Linux, Windows,
and macOS work must decide split-symbol, stripping, panic, LTO, and codegen-unit
settings and measure their build/runtime consequences. The invariant is the
inheritance and parity relationship, not those illustrative values.

### Build identity before build optimization

The existing architecture lists the authoritative build-identity fields. The
implementation should derive them mechanically in one release/build tool and
expose the same canonical object through:

- `rrflow version --json` and the installed diagnostic surface;
- every test, fault, profiler, benchmark, and capture manifest;
- distribution manifests and provenance subjects; and
- a semantic fingerprint used by release/diagnostic parity tests.

The generator should consume the Git commit/tree/dirty state, `Cargo.lock`,
pinned toolchain, target, profile, enabled first-party features, executable
digest, operation/capability catalogue digest, format/schema identities, and
sanitized configuration digests. It should write into an isolated build or
staging directory, never download data, silently inspect a sibling checkout,
or hand-edit a tracked semantic owner.

Build timestamps should not masquerade as identity. `SOURCE_DATE_EPOCH`
defines a deterministic source-derived timestamp and requires malformed values
to fail rather than fall back silently.[^3] `rustc --remap-path-prefix` can
normalize compiler-emitted paths, but Rust documents it as best effort and
notes that relative and absolute paths may both need handling.[^2] Therefore a
reproducible-build test must build the same source independently in different
roots and compare artifacts after the target's documented signature/debug-file
normalization; flags alone are not proof.

### Symbols and sensitive paths

Release-debugging data should be separable from the default operator bundle:

1. build the release and diagnostic candidates from the same semantic input
   manifest;
2. generate platform-appropriate symbol artifacts and record their digests;
3. verify the executable-to-symbol association on that target;
4. scan both executable and symbols for unremapped workspace paths and secrets;
5. protect symbols with a retention/access policy because they disclose code
   structure and may expose paths; and
6. prove a retained crash address can be symbolized back to the exact source
   revision.

Rust backtraces are explicitly best effort, need debug information for useful
file/line resolution on most platforms, and can be expensive.[^17] A backtrace
is consequently an opt-in diagnostic attachment, never a stable error ID or
the only failure coordinate.

### Provenance and reproducibility are different proofs

SLSA provenance connects artifact digests to a build definition, resolved
dependencies, builder, invocation, and selected byproducts useful for incident
response.[^4] Reproducibility asks whether independent equivalent builds yield
the same bytes. Native signing authenticates a platform artifact. An SBOM
describes contents. RRFlow needs all four for J-05, but none proves runtime
correctness.

GitHub can generate build and SBOM attestations, but private-repository support
depends on the account plan and its private Sigstore instance.[^5] The
provider-neutral design is therefore:

- generate RRFlow's manifest, dependency inventory, and provenance predicate
  from repository-owned tooling first;
- sign or attest that canonical material through an explicitly selected
  provider when available;
- retain enough material for offline verification; and
- never make one hosted service the only way to understand or verify an
  already acquired default bundle.

## Catalogue-driven generation without over-schema

The current checkout already derives its OpenAPI operation identifiers and
several SDK operation surfaces from executable contract code. OpenAPI requires
an `operationId` to be unique because tools use it as the stable operation
handle.[^34] The next step is convergence, not a second telemetry catalogue.

The recommended compiler-like flow is:

```text
validated wire types + executable operation/capability descriptors
    -> engine dispatch and authorization binding
    -> OpenAPI and transport fixtures
    -> internal Rust client
    -> TypeScript/Python/Go/Java/.NET operations and models
    -> shared success/error/denial/cancellation/fault corpus
    -> operation/span names and allowed telemetry descriptors
    -> reference and coverage projections
```

Three safeguards keep this fluid:

1. The executable catalogue owns interoperable semantics; generated files are
   replaceable projections and cannot declare availability or completion.
2. Physical algorithms, providers, model choices, local profiling details, and
   exploratory concepts are not public operations merely because telemetry can
   describe them.
3. The generator fails on duplicate identities, missing handlers, incompatible
   schemas, handwritten drift, or unsupported projections, while clients can
   preserve or ignore explicitly allowed diagnostic extensions.

OpenTelemetry's semantic-convention tooling similarly generates code and
documentation from named groups with explicit stability and deprecation.[^14]
RRFlow can retain that useful mechanism without copying OpenTelemetry's schema
or making its maturity model a product lifecycle.

### What should be generated

- operation and capability identifiers, route/dispatch tables, and authorization
  metadata;
- request/result/error models, OpenAPI, internal client operations, and language
  SDK bindings;
- static trace call-site names and the allowed operation/boundary/stage values;
- metric descriptor names, kind, unit, allowed dimensions, and documentation;
- shared conformance fixtures and coverage reports; and
- documentation indexes and evidence discovery from normal links.

### What should remain authored

- why an effect exists, its safety and authorization semantics, and its owning
  architecture decision;
- budgets, failure semantics, redaction policy, deprecation decision, and stop
  conditions;
- workload choice, performance hypothesis, acceptance threshold, and release
  decision; and
- novel reasoning, hypotheses, and unfamiliar project concepts that have not
  crossed an interoperable effect boundary.

## Errors and debugging contract

### Stable coordinates plus protected detail

Future code should return a typed failure object at each public or engine
boundary with at least these conceptual layers:

| Layer | Purpose | Cardinality/visibility |
|---|---|---|
| Stable class | Machine routing and aggregation, such as validation, denied, conflict, exhausted, unavailable, corrupt, uncertain, or internal | Closed and low cardinality |
| Failed boundary/stage | Locate the subsystem and operation stage | Closed catalogue |
| Outcome semantics | Retryable, terminal, cancelled, uncertain, or already-completed receipt | Typed and contract-owned |
| Causal coordinates | Request/operation, trace/span, source/read stamp, job/routine, receipt, artifact, or fault IDs as applicable | Protected per-event fields, never metric labels |
| Safe presentation | Concise operator/client message and remediation hint | Bounded and redacted |
| Diagnostic attachment | Source chain, backtrace, plan/counter snapshot, native crash reference | Opt-in, size/retention bounded, access controlled |

OpenTelemetry recommends one predictable, low-cardinality `error.type` shared
between a failed operation's span and metric, and no error type on successful
operations.[^13] RRFlow should map its stable error class to that projection.
Free-form messages and backtraces stay out of metric attributes and default
public responses.

Expected denials, cancellations, cache misses, and handled retries should not
be mislabeled as internal faults. The terminal parent operation reports what
the caller experienced; individual attempts and handled failures remain linked
child events. This prevents retry noise from inflating user-visible error rates
or turning a governed denial into a server incident.

### Panic and crash policy

Recoverable input, authorization, budget, I/O, corruption, and dependency
failures should remain typed `Result` paths. Panic is reserved for violated
internal invariants where continuing could compound harm. At the executable
boundary, the crash handler may record only async-signal/platform-safe minimal
identity and point to an externally written native dump or prior bounded ring
buffer. It must not attempt a new database transaction, flush arbitrary logs,
or claim a final receipt after process integrity is unknown.

A crash is resolved only after restart/reopen verification establishes the
last accepted durable state. A stack trace without the last authoritative
stamp and receipt cannot answer whether the effect happened.

### Structured logs and redaction

The log event name should identify a stable event type; trace/span coordinates
provide correlation; the body provides a short human description; structured
attributes carry bounded safe values. The OpenTelemetry log model keeps those
roles distinct.[^15] Default logging must exclude credentials, tokens,
connection strings, prompts, source bodies, raw query parameters, vectors,
model output, hidden reasoning, and arbitrary project paths. OWASP additionally
recommends sanitizing control characters and protecting logs from disclosure,
tampering, and disk-exhaustion attacks.[^16]

Redaction should happen before enqueue or serialization, not only at an
exporter. Otherwise a disabled or failed exporter can still leave sensitive
material in memory, a local file, or a crash dump.

## Causal tracing and metrics

### One operation, several projections

OpenTelemetry spans represent operations with context, events, links, status,
and duration.[^6] W3C `traceparent`/`tracestate` supplies interoperable
transport continuation but warns that trace context is not a place for PII and
may need explicit restart at a security boundary.[^7] RRFlow should adapt this
as follows:

1. The accepted engine operation creates or validates the causal root and
   durable start evidence where required.
2. Transport adapters parse bounded W3C context, never treat it as identity or
   authorization, and start a fresh root on invalid or policy-rejected input.
3. Synchronous child work uses parentage; queued, retried, resumed, replayed,
   or fan-out work uses explicit links instead of false nesting.
4. Static call-site names come from the canonical operation catalogue.
   Per-request identities are fields, not names.
5. The terminal engine result determines durable outcome. The diagnostic span
   projects it; a missing or dropped span changes nothing.

Rust `tracing` is suited to async code because structured spans and events
preserve temporal and causal context that interleaved log lines lose.[^8]
Subscribers belong at the executable composition boundary, and composable
layers can independently filter formatting, metrics, local capture, or
OpenTelemetry export.[^9] Libraries should emit provider-neutral call sites;
they should not install global subscribers or require a collector.

Async instrumentation must not hold a `Span::enter` guard across `.await`,
which the `tracing` API documents as producing incorrect traces. Instrument
the future or use the async-aware attribute/combinator instead.[^8]

### Signal placement rule

| Question | Correct signal |
|---|---|
| Did a governed effect commit and what receipt proves it? | Canonical state and durable trace/receipt |
| What happened in this one operation? | Span tree/links plus bounded events |
| Is latency, traffic, failure, or saturation changing across operations? | Counter, gauge, and histogram series |
| What exact text helps an operator understand one event? | Structured redacted log correlated to trace/span |
| What code consumed CPU or allocated memory during a retained workload? | Manifest-bound native profiler artifact |
| What did a release candidate prove? | Signed/hashed evidence bundle linked to exact commands and results |

### Metric design

OpenTelemetry metrics preserve aggregatable sums, gauges, histograms,
temporality, and exemplars.[^10] Prometheus guidance emphasizes request count,
errors, latency, queues/caches, and resource use, while warning that label
dimensions multiply into potentially unbounded series.[^11] The
architecture-owned RRFlow metric catalogue and allowed dimensions should be
implemented literally rather than rediscovered in each crate.

Implementation rules:

- use a monotonic clock for duration and wall time only for correlation;
- record one terminal duration per named boundary and keep queue wait separate
  from execution;
- aggregate successful, denied, cancelled, uncertain, and failed outcomes
  without hiding failed latency;
- never put engram, actor, request, trace, query, record, path, provider, model,
  error message, or user data in labels;
- use an explicit overflow series when the configured cardinality limit is
  reached;
- attach trace/span exemplars only to selected histogram samples; and
- count dropped/sampled/export-failed telemetry without recursively generating
  more unbounded telemetry.

OpenTelemetry recommends attribute-count and value-length limits because bad
instrumentation can otherwise exhaust memory.[^14] Limits, queue capacity,
batch size, timeout, and sampling configuration should have a sanitized digest
in every capture. Their actual values are configuration evidence, not hidden
defaults in an exporter.

### Hot-path cost discipline

Normal mode should do only the bounded measurements required to answer
operation count, outcome, latency, queue/saturation, and essential work. Use
static call sites, cheap enums, predeclared metric handles, monotonic timestamps,
and no message formatting or large field construction when the signal is
disabled. Detailed counters, plans, backtraces, allocation stacks, scheduler
events, and content capture require explicit scoped elevation.

The overhead test should run the same release workload with `off`, `normal`,
and `detailed` signal policies and record latency distributions, throughput,
CPU, allocations/RSS, disk/network bytes, queue depth, and dropped telemetry.
The owning gate must set the acceptable threshold; this research does not
invent one.

## Deterministic failure and verification strategy

FoundationDB's central lesson is not to copy its simulator. It is that
deterministic seeded failure simulation, live performance tests, and hardware
failure tests are complementary, and deterministic reproduction dramatically
improves diagnosis.[^22] RRFlow should adapt that behavior at its own I/O,
clock, scheduler, provider, authorization, and transport boundaries.

### Test ladder

| Lane | Finds | Required retained evidence | Known limit |
|---|---|---|---|
| Unit and golden | Local invariants, encoding, validation, deterministic transformations | Exact input/output or stable digest | Weak cross-boundary coverage |
| Property/state machine | Long operation sequences and invariant violations | Seed, operation sequence, shrunk counterexample | Model may omit real-system behavior |
| Differential | Divergence from a simple oracle or prior accepted implementation | Both results, configuration, corpus digest | Agreement does not prove both are correct |
| Fuzz corpus | Parser/format/state-machine crashes and pathological inputs | Minimized input, target/build/sanitizer identity | Coverage and time bounded |
| Loom | Interleavings of small synchronization models | Model settings and reproducible schedule | Sees only Loom-aware primitives; state explosion[^19] |
| Miri | Many undefined-behavior classes and selected weak-memory effects | Toolchain, flags, failing test | Interpreter/FFI/platform limitations; not exhaustive[^18] |
| Sanitizer | Native memory, leak, and race faults on supported targets | Target, runtime options, symbolized report | Instrumentation overhead and partial target/code coverage[^20] |
| Deterministic fault | Typed I/O, clock, provider, exporter, and process-boundary failures | Fault point, occurrence, seed/schedule, last stamp/receipt | Injection boundary may differ from hardware |
| Real process | Kill, restart, socket, filesystem, ENOSPC, permission, and native packaging behavior | Command transcript, process/artifact IDs, reopen verification | More expensive and less schedule-deterministic |
| Fixed hardware | Long-running contention, tail, memory, device, and thermal behavior | Complete host/workload/build provenance and raw results | Does not explore every environment |

The Rust sanitizer documentation explicitly recommends combining sanitizers
with other testing and notes target, standard-library, build-script, and
procedural-macro constraints.[^20] Miri and Loom likewise cover different
classes. A green result should always name what the lane could not observe.

### Fault injection architecture

Fault points should be typed stable identifiers attached to a real operation
boundary, not string comparisons scattered through production code. A fault
plan should declare:

- build identity and exact workload/corpus digest;
- operation/boundary/stage and the Nth eligible occurrence;
- injected behavior such as short read/write, rejection, torn suffix,
  corruption, ENOSPC, fsync uncertainty, timeout, cancellation, delayed
  delivery, dropped telemetry, or process termination;
- deterministic seed and schedule where applicable;
- expected pre-crash result and allowed durable states; and
- mandatory close/reopen/verify oracle.

Prefer a thin fault-decorating implementation of an existing explicit I/O or
external-capability interface. Keep injection disabled by construction in
normal/release execution. Do not introduce a `cfg(test)` storage algorithm that
bypasses production code, and do not let a fault hook perform or authorize the
effect it observes.

### Fuzz corpus lifecycle

The Rust Fuzz Book defines fuzzing as pseudo-random input used to find security
and stability issues and documents `cargo-fuzz`/libFuzzer as one Rust path.[^21]
For RRFlow, a useful target must have:

1. a narrow stable parser or state-machine boundary;
2. a correctness, no-panic, resource, or differential oracle;
3. seed corpus cases for every version and structural family;
4. bounded input length, operation count, allocation, and time;
5. minimized failures promoted into named regression fixtures; and
6. a corpus/toolchain/sanitizer digest in release evidence.

Future targets should prioritize public envelopes and streaming frames,
operation/catalogue fixtures, WAL/manifest/segment/page formats, rrflowQL,
graph/BM25/vector inputs, diagnostic capture manifests, and SDK decoders. The
existing two rrflowKV targets remain valid inventory; they are not a workspace
fuzz strategy by themselves.

### Failure artifact contract

Every nontrivial failing run should emit a small manifest even if the large
logs/profile/dump are stored separately:

```text
failure kind + stable error class
build/distribution identity
target OS/architecture and tool versions
test/workload/corpus identity and configuration digest
seed, schedule, fault point, and occurrence
operation/trace/source/read/receipt coordinates available before failure
expected versus observed terminal result
reopen/verification result
artifact paths, byte sizes, digests, redaction class, and retention
reproduction command and required permissions
```

If the failure is nondeterministic, retain every observed coordinate and narrow
the state space. Increasing timeouts, rerunning until green, or deleting a
failing sample is not a correction.

## Profiling strategy

Profiling should start after ordinary metrics and one causal trace identify a
boundary, workload, and symptom. Otherwise the most detailed profile often
answers the wrong question.

| Symptom | First instrument | Escalation | Evidence caveat |
|---|---|---|---|
| High CPU or slower instruction path | RRFlow stage CPU/work counters and release-derived sampling profile | Linux `perf_event`, Windows WPR/WPA, macOS Instruments; Cachegrind for reproducible instruction comparison | Sampling is noisy; instruction count is not elapsed time |
| Long wall time with low CPU | Queue wait, inflight, stage spans, disk/network counters | Native scheduler/I/O trace; Tokio Console in `runtime-analysis` | Scheduler instrumentation changes timing |
| Growing RSS or suspected leak | RSS plus allocator-owned bytes and workload phase | Heap profiler/Massif, LeakSanitizer, long-duration native run | RSS includes mappings/caches and is not allocation ownership |
| Lock/contention stall | Queue/lock wait events and bounded concurrency schedule | Loom for small primitive, TSan for native race, native scheduler profile | No one tool proves deadlock freedom |
| Unexpected storage amplification | logical/stored/mapped/read/written/copied/decode counters | device/filesystem trace and allocation profile | Page cache and compression state must be recorded |
| Async task starvation | Operation links, queue wait, cancellation and task lifetimes | Tokio Console with unstable instrumentation isolated to runtime analysis | Never use as release conformance evidence |

Linux `perf_event_open` supports both counting and sampling hardware/software
events and has explicit permission constraints.[^31] Windows WPR records ETW
CPU, disk, memory, scheduling, and application events for WPA analysis.[^32]
Apple signposts mark named intervals/events for correlation in Instruments.[^33]
These native paths are complementary; a Linux profile cannot qualify Windows
or macOS behavior.

Cachegrind's instruction counts are precise and often highly reproducible, so
they can reveal a small code-path delta hidden by wall-clock noise; its cache
simulation is not a model of modern hardware and the tool runs slowly.[^29]
Massif separates data collection from display and can track heap growth by
instructions, wall time, or allocated bytes.[^30] Both belong to bounded
analysis, never the default runtime.

All profiler outputs need the same manifest, symbol association, phase markers,
redaction, byte limit, digest, and retention policy as diagnostic captures. A
flame graph image without the raw profile and exact workload is a presentation,
not reproducible evidence.

## Benchmark and optimization science

### Valid measurement before faster code

DataFusion recommends the smallest relevant tests first, then wider SQL and
integration coverage; its Criterion microbenchmarks guide focused optimization,
while retained identical datasets support comparison across code versions.[^23]
Criterion exposes confidence, significance, sample, measurement-time,
throughput, and practical noise controls.[^24] Those facilities are useful for
microbenchmarks but cannot replace RRFlow's end-to-end evidence.

Every optimization package should follow this sequence:

1. Bind one product-visible behavior and its correctness/quality oracle.
2. Name the exact client/server/engine/stage boundary and resource suspected.
3. Add or retain the workload before changing the implementation.
4. Record baseline build, host, filesystem/device, configuration, corpus/seed,
   cache state, concurrency, offered load, warmup, sample count, and failures.
5. Profile that exact workload to form a falsifiable cause hypothesis.
6. Make the smallest canonical-boundary change; do not retain a fallback lane
   merely because the new path is incomplete.
7. Rerun correctness, differential/fault, resource, and latency evidence.
8. Compare raw distributions and practical magnitude across interleaved or
   randomized baseline/candidate runs.
9. Accept, revise, or reject the change and record regressions, uncertainty,
   and checks not run.

### Latency distributions and offered load

Tail behavior matters because temporary component stalls can dominate an
interactive service even when medians remain healthy.[^26] RRFlow release
evidence should retain raw histograms and report p50/p95/p99/p99.9 with sample
counts, failures, throughput, offered load, concurrency, and saturation. It
must separate success, denial, cancellation, error, cold/warm cache, and each
declared durability policy.

Closed-loop drivers stop sending work while waiting for a slow response and
can therefore omit the requests that would have arrived during the stall.
HdrHistogram documents recording/correcting against an expected sampling
interval to address this coordinated-omission problem.[^25] Prefer open-loop
arrival scheduling when modeling external demand. If a closed loop is
necessary, record its exact model and correction parameters and retain both raw
and corrected views.

Google SRE guidance ties latency to traffic, errors, and saturation rather than
treating a single metric as health.[^27] For RRFlow that means every latency
comparison also needs CPU, RSS/allocator, disk I/O, queue/inflight, telemetry
drops, logical/stored/allocated bytes, and domain quality such as exact result
equality, recall, ranking quality, context contribution, or successful reopen.

### Benchmark layers

| Layer | Question | Example RRFlow scope |
|---|---|---|
| Micro | Did one pure implementation primitive change? | key compare, codec, scoring kernel, distance function, bounded serialization |
| Component | Did one production subsystem improve under a stable oracle? | rrflowKV scan, graph traversal, BM25, HNSW/rerank, DataFusion operator, context packing |
| Integrated | Did the authenticated stamped engine path improve? | commit, query, context assembly, routine step, delivery |
| End to end | Did an installed client-visible operation improve? | send through validated complete response, restart/readback |
| Clean rollout | Did acquisition/install/attunement/readiness improve? | offline native distribution on empty and existing projects |

An improvement at one layer is a hypothesis for the next layer, not proof of
it. Faster vector distance with worse recall, fewer copied bytes with longer
tail stalls, or faster warm queries with a larger cold-start/installed footprint
is a trade, not an unconditional optimization.

### Domain-specific evidence questions

- **rrflowKV:** Which keys/pages/bytes were examined, mapped, decoded,
  decompressed, copied, allocated, written, flushed, compacted, or spilled?
  Did mixed-family load evict a hot point-read set? Did every acknowledged
  state survive kill/reopen and ENOSPC?
- **Graph and BM25:** Did work fall because of better pruning/indexing or because
  eligible results disappeared? Record traversal/posting work, freshness,
  update amplification, memory, ranking equality/quality, and tail latency.
- **Vector/TurboQuant candidates:** Bind corpus, dimensions, distance, filter,
  exact oracle, recall@k, rerank depth, index build/update cost, footprint, and
  cold/warm search distributions. Never compare unlike recall or filters.
- **rrflowQL/DataFusion:** Bind logical and physical plan digests, row/batch/byte
  counts, pool reservation, spill, queue wait, last-batch consumption, and
  result equality. Operator compute metrics do not include the whole request.
- **Context/LFG:** Measure selected/skipped sources, contribution, repeated or
  conflicting material, bytes/tokens, truncation/compaction loss, quality
  rubric, model/tool attempts, verification, and delivery—not just generation
  latency.
- **Adapters/SDKs:** Measure the same canonical operation across embedded,
  HTTP, WebSocket, CLI, MCP, and generated SDKs with transport/decode time
  separated from engine time and identical denial/error/cancellation semantics.

### PGO is late-stage, workload-bound optimization

Rust supports instrumented and sampling-based profile-guided optimization and
uses representative execution profiles to influence inlining, layout, register
allocation, and other decisions.[^28] PGO should be deferred until J-04 has
stable versioned workloads. The training corpus, target, compiler, profile
merge procedure, and resulting artifact digest become build inputs. Each
candidate still runs the complete correctness/fault/native matrix; an
unrepresentative profile can improve one path while harming another.

## Implementation handoff to existing owners

This is a dependency recommendation, not a new roadmap. The owning Gate H/J
records control actual order and acceptance.

### H-05a: identity before instrumentation

1. Make the operation and metric descriptors executable and generate their
   documentation/conformance projection.
2. Add root-owned release-derived diagnostic/benchmark profiles without
   changing semantic features.
3. Implement one canonical build identity and `version --json` projection.
4. Build release and diagnostic candidates from the same input manifest and
   compare operation catalogue, feature closure, formats, results, receipts,
   limits, and executable-reported identity.
5. Measure `off`/`normal`/`detailed` overhead before admitting normal defaults.

This foundation should precede D-01 installed-binary claims, because an
unidentified binary cannot produce reproducible installation or failure
evidence.

### H-05b: causal context

1. Keep the current `TraceOperation` catalogue as the semantic name source.
2. Route direct-store trace creation through the admitted `RrdEngine` operation.
3. Add bounded W3C parsing/serialization at server/client edges.
4. Model asynchronous work with links and record incomplete durable starts
   across crash/reopen.
5. Prove cross-request, cross-engram, and invalid-context isolation.

### H-05c: correlated diagnostics

1. Define provider-neutral trace, metric, log, and redaction contracts below
   exporter adapters.
2. Compose `tracing-subscriber` layers at binaries for local structured output,
   metrics, capture, and optional OpenTelemetry export.
3. Enforce descriptor-defined units/dimensions/cardinality, bounded queues,
   and self-telemetry.
4. Prove exporter timeout/rejection/overflow cannot alter engine result,
   durability, or latency budget beyond the accepted overhead threshold.

### H-05d: physical and reasoning coverage

Add stage/work/resource evidence only with the owning C-through-I behavior.
Instrument at production abstraction boundaries, reset per-operation detailed
counters, and roll them into aggregate metrics after terminal capture. Import
DataFusion operator metrics into the RRFlow query evidence object; do not let
DataFusion define request outcome or stamp.

### H-05e and J-02: reproduce failures

Implement the sanitized manifest-bound capture, fixed fault corpus, shared
failure-artifact schema, fuzz/Miri/sanitizer/concurrency lanes, process crash/
reopen and storage-full tests, and release/diagnostic parity. The future
`release-evidence` runner should invoke tools and record exact pass/fail/skip
counts; it should not translate a skipped unavailable tool into success.

### J-04 and J-05: measure and ship

Use the same build/workload identity for fixed-hardware distributions and
profiling. Keep raw histograms and resources, qualify native offline bundles,
then assemble the byte manifest, SBOM, provenance, checksums, platform
signatures, verifier, and retained evidence. Promotion remains a separate
explicit owner decision after every prerequisite passes.

## Adopt, defer, and reject

| Decision | Mechanism | Reason |
|---|---|---|
| Adopt | Release-inherited diagnostic profile and machine-readable semantic fingerprint | Makes a failure attributable without creating a debug implementation |
| Adopt | Existing Rust operation catalogue as the generated span/metric operation source | Prevents name drift across engine, transport, docs, and SDKs |
| Adopt | Provider-neutral signal contracts with `tracing` at libraries and subscriber layers at executables | Keeps exporters optional and preserves composition |
| Adopt | W3C trace context at validated transport edges | Provides interoperable correlation without treating headers as authority |
| Adopt | Low-cardinality aggregate metrics plus histogram exemplars | Preserves system trends and selected trace correlation without series explosion |
| Adopt | Typed deterministic fault plans and manifest-bound failure artifacts | Converts intermittent failures into reviewable reproduction inputs |
| Adopt | Layered property/differential/fuzz/Loom/Miri/sanitizer/process/native evidence | Each lane covers a distinct failure class |
| Adopt | Symptom-specific Linux, Windows, and macOS profiling | Platform behavior requires native evidence |
| Adopt | Fixed-corpus micro/component/integrated/end-to-end/rollout benchmarks | Prevents a local speedup from becoming an unsupported product claim |
| Defer | PGO | Requires stable representative J-04 training workloads and native qualification |
| Defer | Continuous always-on eBPF/ETW/signpost collection | Operational need, privilege model, overhead, retention, and supported targets are not yet proven |
| Defer | Protected content capture | Requires separate authorization, encryption, audit, retention, and absence from the default distribution |
| Reject | Debug-only semantic branches or a second diagnostic engine | A passing debug path would not reproduce release behavior |
| Reject | Logs, spans, exporter ACKs, or profiler output as lifecycle state | Observers are lossy and cannot establish a governed effect |
| Reject | Per-request/project/model/path/error-message metric labels | Unbounded cardinality and sensitive-data risk |
| Reject | Always-on backtraces, plan dumps, allocator stacks, or Tokio Console | High/variable overhead and information disclosure |
| Reject | Average-only, development-profile, unpinned-host, or coordinated-omission benchmark claims | They do not represent tail behavior of a release candidate |
| Reject | Optimization without correctness/quality and failure oracles | Faster wrong, stale, incomplete, or unrecoverable output is regression |
| Reject | Hand-maintained duplicate API/SDK/telemetry operation lists | They inevitably drift from executable authority |
| Reject | GitHub, a collector, profiler, model provider, database, or sibling checkout as default readiness | External capabilities remain explicit optional integrations |

## Open implementation questions

The owning packages still need measured answers to these questions:

1. Which split-symbol and path-remapping settings work reproducibly for each
   supported target with Rust `1.98.0`?
2. Where should the canonical build manifest be generated and embedded so it
   remains deterministic, source-complete, and available to every binary?
3. Which metric SDK/exporter implementation meets the closed catalogue,
   cardinality, queue, shutdown, and no-semantic-impact requirements without
   pulling provider code into kernel/authority crates?
4. What normal-mode telemetry overhead threshold is acceptable for each
   latency class, and which fixed corpus can detect regressions above noise?
5. Which current storage/engine/external boundaries can host deterministic
   fault decorators without creating test-only algorithms?
6. Which Loom models are small enough to explore usefully, and which unsafe or
   FFI-bearing paths are executable under Miri and supported sanitizers?
7. What native host inventory, clock discipline, filesystem/device controls,
   warmup, run count, and randomization are required for stable J-04 evidence?
8. What private-repository signing/attestation facility is actually available,
   and what provider-neutral offline verification material must accompany it?

Each answer belongs in that bounded package's active plan and linked journal.
This research should be amended only by a new reviewed research record or an
explicit superseding owner, not silently converted into evidence after tools
or versions change.

## Sources

[^1]: Rust Project, [Profiles — The Cargo Book](https://doc.rust-lang.org/cargo/reference/profiles.html), current documentation accessed 2026-09-14.
[^2]: Rust Project, [Remap source paths — The rustc book](https://doc.rust-lang.org/rustc/remap-source-paths.html), accessed 2026-09-14.
[^3]: Reproducible Builds project, [`SOURCE_DATE_EPOCH` specification](https://reproducible-builds.org/specs/source-date-epoch/), revision 1.1, accessed 2026-09-14.
[^4]: SLSA, [Build provenance](https://slsa.dev/spec/v1.2/build-provenance), specification 1.2, accessed 2026-09-14.
[^5]: GitHub, [Artifact attestations](https://docs.github.com/en/actions/concepts/security/artifact-attestations) and [using artifact attestations](https://docs.github.com/en/actions/how-tos/secure-your-work/use-artifact-attestations/use-artifact-attestations), accessed 2026-09-14.
[^6]: OpenTelemetry, [Tracing API](https://opentelemetry.io/docs/specs/otel/trace/api/), specification 1.60.0 site, accessed 2026-09-14.
[^7]: W3C, [Trace Context](https://www.w3.org/TR/trace-context/), Recommendation, accessed 2026-09-14.
[^8]: Tokio project, [Getting started with Tracing](https://tokio.rs/tokio/topics/tracing) and [`tracing::Span` async guidance](https://docs.rs/tracing/latest/tracing/struct.Span.html), accessed 2026-09-14.
[^9]: Tokio project, [`tracing-subscriber` layers and filters](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/layer/), accessed 2026-09-14.
[^10]: OpenTelemetry, [Metrics data model](https://opentelemetry.io/docs/specs/otel/metrics/data-model/), accessed 2026-09-14.
[^11]: Prometheus, [Instrumentation practices](https://prometheus.io/docs/practices/instrumentation/) and [histograms and summaries](https://prometheus.io/docs/practices/histograms/), accessed 2026-09-14.
[^12]: OpenTelemetry, [Error handling](https://opentelemetry.io/docs/specs/otel/error-handling/), accessed 2026-09-14.
[^13]: OpenTelemetry, [Recording errors](https://opentelemetry.io/docs/specs/semconv/general/recording-errors/) and [`error.type`](https://opentelemetry.io/docs/specs/semconv/registry/attributes/error/), accessed 2026-09-14.
[^14]: OpenTelemetry, [Common attribute limits](https://opentelemetry.io/docs/specs/otel/common/), [semantic convention groups](https://opentelemetry.io/docs/specs/semconv/general/semantic-convention-groups/), and [telemetry schemas](https://opentelemetry.io/docs/specs/otel/schemas/), accessed 2026-09-14.
[^15]: OpenTelemetry, [Logs data model](https://opentelemetry.io/docs/specs/otel/logs/data-model/), accessed 2026-09-14.
[^16]: OWASP Foundation, [Logging Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Logging_Cheat_Sheet.html), accessed 2026-09-14.
[^17]: Rust Project, [`std::backtrace`](https://doc.rust-lang.org/stable/std/backtrace/index.html), accessed 2026-09-14.
[^18]: Rust Project, [Miri](https://github.com/rust-lang/miri/), accessed 2026-09-14.
[^19]: Tokio project, [Loom concurrency testing](https://docs.rs/loom/latest/loom/), version 0.7.2 documentation accessed 2026-09-14.
[^20]: Rust Project, [Sanitizer compiler support](https://doc.rust-lang.org/beta/unstable-book/compiler-flags/sanitizer.html), accessed 2026-09-14.
[^21]: Rust Fuzz project, [Rust Fuzz Book](https://rust-fuzz.github.io/book/), accessed 2026-09-14.
[^22]: FoundationDB project, [Simulation and testing](https://apple.github.io/foundationdb/testing.html), version 7.4.7 documentation accessed 2026-09-14.
[^23]: Apache DataFusion, [Testing](https://datafusion.apache.org/contributor-guide/testing.html), documentation for the current project accessed 2026-09-14; RRFlow separately pins DataFusion 55.0.0.
[^24]: Criterion.rs, [`BenchmarkGroup`](https://docs.rs/criterion/latest/criterion/struct.BenchmarkGroup.html), accessed 2026-09-14.
[^25]: HdrHistogram project, [HdrHistogram](https://github.com/HdrHistogram/HdrHistogram), accessed 2026-09-14.
[^26]: Jeffrey Dean and Luiz André Barroso, [The Tail at Scale](https://research.google/pubs/the-tail-at-scale/), Communications of the ACM 56 (2013), accessed 2026-09-14.
[^27]: Google, [Monitoring distributed systems](https://sre.google/sre-book/monitoring-distributed-systems/) and [Service level objectives](https://sre.google/sre-book/service-level-objectives/), Site Reliability Engineering, accessed 2026-09-14.
[^28]: Rust Project, [Profile-guided optimization](https://doc.rust-lang.org/nightly/rustc/profile-guided-optimization.html), accessed 2026-09-14.
[^29]: Valgrind project, [Cachegrind: a high-precision tracing profiler](https://valgrind.org/docs/manual/cg-manual.html), accessed 2026-09-14.
[^30]: Valgrind project, [Massif: a heap profiler](https://valgrind.org/docs/manual/ms-manual.html), accessed 2026-09-14.
[^31]: Linux man-pages project, [`perf_event_open(2)`](https://man7.org/linux/man-pages/man2/perf_event_open.2.html), man-pages 6.19, accessed 2026-09-14.
[^32]: Microsoft, [Windows Performance Toolkit](https://learn.microsoft.com/en-us/windows-hardware/test/wpt/), accessed 2026-09-14.
[^33]: Apple, [Recording performance data](https://developer.apple.com/documentation/os/recording-performance-data), accessed 2026-09-14.
[^34]: OpenAPI Initiative, [OpenAPI Specification 3.1.1](https://spec.openapis.org/oas/v3.1.1.html), 2024-10-24, accessed 2026-09-14.
