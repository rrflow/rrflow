# D-01 canonical estate layout and configuration evidence journal

**Status:** active immutable change-journal evidence
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-d/d-01-canonical-estate-layout-and-configuration`
**Owner:** package receipt for
`D01-02-canonical-estate-layout-and-configuration-v1`; the roadmap retains
completion authority

This record reports one bounded D-01 prerequisite. It establishes a canonical
project-local estate tree, separates installation identity from deployment
composition, seals operator-configured reasoning, recall, and query ceilings
into installation, and projects that same state through the public contract and
five SDK fixtures. It does not mark D-01, D-02, F-01, H-04, a Gate J
prerequisite, POAM-006, POAM-020, POAM-022, the alpha objective, or a release
complete.

## Bound authority and baseline

- Alpha prerequisite advanced: a clean project-local installation and one
  generated configuration/deployment contract that a future layperson UI can
  discover without becoming another authority.
- Starting revision: `f2f6991627996f1120e7ffa55f19cc740556a6fa`;
  tree: `7cee601125aa9d380ce5b59689b47f9b971c34bd`.
- Planning-only commit: `da430522598d7c26874c5f4ce8b229e21fc692c1`.
  Its sole parent is the starting revision, and its only changed path is
  `docs/roadmap/rrflow-1.0-active-change.json`.
- Branch: `agent/connectome-temporal-runtime-visualizer`; baseline worktree:
  clean; locked dependency digest:
  `316533e7512dbb9029e494386503ca8430b0c69853b3b78ef02319dcbab93a27`.
- Incremental destination remains `development` at
  `https://github.com/rrflow/rrflow-development.git`. `origin` at
  `https://github.com/rrflow/rrflow.git` remains the explicit-promotion-only
  remote. No push or official promotion occurred in this package.

## Change brief

At baseline, the walking product installed into `.rrflow/rrd` but had no single
layout owner. Its project locator included an absolute host path. Installation
and attunement represented the target as a tenant-style resource path even
though a local project has no organization. The public capability response
collapsed process form, storage, and endpoint reachability into one
`deployment_mode` value inferred partly from storage and TLS. Operator-facing
reasoning/recall ceilings did not exist; query hard limits existed at request
level but were not sealed as estate policy. There was no install-time
configuration input, no durable effective configuration record, and SDK/UI
fixtures could not discover those facts.

The target was one canonical contract and one engine-owned lifecycle:

- `InstalledEstateIdentity` identifies project, estate, and instance without a
  fabricated organization or absolute filesystem identity;
- `DeploymentProfile` keeps deployment form, storage profile, and endpoint
  presentation as independent validated axes;
- `EstateConfiguration` carries a format version, monotonic revision,
  domain-separated content digest, and bounded reasoning, recall, and query
  ceilings;
- install planning accepts at most one explicit, regular, non-symbolic TOML
  configuration file of at most 64 KiB, validates it, and seals the complete
  effective value into the plan;
- apply reconstructs the sealed plan and never rereads mutable configuration;
- the active configuration is committed through the same bootstrap physical
  transaction as other installed state, then verified on installed open;
- the locator contains only project-relative managed paths plus typed identity
  and digests; and
- the server binds its endpoint presentation from the listener actually
  opened, then exposes the engine-owned identity/configuration and composed
  deployment through generated OpenAPI and SDK surfaces.

The engine deliberately does not claim a universal deployment form. It owns
its storage profile; an installed engine may carry the planned server profile;
the server composition owns the listener-derived endpoint. TLS is a security
property and does not choose storage or convert loopback into network
presentation.

The canonical project tree for this slice is:

```text
<project>/
└── .rrflow/                       RRFlow-owned estate directory
    ├── config.toml                portable locator, not operator policy text
    ├── credentials/               owner-only credential directory
    │   └── <principal-id>.json    generated local operator credential
    └── rrd/
        └── roots/
            └── <storage-root-id>/
                ├── RRD.TOKEN      engine token-signing key
                ├── immutable/     engine-owned immutable object material
                └── ...            canonical rrflowKV state
```

The four installation-created directories are owner-only on Unix. The plan
enumerates all eight managed paths and their exact removal classes. A
pre-existing `.rrflow` tree, including the legacy `.rrflow/instance.toml` or
old direct `.rrflow/rrd` authority, fails closed rather than being silently
migrated or allowed to coexist.

Unchanged behavior and stop boundaries were enforced:

- product version remains `1.0.0`; no roadmap gate, POA&M item, objective, or
  promotion status was closed;
