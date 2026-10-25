# A-07 TypeScript SDK transport free-function journal

**Status:** active immutable change-journal evidence
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-a/a-07-typescript-transport-free-function-journal`
**Owner:** A-07 TypeScript SDK package receipt; the roadmap retains completion authority

This record reports one bounded correction to the TypeScript SDK: the client
invokes its Fetch transport as a free function, so a browser's `fetch` — which
throws "Illegal invocation" when called as another object's method — works
unchanged. A-07 stays as it is; the
[Gate A owner](../../../roadmap/rrflow-1.0/gate-a.md) retains acceptance.

## Evidence record

```text
gate/package and alpha outcome or prerequisite advanced: A-07 TypeScript SDK (browser transport profile) toward the delivery outcomes; found by Connectome consuming the generated SDK for its public RRD handshake (Connectome sprint 008, Slice C).
starting revision/tree and pull request: main at 470594178d25f1bf12d3ec6d9041488817a9e931; branch sdk/typescript-transport-binding; pull request #15 in rrflow/rrflow-development.
change brief (current -> target behavior, owner, exact scope, unchanged behavior, stop conditions): current — RrdClient stored config.fetch ?? globalThis.fetch as a property and called it as this.transport(url, init); in Node any this is accepted, but a browser's fetch requires this to be the global object (or undefined) and throws TypeError "Failed to execute 'fetch' on 'Window': Illegal invocation", so the SDK could not run in a browser page with the page's own fetch — Connectome's render smoke failed on every handshake. Target — the constructor keeps the caller's transport in a local and stores an arrow function that calls it as a free function; the injected transport observes this === undefined; no other behavior changes. Owner: A-07 (TypeScript SDK). Scope: sdks/typescript/src/client.ts (constructor), sdks/typescript/tests/client.test.ts (one new test), docs/reference/sdk/typescript.md (one sentence), this journal, regenerated indexes and execution inventory. Unchanged: the generated surfaces, the OpenAPI digest, request construction, retry, decoding, every other SDK, every crate. Stop on any change to the generated files or the public ClientConfig.
files and symbols read in full: sdks/typescript/src/client.ts; sdks/typescript/src/endpoint.ts; sdks/typescript/src/transport.ts; sdks/typescript/tests/client.test.ts; docs/reference/sdk/typescript.md bootstrap/browser profile section.
implementation/evidence paths created, changed, moved, or deleted: created this journal; changed sdks/typescript/src/client.ts, sdks/typescript/tests/client.test.ts, docs/reference/sdk/typescript.md; regenerated docs/evidence/change-journals/gate-a/README.md and docs/roadmap/rrflow-1.0-file-plan.jsonl. Nothing moved or deleted.
research decision and primary-source/adaptation record: no external source. The browser rule is the WHATWG Fetch binding's brand check on Window.fetch, observed directly in Chromium through Connectome's Playwright smoke before the correction.
trace/resource/debug decision (add/preserve/not applicable, with reason): not applicable — one constructor line.
first-failure or characterization oracle and result: tests/client.test.ts "the transport is invoked as a free function, as a browser's fetch requires" injects a transport that throws the browser's TypeError whenever this is neither undefined nor globalThis; against the previous client it fails ("RRD transport failed: Failed to execute 'fetch' on 'Window': Illegal invocation", 5 passed, 1 failed), against the corrected client it passes (6 passed).
acceptance commands and exact results: pnpm test (sdks/typescript): 6 passed, 0 failed. pnpm lint: clean (15 files). pnpm typecheck: clean. pnpm generate:check: current. python3 scripts/ci/check_generated_surfaces.py --check: OK, OpenAPI 07bef046… unchanged. python3 scripts/ci/check_documentation.py: OK. python3 scripts/knowledge/render_navigation.py --check: OK. Two knowledge exports at the working revision: byte-identical; python3 scripts/knowledge/test_export.py: OK. python3 scripts/ci/build_execution_inventory.py --check: OK. python3 scripts/ci/check_workflow.py: OK. python3 scripts/check_version.py: OK, 1.0.0.
surfaced failures and corrections: none beyond the defect itself.
checks not run and reason: the SDK conformance harness and the other SDK suites run on the pull request; a real-browser run of this SDK test is Connectome's render smoke, which passes with an explicitly bound fetch today and will run the SDK's default transport once Connectome's pin moves to this revision.
remaining known errors and owner: the SDK's browser profile remains loopback-only, without TLS, redirect policy, or browser session acquisition (the reference's qualified-connection steps; A-07/H-04).
full-file reread and diff review: complete for every changed authored file before commit.
change checklist: complete — owner chain, reference record, link closure, generated indexes, knowledge package, and execution inventory reconciled; no POA&M row changes.
roadmap/POA&M status change, or explicit none: none.
pull request, CI run, and merge evidence: pull request #15 carries the candidate CI runs and the merge; this record does not restate them.
```

## Links

- [Gate A](../../../roadmap/rrflow-1.0/gate-a.md)
- [TypeScript SDK](../../../reference/sdk/typescript.md)
