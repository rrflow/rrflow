# RRFlow canonical estate configuration and runtime controls research

**Status:** active primary-source research and code-grounded implementation guidance
**Coordinate:** `rrflow://rrflow-instance/data/research/rrflow-canonical-estate-configuration-and-runtime-controls`
**Owner:** research basis for canonical estate layout, configuration activation, bounded compute, discovery, and rollout; not lifecycle or roadmap authority
**Reviewed:** 2026-09-14

This record answers one implementation question: how should RRFlow establish a
clean project-local estate, accept useful operator configuration, expose that
configuration to clients, and preserve adaptive AI reasoning without creating
a second authority or a schema of thought?

The answer is a narrow control boundary. RRFlow keeps exploratory reading,
search, hypothesis formation, and private reasoning fluid. It governs the
moment a request spends bounded shared resources, persists canonical state,
mutates the project, invokes an external effect, or claims reproducible
evidence. One installed estate configuration supplies operator ceilings at
that boundary. It does not prescribe how a model thinks.

This research informed package
`D01-02-canonical-estate-layout-and-configuration-v1`. The roadmap remains the
completion authority, and the package does not close D-01, reasoning,
attunement, Connectome, or release qualification.

## Code-grounded starting point

Complete source review found four coupled problems:

1. `.rrflow/config.toml`, `.rrflow/instance.toml`, direct `.rrflow/rrd`, and
   raw-root server paths represented overlapping installation concepts.
2. `DeploymentMode` mixed storage (`rrflow_mx`), process form (`embedded`,
   `local_daemon`, `distributed`), transport location (`remote`), and an
   offline artifact (`edge`) in one value. The engine inferred it from storage
   and the server inferred it from TLS.
3. installation sealed an opaque configuration digest but did not carry a
   typed, inspectable effective configuration that query, recall, SDK, or UI
   could use.
4. the code had hard safety bounds and request budgets, but no single durable
   operator layer between those two levels. Reasoning-time configuration was
   absent, and advertising it as implemented would have been false because the
   governed reasoning executor is still open work.

The useful existing boundaries were retained: `rrd-contract` owns public
types, `RrdEngine` owns durable effects, rrflowKV owns durable physical state,
rrflowMX remains volatile, Arrow/DataFusion remain compute, server/CLI are
adapters, OpenAPI is generated, and clients consume discovery.

## Primary-source findings

### Configuration and identity must not be accidental runtime inference

