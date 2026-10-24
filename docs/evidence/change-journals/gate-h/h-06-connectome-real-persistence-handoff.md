# H-06 Connectome real-persistence handoff journal

**Status:** active immutable change-journal evidence
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-h/h-06-connectome-real-persistence-handoff`
**Owner:** D-01/H-04/H-06 prerequisite receipt; not product, roadmap, POA&M, or release authority

This record proves one bounded engine persistence oracle and the source-audited
implementation handoff for the separate Connectome repository. The
[Connectome contract](../../../reference/client/connectome.md) owns the client
boundary, the [installed lifecycle](../../../reference/deployment/installed-lifecycle.md)
owns the operator journey, and [Gate H](../../../roadmap/rrflow-1.0/gate-h.md)
owns H-06 acceptance.

## Package and outcome boundary

- **Package:** `D01-H06-connectome-real-persistence-handoff-v1`.
- **Prerequisite advanced:** make the installed RRFlow process a falsifiable
  Connectome target by proving one committed rrflowDB value survives abrupt
  process loss, restart of the same installation, fresh discovery, fresh
  authentication, and a public-client query.
- **Documentation outcome:** give the separate Connectome repository an exact
  implementation order and source-review map that cannot mistake a mock,
  renamed SurrealDB surface, fixed response, build, open port, or unauthenticated
  health probe for RRFlow connection or persistence.
- **Not proved:** Connectome does not yet perform this journey. Secure installed
  credential handoff, packaged generated clients, complete operation coverage,
  WebSocket delivery, signed bundles, and full H-06 conformance remain open.

## Starting state and planning boundary

- Starting revision: `3fa90fadf6e00d21345523350df1c960f0e10501`.
- Starting tree: `451a105ec3085273831bdb7cf0db9a37ef84893e`.
- Branch: `agent/connectome-temporal-runtime-visualizer`; the starting worktree
  was clean.
- Planning-only commit:
  `f9c0012d6f773c414a137e14f584c3b44dece1dd`; its only changed path is
  `docs/roadmap/rrflow-1.0-active-change.json`.
- The planning validator accepted that commit with zero post-plan paths before
  implementation began.
- Incremental development target: `development` at
  `https://github.com/rrflow/rrflow-development.git`.
- Promotion-only target: `origin` at `https://github.com/rrflow/rrflow.git`.
- `Cargo.lock` began at SHA-256
  `316533e7512dbb9029e494386503ca8430b0c69853b3b78ef02319dcbab93a27`
  and is unchanged.

The separate Connectome checkout was inspected read-only at commit
`38f68ce7adda9d03501f3591165f0e14996899ec`, branch `main`, one commit ahead
of `origin/main`, with no worktree changes. Its origin was
`https://github.com/EonsofStupid/rrflow-connectome.git`. This package does not
edit, import, build from, commit, or push that checkout.

## Change brief and authority

At baseline, the installed-lifecycle test installed and started a real RRFlow
process, authenticated with the public Rust client, committed a claim, queried
it, killed the process, and then performed only an offline structural verify.
It did not prove a different process could reopen the same estate and return
the acknowledged semantic state.

At the audited Connectome baseline, the native Tauri layer contained a useful
but handwritten RRD prototype. No screen invoked its authenticated client. The
runtime screen called three public GETs and labelled their success `RRD
connected` and every stage `verified`; the browser smoke test intercepted those
GETs and supplied fixed version, deployment, backend, and capability values.
The checkout also retained renamed SurrealDB packages, an incompatible old
`rrflow start` launcher, a hard-coded cloud endpoint, static attunement and
automation truth, and separate diagnostics/control-plane protocols.

The target adds no runtime operation or UI feature. It strengthens the existing
test so a second real process must announce the same installed instance,
complete readiness, accept a new authenticated session, and return the same
claim at or beyond the commit sequence. It adds a supporting guide that maps
the audited UI code to retain/remove/replace packages and makes the same
write/kill/restart/read acceptance journey Connectome's first real product
proof.

