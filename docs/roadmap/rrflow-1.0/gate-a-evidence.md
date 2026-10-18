# Gate A accepted evidence

**Status:** active canonical RRFlow 1.0 roadmap chapter
**Coordinate:** `rrflow://rrflow-instance/data/roadmap/rrflow-1.0/gate-a-evidence`
**Owner:** [RRFlow 1.0 release roadmap](../rrflow-1.0.md)

This chapter is part of the canonical RRFlow 1.0 roadmap. The parent
record owns release order and completion status; this file cannot change
either independently.

## KB-02 evidence (2026-09-05):

- `rrd-contract` now owns a closed, provider-neutral version-one knowledge
  record, provenance, manifest disposition, exclusion ledger, independent
  source inventory, and deterministic package contract. It performs no
  Markdown discovery, import, engine mutation, persistence, or HTTP work.
- Record and package SHA-256 inputs use versioned domain separators and
  explicit unsigned 64-bit length frames. Validation binds normalized relative
  paths, canonical RRFlow coordinates, UTF-8/LF bodies, classification, owner,
  provenance, stable ordering, unique membership, exclusions, and complete
  independently discovered source inventory.
- `knowledge-package-v1.json` freezes the encoded contract. Seven focused
  tests cover golden reopen, closed generated schemas, unsafe coordinates and
  paths, line-ending and body drift, ordering and duplicate rejection,
  independently detectable omission, exclusion binding, provenance changes,
  and record/package digest substitution.
- All 58 `rrd-contract` tests, strict package Clippy, and the
  implementation-free contract architecture test passed. The OpenAPI exporter
  and fixture remain unchanged because KB-02 adds no public operation.

## KB-03 evidence (2026-09-06):

- `scripts/knowledge/export.py` discovers the bootstrap product `README.md`,
  optional `SPEC.md`, and every `docs/**/*.md` source before classification.
  It includes coordinated active/historical records and emits an explicit
  reason-bound exclusion for every other discovered source; no input can
  disappear from both the manifest and exclusion ledger.
- Included records derive their classification from the documentation
  taxonomy and their owner coordinate from the nearest linked index. Missing
  UTF-8, unsafe paths, duplicate coordinates, missing index links, invalid
  metadata, source changes during export, and unignored in-repository output
  fail closed.
- Python digest calculations match the Rust KB-02 golden vectors exactly. At
  the KB-03 acceptance revision, the repository exported twice to independent
  temporary paths with byte-identical output: 79 manifested sources, 26
  included records, 53 explicit exclusions, and one package digest. Later
  records must pass the same reproducibility check instead of rewriting this
  accepted count.
- The real export exposed and repaired a KB-02 validation error: canonical
  nested coordinates such as `/data/reference/storage/<record-id>` are now
  accepted, while missing, empty, non-canonical, query, and fragment segments
  remain rejected.
- Seven focused exporter tests and all seven Rust knowledge-contract tests
  passed. Generated packages remain temporary/ignored artifacts and are not
  checked in as editable documentation authority.

## KB-04 evidence (2026-09-07):

- The documentation policy loads the KB-03 exporter as the single eligibility,
  classification, ownership, and package-construction implementation. It
  exports the repository twice with fixed provenance, compares canonical
  bytes, and independently validates manifest coverage, ordering, unique
  coordinates and paths, body/record/package digests, and the inclusion versus
  exclusion partition.
- Active or historical records under every exporter-recognized documentation
  taxonomy fail when excluded or unclassified. The prior smaller hard-coded
  policy directory list was removed so `guides`, `operations`, and `evidence`
  cannot silently bypass the exporter rules.
- The existing CI authority performs two real exports at the checked-out Git
  revision into independent temporary files, requires byte identity, and runs
  the focused exporter/policy corpus. Generated output remains temporary and
  cannot become editable documentation authority.
- Eleven Python tests cover Rust/Python golden-digest parity, repeated export,
  complete independent inventory, duplicate coordinates, missing owner links,
  unclassified eligible records, changed bodies with stale digests, unstable
  record ordering, missing exclusions, non-UTF-8 input, and unsafe tracked
  output. All passed along with Ruff, documentation policy, CI policy, two real
  80-source exports (27 included and 53 explicitly excluded), the seven Rust
  knowledge-contract tests, strict `rrd-contract` Clippy, version policy,
  formatting, and generated-surface parity.

