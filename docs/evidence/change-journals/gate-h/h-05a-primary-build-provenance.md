# H-05a primary build provenance

**Status:** active immutable implementation evidence
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-h/h-05a-primary-build-provenance`
**Owner:** bounded H-05a prerequisite evidence; [Gate H](../../../roadmap/rrflow-1.0/gate-h.md#h-05a--build-and-signal-identity) owns acceptance

## Change brief and baseline

Starting revision: `5e9cdbfa4e170551856b9ac8e28afb74b8265fc5`, the merged
research plan. The isolated `agent/build-signal-identity` worktree preserves
all other worktrees. This package advances the build-identity prerequisite to
OBJ-07/OBJ-10 and D-01, under H-05a and POAM-026.

Current to target: retain product/protocol/stage and installed lifecycle
behavior; add one local build report shared by version and serve, a declared
build-input manifest and actual-compilation capture helper, and a
release-derived symbols-only diagnostic profile. Ordinary Cargo builds expose
compiler facts with unavailable source provenance. The helper rejects dirty
source by default, source/compiled-unit drift and inconsistent identity; it
never certifies a release.

The pull-request brief was prepared outside the worktree before edits.
GitHub requires a differing commit before it can open the PR, so the brief
is submitted with the first coherent implementation commit rather than
creating the retired planning-only commit. Stop conditions remain runtime
scope expansion, duplicated authorities/catalogues, undeclared dependencies,
source drift, unsupported qualification claims or failed candidate CI.

## Read and implementation traceability

Read in full: repository AGENTS and README; change-authoring procedure;
root and CLI Cargo manifests; CLI `main.rs`, `installed.rs` and
`tests/installed_lifecycle.rs`; every new Rust/Python source and test in this
package; release reference/index and POAM-026; CI reusable workflow.
Read scoped owning sections: alpha OBJ-07/OBJ-10; D-01; H-05a through H-05e;
the researched scale plan's next cut; engine-data-flow build profiles and
identity; CI candidate chain; public `ServiceCapabilities` and
`openapi_document`; HTTP capabilities construction; kernel telemetry types
and signal fingerprint; engine manifest and workspace dependency/source
guards. Relevant commands, generated inventory and the diff derive
mechanical path facts rather than a second semantic inventory.

Existing `VersionReport` fields stay in `installed.rs`. New
`build_identity::report` is reused by the version and local startup report.
`build_metadata` owns the strict, bounded manifest shape used by both the
build script and executable. `build.rs` observes compiler facts and consumes
declared provenance; Git is confined to repository build tooling.
The actual primary executable test, independent executable hash and unchanged
installed lifecycle corpus characterize and prove this additive replacement.
No engine, storage, lifecycle or public protocol implementation is moved,
deleted or replaced.

Created: CLI `build.rs`, `src/build_metadata.rs`, `src/build_identity.rs`,
`tests/build_identity.rs`; `scripts/build_rrflow.py` and its test;
the [build identity reference](../../../reference/release/build-identity.md)
and this journal. Changed: root/CLI manifests, CLI main/installed and lifecycle
test, CI workflow/operations, release index, Gate H and POAM-026; generated
journal index and execution inventory. Cargo.lock requires no change because
the build uses existing locked serde/serde_json dependencies. README's stale
no-native-binary statement is corrected against accepted PR32 evidence;
POAM-026's stale absent-metric-catalogue wording is corrected against the
existing kernel/generated catalogue. Neither correction changes maturity.

## Research, trace and resource decisions

Primary research:
[Cargo artifact messages](https://doc.rust-lang.org/cargo/reference/external-tools.html#artifact-messages)
and [custom profiles](https://doc.rust-lang.org/cargo/reference/profiles.html#custom-profiles).
Actual compiled units/features are observed twice instead of inferred from
dependency-resolution inventory. The helper records source and selected
environment digests, not raw environment values; metadata contains no
absolute source paths, timestamps, credentials or estate state.
No upstream source is adapted.

No new runtime signal is introduced. Executable hashing streams through a
64 KiB buffer; the input manifest is bounded at 256 KiB with typed field and
collection limits. Runtime Git, source discovery, network or provider access
is absent. This is not a hermetic build, trusted attestation, loaded-memory
measurement, diagnostic-level implementation or latency qualification.

## Failure oracle and executed evidence

Pinned Windows Rust 1.99.1; shared
`CARGO_TARGET_DIR=D:/localdev/devlabs/rrflow/target`.

- First real-binary oracle: `cargo test --locked -p rrflow-cli --test
  build_identity -- --nocapture` failed because `build.format_version` was
  absent (`null`, expected `1`). After implementation it passes: relocation
  outside the checkout, empty PATH, ignored runtime manifest override,
  independent streamed executable digest, compiler/assertion identity,
  contract digests and repeated-output equality.
- The initial test harness used unsupported `LowerHex` formatting for the
  locked sha2 0.11 digest type. It was corrected to bytewise hexadecimal
  before the red behavior assertion. The first implementation also guessed
  the OpenAPI extension prefix incorrectly; the existing
  `x-rrd-signal-catalogue-sha256` owner corrected that surfaced error.
- `python scripts/test_build_rrflow.py`: 4 passed, covering real temporary
  Git source dirtiness/content changes, changed compiled features/semantic
  settings and missing artifacts, environment digest redaction, and rejection
  of semantic diagnostic-profile overrides.
- `cargo test --locked -p rrflow-cli`: final run 16 passed, zero failures.
  This includes two malformed/unknown-manifest unit tests, the relocated
  executable test, controller/admin tests and five installed lifecycle tests.
  Fresh/existing native tests include version/startup identity equality,
  commit/query, restart, backup and restore readback; their group completed
  in 194.20 seconds. This development-profile duration is a test observation,
  not performance evidence.
- `cargo clippy --locked -p rrflow-cli --all-targets -- -D warnings` passed.
- The first `cargo test --locked -p rrd-engine --test workspace_architecture`
  run passed 30 checks and rejected the two new Cargo targets while they were
  still untracked. Staging the new build script and integration test resolves
  that source-closure prerequisite; the candidate records the subsequent run.
  The next run exposed a Unix-specific absolute path in the Python fixture.
  The fixture now derives its path from the platform temporary directory;
  the architecture guard remains unchanged.
- `python scripts/build_rrflow.py --profile dev --target-dir
  D:/localdev/devlabs/rrflow/target --output <new-receipt>` correctly rejected
  dirty source with no receipt.
- The same helper with `--allow-dirty` passed two real native builds and
  embedded/read back the actual compilation closure. Receipt:
  `target/rrflow-build-identity-dirty-smoke.json` in the parent workspace.
  `qualification` remains `unqualified`; no dirty build is release evidence.
- `cargo fmt --all -- --check`, whitespace, workflow, documentation,
  navigation and execution-inventory validation are required before commit.
  Their final results and the clean committed-source build receipt are bound
  to the candidate in the PR, avoiding a self-referential journal revision.

Some initial path guesses and PowerShell literal wildcard searches missed
their targets; repository `rg --files` located the actual paths. No source
changes followed those missing-path results. Large mixed inspection output
was reread in bounded sections; full-file review and complete diff review
precede the candidate commit.

## Checklist, remaining work and candidate binding

Owner/baseline/scope recorded; current and target behavior mapped; first
failure retained; research and trace/resource choices explicit; canonical
boundary preserved; tests and residual scope stated; source and diff reread;
one linked journal with generated navigation/inventory; candidate CI required.
The PR carries the exact destination/ref/revision receipt, CI results and
merge receipt. Only the public development repository is a push target.

Not run or claimed locally: optimized release/diagnostic builds and full
semantic parity, capacity/soak/real-model benchmarks, signed/offline bundle
qualification, cloud synchronization or PostgreSQL compatibility. Candidate
CI owns the cross-platform/workspace checks and clean Linux build-helper run.
The first-party compilation closure is observed, but external compiler/linker
trust and arbitrary global configuration are not qualified.

H-05a still requires aggregate storage/index formats, bundle identity, remote
discovery and other executables, runtime diagnostic levels and actual
release/diagnostic result/receipt/limit parity. POAM-026 remains open; D-01,
objective status, release maturity and frozen `1.0.0` are unchanged.