README, objective, roadmap, POA&M, public operation catalogue, OpenAPI,
generated SDK projections, installed command tree, version `1.0.0`, benchmark
policy, and release state remain unchanged. Work stops before adding a
credential broker, public operation, browser authentication, WebSocket
completion, SDK packaging, Connectome edit, signing, or promotion.

## Complete owner and implementation reads

The active-change record binds the baseline digest, line count, complete line
range, and relevant symbols for every RRFlow file below, each read in full
before implementation:

- `AGENTS.md`, `README.md`, `docs/objectives/rrflow-1.0-alpha.md`,
  `docs/roadmap/rrflow-1.0.md`, `docs/roadmap/rrflow-1.0/gate-d.md`,
  `docs/roadmap/rrflow-1.0/gate-h.md`,
  `docs/poam/rrflow-1.0-alpha.md`,
  `docs/poam/rrflow-1.0-alpha/poam-006.md`, and
  `docs/poam/rrflow-1.0-alpha/poam-011.md`;
- `docs/roadmap/rrflow-1.0-execution-map.md`,
  `docs/roadmap/rrflow-1.0-execution/change-authoring.md`,
  `docs/reference/deployment/installed-lifecycle.md`,
  `docs/reference/protocol/public-contract.md`,
  `docs/reference/client/connectome.md`,
  `docs/reference/client/README.md`,
  `docs/reference/sdk/rust.md`, and `docs/reference/sdk/typescript.md`;
- `crates/adapters/rrflow-cli/tests/installed_lifecycle.rs`, the complete
  633-line test at baseline;
- `scripts/ci/build_execution_inventory.py`, the complete 1,969-line
  generator; `scripts/knowledge/render_navigation.py`; the previous active
  change record; and the generated Gate H index.

The following directly relevant Connectome files were read completely at the
pinned revision:

- `README.md`, `REVIEW.md`, `package.json`, `.github/workflows/connectome-ci.yml`,
  and `scripts/check-retired-identities.mjs`;
- `src-tauri/Cargo.toml`, `src-tauri/src/main.rs`, `src-tauri/src/rrd.rs`,
  `src-tauri/src/rrd/client.rs`, `src-tauri/src/rrd/transport.rs`,
  `src-tauri/src/database/mod.rs`, and `src-tauri/src/database/shell.rs`;
- `src/rrflow/runtime.ts`, `src/rrflow/attunement.ts`,
  `src/rrflow/diagnostics.ts`, `src/rrflow/control-plane.ts`,
  `src/providers/Context/index.tsx`, and `src/vendor/rrflow-client.ts`;
- `src/screens/connectome/pages/RuntimeConnection/index.tsx` and
  `tests/smoke/connectome.spec.ts`.

Additional inherited connection types, defaults, schema, and connection-screen
regions were navigated to confirm field flow and call sites; they are not
represented here as complete reads. The guide therefore assigns a mandatory
full-file reread inside each future Connectome implementation package rather
than pretending this RRFlow package changed them.

## Paths and implemented behavior

### Engine-side persistence oracle

- Changed `crates/adapters/rrflow-cli/tests/installed_lifecycle.rs`.
- Extracted bounded process-start, termination/capture, and query-budget test
  helpers without changing the installed product runtime.
- Retained the first real install/serve/ready/session/transaction/commit/query
  sequence and captured the acknowledged claim sequence and first query result.
- Abruptly terminated the first child, started `serve` again against the same
  project and executable, and required the same installed instance identity.
- Re-ran `ready`, required its authenticated probe session to close, re-read the
  locator, created a fresh public-client session, ran the same bounded rrflowQL
  query, and closed the fresh session.
- Required the post-restart cursor to be at or beyond the committed sequence and
  required exact canonical query, scope, schema revision, read-manifest digest,
  row set, claim identity, and `ui-queryable` value equality.
- Killed the second child before the existing final offline verify and retained
  the test's credential-leak search.

This is a process-boundary characterization test, not a new persistence
implementation. No implementation was deleted, moved, merged, or replaced, so
the implementation traceability procedure did not require a replacement map.

### Connectome source-audited handoff

- Added
  `docs/reference/client/connectome-implementation-guide.md` as supporting
  guidance under the existing Connectome contract.
