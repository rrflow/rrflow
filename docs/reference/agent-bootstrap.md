# Provider-neutral agent bootstrap and specialization

**Status:** active reference; bundled installed specialization subset implemented, full attunement and provider qualification remain open
**Coordinate:** `rrflow://rrflow-instance/data/reference/agent-bootstrap`
**Owner:** provider-neutral AI instruction, installation, and specialization boundary

The repository root [`AGENTS.md`](../../AGENTS.md) is the single checked-out
instruction entry point. It tells an AI how to find authority; it does not copy
the product memory into every prompt and it is not a database. The root
[`README.md`](../../README.md) owns product identity, current maturity, and
warp points. Installed RRFlow records will replace Markdown as durable project
memory after the persistence and installation gates pass.

The [adaptive-reasoning decision](../decisions/0002-adaptive-reasoning-governed-effects.md)
governs this boundary. Bootstrap and attunement make durable project claims and
external effects reproducible; they do not require an AI to encode exploratory
reading, context selection, or private reasoning as a workflow.

## Host boundary

| Host | Checkout behavior | Rule |
|---|---|---|
| Codex | Discovers `AGENTS.md` hierarchically. | Consume `AGENTS.md` directly. |
| Claude Code | Loads `CLAUDE.md` and supports file imports. | Root `CLAUDE.md` imports `AGENTS.md` and contains no copied policy. |
| Gemini CLI | Loads `GEMINI.md` hierarchically and supports `@` imports. | Root `GEMINI.md` imports `AGENTS.md` and contains no copied policy. |
| Grok | Current tooling discovers the `AGENTS.md` file family. | Consume `AGENTS.md` directly; do not add a second instruction body. |
| LFG or another host | No behavior is assumed. | An explicit adapter must prove how it supplies the same digest-bound specialization and public RRFlow operations. |

