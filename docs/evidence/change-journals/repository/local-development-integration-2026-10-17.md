# Local development integration, 2026-10-17

**Status:** active immutable change-journal evidence
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/repository/local-development-integration-2026-10-17`
**Owner:** local integration evidence; no roadmap, release, or CI authority

## Change brief and scope

The owner requested synchronization of the existing local RRFlow branch with
fetched development history. The local baseline was `985d961e60c781d88acfc71322e303ba14fb9b04`
on `agent/connectome-temporal-runtime-visualizer`, with a clean working tree.
The incoming revision was `b27579299b91dbe8fe13893ff66c2cd8906c8466`
from `development/main` at `https://github.com/rrflow/rrflow-development.git`.
The histories diverged by 14 local commits and 23 incoming commits. A merge
preview reported 22 conflicting paths. The baseline is retained at
`backup/local-sync-20261017-985d961e`; no history was rewritten.

This package integrates the histories on the existing local branch. It retains
the local installation-clock policy, recovery checks, Connectome restart
persistence proof, SDK endpoint-identity assertions, and development container.
It also retains the incoming storage diagnostics, SDK/toolchain refresh,
portability fixes, and retirement of planning-only change validation.
The historical active-change file is preserved from the local branch, not
reauthored as a new plan. The owner requested local integration; no push,
pull request, remote main landing, or official-repository promotion is claimed.
Any later main landing remains subject to the current PR and candidate-CI policy.

Owning prerequisites are D-01 installation, H-04 client conformance, H-05
storage diagnostics, and J-02 development reproduction. No gate or POA&M
status changes. Stop conditions: do not discard either branch's behavior,
invent checksums, weaken a failure oracle, or report unrun checks as passing.

## Resolution and traceability

- Contract exports combine the local clock types and incoming diagnostic types.
  The canonical exporter produces OpenAPI SHA-256
  `c36969467fa04b235fa6b0a1312f10deb373fd9424733262e6a3216312d7f91c`.
  Endpoint/schema and signal projections are regenerated with the repository's
  generators. SDK tests retain cross-projection identity assertions.
- CI retains the current incoming tool/action pins and the local Windows/macOS
  installed-lifecycle coverage and container-parity checks.
- Container pins and the optional pgvector image match candidate CI. Temurin's
  download release `25.0.4.1+1` corresponds to CI's `25.0.4+101.0.LTS` selector.
  Certificate bootstrap now extracts Apt-authenticated trust roots before
  snapshot installation, avoiding an unpinned OpenSSL dependency upgrade.
- The real SDK harness exposed the existing stale corpus installation identity.
  Six instance/estate/scope references in `fixtures/rrd-sdk-conformance-v1.json`
  now match the actual deterministic installer result. The harness still fails
  on identity mismatch; no assertion or authorization boundary was removed.
  The resulting corpus SHA-256 is
  `28c22d2f08f457910b5d1b78a03ecd2480b5cc5e5c018c144ac70719251da3db`.
- The Connectome guide's retired-name search expressions use equivalent bracket
  expressions, and its audit prose describes the retired diagnostics protocol.
  This preserves the cleanup search without violating the repository identity
  scan. Python's merged fixture is formatted with the incoming Ruff version.
- Navigation and execution inventory are regenerated from the combined tree.
  No implementation is deleted beyond the incoming retired planning validator.

The reviewed material includes both AGENTS instructions, README status, CI and
change-authoring records, the conflict hunks, Containerfile/compose definition,
SDK generators and identity tests, contract exports, the SDK harness/corpus,
architecture identity scanner, and relevant installation/diagnostics tests.
Full authored integration diffs and generated-source diffs were reread; broad
incoming feature implementations were validated at the affected boundaries,
not represented as a new full implementation audit.

## Research and diagnostics

Official checksum/version metadata was read from Node's v24.22.0 SHASUMS256,
the npm pnpm/12.14.0 registry record, GitHub releases for astral-sh/uv 0.13.1,
actions/python-versions 3.15.0-38076878863, and
adoptium/temurin25-binaries jdk-25.0.4.1+1, Go's downloads JSON, and .NET 10
release metadata. Maven uses the checksum already accepted by incoming CI.
No upstream implementation was adapted. Runtime traces and performance
measurement are not applicable to this integration; existing trace behavior
and budgets are preserved. Commands ran in rootless Podman on Windows with
Linux Rust 1.99.1 and the rebuilt SDK toolchains.

## Validation

- `cargo fmt --all -- --check`: passed.
- `cargo test -p rrd-contract --all-targets --locked`: 91 passed.
- `cargo test -p rrd-engine --lib installation --locked`: 18 passed.
- `cargo test -p rrd-engine --test engine_authority --test workspace_architecture --locked`:
  3 authority tests passed; architecture initially 28 passed, 1 failed on the
  guide's literal retired name. After correction, the focused test and the
  complete 29-test architecture suite passed.
- `cargo test -p rrflow-cli --all-targets --locked`: 12 passed, including the
  four installed-lifecycle tests and backup/estate controller tests.
- `cargo clippy -p rrflow-cli -p rrd-contract --all-targets --locked -- -D warnings`:
  passed, including the dependency closure.
- `cargo test -p rrd-contract --test public_contract supported_sdks_share_one_strict_semantic_corpus --locked`:
  passed after refreshing the fixture identity.
- `podman build --target full -t localhost/rrflow-dev:sync-20261017 -f .devcontainer/Containerfile .devcontainer`:
  passed, including both stages and all nine checksum verifications. Image
  `4b9dd35e307a5d5f728a9aa964aeb350c1daaed8cb01ae78e0c1ee2bf02a3ffd`.
- `python3 scripts/ci/run_sdk_conformance.py`: all six supported languages passed
  against one real daemon and the same corpus. The .NET runner passed all five
  tests with no skips.
- TypeScript frozen install, Biome, typecheck, and five unit tests passed.
  Python Ruff lint/format, strict mypy (11 files), and five tests passed;
  Go `go test ./...` passed. The Python formatting check initially found the
  merged digest line wrapping, which was corrected without a semantic change.

- Every endpoint generator's check mode and `check_generated_surfaces.py` passed:
  33 HTTP operations and seven signal projections share the canonical export.
- Java `mvn -q -f sdks/java/pom.xml test`: four passed, with the live test
  skipped because this separate invocation has no harness. That live Java
  conformance test passed in the six-language harness above.
- `python3 -m unittest scripts.knowledge.test_navigation scripts.knowledge.test_export`:
  17 passed.
- Workflow policy (19 container pins), documentation, navigation, execution
  inventory (1,167 paths), version policy, and `git diff --check` passed.
- The unmerged-path list is empty; the original local branch tip and incoming
  development tip are retained as the two merge histories.

## Limits and completion boundary

Full workspace/optional-feature suites, native Windows/macOS execution,
pgvector service integration, remote candidate CI, and signed distribution
qualification were not run. The local checks above do not close those owners.
No claim is made that Connectome, LFG, managed cloud onboarding, or DALAL is
complete. No product feature work was substituted for synchronization.

The integration commit containing this record is the local merge evidence.
No remote publication was performed. The current user authorization covers
this local integration and its necessary consistency fixes.

## Links

- [CI operations](../../../operations/ci.md)
- [Installed lifecycle](../../../reference/deployment/installed-lifecycle.md)
- [Connectome contract](../../../reference/client/connectome.md)
- [SDK index](../../../reference/sdk/README.md)