## KB-05/A-06 evidence (2026-09-08):

- Every queued flat supporting record was read and resolved in its own
  journaled commit. `docs/README.md` is now the only top-level `docs/*.md`
  record; every other current or historical record is nested under one
  taxonomy owner and nearest parent index. No compatibility archive or copied
  authority body was created.
- The final retained CI record now has the stable
  `rrflow://rrflow-instance/data/operations/ci` coordinate and a canonical
  operations index. Source tracing also corrected its unqualified workflow
  security claims: all three workflows are checked for full-SHA actions,
  digest-pinned service images, read-only permission, absent
  `pull_request_target`, and non-persisted checkout credentials. The scheduled
  storage workflow remains explicitly diagnostic rather than engine or release
  evidence.
- Two independent real exports of the final candidate tree were byte-identical
  and contained 91 manifested Markdown sources partitioned into 89 coordinated
  records and two explicit exclusions. The manifest, record coordinates,
  source membership, inclusion/exclusion partition, and package digest were
  independently checked.
- Eleven Python exporter/drift tests, seven Rust knowledge-contract tests, all
  58 `rrd-contract` tests, strict `rrd-contract` Clippy, documentation policy,
  workflow policy plus three negative workflow-security probes, inventory,
  version, generated-surface, workspace-architecture, formatting, and complete
  workspace all-target checks passed. The package journal records exact
  commands and corrections. The resulting commit binds the final tree and
  package digests because an included journal cannot contain its own final
  content digest without changing it; post-commit export verifies that pair.
- A-06 qualifies the deterministic checkout bootstrap package only. It does
  not claim knowledge import, persistence, readback, normal public warp
  resolution, or any rrflowKV/graph/index/Arrow/DataFusion/reasoning capability;
  KB-06 through KB-08 and the runtime gates remain open.

## A-07.0 evidence (2026-09-08):

- The reviewed starting point is commit
  `b5ec39162771ced38a83dd512dda5b0720aef714`, tree
  `3c8d11fe697cd88645179f15441a7485d934f240`. The deterministic execution
  inventory contains 896 current, generated, and planned records.
- Every root/workspace Rust manifest, locked dependency edge, and all 32
  library/binary module roots were read completely, as were all generated-SDK
  package boundaries and public source roots. The execution map now records the
  20-package dependency/public-surface baseline and the five SDK boundaries.
- The package map, capability-family matrix, and mandatory direct-convergence
  table jointly bind current modules and public symbols to characterization
  tests/fixtures/examples/benchmarks, accepted invariants, one canonical
  destination, owning gates, equal-or-stronger proof, and each conflicting
  authority or successful old surface that must later be removed.
- Deterministic inventory and documentation checks, all 11 knowledge-exporter
  tests, locked Cargo metadata, all 16 workspace-architecture tests, generated
  parity for 33 HTTP operations, frozen-version policy, Cargo formatting,
  diff integrity, and the complete workspace all-target check passed.
- This was documentation-only traceability. It did not rename, move, delete,
  or qualify runtime code; it did not prove rrflowMX/rrflowKV equivalence,
  rrflowKV durability, graph/index atomicity, streamed Arrow/DataFusion
  execution, or persisted reasoning/recall. At that revision A-07 remained
  unchecked and A-07.1 was the next package.

## A-07.1a governed-function vocabulary evidence (2026-09-08):

- The public and engine type/field/method family now uses
  `FunctionCatalogue`, `TransactionFunctionBinding`,
  `TransactionMutationKind`, `TransactionFunctionEffect`, `binding_id`,
  and `transaction_bindings` directly. The former source symbols and wire
  fields have no alias or successful decoder.
- The overloaded `engine/automation.rs` authority is absent. Catalogue,
  execution, JavaScript, WebAssembly, and transaction-binding responsibilities
  are isolated under `engine/function/`; engine events, post-commit triggers,
  routines, skills, installation, and external activities were not invented by
  this move.
