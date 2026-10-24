# Primary executable build identity

**Status:** active implementation reference; release qualification remains open
**Coordinate:** `rrflow://rrflow-instance/data/reference/release/build-identity`
**Owner:** primary executable provenance for the H-05a prerequisite; [Gate H](../../roadmap/rrflow-1.0/gate-h.md#h-05a--build-and-signal-identity) owns acceptance

The primary `rrflow --json version` result contains one `build` record.
The local `serve --json` startup announcement projects the same record.
It observes the executable and its declared build inputs; it does not grant
authority, change engine behavior, or certify a release.

## Build and inspect

From a clean checkout with the pinned Rust toolchain and Python 3.11 or later:

```text
python scripts/build_rrflow.py --profile release --output target/release-build-receipt.json
python scripts/build_rrflow.py --profile diagnostic --output target/diagnostic-build-receipt.json
```

The output path must be new. `--target-dir` selects Cargo's build directory.
`--profile dev --allow-dirty` is available for local debugging and records
dirty source explicitly. The helper uses locked dependencies and the native
target. It is build tooling; the installed executable needs neither Python,
Cargo, Git nor a checkout to report its identity. Dependency acquisition still
follows Cargo's configured cache/network policy; this is not an offline bundle.

The helper first observes actual `compiler-artifact` messages for the selected
primary binary, then embeds a bounded manifest and repeats the same build.
It rejects source changes or a different first-party feature/semantic-profile
closure between passes, mismatched compiler/target/root profile, malformed or
oversized manifests, missing primary artifacts, and an executable hash that
differs from the report. It writes the receipt only after these checks pass.
The primary executable's build script consumes the declared manifest; it
never runs Git or tries to infer transitive features.

The diagnostic Cargo profile inherits release and overrides only retained
line tables and symbol stripping. Policy tests reject semantic settings
introduced into that profile. This checks profile structure, **not actual
release/diagnostic result, receipt, format or limit equivalence**.

## Record semantics

| Field | Meaning |
|---|---|
| `format_version` | Version 1 of this local executable observation. |
| `qualification` | Always `unqualified`; provenance cannot close a roadmap gate. |
| `compiler` | Compiler version, target, Cargo root profile, optimization level, panic strategy and debug-assertion setting captured during compilation. A custom profile's exact name belongs to `provenance.cargo_profile`. |
| `provenance` | Declared source commit and Git tree; SHA-256 of length-framed source inventory entries (path, Git mode and content digest, symlink target, deletion or gitlink identity); dirty status; Cargo.lock digest; compiler/target/profile; a digest of selected build-affecting environment values; and actual first-party compilation units with their features, optimization, assertions and overflow checks. |
| `metadata_sha256` | Digest of the exact embedded compiler/provenance JSON bytes. |
| `executable_sha256` | SHA-256 of the file found at the running process's executable path, streamed with a 64 KiB buffer. This is an observation, not an operating-system measurement of loaded memory or a signature. |
| `public_contract_sha256` | Digest of the generated OpenAPI document, including its operation and schema projection. |
| `signal_catalogue_sha256` | Existing canonical signal fingerprint from that generated document. |

An ordinary `cargo build` embeds compiler facts but reports `provenance: null`.
It must not invent a clean commit, exact custom profile, or complete feature
closure. `RRFLOW_BUILD_MANIFEST` is a **build-time** input; setting it on an
installed process cannot change its embedded record. The manifest contains no
raw environment values, absolute source paths, credentials, clock timestamp
or estate configuration. The local receipt identifies the binary path outside
the public build record.

The build helper is not a hermetic-build or supply-chain attestation service.
It records selected environment overrides as a digest; it does not prove
compiler/linker trust, native dependency reproducibility, arbitrary global
Cargo configuration, or byte-identical output across hosts. Declared metadata
can be forged by a different builder. Clean source plus matching compilation
units is necessary evidence, never sufficient release qualification.

Effective estate configuration remains the engine-owned
`ServiceCapabilities.configuration` with its existing revision and digest.
The startup record identifies the **local primary executable**. It does not
pretend that a local CLI `ready` invocation identifies a remote server.

## Remaining H-05a acceptance

Aggregate storage/index format identities, distribution manifest identity,
public remote discovery and other bundled executables, enforced runtime
diagnostic levels, and actual release/diagnostic result/receipt/limit parity
remain open. The installed lifecycle corpus continues to exercise the same
primary binary and now checks that startup and version identity agree.
No latency, capacity, signed-release or complete H-05a claim follows.

The implementation follows Cargo's documented
[artifact message contract](https://doc.rust-lang.org/cargo/reference/external-tools.html#artifact-messages)
and [profile inheritance](https://doc.rust-lang.org/cargo/reference/profiles.html#custom-profiles).
Cargo's artifact observations are used instead of treating dependency
resolution inventory as proof of the compiled feature closure.