- `RrdEngine` remains the sole durable-effect authority; the CLI, server,
  generated OpenAPI, SDKs, UI, Connectome, Arrow, and DataFusion do not own a
  second identity, configuration, lifecycle, or status record;
- exploratory/private reasoning remains adaptive under ADR-0002. The new
  reasoning fields bound a future governed durable run, not model thought;
- the absent governed-reasoning runner is advertised as unavailable instead
  of being inferred from configuration;
- Arrow/DataFusion remain in-process bounded compute for the current slice.
  No unmeasured subprocess, external service, or zero-copy claim was added;
- authentication, durability, hard compiled maxima, storage bytes, benchmark
  thresholds, and native-storage promotion status were not weakened;
- no source dependency, install-time fetch, provider activation, Connectome
  implementation, release bundle, repair, uninstall, or legacy migration was
  added; and
- no file was moved, deleted, or replaced under an unproven compatibility
  lane.

## Complete file review and implementation traceability

Fifty-five baseline files were read completely before their dependent edits.
The machine-bound line coverage and SHA-256 for each file remain in the
planning-only active change record. The complete review comprised:

- `README.md`, `AGENTS.md`, the alpha objective, canonical roadmap, Gate D,
  complete POA&M index and POAM-006/-020/-022, instance topology, ADR-0002,
  agent bootstrap, installed lifecycle, deployment modes, public contract,
  Connectome contract, research index, execution portal, change-authoring
  procedure, implementation-navigation procedure, and the prior active-change
  record;
- the complete 6,368-line `rrd-contract` library, 1,239-line attunement
  module, both affected contract test files, and both complete golden fixtures;
- the install profile; complete engine public/module/core/error/installation/
  context/query sources and all three affected engine test modules;
- complete server HTTP module/capability/server sources, the 2,998-line real
  HTTP process test, and the 1,451-line real-server client test;
- the complete 1,512-line CLI command vocabulary, installed adapter, and
  real-process lifecycle test;
- the TypeScript generator and the complete TypeScript, Python, Go, Java, and
  .NET SDK fixture tests; and
- the complete change-plan validator, 1,969-line execution-inventory
  generator, and navigation renderer.

Generated TypeScript output was reviewed as a replaceable projection after its
76-line owner generator had been read in full. It was not treated as an API
authority.

The package-specific replacement map is:

| Baseline behavior | Canonical destination | Characterization/replacement evidence | Owner |
|---|---|---|---|
| Tenant-shaped local target and absolute project root | `InstalledEstateIdentity`, relative `ProjectLocator` | contract fixture, install plan/open and real-process tests | D-01, POAM-020 |
| Scattered implicit `.rrflow` paths | `engine::estate_layout::EstateLayout` | exact eight-path planning and canonical-tree tests | D-01 |
| Scalar `DeploymentMode` inferred from storage/TLS | three-axis `DeploymentProfile`, listener-bound server composition | contract negative test, MX/KV conformance, loopback/network process tests | POAM-022 |
| Request-only limits and no operator configuration | sealed `EstateConfiguration` in contract, plan, durable state, capabilities | digest/maximum tests, custom-config plan/apply/open, query/context denials | D-01, F-01 |
| Hand-consumed capability fixture | contract-generated OpenAPI and SDK projections | OpenAPI golden plus five language suites | D-02, H-04 |

No old implementation was deleted in this package. Legacy manifest/root and
supervisor convergence still require their own traceability package and parity
proof before removal.

## Implementation and evidence paths

Created implementation paths:

- `crates/transport/rrd-contract/src/deployment.rs`;
- `crates/transport/rrd-contract/tests/deployment_configuration_contract.rs`;
- `crates/authority/rrd-engine/assets/install/default-estate-configuration-v1.toml`;
  and
- `crates/authority/rrd-engine/src/engine/estate_layout.rs`.

Changed implementation paths comprise the contract exports, attunement
contract/tests/fixtures, install profile, engine composition/errors/install/
query/context/tests, server capability composition/process tests, real Rust
client test, CLI parser/adapter/process test, and the five language SDK
fixtures.

Created evidence paths are the focused research synthesis and this journal.
Changed evidence paths are instance topology, installed lifecycle, deployment
modes, public contract, Connectome consumer guidance, POAM-022, and the
research index. The TypeScript OpenAPI/endpoints, Gate D journal index, and
global file plan are generated projections. No file was moved or deleted, and
`Cargo.lock` did not change.

## Research, source adaptation, trace, resource, and debugging decisions