- A closed `function-contract-v1.json` fixture and two golden/schema tests
  freeze the current canonical shape. A shared engine fixture additionally
  proves the selected JavaScript catalogue read/invocation/transaction result
  equal on rrflowMX and rrflowKV, then closes/reopens rrflowKV and repeats
  catalogue read and invocation.
- All 17 workspace-architecture tests, two function-contract tests, four
  engine function tests, and the one focused cross-profile/reopen test passed.
  The architecture guard finds no former function-authority symbol in
  executable contract/engine source.
- This is structural convergence plus a first characterization corpus, not
  capability qualification. Private monolithic control JSON, direct storage
  access, split allowed audits, unbound runtime replay, inline artifacts,
  incomplete runtime profiles, and absent install/public/cross-language proof
  remain owned by C/D/H/I/J and POAM-008. A-07 and A-07.1 remain unchecked;
  A-07.1b follows as the next bounded subpackage.

## A-07.1b Rust SDK responsibility-boundary evidence (2026-09-08):

- The former 1,235-line `rrd-client/src/lib.rs` implementation body is absent.
  Its existing responsibilities now live directly in `client`, `endpoint`,
  `error`, `operation`, `retry`, `session`, `subscription`, and `transport`;
  `lib.rs` contains only crate policy, module declarations, and public exports.
- Every pre-split public method remains. The intentional pre-release API change
  removes direct access to the bearer-bearing `SessionLease`: `Session` exposes
  only non-secret identity, expiry, and limit metadata, keeps the bearer
  crate-private, and uses a focused, tested redacted `Debug` implementation.
- The package all-target corpus passed, including the redaction unit test and
  three real-server tests covering loopback HTTP, mutual TLS, WSS, and current
  durable subscription ACK/reconnect behavior. Strict package Clippy and the
  workspace architecture suite also passed.
- This is a direct responsibility split and public secret-boundary correction,
  not SDK conformance. The client still implements 28 of 33 HTTP operations;
  complete request/response/status/media validation, semantic retry and
  uncertainty, correlated cancellation, bounded multiplexed frames, W3C
  propagation, endpoint rotation, secret zeroization, installed MX/KV
  conformance, and fail-closed harness behavior remain owned by B/D/H/J and
  POAM-011. A manifest-absent conformance invocation still reports a Cargo
  success without executing a scenario and is explicitly not evidence.
- A-07 and A-07.1 remain unchecked; A-07.1c follows as the next bounded
  subpackage.

## A-07.1c TypeScript SDK responsibility-boundary evidence (2026-09-08):

- The former 394-line `sdks/typescript/src/index.ts` implementation body is
  absent. Existing construction/dispatch, loopback endpoint, error, generated
  operation typing/request coordinates, attempt/deadline, plain session, and
  bounded Fetch/envelope behavior now live directly in their named modules;
  `index.ts` is only the public export root.
- All twelve former root exports, the `RrdClient` constructor, and its five
  public methods remain. The current broad replay rule, partial ArkType
  response validation, serializable credentials, loopback-only transport, and
  other characterized defects were exposed in their direct owners rather than
  silently changed or hidden.
- `tests/sdk_conformance.ts` was directly renamed to
  `tests/sdk-conformance.ts`, and the sole shared orchestrator caller now uses
  that path. Missing manifest input still fails immediately; the shared real
  daemon corpus ran successfully through Rust, TypeScript, Python, Go, Java,
  and .NET and reported the expected corpus digest for each.
- Generation freshness, Biome over 14 files, strict no-emit type checking, and
  all four mock-focused tests passed. Generated parity remains 33 HTTP
  descriptors at the unchanged OpenAPI digest.
- This is structural convergence, not TypeScript SDK qualification. No
  `subscription.ts` exists because there is no current socket behavior to
  move; B-04 owns the real multiplexed protocol. Generated runtime validators,
  opaque credentials, exact status/media/payload enforcement, semantic
  retry/uncertainty, server cancellation, W3C propagation, HTTPS/mesh endpoint
  resolution, browser proof, built ESM/declarations, and installed MX/KV
  conformance remain owned by B/D/H/J and POAM-011.
- A-07 and A-07.1 remain unchecked; A-07.1d is next.

## A-07.1d Python SDK responsibility-boundary evidence (2026-09-08):