Kubernetes distinguishes declarative configuration from imperative commands
and versions its API objects. The relevant lesson is not to copy Kubernetes'
schema or controller model; it is that desired configuration must be explicit,
versioned, validated, and observable rather than reconstructed from incidental
runtime facts. See the Kubernetes documentation for
[configuration](https://kubernetes.io/docs/concepts/configuration/),
[declarative object management](https://kubernetes.io/docs/concepts/overview/kubectl/),
and [API versioning](https://kubernetes.io/docs/concepts/overview/kubernetes-api/).

RRFlow therefore treats storage profile, process form, endpoint presentation,
and transport security as separate facts. A TLS listener does not make the
database “remote”; a persistent root does not prove whether the engine is
embedded or served; and a UI cannot infer durability from its own location.

### A configuration digest needs one precisely named encoding

[RFC 8785](https://www.rfc-editor.org/rfc/rfc8785.html) defines a JSON
canonicalization scheme for interoperable cryptographic hashing. RRFlow's
current configuration digest is deliberately narrower: it hashes a
domain-separated serialization of the typed Rust fields in a fixed tuple
order. It must be called the RRFlow estate-configuration v2 digest, not “JCS”
or generic canonical JSON. If independent implementations must calculate the
digest later, RRFlow should either publish byte vectors for this encoding or
adopt an explicitly versioned interoperable canonicalization in a separately
reviewed protocol change.

The important current rule is that the engine assigns `revision` and
`configuration_sha256`; operator TOML cannot forge either. The full effective
configuration—not only its digest—is sealed in the install plan and committed
to rrflowDB.

### Wall time is an observed input, not absolute authority

Rust documents that [`SystemTime`](https://doc.rust-lang.org/stable/std/time/struct.SystemTime.html)
can move backward and that comparison is fallible. Its
[`Instant`](https://doc.rust-lang.org/stable/std/time/struct.Instant.html) is
appropriate for process-local elapsed work, but it cannot be serialized as a
civil-time or recovery coordinate. NTP deployment guidance in
[RFC 8633](https://www.rfc-editor.org/rfc/rfc8633.html) treats source selection,
large corrections, monitoring, and failure handling as operational policy;
[RFC 8915](https://www.rfc-editor.org/rfc/rfc8915.html) adds authenticated time
exchange rather than making an arbitrary network response trustworthy.

RRFlow therefore separates four meanings:

- monotonic elapsed time measures one live operation or wait;
- observed wall time carries a named source and trust classification;
- semantic valid/effective time belongs to the knowledge or project fact; and
- durable read, transaction, journal, and sequence coordinates order canonical
  state independently of the host clock.

Models and adapters must not become the authority for installation,
credential-validity, audit, or receipt time. This package implements that rule
for installation; runtime-wide timestamp ownership remains open. `RrdEngine`
must attach a wall observation when a governed durable boundary needs it. That
rule constrains effects and evidence, not hypotheses, private reasoning, or
unfamiliar semantic content. Retrieval may explicitly ask for valid-time/as-of
semantics, but it cannot silently substitute ingestion time, host time, or
model prose for those coordinates.

The implemented clock substrate deliberately makes the limited claim its
evidence supports. A fallible host `SystemTime` observation is encoded with
format, `host_system_time` source, `unverified` trust, and Unix milliseconds.
The first accepted observation is sealed into the owner-only install intent
and then the installed record. Recovery, replay, and open reject an unavailable
observation or rollback beyond the configured bound. Inspection remains
read-only and reports the exact anchor, observation/failure, rollback, and
policy. This detects relative rollback from the sealed anchor; it cannot detect
a consistently wrong forward clock, authenticate UTC, or prove synchronization.

Clock diagnosis and correction remain different capabilities. A future
diagnostic can report platform synchronization state and authenticated-source
evidence. Any correction is an explicit privileged external effect with a
previewed source, network and command authorization, acceptable step/slew,
rollback impact, receipt, and post-correction verification. Installation never
silently changes the OS clock. Signed, expiring distribution metadata is also
a separate rollback/freeze defense; it does not repair host time.

### Create-new is necessary but not the whole filesystem safety story

Rust's [`OpenOptions::create_new`](https://doc.rust-lang.org/std/fs/struct.OpenOptions.html#method.create_new)
provides atomic create-if-absent behavior for the final file. It prevents an
ordinary check-then-create race at that leaf. It does not, by itself, pin every
ancestor directory against hostile replacement. RRFlow should use create-new
for storage, keys, credentials, locator publication, and leases while keeping
ancestor containment, symlink rejection, same-filesystem publication, native
ACL qualification, crash cleanup, and handle-relative/openat-style hardening
as explicit acceptance work.

This distinction matters operationally. A passing local test proves the
declared no-overwrite path; it does not prove adversarial filesystem safety on
Linux, macOS, and Windows.

### Arrow and DataFusion are compute boundaries, not estate authority

The Arrow specification describes a language-independent columnar memory
format and separates the physical layout from algorithms and lifecycle. Its
[format introduction](https://arrow.apache.org/docs/format/Intro.html),
[columnar format](https://arrow.apache.org/docs/format/Columnar.html), and
[FAQ](https://arrow.apache.org/faq/) support efficient sharing and
interoperability, but do not make every process transition zero-copy. The
[Arrow security guidance](https://arrow.apache.org/docs/format/Security.html)
also makes validation of untrusted metadata and bounded allocation an
application responsibility.

DataFusion similarly separates shared runtime resources from session and task
state. Its [execution API](https://docs.rs/datafusion/latest/datafusion/execution/index.html),
[crate documentation](https://docs.rs/datafusion/latest/datafusion/), and
[configuration reference](https://datafusion.apache.org/user-guide/configs.html)
provide controls for memory, spilling, batch sizes, and execution behavior.
RRFlow should translate validated estate/query configuration into these
scoped controls. DataFusion must not read `.rrflow/config.toml`, select the
estate, write rrflowKV, activate adapters, or become a reasoning scheduler.

The default boundary remains in-process:

```text
authenticated request + installed configuration + read stamp
  -> RrdEngine chooses native or analytical access
    -> bounded Arrow batches
      -> DataFusion SessionState / TaskContext
        -> bounded result, cancellation, counters, and evidence
  -> RrdEngine returns or commits the authoritative outcome
```

A dedicated subprocess is a later isolation option, not a default speed
optimization. It adds serialization, IPC, cancellation, supervision, and
crash-reconciliation costs. It should be selected only when measured workload
evidence shows that fault or resource isolation outweighs those costs, and it
must preserve the same public operation, stamp, budgets, and receipts.

### Discovery should be generated and descriptive

The [OpenAPI Specification](https://spec.openapis.org/oas/latest.html)
provides a language-neutral description of HTTP APIs. RRFlow uses this as a
projection: executable Rust types and the operation catalogue generate
OpenAPI, and generated SDK surfaces consume it. OpenAPI cannot own engine
semantics or lifecycle status.

For Connectome, `/v1/capabilities` is the runtime truth for the connected
instance. It now carries independent deployment coordinates and the effective
estate configuration. A client must disable or label a feature according to
the advertised capability state; the existence of a generated type is not
availability.

### Tracing observes governed work; it does not capture private reasoning

OpenTelemetry separates traces, metrics, logs, and baggage as signals and
defines spans through its [trace API](https://opentelemetry.io/docs/specs/otel/trace/api/).
Its [signals overview](https://opentelemetry.io/docs/concepts/signals/) and
[general trace conventions](https://opentelemetry.io/docs/specs/semconv/general/trace/)
support correlated operation identities and bounded attributes. Schema URLs
provide a route for evolving telemetry conventions; see
[OpenTelemetry schemas](https://opentelemetry.io/docs/specs/otel/schemas/).

RRFlow should trace public operation, installed identity, configuration
revision/digest, read/commit coordinates, selected access path, resource
counters, cancellation, and result classification. It must not export prompts,
credentials, arbitrary project content, model chain-of-thought, or unbounded
query text. A debug build may add phase timings and structured failure
coordinates under explicit activation while retaining identical semantics.

### Release trust and project installation are separate layers

The Update Framework specifies signed repository metadata and separated roles
for securely distributing artifacts. See the
[TUF specification](https://theupdateframework.github.io/specification/latest/).
That supports RRFlow's future archive/manifest/signature acquisition layer.
It should not be repurposed as project lifecycle state. After a bundle is
acquired, `rrflow install` must work offline from bundle-resident assets and
produce its own digest-bound project plan and engine receipts.

## Canonical estate boundary

The implemented project-local shape is:

```text
<project>/
└── .rrflow/                         RRFlow-owned container, mode 0700 on Unix
    ├── config.toml                  minimal non-secret active locator
    ├── credentials/                 credential references/documents, owner-only
    │   └── local-operator.json
    └── rrd/
        └── roots/
            └── <storage-root-id>/   active create-new rrflowKV root
                └── RRD.TOKEN        owner-only engine token key
```

The install plan names all four directories and all four leaf/tree paths in
that order, with an explicit removal rule for each. The locator contains only:
format/product identity; project/estate/instance IDs; project-relative storage,
token, and credential locations; plan/profile/executable/installed-record
digests; and active configuration revision/digest. It carries no absolute
project path, plaintext secret, mutable canonical state, provider hook, or UI
preference.

Canonical mutable configuration is stored at the engine control key
`server/state/<instance>/configuration/active`. The installed record, locator,
and active configuration must agree before the engine opens. A future
configure operation may advance this record through a plan/apply transaction;
editing the locator or startup TOML is not reconfiguration.

Pre-canonical `.rrflow/instance.toml` or any other pre-existing `.rrflow` tree is
rejected for a new install. This package intentionally does not guess whether
those user-owned bytes may be deleted or migrated. R1 supplied its own full
traceability and interruption proof before deleting the product reader; the
collision remains preserved and fail-closed rather than becoming migration.

## Configuration model and precedence

The effective v2 input is strict TOML:

```toml
format_version = 2

[clock]
maximum_rollback_ms = 0

[reasoning]
max_run_elapsed_ms = 900000
max_steps = 256
max_step_elapsed_ms = 60000

[recall]
max_graph_depth = 4
max_items = 128
max_output_bytes = 524288
max_storage_keys = 100000

[query]
max_storage_keys = 100000
max_rows = 10000
max_output_bytes = 524288
max_batch_rows = 256
max_memory_bytes = 67108864
max_spill_bytes = 268435456
max_elapsed_ms = 30000
```

Unknown fields, missing fields, zero values where work must be positive,
values above compiled maxima, a reasoning step ceiling above its run ceiling,
non-files, symlinks, invalid UTF-8, and inputs over 64 KiB fail during planning.
Clock rollback tolerance is explicit, defaults to zero, and cannot exceed the
compiled one-hour safety maximum. It is tolerance for relative rollback, not a
claim that the host clock is correct.
The precedence is:

```text
compiled hard safety maximum
  >= installed durable estate ceiling
       >= individual request budget
```

An attunement result may later recommend a new ceiling with evidence. It never
silently edits or activates the durable configuration. A separate
`configure plan` / `configure apply --expect <digest>` boundary must compare
the active revision, preview the exact new effective value and operational
impact, authorize it, commit it, and emit a receipt.

The reasoning fields are future governed-run limits. They do not constrain
private exploratory reasoning, and their presence is not evidence that a
reasoning runner exists. Capability discovery therefore advertises
`governed-reasoning-runner` as unavailable while exposing the configured
ceilings. Installed public HTTP query, live-query, index, and context-assembly
paths enforce their applicable ceilings before data access; hard maxima still
cannot be raised by configuration. The older direct operator CLI query path
still owns a separate explicit budget and is R1 convergence work.

## Rollout sequence

The real-world rollout is a sequence of independently testable packages. Each
package must preserve the one contract and delete a conflicting predecessor
only after replacement evidence passes.

### R0 — canonical contract and local estate slice (implemented here)

- freeze `InstalledEstateIdentity`, the three deployment axes, and
  `EstateConfiguration` in `rrd-contract`;
- generate OpenAPI/SDK discovery from that owner and reject
  `deployment_mode`;
- make install planning accept bundled default or explicit bounded TOML;
- seal effective configuration into the byte-stable, no-write plan;
- create the declared `.rrflow` shape and publish the locator after durable
  engine readback;
- reopen using the locator and engine-owned active configuration;
- enforce applicable ceilings on the installed public HTTP query,
  live-query, index, and context paths and expose all ceilings through
  capabilities; and
- prove the exact primary-binary lifecycle with a custom configuration.

This is a usable development slice, not D-01 completion. Its successor R1
supplies bounded install retry and direct pre-canonical startup removal;
adversarial path replacement, native ACL qualification, and release artifacts
remain open.

### R1 — direct pre-canonical convergence and interrupted-install recovery (implemented)

- characterize every `InstanceManifest`, `ensure_dedicated`, raw-root server,
  create-or-open, supervisor, and standalone initializer caller;
- map useful behavior and tests into the installed estate boundary;
- add fault points before/after directory creation, key/credential creation,
  engine commit, locator publication, and acknowledgement;
- resume the same sealed plan, exactly verify owned partial state, and remove
  only its acknowledged intent/pending locator; and
- delete the competing success paths without a compatibility lane.

Implemented result: one owner-only plan-addressed intent freezes the first
engine clock anchor and generated token/credential bytes. Typed tests stop at eight durable
stages and prove exact-plan retry, one credential/token and semantic install
outcome, locator publication, intent cleanup, and fail-closed malformed,
symbolic, foreign, drifted, partial-control, or missing-secret state. Primary
CLI, internal server, embedded MCP, installed SDK conformance, deployment
catalogue, and Kubernetes rendering use canonical install/open; manifest,
private binding, raw product commands, supervisor, security initializer, and
server initializer are absent. This exits R1 locally without closing D-01,
native-platform qualification, external secret delivery, or release proof.

### R1a — engine-owned installation clock substrate (implemented)

- remove wall-clock input from the public installation API and observe it once
  inside `RrdEngine`;
- obtain the versioned source/trust observation before project effects, persist
  it as the first owner-only effect, and preserve it across every interrupted
  retry;
- bind a strict-by-default, bounded rollback policy into configuration v2;
- reject unavailable or out-of-policy time at recovery, replay, and installed
  open while returning typed read-only inspection evidence; and
- run the same bounded abrupt-kill/reopen product journey in Linux, macOS, and
  Windows development CI.

Implemented result: a caller cannot fabricate installation time; exact retry
retains the first anchor; one millisecond of rollback fails under the default;
an explicitly configured bounded rollback is visible; and verification reports
`current`, `rollback_within_tolerance`, `rollback_exceeded`, or `unavailable`.
The host source remains `unverified`. Platform synchronization diagnosis,
authorized correction, runtime-wide temporal high-water rules, and signed
clean-machine distribution qualification remain open.

### R2 — governed reconfiguration and attunement recommendations

- add `configuration inspect`, `configure plan`, and `configure apply` through
  `RrdEngine`;
- version active and retained configuration records with compare-and-swap and
  audit/receipt evidence;
- make attunement persist evidence-backed recommendations only;
- require explicit operator authorization before adapter activation or
  configuration change; and
- prove downgrade/rollback, stale-plan rejection, restart, and client
  catalogue-change behavior.

Exit proof: a recommendation cannot activate itself, and every effective
runtime limit resolves from one committed revision.

### R3 — bounded reasoning and adaptive recall execution

- implement the governed reasoning-run operation around persisted plan/tree,
  step, budget, cancellation, checkpoint, effect, and outcome boundaries;
- retain free-form hypotheses/content inside versioned extensible values while
  typing only durable capabilities and effects;
- route context selection through native graph/BM25/vector/RRF paths and
  bounded Arrow/DataFusion escalation at one read stamp;
- record selection/skipping/resource evidence without chain-of-thought; and
- close capability status only with restart/replay/limit/denial evidence.

Exit proof: lowering a configured ceiling changes admitted work predictably,
raising it above hard safety fails, and adaptive reasoning remains expressive
without unknown durable effects.

### R4 — Connectome and cross-surface product proof

- consume generated OpenAPI/SDK types at the exact digest;
- perform live/ready/capabilities/session bootstrap against the installed
  primary process;
- render the installed identity, three deployment axes, configuration revision
  and effective limits;
- gate actions by operation/capability status, including an honest disabled
  reasoning runner until R3; and
- run the same authenticated engine corpus across CLI, Rust SDK, every
  supported language SDK, MCP, and Connectome.

Exit proof: the UI contains no old scalar, static lifecycle authority, secret,
or mock-only success claim and preserves exact stamps, budgets, receipts, and
failure states.

### R5 — offline distribution and platform qualification

- assemble one manifest-bound archive with the primary executable, default
  configuration, schemas, templates, licences, SBOM, and required runtime
  assets;
- sign and verify distribution metadata independently of project installation;
- test clean Linux, Windows, and macOS machines without compiler, checkout,
  sibling repository, registry, or outbound network;
- qualify install, run, restart, verify, repair, backup/restore, and uninstall;
  and
- preserve the frozen product version until the repository owner authorizes
  exact promotion.

Exit proof: the acquired bundle is sufficient to install and operate RRFlow,
and official promotion remains an explicit Gate J action.

## Debug, trace, and optimization plan

Development should use three build/evidence levels:

| Level | Purpose | Required evidence |
|---|---|---|
| Contract/unit | Fail fast on invalid shape, digest, precedence, limits, path ordering, and old fields. | Exact typed error and no-read/no-write assertion. |
| Debug integration | Exercise real rrflowKV, server, client, configuration, cancellation, and restart with phase spans/counters enabled. | Correlated operation/config/read/commit identities, bounded counters, and captured failure stage. |
| Release-equivalent workload | Optimize only after correctness with symbols/profiles on controlled hardware and representative AI retrieval/reasoning mixtures. | Raw workload/config/build/host digests, distributions rather than averages, and unchanged correctness gates. |

For the known WAL `sync_data` write-tail result, instrument batch size, bytes,
queueing, WAL append, `sync_data`, manifest publication, commit acknowledgement,
and scheduling delay independently. Compare durability policies only as
explicit profiles with crash-loss semantics; do not hide latency by weakening
the authoritative acknowledgement contract. Arrow/DataFusion profiles should
separately record batch rows/bytes, scan/prune counts, memory/spill, task time,
cancellation latency, and result serialization so storage sync is not blamed
for analytical work or vice versa.

Clock tracing records only boundary, source/trust, assessment, anchor,
observation, rollback, tolerance, and failure class. It excludes credentials,
project content, prompts, private reasoning, and arbitrary paths. Test-only
fixed and unavailable sources remain private to the engine so deterministic
failure proof does not become a public caller-injection API.

## Rejected designs

- **One broad configuration schema for “AI behavior”:** rejected because it
  would freeze thought representation and make unfamiliar reasoning
  unrepresentable. RRFlow types capabilities, budgets, durable state, and
  effects instead.
- **Config-file reload on startup:** rejected because mutable disk text would
  bypass plan digest, authorization, audit, and reproducibility.
- **Attunement auto-activation:** rejected because discovery is not authority
  and recommendations can be wrong or environment-dependent.
- **DataFusion-owned memory/configuration database:** rejected because compute
  configuration is not estate identity or transaction authority.
- **Default analytical subprocess:** rejected until measured isolation evidence
  justifies IPC and copy/ownership costs.
- **Preserving `deployment_mode` as an alias:** rejected because successful old
  decoding would perpetuate conflicting semantics.
- **Using TUF/release metadata as installed state:** rejected because artifact
  provenance and project lifecycle are separate trust domains.
- **Closing D-01 from the local startup package:** rejected because typed
  interrupted apply and pre-canonical convergence now pass, but hostile
  filesystem/native platform security, attunement execution, maintenance, and
  release-bundle proofs remain absent.

## Immediate implementation handoff

The next RRFlow package should begin with R2's governed configuration and
attunement boundary, not a second UI-owned schema. A separate Connectome session
can proceed against the generated discovery shape and build installation,
readiness, and configuration screens, but must keep unavailable operations
disabled and retain capability/catalogue change handling. R2 and R3 can add
operations without changing the estate identity or reintroducing client-owned
lifecycle state.

The stable handoff is:

```text
GET /v1/capabilities
  -> installed_estate (required for the D-01-installed product)
  -> deployment { contract_version, deployment_form, storage_profile,
                  endpoint_presentation }
  -> configuration { format_version, revision, configuration_sha256,
                     clock, reasoning, recall, query }
  -> capabilities[] and product_capabilities
```

Connectome should pin the protocol/OpenAPI identity used to generate its
client, validate the runtime response, and render effective limits. It should
not calculate estate identity, infer profile facts, edit `.rrflow`, read the
operator credential in the renderer, or interpret a configured ceiling as an
implemented engine feature.