Deep research was required because the design crosses portable identity,
configuration integrity, memory execution, observability, installation
security, SDK generation, and later release trust. The new
[code-grounded research synthesis](../../../research/rrflow-canonical-engram-configuration-and-runtime-controls.md)
uses primary specifications and official documentation for RFC 8785, Apache
Arrow, Apache DataFusion, OpenTelemetry, Rust atomic create-new behavior,
OpenAPI, The Update Framework, and Kubernetes configuration/API separation.
It maps each useful behavior to an RRFlow boundary and records rejected
designs; no upstream crate tree, public model, runtime, or compatibility
surface was copied.

The resulting decisions are:

- Arrow/DataFusion guide bounded in-memory execution. They do not own durable
  identity, configuration, authorization, lifecycle, or semantic status.
- The default remains in-process. A dedicated subprocess is a future measured
  isolation option with explicit transport/backpressure/crash semantics, not a
  prerequisite disguised as architecture.
- Operator configuration narrows compiled safety maxima. It cannot raise them,
  and per-request budgets can only narrow the effective ceiling again.
- Attunement may recommend a versioned configuration change but cannot silently
  activate it. A future configure-plan/configure-apply operation must own that
  transition.
- The current digest is a documented, domain-separated typed encoding. This
  package does not mislabel it RFC 8785/JCS.
- Release/update trust remains distinct from project installation; no signing
  or official promotion claim is made here.

Opt-in `rrflow::configuration` debug events now report operation, instance,
configuration revision/digest, and numeric requested-versus-configured query or
recall ceilings when policy rejects work. Opt-in `rrflow::deployment` reports
the three composed deployment axes and bounded security booleans after the
listener and capability contract validate. These events never include prompts,
query text, recall contents, project contents, credentials, tokens, or private
reasoning. Existing typed errors still own the failure response.

Resource evidence is the complete effective configuration in capabilities,
the exact managed-path/action inventory in the sealed plan, and tests proving
that over-ceiling query and recall work returns resource-exhausted before data
reads or mutation. No new performance threshold or benchmark interpretation
was introduced.

Debug failures distinguish invalid/oversized/symbolic configuration input,
configuration-limit denial, legacy-layout collision, installed record or
configuration drift, identity/storage mismatch, and endpoint-presentation
mismatch. No new stable OpenTelemetry signal identifier was declared; the
research record defines how a later observability package can version public
signals without exposing reasoning content.

## First failures and corrections

The first contract oracle failed because all eleven planned canonical
deployment/configuration types and fields were absent. That was the intended
test-first proof of the baseline gap. Subsequent failures exposed these real
boundary mistakes:

- the public-contract suite found the expected OpenAPI digest drift. The
  generated contract was reviewed and the one pinned digest updated to
  `0ef644d8b65019d3fdbb3cf6bf5dd0961473d41d66b0080529801d0008cd9040`;
- the real CLI lifecycle expected bootstrap control sequence 7, but persisting
  the active configuration correctly adds one authoritative transition. The
  evidence was corrected to sequence 8 rather than suppressing the new state;
- the same process test then received HTTP 429 because `QueryBudget::default()`
  exceeded the intentionally lower custom `max_storage_keys=75000`. The test
  now requests an explicit budget within the installed ceiling; neither limit
  nor HTTP behavior was weakened;
- API review found a proposed engine method that reported every engine as
  embedded. It was removed. The engine exposes only its owned storage fact and
  verified installed profile; the outward composition supplies process form
  and endpoint presentation;
- an architecture test rejected the newly created Cargo test target while it
  was untracked. That exact declared target was staged, and the focused tracked-
  target test then passed;
- the final contract reread found the golden install builder still named its
  token leaf `token.key` while the engine-owned layout uses `RRD.TOKEN`. The
  owner-side builder and generated fixture now use the one canonical name, and
  the complete eight-test attunement contract suite passed afterward;
- the same reread found `DeploymentProfile::validate` enforced form/endpoint
  combinations but not the documented ban on volatile rrflowMX clustered
  storage. The canonical validator now rejects
  `clustered_server + rrflow_mx`, and the focused four-test contract target
  passes that negative case;
- the loader implemented bounded regular-file and symlink rejection, but the
  focused engine suite had not directly exercised those input boundaries. A
  new oracle now rejects an unknown-field TOML document, a 64-KiB-plus-one
  input, and a symbolic configuration source;
- strict clippy first exposed a missing import after the API correction and
  then a field-reassign-with-default pattern. Both source issues were corrected
  and strict clippy passed;