- The former catch-all `sdks/python/src/rrd_client/models.py` is absent. The
  synchronous facade remains in `client.py`; current loopback endpoint policy,
  errors, request/resource construction, broad retry/deadline policy, plain
  session shape, and bounded HTTP/envelope decoding now live directly in their
  named modules. `__init__.py` is only the eight-symbol public export root.
- All eight former root exports and all six public `RrdClient` methods remain.
  Existing request bytes, 33-descriptor dispatch, loopback/redirect denial,
  connection reuse, context-manager closure, partial response validation,
  immediate retry behavior, and mutable bearer-bearing session dictionaries
  are characterized rather than silently changed. No async or WebSocket file
  was created.
- Lock and generator freshness, Ruff over 13 Python files, strict mypy over 10
  source files, and all four mock-focused tests passed. A locked offline build
  produced a 9,294-byte wheel and 48,486-byte sdist; both include `py.typed`
  and exclude the removed model module. Direct manifest-absent conformance
  exited 1, while the shared real-daemon corpus passed through all six SDKs at
  the expected corpus digest.
- This is structural and packaging-topology convergence, not Python SDK or
  engine qualification. Runtime request/result models, opaque credentials,
  exact status/media/identity enforcement, semantic retry/uncertainty, native
  async, multiplexed WebSocket/cancellation, W3C propagation, HTTPS/mesh
  resolution, supported interpreter/platform consumers, signed offline
  closure, and installed MX/KV conformance remain owned by B/D/H/J and
  POAM-011.
- A-07 and A-07.1 remain unchecked; A-07.1e is next.

## A-07.1e Go SDK responsibility-boundary evidence (2026-09-08):

- The former 498-line `sdks/go/client.go` responsibility monolith is now a
  114-line public facade. Current construction/configuration, loopback endpoint
  policy, API errors, operation/resource/envelope construction, broad
  retry/deadline behavior, plain credentials/sessions, and bounded HTTP
  response decoding live directly in `config.go`, `endpoint.go`, `errors.go`,
  `operation.go`, `retry.go`, `session.go`, and `transport.go`; `doc.go`
  declares the package boundary.
- The catch-all `models.go` is absent with no alias or forwarding file. All ten
  former exported types and all six public `Client` methods remain, the
  generated 33-operation projection is byte-stable, and future
  `subscription.go`, `models_gen.go`, and qualification tests remain absent
  until their B/H/J behavior exists.
- Go test, race, vet, generator drift, module-tidy, and network-disabled
  local-toolchain checks passed on Go 1.26.0/Linux amd64. Direct
  manifest-absent conformance failed closed, while the shared real-daemon
  corpus passed through all six SDKs at corpus digest
  `b3977c57c8d268f861e9d5158e5609bf3e7d2e1db01f914a12911b95c3404cb2`.
- This is structural source convergence, not Go SDK or engine qualification.
  Concrete operation models, exact status/media/payload/identity validation,
  opaque credentials, semantic retry/uncertainty, correlated server
  cancellation, explicit bounded HTTP/WebSocket carriage, W3C propagation,
  HTTPS/mesh resolution, supported toolchain/platform/consumer artifacts, and
  installed MX/KV conformance remain owned by B/D/H/J and POAM-011.
- A-07 and A-07.1 remain unchecked; A-07.1f is next.

## A-07.1f Java SDK responsibility-boundary evidence (2026-09-08):

- The former 390-line `RrdClient.java` responsibility monolith is now a
  72-line public facade. Current validated construction, loopback endpoint and
  route resolution, bounded HTTP carriage, common operation binding,
  synchronous execution, partial protocol codec, and broad retry/deadline
  behavior live directly in package-private `ClientConfig`,
  `EndpointResolver`, `HttpTransport`, `OperationBinding`,
  `OperationExecutor`, `ProtocolCodec`, and `RetryPolicy` classes;
  `package-info.java` declares the client-only boundary.
- Both public constructors, all five public `RrdClient` methods, and every
  existing public record, enum, and exception signature remain. The generated
  33-operation enum, existing public value classes, tests, generator, and POM
  are byte-stable; future async, WebSocket, generated-model, and qualification
  classes remain absent until their B/H/J behavior exists.
