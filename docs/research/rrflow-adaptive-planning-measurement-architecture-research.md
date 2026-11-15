# Adaptive planning and memory: measurement research

**Status:** active supporting research; no new product capability or qualification claim
**Coordinate:** `rrflow://rrflow-instance/data/research/adaptive-planning-measurement`
**Owner:** [J-04 workload and evidence structure](../roadmap/rrflow-1.0/gate-j.md#workload-and-evidence-structure), [OBJ-10/11/14](../objectives/rrflow-1.0-alpha.md#required-outcomes), and [ADR-0002](../decisions/0002-adaptive-reasoning-governed-effects.md)

This research informs those owners; it cannot change their status, create a
second evaluation or runtime authority, or establish a performance result.
Inspected public main `3a72a6b4adc43c842f67a59f4fc91d725eac7580` and the
research described below. Dates, source revisions, results and limitations are
facts about the particular run, not assumptions about later code.

## Question and current evidence

Can an engine-selected plan and project memory improve a real developer task
without stale or unauthorized effects, at a measurable quality and resource
cost? This is an empirical question. It cannot be answered by a smaller prompt,
a benchmark score from another product, a passing substring, or a schema alone.

At this revision, both bundled installation profiles still contain the same
eleven attunement phases. The installation planner accepts an explicit
alternative proposal, but its default copies the profile; `AttunementPlan`
remains a linear, closed phase sequence ending in Verify. The worker executes
Connect and Inventory; other phase dispatches report unavailable. These facts
describe attunement, not a universal AI reasoning engine. `engine/flight.rs`
persists prompt-flight events and uses stamped context; its observe path does
not prove external model execution. Actual parsing, task-specific planning,
handoff and a full project-to-query prompt remain separate work.

The existing provider runner in `crates/evaluation/rrflow-eval/src/main.rs`
labels each run `baseline` or `runtime` but does not itself attach an RRFlow
treatment. Its screening outcome is process success plus substring presence in
raw provider output; raw output can include echoed or incidental text. It does
not run a hidden independent checker. Historical
`eval/results/2026-08-18-summary.json` records only one run per provider and
scenario/arm, lacks repository identity per trial, and retains no raw provider
outputs in the repository. It is historical, not statistical evidence of
RRFlow's causal benefit.

At the inspected base, the original verifier accepted an eight-row synthetic
report with `success=false` in all rows, `successes=8`, and duplicated provider
and repository names. The existing three provider-compatibility tests passed.
The exact counterexample and command are recorded in the linked change journal.
This is a defect in provider-summary verification, not a claim about the
separate C-07 storage manifest verifier. This candidate strengthens screening
consistency checks; it does not establish treatment attachment or independent
answer correctness.

The bundled default currently sets inventory limits of 100,000 entries,
30,000 ms, one worker and 32 open files; the default engram configuration
sets a 4 MiB page cache, graph depth 4, 128 recall items, and a 256-step
reasoning *budget*. These are settings in the profile/configuration assets,
not proof that the 100,000-entry case passes or that these settings are
optimal. The inventory policy is bundled profile input, not a mutable global
settings API. The 256-step budget is not the eleven-phase default. Record
effective values and origins, then vary one bounded setting at a time only
against a frozen correctness/quality oracle and declared workload.

## Research and decisions

| Source and scope | What it supports | RRFlow-specific decision and limit |
| --- | --- | --- |
| [BEIR](https://arxiv.org/abs/2104.08663), heterogeneous retrieval tasks | Retrieval quality varies across tasks; lexical BM25 is an important baseline, while more elaborate ranking may cost more. | Measure exact, lexical, vector, hybrid and graph contributions where implemented. Use held-out real project questions; BEIR scores do not prove RRFlow source-code retrieval. |
| [LongMemEval](https://arxiv.org/abs/2410.10813), long-term interactive memory | Extraction, cross-session reasoning, temporal reasoning, updates and abstention are distinct evaluation dimensions. | Add dated updates, conflicts, forgotten/unauthorized facts and unanswerable project questions. A relevant document hit is not proof that its latest fact was used. |
| [RAGTruth](https://arxiv.org/abs/2401.00396), annotated retrieved-answer hallucinations | An answer can contain unsupported claims even when retrieval supplies material. | Judge source attribution and claim support separately from retrieval recall; deterministic source/range checks and independently reviewed claim judgments have different coverage. |
| [NIST measurement guidance](https://airc.nist.gov/airmf-resources/airmf/5-sec-core/) | Document test data, methods, uncertainty and the limits of generalization; independent assessment improves evaluation. | Freeze data, baseline and evaluator before trials; retain failed cases and use a checker separate from the engine. No claim of regulatory certification follows. |
| [Qdrant hybrid queries](https://qdrant.tech/documentation/hybrid-queries/) and [quantization](https://qdrant.tech/documentation/quantization/) | Lexical/dense fusion and rescoring are options; recall, latency, memory and candidate generation trade off. | Compare filtered recall against exact results and measure ingestion/update and query costs at matched quality. Qdrant is an optional external knowledge path and a comparator, not the RRFlow authority. |
| SurrealDB [transactions](https://surrealdb.com/docs/reference/query-language/statements/begin) and [relations](https://surrealdb.com/docs/reference/query-language/statements/relate) | Related records can be queried as graph edges within a database transaction; the documented transaction mode is snapshot isolation. | Test RRFlow's stated consistency and graph semantics independently. Do not infer serializability, or an atomic transaction spanning an external filesystem and RRFlow, from another database's design. |
| [OpenTelemetry propagation](https://opentelemetry.io/docs/concepts/context-propagation/) and [sampling](https://opentelemetry.io/docs/specs/otel/trace/sdk/) | Traces can correlate operations, and sampled/non-recording spans can omit events. | Correlate run/attempt identifiers with diagnostic spans; required durable audit/receipt readback must not rely on sampled telemetry. Sensitive content capture remains explicit. |
| [Temporal retry guidance](https://docs.temporal.io/encyclopedia/retry-policies) | Deterministic replay and failure-prone activities have different requirements. | An accepted local effect and its receipt use the existing engine transaction where supported; an external call with uncertain outcome requires idempotency or reconciliation before retry. No Temporal dependency is implied. |

Public benchmark datasets should be checked for license, permitted use,
source/label provenance and contamination before acquisition. They complement,
not replace, a private or otherwise held-out project task set. Do not call a
model-graded rubric ground truth without calibration and reviewed disagreement.

## A reproducible experiment, before an optimization

The evaluation unit is an independently defined task at a frozen project
snapshot, not a token or repeated measurement of the same task. Before running,
record the question, comparison, expected direction, practical decision margin,
correctness and quality gates, selection rule, sample plan and stop condition.
Use a pilot to estimate variance and feasibility; a pilot is not a promotion.
Keep tuning tasks distinct from held-out projects/time periods and never use
the held-out answers to adjust a candidate. Preserve raw per-task results and
all failures, timeouts, denials, retries and unknown values.

For paired trials, pin project/source snapshot, user task, authorized corpus,
model/provider revision, permissions and task budget. Define the control as an
actual executable configuration and the candidate as a different actual
configuration; record engine/binary/configuration/adapter digests and proof of
what each received. Randomize or counterbalance order across independent runs;
record warmup and cache reset/retention rather than assuming equal state. Use
the same independent task oracle for both arms. A provider CLI's exit status,
arm string, and expected-output substring are screening signals only.

Report paired differences with task/project counts and confidence intervals
appropriate to the sampling design. When tasks share a repository, resample or
otherwise account for project clustering rather than treating every prompt or
repetition as independent. A small number of projects cannot support a precise
population claim. Predeclare handling of ties, missing/unknown metrics and
failed trials; do not drop them from the denominator. Multiple tuning trials
make a selected winner optimistic; evaluate the frozen winner once on held-out
cases. Record variation across model runs separately from variation across
projects. For a binary destructive-effect invariant, any observed violation is
a failed candidate; zero observed failures does not prove zero failure rate.

### Measurements and independent oracles

| Question | What is measured | Independent oracle / required limitation |
| --- | --- | --- |
| Planning and handoff | Selected/available/skipped capabilities and reasons; accepted plan/schema/config/source digests; unanswered-question disposition; effect and handoff outcomes. | Two distinct project/config cases and no-provider case use the public engine path. Unknown capability, altered plan, revoked grant, changed source and changed required answer must refuse the affected effect before mutation. Unrelated work remains possible only when its dependency set is complete. |
| Project ingestion | Time to first usable result and complete indexed source, read/skip/error counts and bytes, CPU/RSS/I/O, incremental work, and excluded data. | Independent expected file inventory, source span/range bytes and relationship set for frozen projects; edit/delete/permission denial and kill/reopen must produce expected results. A metadata count alone is not knowledge. |
| Retrieval and memory | Eligible relevant evidence recall@k, ranking (for graded labels use nDCG@k), stale/unauthorized hit rate, latest-fact accuracy, conflicting-fact handling and correct abstention. Record selected, delivered and actually cited/used evidence separately. | Freeze labeled queries, eligible corpus, current source revision and permissions; compute exact relevant sets separately. For zero relevant items report the no-answer/abstention stratum, not an invented recall value. Approximate-vector recall compares the same filter and distance against exact search. |
| Task/answer quality | Task success and independently checkable source support, missing/contradicted claims, ability to abstain, independent test result. | Use hidden tests and frozen expected effects outside the agent's write access. Model judges are secondary and calibrated on reviewed cases; they cannot mark their own effects correct. |
| Costs and latency | Input/output/cache tokens only if actually reported with provider-specific semantics; model charges/price revision when available; local indexing and retrieval work; p50/p95/p99/p99.9 with sample counts, throughput, errors and saturation. | Measure from named client, engine and provider boundaries. Separate ingestion/build/update from amortized query costs; treat absent usage and price as unknown. Open-loop arrival or documented coordinated-omission correction is required where J-04 says so. |
| Durable safety | Plan/effect IDs, source and authorization observations, commit/receipt identity, memory read/write, attempts and uncertain outcomes. | Compare independent expected history to raw readback after real process kill/restart, lost acknowledgement and replay. Traces can be missing and are never the audit oracle. |

For recall@k, count the relevant eligible IDs retrieved in the first k divided
by the number of relevant eligible IDs. For approximate-neighbor recall@k,
compare returned IDs with an independently computed exact top-k using a
declared tie rule. These answer different questions and must not share a label
without an explicit qualifier. Report k, filter, source/read stamp and corpus.

## Runtime information needed for useful debugging

Extend the existing H-05 signal and executable operation owners, not a new
parallel catalogue. Each observable attempt should link project/engram,
correlation/trace ID, actor, plan/effect/attempt ID and effective config
revision; selected capability and availability; source/read stamp; budget and
actual units; queue and execution timing; admitted/denied/failed/uncertain
outcome; and durable receipt/readback reference. Include whether work came from
rrflowKV, rrflowQL, a model, a cache or an external adapter without exposing
secrets or private reasoning. Record collection level, export errors, dropped
samples, clock source and measurement overhead. Missing instrumentation is
`unavailable`, not zero. Stable low-cardinality labels go in metrics; unique
plan/attempt IDs belong in bounded records or traces to avoid metric explosion.

Long-lived authoritative data reside in rrflowDB/rrflowKV, and per-prompt
packets/caches are bounded ephemeral selections derived at a permitted read
stamp. A memory write is a separately authorized durable effect. It is not
safe to claim that an ephemeral cache entry is durable memory or that reducing
context tokens automatically reduces total end-to-end cost.

## Execution handoff

1. Retain the existing C-07/J-04 storage verifier, inventory, source identity,
   leases, audit and raw-failure handling. Land the provider screening repair
   only after its exact candidate CI. Historical receipts remain history; the
   new screening check is not the J-04 workload manifest.
2. Implement the engine-owned capability/effect selection and successor-plan
   boundary at the existing installation/attunement and task/flight owners.
   Characterize persisted plan/job/checkpoint formats first. Replace the two
   fixed defaults only after the selected executable work and preserved data
   are demonstrable. The plan schema validates effects, not private thought.
3. Deliver one installed existing-project path through inventory, source-linked
   parse/relations, rrflowQL readback, restart, edit/delete and targeted
   invalidation. Reconcile draft parser and inherited-directory work; neither
   a mock nor a source tree alone qualifies. The first result is a developer
   checkpoint, not alpha completion.
4. Bind an independent task corpus, actual treatment identity, traces and
   durable effect receipts to the existing evaluation crate. Add the missing
   public engine operations and Connectome/SDK readback through the executable
   catalogue. LFG/SWAG are explicit optional bindings; verify absent,
   unauthorized, unavailable and actual-result cases.
5. Only then compare configurations at frozen identity and predeclared quality.
   Run the existing J-04 native matrix and C-07 capacity/soak on their accepted
   integrated revision. Do not substitute this research or screening fixture
   for those required results.

The numeric defaults in the current engram/profile are operating settings, not
scientifically optimal values. Until actual installed runs exist, no measured
winner, competitive-equivalence claim, optimum cache size, graph depth, token
saving or alpha readiness can be reported from this document.