The upstream behavior references are [Codex `AGENTS.md`](https://learn.chatgpt.com/docs/agent-configuration/agents-md),
[Claude Code memory](https://docs.anthropic.com/en/docs/claude-code/memory),
[Gemini CLI context files](https://geminicli.com/docs/cli/gemini-md/), and
[Grok instruction compatibility](https://docs.x.ai/build/features/skills-plugins-marketplaces).
These links justify only bootstrap-file behavior. They do not give a provider
authority over RRFlow lifecycle or state.

## Host hooks

A host hook is a command that an editor's or AI provider's configuration binds
to a host event such as a session start, a prompt, a tool call or a stop.
RRFlow permits a host hook only as an alias stub onto one RRFlow owner, the
same way the root `CLAUDE.md` imports `AGENTS.md`
([ADR-0004](../decisions/0004-host-hooks-as-alias-stubs.md)). The rule covers
the hooks RRFlow installs into a project and any a developer adds by hand.

| Aspect | Rule for a hook stub |
|---|---|
| Content | One invocation of a public `rrflow` command or RRD operation, or one import of an owning record. No copied rule or instruction text, decision logic, state, schedule, retry loop or credential. |
| Owner | An RRFlow-owned host binding names the host, the host event, the target command or operation, the binding revision and the stub's digest. The stub is generated from that binding. |
| Change | Changing the binding regenerates or retires its stubs. Verification compares each installed stub with its binding and reports drift; a drifted stub is not trusted as current. |
| Installation | The install plan previews the stub and its exact host file. RRFlow writes it only after authorization and removes it on uninstall, leaving project-owned host configuration as it was. |
| Effect | The stub reports a host occurrence as a typed engine event or asks RRFlow what applies now. It authorizes nothing: indexing, project mutation and external effects still need explicit authorization through `RrdEngine`, such as the AI asking before attunement starts. |
| Evidence | A hook that ran proves only that the host invoked it. Readiness, progress and completion come from the engine. |

A host whose hook would need logic of its own is supported only after an
RRFlow adapter carries that logic behind a public operation. Gate I's I-04
implements host bindings and I-06 their scaffolding; until then RRFlow installs
no hook.

## Installed specialization

Installation must eventually persist one immutable, versioned specialization
manifest through `RrdEngine`. Its identity and digest bind:

- the primary RRFlow seat definition and allowed provider-representation
  candidates, without embedding provider credentials;
- engram, project, instance, source snapshot, schema, catalogue, and policy
  coordinates;
- discovered languages, frameworks, build systems, data sources, and verified
  capabilities;
- eligible routine, trigger, skill, model, embedding, and adapter revisions;
- explicit budgets, permissions, secrets references, and network constraints;
- the current attunement job and checkpoint chain; and
- evidence showing why each capability is active, inactive, or unsupported.

Providers receive a bounded projection of this manifest through public RRFlow
operations. They never receive raw KV keys, plaintext credentials, unrestricted
workspace content, or authority to mutate the manifest directly.

The current D-01.4 implementation binds the initial subset through installation
plan version 4 and its persisted installed record: bundled template bytes,
selected specialization/seat, configuration and phase digests, exact managed
paths, initial record kinds, secret-generation bounds and inactive capability
policy. The generic template materializes only at `.rrflow/bootstrap.md`.
Project-owned `AGENTS.md`, provider instructions and other source remain
unchanged. Select `--profile clyffy` explicitly to install the bundled Clyffy
specialization; the default remains generic. Candidate endpoint profiles use
the existing revisioned `EndpointProfile` contract and never imply activation;
the shipped profiles declare them absent. Discovered source/capability evidence,
provider activation and ongoing attunement remain later gated work. See the
[installed lifecycle](deployment/installed-lifecycle.md#install-plan) for the
current effect and recovery contract.

### Seat and provider truth

The generic template has no built-in persona or provider. A specialization may
select a durable primary seat and propose provider identities that can represent
it. The bundled `clyffy` profile explicitly selects **Clyffy**; the default
profile remains generic. Clyffy is
a seat, not another product, kernel, repository, daemon, database, model, or
lifecycle. Changing Claude, OpenAI, Gemini, Grok, LFG, or another provider does
not replace that identity.

An installed provider binding records the provider and model identifiers,
adapter and protocol revisions, declared context/tool/streaming capabilities,
input and output bounds, quota/rate-limit behavior, timeout/retry policy, and
observable latency/resource fields. It references a credential capability but
never pools or copies credentials between providers. RRFlow does not equate
provider-specific effort labels, infer hidden reasoning, or present an
unobservable capability as measured fact.

At invocation time, `RrdEngine` must authenticate the exact provider identity,
resolve its current `rrflow-represents` edge and the durable seat at one
`ReadStamp`, then independently authorize the requested operation. A
representation proves attribution only. The complete contract and current gaps
are owned by the [seat-identity reference](seat-identity.md).

## Independent engine and project integration

RRFlow is the product being installed, not glue that becomes usable only when
an application supplies a database, generator, harness, provider, mesh, or
second reasoning runtime. After the signed bundle has been acquired, the
default installation must contain everything required to preview, install,
start, authenticate, persist, close, reopen, recover, and verify the baseline
RRFlow engine with outbound network access denied. Optional integrations can
extend what RRFlow knows or can invoke; they cannot supply rrflowDB's canonical
state, transaction authority, query authority, reasoning lifecycle, or
readiness.

Installation has two explicit modes under one provider-neutral contract:

| Mode | Required scaffolding behavior | Forbidden shortcut |
|---|---|---|
| Fresh project | Resolve a minimal, versioned, bundle-resident project template and specialization profile; preview every RRFlow-owned file and record before apply. | Assuming a language, framework, provider, external database, or generator is present. |
| Existing project | Preserve project-owned content; permit bounded exploratory inspection for planning; before persisting project knowledge or applying integration, commit the relevant deterministic inventory and derive inactive candidates from that evidence. | Executing a detected command, importing credentials, crawling sibling repositories, treating detection as consent, or presenting provisional inspection as snapshot-complete evidence. |

An operator may configure a project-owned or externally supplied code/schema
generator, build system, test or evaluation harness, CI/deployment system,
database, model, mesh, or developer tool. Each integration is a versioned
adapter binding that declares at least:

- capability identity, kind, adapter revision, source evidence, and content or
  executable digest;
- command or endpoint schema, bounded working-directory policy, allowed input
  and output schemas, and environment-variable names without secret values;
- referenced credentials, network and filesystem permissions, resource and
  time budgets, cancellation behavior, and authorization scope;
- health and verification operations, activation status, and the exact
  RRFlow-owned files and records that uninstall may remove; and
- the project snapshot, configuration digest, and plan digest against which
  the binding was previewed and approved.

Discovery creates only an integration candidate. Preview is non-mutating and
must show every planned file, record, adapter binding, and external invocation.
Apply accepts the exact plan digest and requires explicit operator policy and
authorization before invoking an external capability. A generator produces a
proposed project change: its output must be contained, re-inventoried, and pass
the normal authorized mutation and verification flow before later attunement
uses it. A harness returns observations and evidence; it cannot mark an engine
job complete or commit canonical state. Uninstall removes only the binding and
RRFlow-owned scaffolding named by the accepted plan and leaves project-owned
artifacts and rrflowDB state intact unless a separate destructive operation is
explicitly authorized.

The [project command capability contract](automation/project-command-capabilities.md)
owns the exact distinction between discovered command facts, inactive
candidates, installed bindings, prepared external activities, adapter
observations, accepted receipts, and project re-inventory. This bootstrap
record does not duplicate those process-execution semantics.

SurrealDB, Qdrant, Lance, Fjall, and other Rust codebases are engineering
references, not RRFlow module templates. Authorized source adaptation starts
by recording the exact useful behavior, algorithm, failure semantics, and
provenance in the execution map. The implementation is then authored at the
one owning RRFlow boundary using RRFlow identities, semantic contracts,
`RrdEngine` authorization and commit rules, physical formats, resource
budgets, and acceptance tests. A copied crate topology, renamed upstream data
model, compatibility API, or embedded second engine does not satisfy a gate.
Characterization, differential, failure-injection, and restart evidence must
show that the adapted behavior belongs to the cohesive RRFlow engine before a
prior implementation is removed.

All first-party RRFlow and Clyffy execution code must remain Rust source inside
this repository and ship in the self-contained RRFlow distribution. Go is an
eligible outward SDK or explicitly installed generator, harness, or project
capability under the adapter rules above. It is never an alternate Clyffy
kernel, state store, router authority, or orchestration runtime.

## Installation and attunement

The contract defines fresh/existing project targets and a bounded vocabulary of
attunement effects. An explicitly selected plan can contain different known
effects; it is not required to include every effect or a Verify step. Its
completed status means its selected effects were checkpointed, not that the
project has been indexed or independently verified. The bundled profiles plan
Connect and bounded Inventory by default and declare the capabilities they
offer; after Inventory the engine derives the successor plan from committed
inventory evidence and the executors it has, through the replan contract.
Connect and bounded Inventory execute; a capability without an executor is not
selected, and an explicitly proposed one fails explicitly as unavailable. Existing
plans and their receipts retain their original identities.

`initialize_instance` is also the only initial-security bootstrap. Its action
digest binds the project/engram/instance identity, primary seat definition,
allowed provider-representation candidates, storage profile, initial
principals/roles/grants, typed credential-verifier policy, opaque credential
source or generation action, and the configuration/template/specialization
digests. Preview displays those semantics but does not open storage, read or
generate a secret, write a locator, contact a provider, or start RRD. Apply
uses a local privileged fresh-target lease, engine-observed time, and the exact
plan digest to atomically commit the installed binding, primary seat, initial
security authority, action checkpoint, audit, and commit evidence through
`RrdEngine`. A provider representation becomes active only after the exact
provider identity is authenticated and the previewed binding is authorized; no
credential or plaintext provider subject enters the seat graph. An already
installed engram requires ordinary authenticated security administration;
missing or damaged policy never re-enables cold start.

Credential values are not installation-plan data. The baseline generates a
high-entropy machine credential and delivers it only through the previewed
create-new sink. Optional file, Kubernetes, hardware, or secret-manager inputs
are adapter-resolved capabilities with explicit revisions, bounds, and
receipts—not absolute paths interpreted by the engine. Failure after any
non-transactional delivery resumes from persisted prepared/effect state or
fails closed; it never silently generates a second credential. The complete
trust, verifier, path-race, replay, crash, and secret-accounting contract is
owned by the
[security authority](security/authority.md#installation-trust-bootstrap).

Inventory is a committed data boundary, not an informal directory listing or
permission to inspect a file.
The exact record contract, Git/non-Git ignore behavior, symlink and secret
safety, deterministic tree digest, incremental change set, and acceptance
corpus are owned by
[Project-tree inventory and incremental attunement](../architecture/engine-data-flow.md#project-tree-inventory-and-incremental-attunement).
No parser, indexer, model, routine, skill, watcher, or provider adapter may
persist project claims, activate derived state, invoke an external capability,
or claim complete evidence ahead of the snapshot required by that guarantee.
Exploratory readers and models may inspect provisional material and retain its
uncertainty without committing it as project truth. Filesystem notifications
can request another bounded inventory pass only after Gate I exists and they
enter as authenticated engine events; they never mutate project knowledge
directly. Before then, installation and explicit refresh invoke the same
inventory operation directly.

Preview must derive its estimate from
the discovered source count, bytes, parser work, model/index work, configured
resources, and reusable checkpoints. A new or unchanged engram may require
less work; a larger or resource-constrained engram may require more. Apply
reports phase progress and remains safely resumable rather than racing a fixed
wall-clock target.

The engine implementation must preserve these rules:

1. Preview resolves exact actions, inputs, estimates, configuration digest,
   template revision, specialization revision, initial security policy, and
   credential source/sink descriptors without mutation or secret exposure.
2. Apply accepts that exact plan digest; it cannot silently re-plan.
3. Application readiness remains false until the installed binding and initial
   security authority commit; no unauthenticated network bootstrap exists.
4. Every phase records a durable, digest-chained checkpoint through
   `RrdEngine`, including an explicit no-work result.
5. Optional capabilities are selected from verified project signals and engram
   policy. Absence of a language, model, database, or provider is ordinary and
   does not fail the whole attunement.
6. Resume continues the next uncommitted phase. It never infers completion from
   a trace, file, hook, or client status.
7. Verify compares persisted records, indexes, graph state, specialization,
   public operations, and restart behavior against the plan.

## Project-operation preflight

Development and maintenance may begin with a model prompt, a bounded host read,
or another authorized exploratory capability. Those observations are
provisional: they may guide questions and planning but do not become canonical
project knowledge or mutation evidence merely because a model saw them.

When the caller asks RRFlow to persist a project claim, apply or verify a source
change, resume durable work, or return snapshot-complete evidence, `RrdEngine`
resolves or captures the relevant authorized project-tree snapshot, binds its
digest to the operation `ReadStamp`, and retrieves the needed tree, ownership,
symbol, dependency, policy, and evidence neighborhoods. If the required
coverage is absent or stale, that effecting operation returns
`inventory-required` and may propose the bounded inventory capability used
during installation. Unrelated exploration remains available.

Every proposed source change retains the input snapshot and affected entry
digests. After an authorized file activity applies the previewed change, RRFlow
re-inventories and commits the resulting change set before later reasoning
products or index state are persisted or claimed as current. A provider's
open-file list, editor callback, cached tree, or model recollection cannot
satisfy that durable precondition, though each may remain useful provisional
context.

## Generic routine package

A routine is optional durable orchestration: a versioned, resumable graph of
authorized engine operations for repeatable, long-running, or side-effecting
work. It is not a representation of every cognitive step and does not govern
unpersisted model reasoning. A generic package is eligible for every engram but
inactive until attunement proves its effecting inputs and policy allows
activation. Each package declares:

- canonical identity, revision, content digest, input/output schemas, and
  minimum engine capabilities;
- accepted canonical engine events and deterministic activation predicates;
- preconditions, ordered or branching steps, operation permissions, budgets,
  timeouts, retry policy, checkpoints, cancellation, and terminal states;
- required context intent rather than storage fields, index names, or model
  providers;
- verification assertions, evidence fields, rollback or compensation behavior,
  and retirement rules; and
- secret references and outbound-network permissions without secret values.

The canonical event-to-context-to-commit sequence and the distinction between
events, triggers, routines, skills, functions, MCP, and physical access paths
are owned by the
[automation, routine, and skill flow](../architecture/engine-data-flow.md#automation-routine-and-skill-flow).
A routine step names a public semantic operation or required capability. It
never names a Rust function, executable path, MCP server, storage key, graph
edge prefix, index implementation, DataFusion node, or model provider.

The first generic routine set is deliberately small:

| Routine | Purpose | Activation evidence |
|---|---|---|
| `project-inventory` | Produce or refresh the bounded authoritative project-tree snapshot. | Explicit install/refresh intent, or an authenticated filesystem hint accepted under engram policy. |
| `change-impact` | Resolve affected symbols, graph neighbors, tests, policies, and documentation. | A committed change set and required graph/index capabilities exist. |
| `error-resolution` | Correlate diagnostics with code, ownership, prior evidence, and verification commands. | A typed diagnostic event and a supported build/test boundary exist. |
| `verification` | Run the smallest owning checks, then broaden according to risk and changed boundaries. | A proposed mutation or completed routine requires proof. |
| `knowledge-maintenance` | Re-attune only stale records and projections, preserving unaffected identities. | A committed project-tree change set or parser, schema, embedding, or catalogue revision drift is proven. |

These names describe target packages, not current implementation. They become
real only after the routine contracts, persistence, authorization, restart,
and conformance evidence required by roadmap Gate I pass.

## Capability growth

```text
committed engine event
        -> bounded trigger match
        -> attunement/capability rule evaluation
        -> authorized routine proposal
        -> RrdEngine validation and commit
        -> checkpoint, trace, verification, and live delta
```

PostgreSQL, Turso, SQLite, Dragonfly, cloud services, UI clients, model
providers, and future tools enter through independently versioned adapters.
Detection records a possible capability; it never installs, activates, reads,
or writes that system without explicit configuration and authorization.

## Conformance

A provider or adapter is supported only when a real-process corpus proves:

- the same specialization digest and public operation schemas;
- equivalent authorization, budgets, read stamp, result, evidence, and denial;
- no provider-specific persistence, context assembler, routine scheduler, or
  mutation path;
- bounded handling of malformed output, stale state, cancellation, timeout,
  reconnect, and restart; and
- clean uninstall that removes only the adapter and leaves RRFlow state
  readable through every other conforming surface.