- Maven test passed three current loopback tests and preserved the explicit
  manifest-absent conformance skip under JDK 21.0.12/Maven 3.9.12. Generation,
  compilation, package creation, public/package-private bytecode inspection,
  and the shared six-SDK real-daemon corpus passed at corpus digest
  `b3977c57c8d268f861e9d5158e5609bf3e7d2e1db01f914a12911b95c3404cb2`.
- This is structural source convergence, not Java SDK, artifact, or engine
  qualification. Concrete operation models, exact status/media/payload and
  causal validation, opaque credentials, semantic retry/uncertainty, async and
  correlated server cancellation, bounded closeable HTTP/WebSocket carriage,
  W3C propagation, HTTPS/mesh resolution, reproducible/offline signed Maven
  artifacts, toolchain/platform consumers, and installed MX/KV conformance
  remain owned by B/D/H/J and POAM-011.
- A-07 and A-07.1 remain unchecked; A-07.1g is next.

## A-07.1g .NET SDK responsibility-boundary evidence (2026-09-08):

- SDK `10.0.111` is selected exactly with roll-forward disabled. Common
  target/compiler/analysis/deterministic-compilation/lock-generation policy
  and the xUnit 3.2.2 version now have one workspace-level owner. The package
  README lives at the .NET root, the byte-identical 33-operation projection
  lives only under `Generated`, and the generator writes only that path.
- The former 455-line `RrdClient.cs` responsibility monolith is an 89-line
  public facade over package-internal validated options, endpoint resolution,
  bounded HTTP carriage, operation binding/execution, partial protocol coding,
  and broad retry/deadline seams. `Models.cs` is absent; each existing public
  option, resource, and session record has one responsibility file. No future
  call, subscription, WebSocket, generated-model, or package-builder surface
  was fabricated.
- A temporary isolated reflection audit compared the committed and candidate
  Release assemblies: all 140 exported type/member signatures matched, with
  zero additions or removals. Locked restore, warning-clean build, all three
  current mock-focused tests, generator drift, and formatting passed under
  .NET SDK 10.0.111/runtime 10.0.11 on Linux x64. The fourth locally reported
  xUnit result remains the documented manifest-absent false pass and is not
  counted as engine conformance.
- Release pack observations produced 24,983-byte and 24,982-byte six-entry
  NuGet packages with the relocated root README, preserving a concrete
  byte-reproducibility failure. The shared six-SDK real-daemon corpus passed at
  corpus digest
  `b3977c57c8d268f861e9d5158e5609bf3e7d2e1db01f914a12911b95c3404cb2`.
  These establish package topology plus direct-seeded rrflowKV client
  characterization, not deterministic/offline/signed artifact or installed
  engine evidence.
- Concrete operation models, exact status/media/payload/causal validation,
  opaque credentials, semantic retry/uncertainty, correlated server
  cancellation, bounded production HTTP/WebSocket carriage, W3C propagation,
  authenticated HTTPS/mesh resolution, external consumers, supported
  toolchain/platform matrices, D-01 installation, rrflowMX/rrflowKV parity,
  crash/reopen, native graph/index atomicity, stamped Arrow/DataFusion,
  persistent reasoning/context/feedback, and Connectome remain owned by their
  B through J gates and POAM-011.
- At the A-07.1g revision, A-07 remained unchecked and A-07.1h was the final
  package/type/path closure pass.

## A-07.1h repository-closure and vocabulary evidence (2026-09-08):

- The contextual vocabulary is now explicit: product prose uses `RRFlow`,
  `RRD`, `RrdEngine`, rrflowDB, rrflowKV, rrflowMX, rrflowQL, Arrow, and
  DataFusion; Rust, frozen wire/path values, and generated languages retain
  only their required identifier casing. Third-party names remain confined to
  locked dependencies, outward adapters, comparisons, or provenance records.
- Workspace architecture now rejects Cargo target or path-dependency inputs
  outside the repository, non-member local dependencies, direct or transitive
  Git dependencies, tracked submodules, escaping or unresolved tracked
  symlinks, case-insensitive tracked-path/package collisions, noncanonical
  package descriptions, the retired uppercase query-language spelling in
  active first-party prose, and host-specific
  absolute paths in build or install inputs. Six focused detector/closure
  checks increased that suite from 17 to 23 tests.
