# A-07 OpenAPI and SDK projection convergence

**Status:** active immutable change-journal evidence
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-a/a-07-openapi-sdk-projection-convergence`
**Owner:** bounded A-07/H-04 generated-surface repair evidence

This record reports one completed projection-repair package. It cannot change
roadmap or POA&M status. Follow the
[canonical roadmap](../../../roadmap/rrflow-1.0.md) for acceptance and the
[change-journal index](../) for discovery.

## Evidence record

**Gate/package and prerequisite advanced:** A-07/H-04 package
`A07-2b-openapi-sdk-projection-convergence-v3`; restore a clean generated
public-surface baseline before the installation-clock contract changes.

**Starting revision/tree and planning commit:** implementation baseline
`03bb278e2f4cb428752ca0aee1d47b3c477b5f58`, tree
`1ef4ae246b18988648e0e5de9f2a58774707fe5f`; final planning-only commit
`8a9fc3395da56cb730f26a180be4a816905cc0af`. Planning corrections
`a407825` and `03bb278` changed only the active plan.

**Change brief:** The unchanged `rrd-contract-export`, Rust constant, public
reference, and TypeScript endpoints carried OpenAPI digest `0ef644d8…`, but
four endpoint projections, five signal projections, and five handwritten SDK
test expectations carried `8f9efc7b…`. Keep the 33 operations, all endpoint
semantics, signal fingerprint, protocol/product versions, dependencies, and
runtime behavior unchanged. Make each endpoint generator emit the digest it
actually consumed, make each signal test compare its two generated identities,
and regenerate the replaceable projections. Stop on any contract/export,
operation, signal, dependency, runtime, release, or undeclared path change.

**Files and symbols read in full:** The authenticated active plan; `AGENTS.md`;
`README.md`; alpha objective; canonical roadmap; Gate A and Gate H; parent
POA&M and POAM-015; public-contract reference; generated-surface design; all
848 lines of the shared surface checker; all five complete endpoint generators;
all five complete signal tests; and the complete navigation and execution-
inventory generators at their recorded baseline digests.

**Implementation/evidence paths created, changed, moved, or deleted:** Changed
the five endpoint generators and five focused signal tests. Regenerated the
TypeScript, Python, Go, Java, and .NET endpoint projections plus the generated
signal reference and five signal SDK projections. Updated POAM-015, created
this journal, and regenerated the Gate A journal index and repository file
plan. No file was moved or deleted; no contract, runtime, dependency, lockfile,
version, release, or ignored `.rrflow` path changed.

**Research decision and adaptation:** External research was not required. This
was fully determined by the repository's canonical exporter, existing
generators, surface checker, tests, and generated-surface owner. No upstream
source or behavior was adapted.

**Trace/resource/debug decision:** Runtime trace and resource evidence were not
applicable because no runtime path changed. Debug evidence was preserved as
the exact baseline checker failure, per-generator checks, and focused language
tests. Generated output remains a replaceable projection, not authority.

**First-failure oracle and result:** At baseline,
`python3 scripts/ci/check_generated_surfaces.py` failed:
`python endpoints is stale: missing OpenAPI digest 0ef644d8…`. Direct
inspection confirmed TypeScript endpoints at `0ef644d8…` and every other
endpoint/signal projection at `8f9efc7b…`. After generation, the shared
checker reported 33 operations, signal fingerprint `239df2ce…`, and OpenAPI
`0ef644d8…`.

**Acceptance commands and exact results:**

- All five endpoint generator `--check` modes passed.
- `python3 scripts/ci/check_generated_surfaces.py --check` passed: 33 HTTP
  operations and seven signal projections at the canonical digests.
- TypeScript `pnpm check` passed generator, Biome, typecheck, and all 5 tests.
- Python focused pytest passed 1 test; Ruff lint passed the package, formatting
  passed all 4 changed files, and mypy passed 11 source files.
- Go `go test ./...` passed; both changed Go files were gofmt-clean.
- Java's focused Maven `SignalCatalogueTest` passed.
- .NET's xUnit v3 executable runner passed exactly 1 selected
  `SignalCatalogueTests.GeneratedSignalCatalogueIsPublicExactAndInert` test;
  the solution build passed with zero warnings and errors.
- Change-plan validation reconciled exactly 25 post-plan paths; documentation
  policy passed 269 statuses and 267 coordinates; navigation checked 268 nodes
  and 1,106 edges with zero updates; all 6 navigation unit tests passed; the
  execution inventory checked 1,154 records; version policy retained `1.0.0`;
  and `git diff --check` passed.

**Surfaced failures and corrections:** The first plan used package aliases
instead of literal generator paths; the validator rejected it. The second plan
used the surface check's implicit default; the validator required explicit
`--check`. Both were corrected in planning-only commits before implementation.
Three initial generator checks were launched from package directories while
also testing repository-relative paths and exited before generation; the exact
root-relative plan commands then passed. A package-wide Python format check
found an untouched pre-existing wrapping difference in
`sdks/python/tests/test_client.py:77`; that undeclared file was preserved and
all changed Python files passed. A .NET run inside `sdks/dotnet` correctly
rejected the host SDK 10.0.112 because that subtree pins 10.0.111. The planned
root-level `dotnet test` built but ran zero xUnit v3 tests, so it was not
counted; the repository-established `dotnet run ... -method ...` command then
ran and passed exactly one test.

**Checks not run and reason:** Full six-SDK live-daemon conformance, complete
workspace Rust tests, persistence faults/benchmarks, installation/attunement,
Connectome, DataFusion, Kubernetes, and release/signing suites were not run
because this package changes only generated SDK identity metadata and its
focused assertions. Native CI still owns the exact .NET 10.0.111 run.

**Remaining known errors and owner:** POAM-015 remains active for sample-route
drift, retired limitation labels, alternate successful shapes, and
`ContextPacket` finite-float wire identity. H-04 still owns convergence of
the five endpoint generators into one compiler and complete cross-surface
models/conformance. The trusted installation clock, portable local identity,
native ACL/filesystem qualification, backup/repair/uninstall, attunement
execution, Kubernetes, and signed distributions remain under Gates D and J.

**Full-file reread and diff review:** Every changed hand-authored generator,
test, POA&M span, and this journal was reread after formatting. Generated
projections were verified by their owning generators and common structural
checker. Final changed-path reconciliation and complete diff review found no
endpoint, signal, contract, runtime, dependency, lockfile, version, deletion,
or undeclared path change.

**Change checklist:** Exact baseline and owners bound; all planned existing
files authenticated and read; smallest failure preserved; research/trace/
resource/debug decisions recorded; one canonical export retained; narrow tests
run before widening; failures and non-runs retained; linked journal authored;
nearest index and inventory regenerated; full diff reviewed.

**Roadmap/POA&M status change:** None. POAM-015, H-04, Gate J, alpha, release,
and promotion remain open.

**Commit/development push evidence:** The result revision is the commit containing
this journal. Its exact revision, destination URL, branch ref, and push result are
resolved and reported immediately before and after the development-only push. No
official-origin push, tag, artifact, release, or promotion is authorized here.
