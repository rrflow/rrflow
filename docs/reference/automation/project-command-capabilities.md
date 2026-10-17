# RRFlow project command capabilities

**Status:** active target contract; no command-capability implementation exists
**Coordinate:** `rrflow://rrflow-instance/data/reference/automation/project-command-capabilities`
**Owner:** project-command discovery, installation, execution, and evidence semantics

A project command is an optional external activity beneath `RrdEngine`, not a
workflow engine and not proof that RRFlow itself works. Package-manager
scripts, compiler commands, generators, test runners, and evaluation harnesses
become executable only through an installed, digest-bound semantic capability.
Discovery alone grants no permission and performs no execution.

The [agent-bootstrap reference](../agent-bootstrap.md#independent-engine-and-project-integration)
owns generic installation and integration rules. The
[automation flow](../../architecture/engine-data-flow.md#automation-routine-and-skill-flow)
owns event, trigger, routine, skill, function, and activity orchestration. The
[local-process adapter](../deployment/local-process-driver.md) owns host process
safety. This record owns the narrower boundary that connects those systems for
project commands. Roadmap [D-03 and D-06](../../roadmap/rrflow-1.0.md#gate-d--install-configure-and-attune-one-real-estate)
own discovery and applied integration; [Gate I](../../roadmap/rrflow-1.0.md#gate-i--add-explicit-automation-scaffolding-without-automatic-hooks)
owns durable execution and installation scaffolding.

## Locked vocabulary

These objects are distinct. No implementation may combine them in a manifest,
client state, process record, or trace and call the result a routine.

| Object | Meaning | Authority |
|---|---|---|
| Discovered command fact | Bounded source evidence that a project manifest, tool configuration, or executable declaration exists at one committed project-tree snapshot. | Project inventory; inert and untrusted. |
| Capability candidate | A pure attunement proposal mapping one semantic capability to discovered evidence and required permissions. | `RrdEngine` may present it for review; it is inactive. |
| Installed capability binding | An immutable revision that binds a semantic capability to an exact invocation closure, sandbox/effect policy, freshness requirements, and verifier. | Created or retired only by an authorized engine operation after exact preview/apply. |
| Activity plan | One engine-issued attempt binding a public operation or routine run/step, installed capability revision, read stamp, inputs, limits, fence, deadline, and idempotency identity. | Prepared and committed by `RrdEngine` before dispatch. |
| Activity observation | Bounded facts returned by an adapter: process identity, timings, resource use, exit/signal state, redacted diagnostics, and output digests. | Untrusted until validated; never advances a routine. |
| Accepted activity receipt | The validated observation plus authoritative outcome, provenance, and causal coordinates committed with the operation result or routine checkpoint, audit, event, and outbox. | `RrdEngine` only. |
| Routine step | A node that requests a semantic operation or capability and consumes an accepted result. | The generic I-03 routine executor; never the process adapter. |

An engine event can make a trigger eligible; a trigger can propose a routine;
a routine can request an activity. None of those transitions skips
authentication, authorization, policy, or a commit receipt.

## Authority and component placement

```text
committed project-tree snapshot
        -> deterministic command facts
        -> inactive capability candidates
        -> operator preview + exact apply
        -> installed capability binding in rrflowDB

public operation or leased routine step
        -> RrdEngine authenticates and authorizes
        -> binds one ReadStamp, freshness, policy, budget, and fence
        -> commits ActivityPlan before the external effect
        -> injected rrflow-local-process activity port
        -> bounded ActivityObservation
        -> RrdEngine verifies and atomically commits
           receipt + operation result/checkpoint + audit + EngineEvent + outbox
        -> later trigger/context/index work consumes committed state
```

The process adapter receives no storage, query, graph, index, policy,
attunement, routine-state, or event-log handle. It cannot open rrflowMX or
rrflowKV, call DataFusion, select an index, schedule a retry, activate a skill,
or decide that an activity succeeded. HTTP, WebSocket, SDK, CLI, MCP, and
Connectome all invoke the same public engine operation.

## Discovery, preview, apply, and retirement

Project inventory records only bounded facts from the accepted source-tree
snapshot: manifest identity and digest, script or target name, exact script
text digest where applicable, declared tool and configuration references, and
the files that supplied those facts. It does not resolve secrets, search an
ambient `PATH`, install dependencies, contact a registry, run a package-manager
lifecycle script, or inspect a sibling repository.

Attunement may propose a candidate only when a generic template requires the
semantic capability and the committed facts support a binding. For example, a
generic `verification` routine may require
`project.verify.typecheck`; it contains no `pnpm`, `npm`, Cargo, Python, Go,
provider, shell, executable path, or repository-specific command. A TypeScript
estate could propose one package-script binding while a Rust estate proposes a
direct Cargo binding. Both remain inactive until policy permits their exact
closure.

Preview is non-mutating and displays:

- the semantic capability and candidate/source digests;
- the exact invocation kind and every resolved executable, manifest,
  configuration, lockfile, toolchain, wrapper, and lifecycle input;
- filesystem, network, process, environment, credential-reference, and
  platform requirements;
- source/catalogue/policy/projection freshness requirements;
- inputs, declared outputs and write set, budgets, cancellation, retry,
  uncertainty, and verification behavior; and
- every RRFlow-owned record or optional forwarding stub that apply or
  uninstall would create or retire.

Apply accepts only that plan digest. It persists the binding, policy grant,
activation state, and audit evidence through `RrdEngine`; it cannot silently
re-resolve a different tool or script. Retirement prevents new activity plans
while preserving prior receipts and project-owned outputs. Uninstall removes
only the adapter registration and RRFlow-owned scaffolding named by the
applied plan. Project files, dependency caches, external-system state, and
rrflowDB history require separately authorized destructive operations.

## Installed capability binding

The eventual closed public contract must bind every field family below. Exact
Rust and wire names are frozen by A-07 and D-06; this record does not pretend an
unimplemented TOML or JSON shape already exists.

| Field family | Required semantics |
|---|---|
| Identity | Estate, semantic capability ID, immutable revision, activation status, definition digest, and predecessor/retirement coordinates. |
| Source evidence | Project-tree snapshot and entry digests, manifest/script/configuration identities, and discovery implementation revision. |
| Invocation closure | Binding kind; artifact identities and digests; ordered argv or package-script identity; package-manager/runtime/toolchain revisions; lockfile and wrapper digests; and every implicit lifecycle step. |
| Scope and effects | Logical working directory beneath the installed project root; read roots; declared write roots; output identities; process-tree limit; network policy; and effect class. |
| Inputs | Closed typed input schema, allowed parameters, secret-reference names, and explicit environment names. No plaintext secret or ambient environment is stored. |
| Freshness | Required source snapshot, schema, policy, capability catalogue, dependency mount, and projection coordinates plus allowed lag. |
| Resources | Wall and CPU time, memory, child process/file descriptor limits, output/diagnostic bytes, disk reserve, cancellation deadline, and platform enforcement requirements. |
| Replay safety | Activity identity derivation, retry admission, effect uncertainty handling, reconciliation operation, and compensation where supported. |
| Verification | Typed verifier revision and assertions over exit/signal state, diagnostics, output artifacts, project change set, and any follow-up RRFlow read. |
| Provenance | Adapter/platform identity, resolved dependency digests, source and plan coordinates, redaction policy, and retained evidence schema. |

Unknown fields, duplicate IDs, zero/overflowing limits, unverifiable artifacts,
unavailable enforcement, cross-estate references, stale sources, unbounded
outputs, undeclared writes, or a mutable/incomplete invocation closure fail
closed.

## Direct-process and package-script bindings

An ordered argv is necessary but not sufficient. The contract supports two
semantically different binding kinds:

### Direct process

A direct-process binding resolves one authenticated executable object and an
ordered literal argv. No shell performs splitting, globbing, substitution,
redirection, pipelines, or environment expansion. The installed binding uses
an artifact handle or protected absolute identity; it never relies on ambient
`PATH` lookup. The adapter starts with a cleared environment and adds only the
explicit values and protected secret channels authorized by the plan.

`sh -c`, `cmd.exe /C`, PowerShell command strings, batch files, response files
with mutable contents, and interpreter `eval` flags are not direct-process
bindings. A platform whose argument encoding cannot preserve the closed argv
must reject the binding.

### Package script

`npm run`, `pnpm run`, Yarn scripts, and similar launchers can execute manifest
script text, pre/post lifecycle companions, wrappers, and child processes via
a shell. Calling the package manager with a literal argv does not make that
transitive execution shell-free.

A package-script binding therefore captures the package-manager artifact and
version, manifest identity, selected script name and body digest, applicable
pre/post or other implicit lifecycle closure, lockfile/toolchain/wrapper
digests, argument-forwarding policy, and sandbox/effect policy. Caller-added
arguments are denied unless each position and value domain is part of the
closed binding. An unenumerable lifecycle step, mutable remote dependency,
unbounded child process, or manager behavior the adapter cannot verify makes
the candidate unsupported. The first alpha does not execute arbitrary shell
text as a generic escape hatch.

In both kinds, an approved executable may itself have defects or spawn
children. RRFlow controls authority with the complete binding, OS-enforced
sandbox, measured process tree, and post-execution verification; it never
equates “no shell at the first spawn” with safety.

## Sandbox, data, and secret boundary

Every activity runs with least authority and an explicit platform capability
report. Required controls include:

- exact project-root-relative working-directory resolution with parent,
  symlink, junction, reparse-point, and mount escape rejection;
- read-only source access unless a reviewed write set is required;
- default-denied network and device access;
- a cleared environment, explicit locale/timezone if relevant, and no ambient
  credentials or inherited interactive handles;
- protected just-in-time secret delivery by reference, with zero plaintext in
  argv, environment catalogues, canonical records, diagnostics, traces, or
  errors;
- bounded descendants, CPU, memory, files, descriptors, output, and elapsed
  time; and
- cancellation that targets only the authenticated activity process tree.

If the host cannot enforce a required control, the binding is unsupported on
that platform. “Best effort” enforcement cannot be recorded as applied.

Effect classes are explicit. A read-only verifier may be admitted for bounded
retry. A workspace-mutating generator declares exact output roots and requires
a new committed project-tree inventory before dependent work. A command with
external or non-idempotent effects requires an effect-specific reconciliation
or compensation contract; otherwise an ambiguous lost acknowledgement blocks
automatic retry.

## Durable execution and uncertain outcomes

The engine commits an activity plan before dispatch. Its identity is derived
from the estate, routine run and step or public operation, binding revision,
input digest, attempt, and fence. The adapter reports heartbeats only for
bounded liveness, progress, cancellation delivery, and resource accounting;
a heartbeat is not a checkpoint or completion record.

After execution, the adapter returns an observation containing at least:

- activity, plan, binding, adapter, platform, executable, and process-tree
  identities;
- start/finish observations, exit code or signal, timeout/cancellation state,
  and enforced/unsupported limits;
- measured CPU, peak resident memory, child count, bytes read/written, and
  diagnostic bytes retained/dropped;
- redacted bounded stdout/stderr or structured diagnostic excerpts plus their
  complete-byte digests and truncation status; and
- declared output artifact identities/digests and a proposed project-tree
  change boundary, never an assertion that canonical state changed.

`RrdEngine` checks the observation against the prepared plan and verifier. It
then commits one accepted receipt, operation result or routine
result/checkpoint, audit, canonical engine event, outbox entry, and causal trace
links atomically with any allowed semantic effect. If the process may have run
but no valid observation can be proven, the attempt enters an explicit
uncertain/reconciliation condition; it is not silently retried or marked
failed. Reopen resumes from committed plan and receipt state, never from a PID
file, trace, log, hook, client cache, or package-manager status.

## Verification and project-change loop

Exit zero is one observation, not general proof. The installed verifier may
require a closed combination of exact exit state, structured diagnostic
schema, no undeclared writes, expected artifact digests, a bounded project
change set, and a follow-up authorized RRFlow query. A harness can contribute
evidence but cannot complete a routine or roadmap gate by itself.

When an activity can modify the project:

```text
accepted activity receipt
  -> bounded D-03 re-inventory against the prior SourceTreeSnapshot
  -> commit new snapshot + exact change set + checkpoint
  -> determine only affected parse/normalize/index/graph/ground phases
  -> update canonical/index projection deltas through RrdEngine
  -> later context request uses one new ReadStamp
```

The routine and command never select rrflowKV keys, graph prefixes, BM25,
HNSW, TurboQuant, or DataFusion. rrflowQL selects native fast or streamed
Arrow/DataFusion work from semantic intent, catalogue freshness, and budgets.
The resulting evidence can verify the activity or feed later reasoning, but
DataFusion remains compute-only and cannot accept the process result directly
as a durable mutation.

rrflowMX and rrflowKV execute the same installed capability and activity state
semantics through `RrdEngine`. rrflowMX loses run state on process loss and
cannot claim resume. rrflowKV must prove prepared-before-effect recovery,
accepted-receipt replay, idempotency/conflict behavior, and close/reopen before
durable routine support is claimed.

## Relationship to other automation objects

| Object | Relationship to project commands |
|---|---|
| Engine event | A committed fact may make a trigger eligible; process output becomes an event only after engine validation and commit. |
| Trigger | Matches committed events and proposes an authorized operation/routine; it never runs the command. |
| Routine | Requests a semantic capability and owns durable step state; it never embeds a project command. |
| Skill | May explain when/how to request a capability; it cannot grant or execute it. |
| Function | Performs a bounded deterministic in-engine transform or proposed-transaction validation; it is not a host process or durable routine. |
| Host-event adapter | May submit a typed external occurrence after explicit install; it cannot invoke a command in response. |
| MCP adapter | May request or observe the same public operation; it is not the scheduler or command runner. |
| Trace | Records causal evidence; it cannot advance the activity or routine. |

There is no session-start, pre-tool, post-tool, stop, compact, editor,
package-manager, provider, or model-owned hook lifecycle. Package-manager
lifecycle scripts are transitive executable inputs inside an explicitly
approved package-script binding, not RRFlow lifecycle events. Installation
ships no automatic host hook.

## Current implementation disposition

The checkout contains useful adjacent mechanisms, but no project command
capability implementation:

| Current implementation | What is real and retained | What it does not prove / required disposition |
|---|---|---|
| `rrd-contract/src/function.rs` and `rrd-engine/src/engine/automation.rs` | Closed, digest-bound, resource-bounded deterministic JavaScript/WebAssembly functions; authorization; immutable catalogue revisions; transaction-pinned replay; synchronous validation/derived-event behavior. | Not a command binding, activity, routine, or post-commit trigger. Rename `AutomationCatalogue` and `FunctionTrigger*` to their narrow function/transaction-binding terms during A-07/I-01/I-02; extract reusable function execution before decomposing the module. |
| `rrd-core::RuntimeEvent` | A typed runtime event value committed in the runtime log. | Missing the complete canonical engine-event identity/authorization/provenance envelope; I-01 converges it directly rather than adding another event database. |
| `rrd-core::RuntimeTraceEvent` and engine trace helpers | Bounded causal trace events, MX/KV parity for current trace bytes, concurrent append, and incomplete-span visibility after reopen. | Traces are evidence, not activity/routine state. Current direct `StorageEngine` trace writes and `Lifecycle`/`Workflow` vocabulary require H-05/I/J convergence through authorized engine operations. |
| `rrd-estate::LocalProcessDriver` | Useful no-shell argv, path, identity, timeout, cleanup, and effect-gap characterization. | It is an RRD deployment implementation inside the wrong domain, not a generic project-activity adapter. Preserve its safety semantics in `rrflow-local-process`; remove direct storage/private JSON/marker authority as specified by its owner. |
| `rrflow-cli::dev::supervisor` | Development-only process observation and cleanup behavior. | A duplicate supervisor and bootstrap authority scheduled for direct removal; it cannot be generalized into routines. |
| `rrflow-eval::run_trial` | A provider-specific controlled-evaluation harness that directly invokes installed provider CLIs and records trial output. | Evaluation evidence only. It is not an installed project capability, public operation, adapter conformance proof, or runtime implementation. |

No current contract type, persistent binding, candidate discovery module,
engine activity operation, local activity adapter, installation surface, or
capability conformance test exists. Passing function, session-lifecycle,
trace, local-process, or evaluation tests cannot close D-06 or Gate I.

## Planned implementation and evidence

The roadmap owns the required dependency sequence below. Exact files for an
active package belong to its pull request and linked journal;
the [execution portal](../../roadmap/rrflow-1.0-execution-map.md) supplies
generated discovery without becoming another order authority:

1. A-07 freezes capability/activity terminology, pure ports, causal links, and
   the disposition of every current direct process caller.
2. D-03 commits deterministic project-tree facts; D-06 adds pure candidate
   discovery plus closed capability and installed-binding envelopes. Discovery,
   preview, and apply still execute no project command.
3. C-03/C-04 provide the atomic typed state and bounded reads required before
   routine/activity durability is implemented.
4. I-01/I-02 establish canonical committed events and triggers; I-03 adds the
   activity envelope, routine execution, prepared activities, accepted
   receipts, reconciliation, and the `rrflow-local-process` activity port.
5. I-06 adds preview/apply/retire/uninstall scaffolding only after the contracts
   and executor pass. I-07 binds project-development runs to current inventory.
6. H-04/H-05 and J prove cross-surface behavior, traces, crash/reopen, resource
   enforcement, clean offline installation, and absence of superseded paths.

The minimum corpus must cover closed decoding and stable digests; duplicate or
stale candidates; discovery-without-execution; preview-without-writes;
operator denial; direct argv and package-script closure; pre/post lifecycle
capture; PATH/environment/working-directory/symlink/mount escape; Windows
argument behavior; secret and diagnostic redaction; network/filesystem/process
limits; timeout and cancellation; read-only retry; non-idempotent ambiguous
outcomes; prepared/effect/receipt crash gaps; restart/reconciliation;
undeclared project writes; deterministic re-inventory; identical authorized
semantics on rrflowMX and rrflowKV with rrflowKV reopen; and equivalent public
results through HTTP, WebSocket, SDK, CLI, MCP, and Connectome.

## External design constraints

RRFlow implements its own contract; the following primary references constrain
the design without becoming dependencies or authorities:

- [Rust `std::process::Command`](https://doc.rust-lang.org/std/process/struct.Command.html)
  distinguishes literal argv from shell parsing, documents ambient environment
  and working-directory defaults, and exposes platform-specific resolution
  hazards.
- [OCI runtime configuration](https://github.com/opencontainers/runtime-spec/blob/main/config.md#process)
  models argv, environment, working directory, user, privileges, and resource
  limits as separate process fields. RRFlow adopts no OCI hook lifecycle.
- [Temporal Activities](https://docs.temporal.io/activities) and
  [Activity execution](https://docs.temporal.io/activity-execution) motivate
  keeping nondeterministic effects outside deterministic orchestration,
  persisting results, using idempotency, and treating heartbeat/cancellation as
  explicit behavior. RRFlow does not embed Temporal.
- [SLSA build provenance](https://slsa.dev/spec/v1.2/build-provenance) motivates
  complete external parameters, resolved dependency identities, builder/run
  identity, and output digests. A receipt is not called SLSA provenance unless
  the actual SLSA predicate and verification requirements are satisfied.

## Acceptance

This contract is implemented only when an existing-project install discovers a
real command without executing it, previews and applies one exact semantic
binding, persists it through `RrdEngine`, runs it as a prepared bounded
activity, commits a validated receipt and operation result or routine
checkpoint atomically, re-inventories any project output, resumes correctly
after every crash gap,
returns identical authorized semantics on rrflowMX and rrflowKV, reopens on
rrflowKV, and exposes the same result through every public surface. The final
tree must contain no automatic provider hook, client scheduler, command string
escape hatch, ambient-secret inheritance, private process/routine state, or
direct storage/query/index access from the adapter.