- Locked Cargo metadata, both governed-function contract/profile fixtures, all
  23 workspace-architecture tests, the exact retired-symbol search, and the
  complete locked workspace all-target check passed. Focused query parser,
  inference pipeline, public-contract, query-trace, and optional cluster/
  operator-knowledge compilation checks also passed for the files whose public
  diagnostics or examples received canonical spelling.
- Deterministic inventory, documentation, workflow, version, generated-surface,
  knowledge-export, formatting, lint, and diff checks passed as recorded in the
  supporting execution-map journal. The public contract retains the same 33
  operation descriptors; only its human-facing unavailable-surface reason uses
  canonical rrflowQL spelling.
- This proved A-07.1 source/package/type/path closure, not an engine
  capability. At that revision A-07 remained unchecked pending A-07.2; the
  following A-07.2 package now closes it.
  rrflowMX/rrflowKV equivalence and durability, native graph/index atomicity,
  streamed stamped Arrow/DataFusion execution, persisted reasoning/recall,
  installation/attunement, SDK/Connectome conformance, and release proof remain
  owned by their open gates.

## A-07.2 causal-evidence vocabulary (2026-09-08):

- `rrd-core` now owns 13 exact `TraceBoundary` values and 53 closed
  `TraceOperation` names. Operation/boundary mismatch, unknown operations, the
  former `domain` field, extra event/link fields, and the retired provider,
  workflow, and operator-knowledge link shapes fail closed with no alias.
- Fourteen generic `TraceLink` forms carry request, actor/scope,
  authorization, read/snapshot, plan/projection, reasoning, source, commit,
  resource, asynchronous-causation, and output coordinates. Forty-six
  `TraceAttribute` names enforce their boolean, unsigned, finite-decimal,
  digest, or bounded lowercase-token value type before persistence.
- Every reviewed current trace producer now uses the exact boundary field and
  generic source link. Its remaining mixed operation and subsystem-specific
  attribute names are admitted only by private sorted inventories that cannot
  overlap the canonical catalogues and can only shrink in their assigned
  C-through-I behavior gates.
- The engine data-flow owner freezes W3C extraction/invalid-context/child and
  asynchronous propagation rules, durable-to-Rust-`tracing`/OpenTelemetry
  mapping, redaction, nonsampling of authoritative evidence, and the physical
  instrumentation each later gate must add. This package does not implement or
  claim H-05.
- The full `rrd-core` suite passed 58 unit tests plus all integration/golden
  suites; the rrflowMX/rrflowKV trace differential passed; the complete
  `rrd-engine` suite passed; and the focused cluster artifact contract passed
  all 10 tests. Strict default-feature Clippy passed for every touched package,
  and all-feature cluster Clippy passed. The feature-gated OpenRaft transport
  test still fails before its trace assertion because application index 8
  reaches the already-recorded unsupported rrflowKV application format; that
  retained storage/cluster defect is not counted as A-07 evidence.
- A-07 is complete, but no C-through-J capability checkbox changes. RRFlow
  still lacks accepted final rrflowKV pages, atomic native graph/BM25/vector
  paths, bounded stamped Arrow/DataFusion streaming, persisted reasoning and
  recall feedback, qualified installation/attunement, public-surface parity,
  Connectome conformance, and clean deployment proof. B-04 is next after the
  completed B-03 model-admission contract.

## A-07 correction (2026-10-17):

- The `rrd-security` crate header still claimed that the crate owns policy
  truth and named a retired provisioning role. The
  [security authority reference](../../reference/security/authority.md)
  assigns the removal of both to A-07, and A-07's closure left them in place.
- The header now states the reference's component responsibilities:
  `RrdEngine` is the only authority boundary; the crate supplies metadata,
  pure decisions, audit values, and cryptographic helpers; and its public
  storage repository remains implementation inventory recorded by
  [POAM-016](../../poam/rrflow-1.0-alpha/poam-016.md). The reference's
  deviation list and acceptance map no longer use the retired term.
- A-07 stays checked. The direct `StorageEngine` repository and the rest of
  POAM-016 remain open.
