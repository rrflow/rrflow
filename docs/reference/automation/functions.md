# RRFlow governed functions and transaction bindings

**Status:** active target contract; C-03 atomic persistence and recovery are accepted, with later capability gaps named below
**Coordinate:** `rrflow://rrflow-instance/data/reference/automation/functions`
**Owner:** governed function definition, execution, transaction-binding, persistence, and evidence semantics

A governed function is bounded computation inside `RrdEngine`. It can transform
one schema-bound value, validate a proposed transaction mutation, or propose a
typed effect for the engine to validate. It is not a database, routine, event
trigger, host hook, project command, model tool, or rrflowQL planner.

The [engine automation flow](../../architecture/engine-data-flow.md#automation-routine-and-skill-flow)
owns how functions relate to committed events, triggers, routines, skills, and
external activities. The [transaction flow](../../architecture/engine-data-flow.md#write-and-commit-flow)
owns final authorization and publication. This record owns the narrower
function boundary. Roadmap A-07 freezes its code vocabulary, C-01 through C-04
make its state and effects transactional, I-01/I-02 converge event and binding
semantics, I-06 supplies installation, and H/J prove public and release behavior.

## Authority invariants

- `RrdEngine` is the only authority that resolves, authorizes, invokes, records,
  or applies a governed function.
- A function receives no storage, query, graph, index, DataFusion, model,
  network, filesystem, process, environment, clock, random, secret, event-log,
  or lifecycle handle. The engine resolves authorized facts first and supplies
  only the bounded typed input.
- A function returns a value or proposal. It never commits. `RrdEngine`
  validates the output schema, independently authorizes every proposed effect,
  and either commits the complete semantic batch or commits none of it.
- rrflowMX and rrflowKV implement the same catalogue, invocation, binding,
  conflict, and result semantics. Only rrflowKV may claim survival after close,
  crash, or reopen.
- Function bytes come from the RRD binary, its manifest-verified distribution,
  or content-addressed rrflowDB state installed by an exact plan. Runtime path
  lookup, sibling repositories, native dynamic libraries, and network fetching
  are forbidden.
- No function event or invocation starts work by itself. Post-commit matching
  belongs to a persisted trigger; multi-step progress belongs to a routine;
  nondeterministic or external work belongs to a prepared activity.

## Locked vocabulary

| Object | Meaning | Persistence and authority |
|---|---|---|
| Function artifact | Immutable executable content, media type, byte length, content digest, runtime profile, and provenance. | A sealed built-in is accounted for by the RRD binary manifest; portable content is stored once by digest in rrflowDB. |
| Function definition | Immutable function identity/revision bound to one artifact or built-in, input/output schema digests, limits, deterministic profile, and maximum proposal capabilities. | Governed rrflowDB record; definitions never contain mutable run state. |
| Function catalogue | Immutable revision listing exact function-definition and transaction-binding identities/digests. | Governed rrflowDB records plus one compare-and-swap head; not an automation-wide catalogue. |
| Function invocation | Engine-created identity binding principal, operation, read stamp, catalogue/definition/artifact revisions, schemas, input digest, limits, and runtime build. | Prepared or executed only by `RrdEngine`. |
| Function invocation receipt | Validated status, output/proposal digest, resource accounting, and causal coordinates for one invocation. | Reused for idempotent recovery and committed with the operation evidence it supports. |
| Transaction function binding | Immutable, ordered selection of a governed function for a declared proposed-transaction mutation kind and optional record/relation/event kind. | Evaluated before publication; not a post-commit trigger. |
| Transaction function effect | Closed interpretation of valid output: require boolean truth or propose one schema-bound engine event. | Revalidated and effect-authorized by `RrdEngine`; the function cannot apply it. |
| rrflowQL native function | Vectorized Arrow expression or physical operator registered in rrflowQL/DataFusion. | Query-compute contract; never inferred from a governed function definition. |

`Automation` is a documentation category, not a catalogue, runtime, state
machine, or Rust authority. `Trigger` means only a persisted predicate over a
committed canonical engine event.

### Direct pre-release convergence

The governed-function slice of A-07.1 has directly removed the overloaded
catalogue and transaction-binding spellings from executable source. It added
no alias or fallback decoder, and the closed-schema fixture rejects the former
field shapes. The remaining event-capability spellings change with the sole
engine-event envelope in I-01 so this slice does not invent an intermediate
event contract.

| Superseded spelling | Canonical spelling | Current disposition |
|---|---|---|
| `AutomationCatalogue` | `FunctionCatalogue` | Removed from executable source in A-07.1. |
| `ReplaceAutomationCatalogue` | `ReplaceFunctionCatalogue` | Removed from executable source in A-07.1. |
| `ListAutomationCatalogue` | `ListFunctionCatalogue` | Removed from executable source in A-07.1. |
| private `AutomationHead` | `FunctionCatalogueHead` | Removed from executable source in A-07.1. |
| `FunctionTrigger` | `TransactionFunctionBinding` | Removed from executable source in A-07.1. |
| `FunctionTriggerMutation` | `TransactionMutationKind` | Removed from executable source in A-07.1. |
| `FunctionTriggerEffect` | `TransactionFunctionEffect` | Removed from executable source in A-07.1. |
| `trigger_id` / `triggers` | `binding_id` / `transaction_bindings` | Former wire fields are rejected. |
| `FunctionCapability::EmitEvent` | `FunctionCapability::ProposeEngineEvent` | Sequenced with I-01 canonical engine-event lowering. |
| `append_event` function effect | `propose_engine_event` | Sequenced with I-01 canonical engine-event lowering. |

The wire contract version is a technical schema identity, not the RRFlow
product version. Its digest must cover every definition, binding, artifact,
schema, limit, capability, runtime-profile, and ordering field.

## Definition and artifact closure

An accepted definition binds at least:

- estate, function identity, immutable revision, predecessor, definition
  digest, activation/retirement state, and catalogue membership;
- implementation kind, exact built-in registry entry or artifact identity,
  content digest, media type, byte length, runtime profile, ABI, and engine
  runtime build digest;
- closed input and output schema identities/digests, canonical encoding,
  maximum nesting/items/bytes, and numeric domain;
- compilation, elapsed-time, instruction/interrupt/fuel, heap, stack, linear
  memory, table, instance, output, and concurrency limits;
- allowed proposal capabilities, each of which remains subject to invocation
  policy and effect-complete authorization; and
- provenance, signed distribution or installation-plan digest, creator
  principal, creation commit, and retirement coordinates.

Unknown fields, duplicate identities, missing artifacts, digest drift,
unsupported runtime profiles, zero or overflowing limits, schema mismatch,
unavailable enforcement, mutable native code, or cross-estate references fail
closed. Advertised catalogue and artifact maxima must fit the actual physical
transaction and record limits; a public limit that storage cannot represent is
a contract defect.

Default functions may be compiled into the RRD binary as sealed Rust registry
entries. The distribution manifest binds their names, implementation digests,
schemas, and engine build. This is the only native form: RRFlow does not load an
operator-provided `.so`, `.dll`, or arbitrary Rust symbol. Portable project or
operator functions are bundled and installed as verified artifacts. Installation
never compiles unreviewed native code inside the daemon or downloads a runtime.

## Common value contract

The current natural JSON/`QueryValue` boundary is useful but incomplete. The
accepted contract keeps bounded canonical values and additionally requires
input/output schemas and encoding identities. The JSON profile permits null,
boolean, string, list, map, exact safe-range signed/unsigned integers, and a
declared canonical decimal representation. Digest values remain distinguishable
from ordinary strings across the engine boundary even if a runtime ABI encodes
both as JSON strings.

Input and output bytes, nesting depth, item count, map-key length/count, string
length, and numeric precision are validated before and after execution. A
runtime cannot widen the limits encoded by the definition. Large content is
passed by an authorized immutable object coordinate and digest, never copied
into an unbounded function value.

Examples of legitimate uses are a reasoning-node transition invariant, a
schema-bound normalization proposal, or a project-tree commit validator. These
are examples, not universal hardcoded lifecycles: the installed definition and
binding determine whether a project uses them.

## Runtime profiles

Each invocation receives fresh mutable runtime state. A bounded compiled-code
cache may reuse only immutable compiled artifacts keyed by content digest,
runtime profile, runtime build, target, and compilation settings; it has byte
accounting, eviction, invalidation, and no guest-visible state. Cache reuse
cannot change outputs or bypass validation.

### Sealed built-in Rust

A built-in is a pure function registered at compile time. It accepts and
returns the same validated contract values as portable functions and receives
no `RrdEngine` or repository handle. Panic containment, elapsed-time and output
bounds, schema validation, and receipt accounting remain mandatory. Core
rrflowQL operators such as native vectorized RRF are not automatically exposed
through this registry.

### Embedded JavaScript

JavaScript source is a content-addressed artifact evaluated by the exact
embedded QuickJS build named by the runtime profile. Each call receives a fresh
runtime/context, one synchronous function, and canonical input. Modules,
promises, host bindings, `Date`, `Intl`, dynamic evaluation, constructors that
recover dynamic evaluation, randomness, locale-sensitive behavior, ambient
environment, and unbounded regular-expression or container amplification are
denied or covered by explicit enforced limits.

Heap, stack, interrupt/instruction, compilation, output, and monotonic deadline
limits are distinct. An interrupt callback count is implementation-specific; it
is not described as portable instruction fuel. JavaScript is eligible for an
alpha transaction binding only after the frozen runtime profile and
supported-target corpus produce identical canonical results or recovery no
longer depends on cross-build re-execution. The present five-global disabling
script and four engine tests do not establish that proof.

### Portable WebAssembly

The first accepted Wasm profile is a no-import, no-WASI, no-start-function,
single-memory deterministic guest. Its feature set, including floating point,
SIMD, relaxed SIMD, threads, reference types, tables, memory count, and memory
growth behavior, is explicitly enabled or rejected rather than inherited from
runtime defaults. The alpha deterministic profile either rejects floating and
relaxed operations or proves their canonicalization against every supported
target.

Module byte limits are not sufficient by themselves. Validation and eager
compilation also bound types, functions, globals, exports, code size,
recursion, stack, memories, tables, instances, compilation memory, and elapsed
time. Execution uses fresh store state plus fixed fuel, memory, stack, output,
and deadline limits. Runtime errors are classified by typed causes rather than
matching human error strings.

The current JSON ABI exports `memory`, `rrd_alloc(i32) -> i32`, and
`rrd_run(i32, i32) -> i64`, with output pointer and length packed into the
result. A-07 freezes the final symbol namespace and a golden module before that
shape becomes a 1.0 contract. Pointer ranges, overlap, UTF-8, canonical JSON,
schema, output length, and fuel accounting all fail closed. A future Arrow
batch ABI, if justified, is a separately versioned rrflowQL function contract;
it is not smuggled into this JSON ABI.

## Standalone invocation flow

```text
authenticated request
  -> effect-complete function-execute authorization
  -> capture ReadStamp + function-catalogue head
  -> resolve exact definition + schemas + artifact/runtime build
  -> validate input and create invocation identity
  -> execute inside the bounded runtime profile
  -> validate canonical output and allowed proposal class
  -> commit invocation receipt + audit/trace evidence when persistence is required
  -> return typed result with the same causal coordinates
```

A direct invocation that returns a value does not imply a data mutation. If its
receipt or audit is durable, that write still goes through the same engine
transaction authority; the sandbox never appends it directly.

## Proposed-transaction binding flow

Bindings inspect only original proposed mutations. Ordering is original
mutation order followed by canonical binding identity. Function-generated
proposals are not rematched, preventing recursive chains.

```text
authenticated transaction commit request
  -> capture one transaction ReadStamp + function-catalogue revision
  -> authorize original semantic effects and every selected binding
  -> build schema-bound input from original mutation + stable transaction facts
  -> execute or reuse the exact prepared FunctionInvocationReceipt
  -> validate output; lower require_true or proposed EngineEvent
  -> independently authorize and validate every derived effect
  -> build one effect-complete semantic write batch
  -> atomically commit:
       records/relations/index deltas + canonical events + outbox
       + function receipts + transaction state + audit + commit cursor
  -> acknowledge only the storage commit receipt
```

`require_true` succeeds only for exact boolean `true`. `propose_engine_event`
accepts only the declared event schema and capability; the engine supplies
identity, producer, scope, provenance, authorization, causation, idempotency,
and commit coordinates. A validator cannot forge those fields.

A runtime error, false validator, malformed proposal, authorization denial,
conflict, or storage failure advances no domain cursor and cannot leave a
terminal allowed audit record. A failed/denied attempt may be recorded through
its own bounded failure operation, but an allowed completion is atomic with
the domain commit it describes.

## Retry, crash, and upgrade semantics

The function has no external side effect and is never blindly retried inside
the runtime. One prepared invocation identity has one accepted receipt. Before
publication, the engine durably binds the validated output/proposal digest,
definition/artifact/runtime identities, resource result, and intended semantic
batch. Idempotent recovery reuses that prepared receipt or the known storage
commit outcome; it does not rerun old bytes under a different runtime build.

If no preparation was durably accepted, re-execution is safe only because the
function has no external capabilities and the same pinned runtime profile is
available. Missing artifacts, definitions, schemas, runtime builds, or prepared
receipts are explicit failed preconditions. A software upgrade cannot silently
substitute a new QuickJS/Wasmi build for an in-flight invocation.

## Persistence and catalogue publication

The final storage shape is not one JSON control value. Function artifacts,
definitions, transaction bindings, catalogue revisions, invocation receipts,
and the catalogue head are separate typed semantic records keyed through the
C-01 ordered codec. The immutable catalogue revision contains references and
digests, not repeated source or base64 module bytes. A compare-and-swap head
advance and its membership records publish through `RrdEngine` at one stamp.

Content-addressed artifact bytes are written once, verified before reference,
and accounted by logical, encoded, allocated, cached, and resident bytes.
Staged but unreferenced content is not active and may be reclaimed only through
a bounded engine-owned maintenance operation. The control journal must not
duplicate complete executable artifacts in every transition.

rrflowMX holds the same typed records in volatile memory. rrflowKV persists
them through the same semantic transaction contract and proves close/reopen,
crash, corruption, compaction, and storage-full behavior. Function artifact
storage is ordinary rrflowKV binary data; it is not forced into Arrow column
pages or queried through DataFusion merely because Arrow is part of RRFlow.

## rrflowQL, Arrow, and DataFusion boundary

Governed functions are scalar control-path invocations. DataFusion UDFs and
native physical operators are vectorized query-path computation over Arrow
arrays. RRFlow does not register every JavaScript/Wasm governed function as a
DataFusion UDF, invoke a VM once per row, or let a function call DataFusion.

`math::rrf()`, graph expansion, BM25 candidates, vector candidates, exact
reranking, and other high-volume operators belong in rrflowQL as native
vectorized implementations. They consume stamped Arrow batches under the
query budget and return compute results to `RrdEngine`; they never commit.

Any future bridge requires an explicit Arrow signature, batch ABI, null and
dictionary semantics, volatility, planner visibility, vectorized resource
accounting, cancellation, and exact result corpus. It is admitted by Gate F
measurements and remains compute-only. A JSON function catalogue is not that
bridge.

## Security, evidence, and surfaces

Catalogue read, definition/artifact publication, catalogue-head replacement,
standalone execution, binding installation, and proposed effects have distinct
deny-by-default actions. Definition capabilities are maximum possible proposals,
not grants. Principal policy, binding policy, event schema policy, field/record
policy, and transaction effect authorization are evaluated at the operation's
captured coordinate.

A function invocation receipt binds at least the estate/instance, invocation,
request, operation, correlation, principal and representation, authorization
digest, read stamp, catalogue revision/digest, definition revision/digest,
artifact and runtime profile/build, input/output schema and digests, status,
resource limits/consumption, proposed-effect digest, and commit/audit/trace
coordinates. Raw executable bytes, secrets, and unnecessarily sensitive input
or output are excluded from audits and traces; referenced evidence remains
authorized and digest-verifiable.

HTTP, WebSocket, SDK, CLI, MCP, and Connectome may administer or invoke only
the same catalogued public operations. Generated schema presence is not
surface proof. H-04 compares result, denial, stamp, revision, digest, and
receipt across real surfaces; capability discovery advertises only bindings
that actually exist.

## Installation, attunement, and retirement

The signed default distribution accounts for the embedded runtimes, built-in
registry, default function artifacts, schemas, and golden vectors. A project
specialization may request additional definitions or bindings, but attunement
only creates an inactive candidate. I-06 exact preview shows every artifact,
definition, binding, schema, permission, resource limit, and installed record.
Apply accepts that digest through `RrdEngine`; it performs no hidden compilation,
download, host-hook registration, or function execution.

Retirement blocks new invocations while preserving pinned in-flight work and
historical receipts. Uninstall removes only RRFlow-owned active bindings and
artifacts proven unreferenced; destructive history removal is a separate
authorized operation. Existing project databases and their functions remain
external unless an operator installs a bounded adapter or capability.

## Current checkout disposition

| Current code | Verified useful behavior | Defect or required convergence |
|---|---|---|
| `rrd-contract/src/function.rs` plus `fixtures/function-contract-v1.json` and `tests/function_contract.rs` | The closed contract now separates one content-addressed binary artifact from immutable typed definitions and bindings; pins input/output schemas, runtime profile/build, catalogue/definition/binding digests, limits, proposals, and invocation receipts; rejects inline source, absent artifacts, stale digests, unsafe values, former fields, more than 3 MiB decoded artifact content, and more than 4 MiB canonical catalogue JSON. | The receipt does not yet carry the complete principal/representation/authorization/read-stamp/trace coordinate set required by H-04/H-05, and no cross-language fixture exists. |
| `rrd-store/src/access/function.rs`, `repository/function.rs`, and `rrd-engine/src/engine/function/catalogue.rs` | rrflowMX and rrflowKV store binary artifacts once by digest, typed definition/binding records by immutable digest, digest-only membership revisions, one CAS head, and typed receipts through the common transaction port. `RrdEngine` alone validates public contracts, exact runtime availability, historical identity lineage, and catalogue publication. The exact full publication closure must fit one physical rrflowKV batch before a transaction starts. The former whole-catalogue control keys and function-level direct-store path have no reader or alias. | Reclamation of proven unreferenced artifacts remains an engine-owned maintenance operation; C-04 owns direct stamped catalogue access where later consumers require it. |
| `rrd-engine/src/engine/function/{execution,javascript,webassembly}.rs` | Fresh QuickJS contexts, memory/stack/interrupt limits, synchronous output; Wasmi eager compilation, fuel, stack/store limits, import denial, and ABI/pointer/output checks remain isolated and characterized. | JavaScript policy disables only selected globals and has no supported-target determinism corpus; interrupt counts are not portable fuel. Wasm start functions and default feature choices remain enabled, compilation structure is not explicitly bounded, and error classes depend partly on message text. |
| `rrd-engine/src/engine/function/transaction_binding.rs` plus `engine/transaction.rs` | Original-mutation matching, stable map order, and no recursive rematch remain. First execution seals the validated output/proposal, exact catalogue/definition/artifact/schema/runtime identities, and resource use into the commit intent. Recovery replays that receipt into the same runtime commit without guest execution; the accepted receipt, derived event, semantic audit, indexes, outbox, cursor, and outcome publish in one store transaction. A known outcome is accepted only when every exact prepared receipt is also present. No pre-domain terminal allowed function audit is written. Lost-acknowledgement recovery is proven not to execute the guest again, and substituted runtime-build identity fails closed before domain mutation. | H-05/POAM-016 still owns same-stamp effect-complete authorization and the full security/trace coordinate closure. |
| `rrd-engine/src/engine/tests/function.rs`, `tests/function_conformance.rs`, `rrd-store/src/rrflow_kv.rs`, `rrd-store/tests/semantic_commit_atomicity.rs`, and `fixtures/rrd-function-conformance-v1.json` | Eight focused engine tests cover runtime bounds, standalone replay/reopen, binding acceptance/rejection, prepared recovery, corrupt build substitution, lost-acknowledgement recovery without re-execution, physical catalogue maxima, lineage, and security. Shared corpora prove selected JavaScript and atomic semantic-receipt results equal on rrflowMX and rrflowKV. The physical fault corpus compares the exact complete semantic key closure across prepared, WAL-appended, WAL-synced, and visible-before-acknowledgement failures and reopen. | Wasm profile equivalence across supported targets, complete effect authorization, and outward-surface equivalence remain open under H/J rather than C-03. |
| capability and outward surfaces | Engine-only list/replace/execute methods are described by capability discovery with current H-04/H-06 ownership and without overstating runtime determinism. | No executable HTTP/WebSocket/SDK/CLI/MCP/Connectome function operation exists. |

At this review, all five contract unit tests, two golden/closure tests, eight
engine function tests, the focused rrflowMX/rrflowKV/reopen function corpus,
the complete semantic physical-fault case, and two semantic-commit receipt
tests pass. Along with the full affected package suites, this closes C-03. It
does not close H-04, H-05, I, or J or accept the governed-function capability
as an outward installed feature.

## Exact implementation sequence

1. **C-04:** retain C-03's typed content-addressed state, prepared receipts,
   physical maxima, and fault evidence while exposing only the bounded stamped
   reads required by later engine access paths.
2. **D-01 and I-06:** include default artifacts and runtime manifests in the
   offline distribution; add candidate/preview/apply/retire/uninstall without
   executing functions during install or attunement.
3. **I-01/I-02:** lower proposed events into the sole canonical engine-event
   envelope and implement post-commit triggers separately from transaction
   function bindings.
4. **F and H-04/H-05:** keep vectorized query operators separate; expose the
   shared public operations with complete causal/resource evidence.
5. **J:** remove every superseded spelling/private key/old fixture, run the
   runtime, differential, crash, security, resource, surface, and clean-install
   corpora, and account for every runtime/artifact in the signed distribution.

The roadmap owns this dependency order. Exact paths and commands for the active
package live in its pull request and linked journal; the
[execution portal](../../roadmap/rrflow-1.0-execution-map.md) routes to those
records and the generated file inventory.

## External design constraints

RRFlow implements its own function contract. These primary sources constrain
the design without becoming runtime dependencies or authorities:

- [SurrealDB custom functions](https://surrealdb.com/docs/learn/querying/concepts-and-guides/custom-functions)
  demonstrate typed reusable database functions and invocation permissions;
  RRFlow narrows sandboxed functions so they cannot query or mutate directly.
- [SurrealDB events](https://surrealdb.com/docs/reference/query-language/statements/define/event)
  show why synchronous transactional effects and asynchronous queued work need
  different atomicity, retry, and depth semantics; RRFlow names them as
  transaction bindings versus committed-event triggers/routines.
- [QuickJS embedding API](https://bellard.org/quickjs/quickjs.html#C-API)
  provides memory, stack, and interrupt controls but does not make a selected
  global denylist or callback count a portable deterministic profile.
- [Wasmi configuration](https://docs.rs/wasmi/1.1.0/wasmi/struct.Config.html)
  exposes fuel, feature, start-function, compilation, and enforced structural
  limits that RRFlow must set explicitly rather than inherit.
- [WebAssembly deterministic profile](https://webassembly.github.io/spec/core/appendix/profiles.html#profile-deterministic)
  still permits resource-dependent growth failure and constrains NaN/relaxed
  behavior; fuel alone is not a cross-runtime determinism proof.
- [DataFusion UDF guidance](https://datafusion.apache.org/library-user-guide/functions/adding-udfs.html)
  defines query UDFs as typed, registered, vectorized Arrow operations, while
  [custom table providers](https://datafusion.apache.org/library-user-guide/custom-table-providers.html)
  produce streaming `RecordBatch` values at execution time. Those are rrflowQL
  contracts, not aliases for a JSON sandbox function.

## Acceptance

This capability is accepted only when one manifest-verified installed project
publishes a schema-bound function and transaction binding through
`RrdEngine`; executes the frozen valid/error/resource corpus identically on
rrflowMX and rrflowKV; commits a derived engine event, function receipt,
indexes, outbox, and allowed audit evidence atomically; survives every prepared
and storage crash boundary without re-executing under another runtime build;
closes/reopens on rrflowKV; rejects corrupt, stale, oversized, unauthorized,
and superseded shapes; returns equivalent coordinates through every public
surface; and installs offline with no external runtime, database, hook, source
path, or fetch.