- the committed TypeScript generation command was invalid from the repository
  root because the SDK-local `tsx` executable could not resolve. The correct
  command, `pnpm --dir sdks/typescript exec tsx scripts/generate.ts --check`,
  passed. The bad command is retained here rather than rewritten out of the
  planning record;
- the committed .NET command incorrectly passed restore-only `--locked-mode`
  to `dotnet test`, and `global.json` requested SDK 10.0.111 while this runner
  provides 10.0.112. `dotnet restore ... --locked-mode` passed with the
  available SDK. `dotnet test ... --no-restore` then exited zero without
  reporting any executed test and therefore was not accepted as test evidence.
  The repository's executable xUnit-v3 test project was run directly with
  `dotnet run --project ... --no-restore`; it reported 5 passed and 0 failed.
  A separate mistyped `--no-progress` attempt was rejected by xUnit and is not
  counted as evidence;
- after adding the final trace helpers, the first compile failed because
  `QueryBudget` and `AssembleContext` were not in the engine module's shared
  imports. The helper signatures now use their explicit `rrd_contract` paths;
  the full 78-test engine library and server process suite then passed; and
- the final source reread found that installed open compared the locator with
  the active configuration and independently verified the sealed installation
  record, but did not directly bind the locator's configuration revision and
  digest back to that record. The match now closes that transitive integrity
  gap, and the existing sealed-configuration test corrupts the locator and
  requires both installed open and inspection to fail;
- one mistyped attempt named a nonexistent engine integration target while
  redirecting stderr and using `|| true`. It produced no evidence and is not
  counted as a pass. It was immediately replaced by the real engine library
  target, whose unsuppressed result is recorded below; and
- the first five-package candidate run reached the workspace architecture
  checks and rejected two trailing spaces in this journal's header. The
  whitespace was removed, the focused architecture oracle passed, and the
  complete five-package suite was rerun from the beginning and passed.

## Focused acceptance evidence

| Command | Exact result |
|---|---|
| `cargo test -p rrd-contract --test deployment_configuration_contract --locked` | passed, 4 tests |
| `cargo test -p rrd-contract --test attunement_contract --locked` | passed, 8 tests |
| `cargo test -p rrd-contract --test public_contract --locked` | passed, 33 tests after reviewed OpenAPI digest update |
| `cargo test -p rrd-engine installation --locked` | passed, 9 installation tests including invalid, oversized, and symbolic configuration sources |
| `cargo test -p rrd-engine deployment_conformance --locked` | passed, 3 tests |
| `cargo test -p rrd-engine context --locked` | passed, 5 tests |
| `cargo test -p rrd-server --test http_process --locked` | passed, 18 tests |
| `cargo test -p rrd-client --test real_server --locked` | passed, 3 tests |
| `cargo test -p rrflow-cli --test installed_lifecycle --locked -- --nocapture` | passed, 1 real-process test |
| `cargo test -p rrd-engine --lib --locked` | passed, 78 tests |
| `cargo clippy -p rrd-contract -p rrd-engine -p rrd-server -p rrd-client -p rrflow-cli --all-targets --locked -- -D warnings` | passed after final trace correction |

The real-process lifecycle installs with non-default values—reasoning
720,000 ms/192 steps/45,000 ms per step; recall depth 3, 96 items, 393,216
output bytes, and 75,000 keys; and narrower query ceilings. It proves that the
same values and digest survive plan, apply, installed open, server discovery,
authenticated readiness, and SDK query. It also proves the eight managed
paths, relative locator, resource-exhausted enforcement, and credential
non-disclosure.

## Generated, SDK, and widened acceptance

The TypeScript package's full `pnpm check` passed deterministic generation,
Biome checks over 15 files, type checking, and 5 tests. The corrected
SDK-local generation check passed independently. Python passed 42 tests, and
Go passed its package suite. Maven completed successfully; its reports record
3 changed-client tests passed, 1 signal-catalogue test passed, and the existing
SDK-conformance test skipped. .NET locked restore passed, and the executable
xUnit-v3 project explicitly reported 5 passed and 0 failed. All five language
fixtures consume the same structured deployment/configuration projection and
have no `deployment_mode` fallback.

The five affected Rust packages passed together with all targets:

```text
cargo test -p rrd-contract -p rrd-engine -p rrd-server \
  -p rrd-client -p rrflow-cli --all-targets --locked
```

The full workspace widening also passed:

```text
cargo test --workspace --all-targets --locked
```