- The package record is the
  [A-07.3 security crate header correction journal](../../evidence/change-journals/gate-a/a-07-3-security-crate-header-correction-journal.md).

## A-02 evidence (2026-09-04):

- `crates/kernel/rrd-core/tests/fixtures/reasoning-tree-v1.json` is the shared frozen
  wire vector used by the kernel and public contract.
- The seven focused reasoning-tree tests reject unknown fields and versions,
  malformed edge topology, missing condition evidence, missing verification,
  mismatched edge selection, and changed read stamps.
- `cargo test -p rrd-core -p rrd-contract` passed all 105 tests in the isolated
  A-02 candidate tree, and `cargo clippy -p rrd-core -p rrd-contract
  --all-targets -- -D warnings` passed.
- `public_contract_and_client_stay_implementation_free` proves `rrd-contract`
  retains zero production workspace dependencies; `rrd-core` is used only by
  the cross-boundary conformance test.

## A-03 evidence (2026-09-04):

- The fixed-stage ledger module, engine projection/API, automatic global trace
  lookup, and CLI record/show surface are absent; the pre-A-02 golden entry was
  removed rather than treated as a supported wire format.
- Reusable source/digest/summary evidence and verification status now live in
  the A-02 generic types, while `TraceLink::ReasoningCursor` preserves exact
  tree, revision, node, step, and read-manifest correlation.
- `retired_fixed_reasoning_ledger_api_is_absent` and the compiled CLI rejection
  test prevent the removed symbols and commands from returning.
- `cargo test -p rrd-core -p rrd-engine -p rrflow-cli` passed all 200 tests, and
  `cargo clippy -p rrd-core -p rrd-engine -p rrflow-cli --all-targets -- -D
  warnings` passed.

## A-04 evidence (2026-09-04):

- The 20 current packages live only under `kernel`, `persistence`, `compute`,
  `authority`, `transport`, `adapters`, `operations`, and `evaluation`;
  the workspace architecture test compares every package to its declared
  manifest path and rejects any additional top-level crate group. A-07 must
  still validate and, where necessary, replace those group names.
- The empty graph boundary and the in-repository Connectome package are absent.
  The separate Connectome repository commit `38f68ce7` supplies its native RRD
  capability handshake, authenticated session, bounded diagnostic/context
  calls, renewal, and close; its Rust tests, strict Clippy, TypeScript check,
  Biome check, and aggregate `pnpm check` passed. That evidence established
  repository separation and a partial native-client characterization only. The
  [current Connectome audit](../../reference/client/connectome.md#current-separate-checkout-audit)
  records the remaining parallel contract, lifecycle, inherited-runtime,
  transport, and real-engine conformance gaps; it does not satisfy H-06.
- `rrflow dev` now supervises only RRD and reports a client endpoint, principal,
  and private credential path. A real-process smoke started the daemon, probed
  readiness and capabilities, observed ready status, and stopped cleanly.
- `cargo metadata --locked`, all 16 workspace-architecture tests, `cargo check
  --workspace --all-targets --locked`, `cargo test --workspace --all-targets
  --locked`, and `cargo clippy --workspace --all-targets --locked -- -D
  warnings` passed.
- Version, recursive CI package/feature routing, and generated-surface policy
  checks passed with 20 default-feature packages, five optional-feature
  packages, and 33 contract-derived HTTP operations.

## A-05 evidence (2026-09-04):

- Every supporting semantic, design, implementation, evidence, research, and
  history document declares status in its first 12 lines; the root README owns
  product identity and the knowledge map, while this linked record alone owns
  the detailed RRFlow 1.0 release checklist.
- Superseded runtime, in-repository Connectome, provider-flight, Clyffy alpha,
  and architecture-triage documents are explicitly historical. Active status
  lines cannot use the retired F/G/M/Q milestone scheme.
- Stale alternate-authority claims, the removed UI/graph layout, obsolete CI
  package counts/topology, a hard-coded branch, and five broken local document
  links were corrected.
- `scripts/ci/check_documentation.py` validates the README knowledge map,
  designated roadmap ownership, supporting status, prohibited alternate
  authority/layout claims, and repository-local links in CI. The documentation policy, CI policy, Ruff, `git diff --check`,
  and all 16 workspace-architecture tests passed.