- Recorded the honest audited state, target renderer/native/RRD/engine/storage
  topology, current source-built install/serve/ready workflow, exact state
  vocabulary, generated-contract boundary, and the missing secure installed
  credential handoff.
- Mapped current Connectome files and symbols to retain/remove/replace actions,
  then ordered C0 truth correction, C1 generated client, C2 secure installed
  handoff, C3 smallest persistent lens, C4 inherited-lens convergence, and C5
  live delivery/H-06 qualification.
- Defined the real-artifact test inputs, random per-run value, kill/restart
  journey, captured evidence, and anti-mock/anti-hard-code review commands.
- Changed `docs/reference/client/connectome.md` and its nearest index only to
  link the supporting guide and distinguish the new engine proof from UI
  qualification. Removed its stale literal OpenAPI digest from implementation
  instructions; Connectome must bind one packaged identity to the live
  advertised identity instead of treating today's generated digest as an
  eternal client constant.
- Changed `scripts/ci/build_execution_inventory.py` to assign the new guide to
  D-01/H-04/H-06/J-03/J-05, then regenerated the exact file inventory and Gate
  H journal index.

No RRFlow or Connectome claim says Fjall is runtime persistence. Fjall remains
only a correctness/performance opponent for RRFlow's native engine.

## Research and adaptation decision

Research was required. The package retained Tauri's narrow native command and
ordered-channel separation, OWASP's rule that long-lived secrets do not belong
in browser storage or serializable renderer state, and OpenAPI 3.1's generated
contract identity. It also pinned the actual Connectome source revision rather
than relying on its README claims.

No upstream code, crate tree, compatibility surface, runtime, schema, or hidden
dependency was copied. Generic JSON tunnelling, renderer credentials, generated
types as proof, mock transport as conformance, and renamed SurrealDB authority
were explicitly rejected.

Primary sources:

- [Tauri v2: calling Rust from the frontend](https://v2.tauri.app/develop/calling-rust/)
- [OWASP HTML5 Security Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/HTML5_Security_Cheat_Sheet.html)
- [OpenAPI Specification 3.1.0](https://spec.openapis.org/oas/v3.1.0.html)
- [Audited Connectome commit](https://github.com/EonsofStupid/rrflow-connectome/commit/38f68ce7adda9d03501f3591165f0e14996899ec)

## Trace, resource, and debugging decision

- **Runtime trace: preserved.** No runtime source changed. Existing request,
  operation, transaction, query, readiness, process, and read-manifest
  coordinates remain the causal surface.
- **Resource evidence: preserved.** The test adds one bounded service/query
  cycle under the existing query limits. It adds no daemon, dependency,
  background matrix, or unbounded workload and makes no performance claim.
- **Debugging evidence: added.** The focused oracle now localizes process-only
  state, identity drift, failed reconnect/authentication, stale read position,
  changed data, credential leakage, and reopen failure. The guide adds
  source-search checks that expose mocked and old-authority paths.

## First failure and characterization results

1. Baseline review established that `installed_lifecycle.rs` had no second
   `serve`, fresh session, or post-restart semantic query; its only post-kill
   evidence was offline verification.
2. Baseline Connectome call-site review established that no screen constructs
   `RrdAuthenticatedClient`; `RuntimeConnectionPage` calls only `handshake()`.
   The browser test intercepts `/v1/**` and returns fixed data.
3. The first changed Rust compile failed with E0382 because the initial
   assertion moved `locator.identity.instance_id` before reuse. The test now
   clones the first locator identity; no production type or ownership rule was
   weakened.
4. The focused restart oracle then passed and returned the exact committed
   claim through the fresh second-process client path.
5. The first post-plan validator run during partial implementation correctly
   rejected not-yet-created declared paths. No scope or gate was bypassed.

## Acceptance commands and exact results

| Command | Result |
|---|---|
| `python3 scripts/ci/check_change_plan.py` | PASS initially with zero post-plan paths and finally with all 8 declared post-plan paths. |
| `cargo test -p rrflow-cli --test installed_lifecycle installed_engine_plans_applies_serves_discovers_authenticates_and_reopens --locked` | PASS, 1 passed, 0 failed, 3 filtered out, approximately 3.48 seconds. |
| `cargo test -p rrflow-cli --test installed_lifecycle --locked` | PASS, 4 passed, 0 failed, 3.50 seconds. |
| `cargo test -p rrd-client --test real_server --locked` | PASS, 3 passed, 0 failed, 2.51 seconds after compilation. |
| `cargo fmt --all -- --check` | PASS with no diff. |
| `python3 scripts/ci/check_documentation.py` | PASS: 272 document statuses, 270 classified coordinates, parent indexes, ownership, and local links. |
| `python3 scripts/knowledge/render_navigation.py --check` | PASS: 14 generated indexes, 271 nodes, 1,118 edges, 0 files updated. |
| `python3 -m unittest scripts.knowledge.test_navigation` | PASS, 6 passed. |
| `python3 scripts/ci/build_execution_inventory.py --check` | PASS: 1,158 current, generated, and planned records. |
| `python3 scripts/ci/check_generated_surfaces.py --check` | PASS: 33 HTTP operations, 7 signal projections, signal catalogue `239df2ced5974d4351ca01565369646964cb65c63d0fbae01dddccb4c3ac5a07`, OpenAPI `96a232bda6cfb3135e75df86e8b97a77b8a5f098545ce9fe8dc0d3e7a6d6ad26`. |
| `python3 scripts/check_version.py` | PASS: release train remains `1.0.0`. |
| `git diff --check` | PASS with no whitespace error. |

## Surfaced failures and corrections

- The E0382 moved-value failure was corrected by preserving the first locator
  identity for the restart comparison.
- Partial-package validation listed missing declared files as designed. The
  files were authored; the validator was not weakened.
- Final generated-surface output showed that a literal digest embedded in the
  older Connectome convergence prose was stale. The prose now requires exact
  packaged-to-advertised identity validation without copying a transient
  digest; no generated contract or compatibility fallback changed.
- No runtime failure, state loss, instance drift, query mismatch, leaked
  credential, or final reopen failure appeared in the passing test.

## Checks not run

The full workspace, release, benchmark, platform, signing, Kubernetes, remote
transport, WebSocket, Connectome artifact, and UI suites are not acceptance
for this bounded package and were not run. In particular, previously passing
Connectome mock/browser and fake-server tests were not relabelled as real
engine evidence. The guide requires a future Connectome artifact lane against
an explicit RRFlow binary.

## Remaining known errors and owners

- H-04/J-03 own qualified packaged generated client surfaces and complete
  operation/model coverage.
- D-01/J work owns the public installed credential/connection handoff, signed
  offline bundle, platform service behavior, and recovery tooling.
- H-06 owns Connectome C0-C5 convergence, real-artifact UI proof, WebSocket
  resume/ACK behavior, bounded rendering, accessibility, and security corpus.
- Gates E/F/G/H own the graph, BM25, vector, RRF, Arrow/DataFusion, context,
  reasoning, feedback, and trace evidence the final UI must render.
- POAM-006, POAM-011, D-01, H-04, H-06, Gate J, alpha, and release remain open.

## Full-file reread, diff review, and checklist

The complete changed test, two client records, new guide, inventory-generator
edit, this journal, generated Gate H index, and generated file inventory are
reread after formatting and generation. The final diff is reviewed for scope,
mock/production language, credentials, unchanged authorities, generated drift,
and accidental sibling-checkout changes.

- [x] Exact starting revision/tree, planning commit, remotes, and sibling
  source revision recorded.
- [x] Complete affected-file reads distinguished from partial navigation.
- [x] One bounded engine oracle implemented through the public client path.
- [x] Separate UI source audit translated into explicit convergence packages.
- [x] Research, trace, resource, debugging, stop, and non-run decisions recorded.
- [x] Final declared commands reconciled after generated outputs.
- [x] Result commit is the commit containing this record; no development push
  had been performed when this immutable journal was authored.

## Status and publication

Roadmap/POA&M status change: **none**. This package closes neither D-01 nor
H-06. The result is the commit containing this record. The exact development
remote, ref, and revision must be resolved immediately before any push; no
official promotion is authorized or performed.