The pre-existing `local_estate_driver` group took 229.10 seconds in the
five-package run and 229.41 seconds in the final workspace run. Those four tests
exercise legacy supervisor/process-marker/effect-gap/bounded-kill behavior.
They are inventory for later convergence, not product-performance or
walking-product evidence. The canonical installed lifecycle in those same runs
passed in 2.38 and 2.47 seconds respectively. No failure appeared in either
widened run.

Documentation validation passed with 266 status records and 264 coordinates;
navigation tests passed 6 tests; export tests passed 11 tests; and the version
policy check passed. Navigation, inventory, documentation, change-plan,
format, diff, and the focused workspace text-hygiene oracle were rerun against
the finalized journal and generated projections. Strict lint and the full
workspace suite passed against the same implementation before the final
evidence-only corrections.

## Checks not run

- Storage performance benchmarks were not rerun because this package changes
  no storage write path, durability policy, format, or promotion threshold.
  The strict native-versus-Fjall promotion failure remains open.
- Dedicated fault-injection, hostile concurrent pathname replacement,
  cross-platform ACL, bundle/signature, repair/uninstall, clustered deployment,
  and subprocess benchmarks were not run because their implementations remain
  outside this package.
- No Connectome repository command was run. This repository supplies its
  generated consumer handoff; the separately authorized Connectome session
  must update that product without duplicating RRFlow state.
- No release assembly, artifact publication, development push, official-origin
  push, tag, or version change was performed.

## Remaining known errors and owners

D-01 remains open. A true prerelease still needs interrupted-install cleanup
or resume semantics; race-resistant native pathname operations rather than
preflight-only symlink checks; platform ACL qualification; release-contained
install/start/verify assets; uninstall/repair/restore; and direct convergence
of the legacy manifest, old raw root, create-or-open paths, and supervisor.

The final repository-wide scalar-vocabulary audit found that the active
supporting server reference still described TLS-derived `local_daemon` versus
`remote` as current behavior. That path was absent from this package's
committed 47-path plan, so it was not silently absorbed after implementation.
It requires an immediately following planning-only documentation-alignment
package; historical records that describe the old baseline remain unchanged.

Configuration is intentionally install-time-only in this slice. A governed,
digest-gated configure-plan/configure-apply transition, revision history,
rollback, attunement recommendations, and explicit activation remain to be
built. Attunement has a typed plan but no complete discovery/executor loop.

Reasoning limits are truthful public configuration, but the
`governed-reasoning-runner` capability remains unavailable. This package does
not implement or pretend to implement the adaptive orchestration engine,
automatic improvement loop, model/provider adapters, or their evaluation
receipts. Recall/query ceilings govern the installed public HTTP query,
live-query, index, and context-assembly paths. The legacy direct operator CLI
query path still accepts its own explicit execution budget and requires a
separate convergence package before it can claim installed-estate policy.

The installed identity is deterministic for the sealed project inventory,
profile, executable, and configuration. Clone and intentional multi-instance
uniqueness must be resolved explicitly before fleet/cluster rollout; no random
or host-path identity was inserted here to conceal that topology decision.

POAM-020 still owns complete instance/topology convergence. POAM-022 has
material remediation evidence but remains open for its full conformance matrix.
POAM-006 still owns the turnkey installation and attunement deficiency.
Connectome remains only a generated contract consumer. Full DataFusion stream,
provider, graph, index, clustered, and operational gates remain with their
roadmap owners.

## Full-file reread, checklist, and status

After implementation, every changed hand-authored file and the complete diff
were reread; generated fixtures and TypeScript projections were checked against
their owners. The final path set was reconciled against the planning-only
record, generated navigation and inventory were refreshed, and unrelated
worktree changes were neither absorbed nor discarded.

Change checklist:

- one D-01 prerequisite and POAM-022 remediation slice is bound to one
  planning-only baseline;
- current and target behavior, exact scope, unchanged behavior, stop
  conditions, complete reads, traceability, research, diagnostics, first
  failures, commands, non-runs, and remaining gaps are recorded here;
- all implementation and evidence paths are declared; no file move or deletion
  occurred; generated output remains replaceable;
- smallest oracles preceded package and workspace widening; failures were
  corrected at their owning boundary rather than hidden; and
- `RrdEngine` remains the sole durable lifecycle authority and no provider,
  UI, SDK, Arrow/DataFusion component, or documentation record became a second
  owner.

Roadmap/POA&M status change: **none**. POAM-022 records bounded remediation
evidence while remaining open. The result revision is the implementation and
evidence commit containing this journal, whose sole parent is planning commit
`da430522598d7c26874c5f4ce8b229e21fc692c1`; its exact hash is reported in the
handoff. No push occurred.
