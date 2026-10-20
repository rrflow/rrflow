# H-04 native rrflowDB local integration

**Status:** active implementation evidence; candidate landing and full gate qualification remain pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/h04-rrflowdb-local-integration`
**Owner:** OBJ-07/OBJ-09 native installed integration prerequisite, D-01/H-04; C-03/C-07 rejection regression

## Change brief and baseline

[PR 26](https://github.com/rrflow/rrflow-development/pull/26) was created with
the scoped brief before new implementation. The branch starts from development
main `9bcc6154e9ee48f3ad8d28786a6abe75689e371d`, carrying the existing database
identity/compatibility plan as `59d12a4a1e59174db2999731dc1405fc08e39890`.
The owner requested a real buildable database, component/relational diagrams,
DevPulse local operation and optional shippin.cloud account synchronization.

Current to target: native persistent transactions and rrflowQL joins existed,
but schema discovery required a broad privileged diagnostic read and there was
no runnable installed relational application example. The package adds bounded
schema discovery, a native client example and installed regression proof.
The first semantic-failure test found the unabortable-intent bug recorded in
[POAM-033](../../../poam/rrflow-1.0-alpha/poam-033.md). The PR brief was expanded
before its canonical transaction fix, within the planned atomic-rejection
oracle. No storage format, default dependency, product version, SQL executor,
account credential, or cloud transfer was introduced.

## Reviewed sources and traceability

Owner records reviewed: README; alpha objective; roadmap D/H and dependency
chain; change-authoring procedure; system overview and instance topology;
installed-lifecycle and Rust SDK references; CI operations; existing PostgreSQL
profile. Prior PostgreSQL/Turso primary-source research remains in the carried
plan and its journal; this package adapts no upstream implementation.

Complete small-file/new-source reads and final review include engine `data.rs`,
CLI `main.rs`/`command.rs`, client `lib.rs`/`client.rs`/`endpoint.rs`, new example,
shared example support, regression and new documentation. Targeted sections of
large files were inspected for the exact modified behavior: contract query,
schema, mutation, catalogue and exporter; engine transaction commit/abort,
session policy, public-schema conversion, diagnostic/schema read, installation,
recovery and query-transaction tests; storage `commit_at_read`, semantic-plan
publication, schema-version reads and error variants; HTTP authenticated
envelope/data dispatch; client operation/session code and real-server tests;
installed CLI/process test; native rrflowQL parser/join corpus. Generator and
SDK metadata/count/digest test sources were inspected before regeneration.

| Behavior | Canonical source retained/extended | Oracle |
|---|---|---|
| Schema discovery | contract descriptor/types, engine `data.rs`, stamped `read_versioned`, HTTP data handler, typed Rust client | preserved bootstrap catalogue, matching revision/cursor, scope/budget/closed-session denial |
| Semantic commit rejection | engine `transaction.rs`; existing storage semantic plan/outcome | unique, wrong-type and stale-cursor failure leaves no partial write and can be aborted |
| Unknown commit outcome | existing intent/receipt reconciliation | five recovery tests and full engine unit suite |
| Relational application | client example uses public schema/begin/commit/query/abort methods | real installed rrflowKV server, parameterized join, update, history, replay, restart |
| Public projections | existing contract exporter and five SDK generators | 37 descriptors and exact OpenAPI fingerprint parity |

No implementation was deleted, relocated, renamed or given a fallback.
Existing read evidence and transaction receipts are retained. New schema reads
charge an explicit key budget and use the administrative diagnostics grant;
restricted data-policy grants fail closed. Tests capture results/cursors rather
than credentials. No performance, zero-copy, complete SQL, cloud-sync or release
qualification is inferred from these checks.

## Commands and outcomes

Cargo commands used the repository-pinned toolchain with `--locked`, shared
`CARGO_TARGET_DIR=D:/localdev/devlabs/rrflow/target`, and dev/test debug info
disabled. This is a native Windows development build.

| Command/check | Result |
|---|---|
| `cargo check --locked -p rrd-client -p rrd-engine -p rrd-server` | passed |
| `cargo test --locked -p rrd-client --test rrflowdb_local` | passed: one full installed relational/atomicity/reopen regression, including stale write and closed-session cases |
| `cargo test --locked -p rrd-client -p rrd-contract -p rrd-server --all-targets` | client targets passed; contract fingerprint first failed, then corrected; later server local-driver target needed its existing controller binary (see below) |
| `cargo test --locked -p rrd-contract -p rrd-server --all-targets` after fingerprint correction | contract tests and server HTTP/process targets passed; local-driver 3/4 passed before missing-binary prerequisite was resolved |
| `cargo build --locked -p rrflow-cli --bin rrd-estate-controller` then `cargo test --locked -p rrd-server --test local_estate_driver` | 4/4 passed, including process-kill matrix |
| `cargo test --locked -p rrd-engine --lib engine::tests::recovery` | 5/5 passed |
| `cargo test --locked -p rrd-engine --lib engine::tests::query_transaction` | 2/2 passed |
| `cargo test --locked -p rrd-engine --lib` | 99/99 passed, including governed-function and recovery tests |
| `cargo build --locked -p rrflow-cli --bin rrflow -p rrd-client --example rrflowdb` | passed; actual Windows executables retained |
| `cargo clippy --locked -p rrd-client -p rrd-contract -p rrd-server -p rrd-engine --all-targets -- -D warnings` | passed |
| `cargo fmt --all`, final format/diff checks | formatted only changed Rust files; no unrelated changes |
| Java/.NET/Python/Go/TypeScript existing SDK generators | ran; generated projections reflect 37 operations |
| `python scripts/ci/check_generated_surfaces.py --write` then without `--write` | passed: 37 operations, OpenAPI `7dc64c487dd6930cf99751cbca84e8fc1debc7ad43c9c4b22e847b5138077ede` |
| TypeScript `corepack pnpm typecheck` and `corepack pnpm test` | passed; 6 tests |
| Go `go test ./...` | passed |
| navigation, documentation policy, execution inventory | regenerated and checked before commit |

The ordinary Rust SDK conformance test returns without work when its harness
environment is absent. Its green line is not claimed as six-language live
conformance. Python/Java/.NET complete test suites, the full workspace,
non-Windows installed binaries, cloud/account sync and PostgreSQL reference
corpus were not run locally; candidate CI owns its broader configured lanes.

## Actual installed process and retained build

The local verification script `target/rrflowdb-installed-smoke.py` copied the
built executables into `target/rrflowdb-devkit-20261019/`, installed its empty
`project/` using the actual binary digest and accepted plan, and started the
primary `rrflow.exe` with an ephemeral loopback address. It authenticated ready,
ran the example's init/read/update/abort, killed the exact owned process,
performed read-only quick verification, restarted, compared rows/relationships
and cursor, then stopped and verified again. It passed with one relationship,
task status `done`, and runtime cursor 7. The persisted project is retained and
the service is stopped. Launch/read/update/verify PowerShell scripts and the
full non-secret `verification.json` are beside the executables.

| Development artifact | SHA-256 |
|---|---|
| `rrflow.exe` | `906147b144542cc5a907031045efe55c2533113be6ada5680e72aef4e68f372c` |
| `rrflowdb.exe` (client example) | `b6d7a003f6baa0acccd400b1e4aa2c7e09eb26a5aba2bf9ea102b38e3601b232` |

This is a local build receipt, not a signed distribution or promotion.

## Failures surfaced and corrected

- Initial client commit omitted the required deadline; the example now binds
  it to the transaction lease expiry.
- Initial query requested zero spill bytes, which the native contract rejects;
  the example now declares a finite 16 MiB spill ceiling.
- Unique rejection left a durable intent and abort returned `CommitInProgress`.
  Typed definitive rejection plus absent outcome now permits explicit abort;
  unknown I/O outcomes keep their recovery intent. The same oracle then passed
  for unique/type/stale-cursor rejection, replay and restart.
- Contract tests exposed the old OpenAPI constant; it and all generated
  projections/count/digest fixtures were intentionally updated together.
- A test used a nonexistent descriptor field; it now asserts
  `action.fixed_action()` through the actual contract.
- The server process test required `rrd-estate-controller`; an initial build
  selected its former-looking package name, then used its actual `rrflow-cli`
  package. The four-test target passed after building the prerequisite.
- Host pnpm 10 could not read the pinned pnpm 12 multi-document lock; using
  Corepack selected pinned pnpm 12.14.0 and frozen installation succeeded.
- Documentation checks caught missing index/POA&M sections and a PowerShell
  type expression parsed as a link. Index/sections and PowerShell 7 UTF-8
  output instructions were corrected. Python generators' Windows CRLF output
  was normalized to the canonical generated LF bytes.

## Checklist and status

The PR brief preceded implementation; effect scope, first oracle, research,
trace/resource decisions, read/changed boundaries, failures and unrun checks
are recorded here. Changed source and new application/diagram records were
reviewed with the full diff. Navigation and inventory are mechanical outputs.
No roadmap item or alpha objective is marked complete. POAM-011 records absent
DevPulse cloud synchronization; POAM-033 remains open pending candidate CI and
reviewed landing. PostgreSQL compatibility and a packaged DevPulse app remain
open. There was no cloud upload, account creation, official push, or release.

PR/CI/merge evidence after this receipt belongs in a subsequent linked receipt;
this journal is immutable once committed.
