# D-06 bundled Parquet query export

**Status:** active change-journal evidence
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-d/d-06-bundled-parquet-export`
**Owner:** bounded export prerequisite receipt; Gate D retains completion authority

## Package and change brief

OBJ-09 provider-neutral interchange prerequisite (with OBJ-07 bundled operation), D-06 bounded Parquet exchange, H-04
public discovery, and D-01 installed-binary characterization. Baseline:
`78a1044b17aaaa438eb9ddf08a751e307c55563b`. [Pull request 38](https://github.com/rrflow/rrflow-development/pull/38)
contained the brief before source implementation. Its empty opening commit
only established the review branch; it provided no implementation evidence.

Previously only bounded JSON query results were available. The new
`query-export` operation runs the existing authorized query once and encodes
its typed Arrow rows using pinned Apache Parquet 59.2.0 inside the primary
binary. It returns complete bytes, SHA-256, source/read/authorization metadata
and bounded resource evidence. No filesystem destination, remote URL, reader,
ingestion path or alternate query/store authority was added. Stop conditions:
unbounded export, weakened authorization, duplicated authority, unrelated
runtime work or failed candidate CI. Ordinary query behavior is preserved.

Scope: new query codec, public export contracts and engine composition;
HTTP dispatch/capability discovery, Rust client, generated SDKs and digest
citations; focused codec/contract/authorization/real-server/installed-process
tests; README, Gate D and the knowledge connector reference. No implementation
was moved or deleted. Existing Arrow conversion, query policy and read evidence
remain canonical. The installed lifecycle's query/readback characterization
is extended; no existing evidence is replaced by compilation alone.

## Review, research and resource decisions

Owners reviewed: README and AGENTS, alpha objectives, dependency spine,
Gate D/F and the change-authoring procedure; knowledge connectors and relevant
DevPulse and LFG/SWAG requirements. Implementation reviewed: rrd-query manifest,
module boundary, Arrow row conversion, query pipeline/error types; engine
query execution, invocation/action mapping, model value conversion, error
mapping and tenant-policy fixture; contract query types, validation helpers,
endpoint/schema catalogue; HTTP router, handler and capability descriptors;
Rust client query/envelope behavior, local database fixtures and installed
primary-binary lifecycle. New authored modules/tests were read in full;
large existing owners/modules were inspected by relevant sections and diffs.
Generated surfaces are checked mechanically against the one executable owner.

The [Apache writer documentation](https://arrow.apache.org/rust/parquet/arrow/arrow_writer/struct.ArrowWriter.html)
and exact pinned crate source establish buffering and API behavior. No upstream
implementation was copied. The dependency matches the existing Arrow family;
default features are disabled and only Arrow support is enabled. Updating it
requires a reviewed locked rebuild and conformance, not a runtime plugin fetch.
Qdrant multitenancy and SurrealDB security were reference research, not adopted
runtimes or vendor qualification. TurboQuant research is not RRFlow benchmark
proof. This package makes no competitive performance claim.

Source rows, columns, serialized source/metadata, output and row-group sizes
have hard profile bounds. A conservative working-set admission estimate and
sampled Arrow/writer memory check supplement those bounds. These are not a
process allocator/RSS guarantee or Gate F streaming proof. Encoding shares
the query elapsed budget. Existing authorization/read evidence is retained;
resource diagnostics contain counts rather than row contents or credentials.
Failure returns no complete artifact and makes no canonical data write.

## Validation

Initial focused results:

- `cargo test -p rrd-query --test parquet_export`: 3 passed; typed/mixed/nested/null
  round-trip, empty file, row groups, metadata and resource rejection.
- `cargo test -p rrd-contract --test query_export_contract`: 2 passed; closed
  request, limits and query-authorization catalogue binding.
- `cargo test -p rrd-engine compiled_role_policy_injects_tenant_rows_and_fields_before_query_planning --lib`:
  1 passed; tenant/field restrictions, bad token and denied field export.
- `cargo test -p rrd-client --test rrflowdb_local parquet_export_rejects_truncation_and_foreign_engine_sessions`:
  1 passed; actual two-server session isolation and explicit truncation rejection.
- Five SDK generators and `python scripts/ci/check_generated_surfaces.py --write`
  completed: 42 HTTP operations, OpenAPI SHA-256
  `24650c053e1bab1c2eccea8c62973701d7eda995541f7bccdb03c36b59c592cb`.

The first codec compile failed because the pinned release exposes `KeyValue`
under `file::metadata`, not the older `format` path. Correcting the import
resolved it. No fallback or suppressed failure was added.

Final local validation:

- `cargo test --locked -p rrd-query -p rrd-contract -p rrd-engine -p rrd-client -p rrd-server -p rrflow-cli --all-targets --no-fail-fast`:
  383 tests passed, zero failed/ignored across 58 targets. The five installed
  primary-binary tests passed in 207.04 seconds, including fresh/existing
  projects and Parquet decoding before/after restart and restore.
- After the final catalogue-derived Rust route, explicit uncompressed writer
  setting and footer-limit test, `cargo test --locked -p rrd-query --test parquet_export`
  passed 3/3 and `cargo test --locked -p rrd-client --test rrflowdb_local` passed 2/2.
  Final exact-head CI repeats installed lifecycle against that source state.
- `cargo clippy --locked -p rrd-query -p rrd-contract -p rrd-engine -p rrd-client -p rrd-server -p rrflow-cli --all-targets -- -D warnings`:
  passed after the final changes; `cargo fmt --all -- --check` passed.
- TypeScript `pnpm lint`, `pnpm typecheck`, `pnpm test`: passed, 6 tests.
  Go `go test ./...`: passed.
- `python scripts/ci/check_generated_surfaces.py --check`: passed, 42 operations
  and seven signal projections at the digest above.
- `python scripts/ci/check_documentation.py`, `python scripts/ci/check_workflow.py`,
  `python scripts/knowledge/render_navigation.py` and execution inventory
  generation/check passed. `python scripts/ci/test_execution_inventory.py`
  passed 4 tests; `python -m unittest scripts.knowledge.test_navigation scripts.knowledge.test_export`
  passed 17 tests. Git diff whitespace check passed after normalization.

The initial broad suite exposed the shared SDK corpus's stale 41-operation
expectation. The canonical fixture and affected language assertions now
expect 42; its focused contract oracle and the complete rerun passed. An
attempt to use Cargo's build-only `--keep-going` flag was rejected before tests;
the rerun used `--no-fail-fast`. Windows generation emitted CRLF in the Python
projection; LF normalization resolved the diff check without semantic edits.

Not run locally: Linux/macOS native execution, the complete workspace's
unaffected suites, pinned Python/Java/.NET SDK qualification and optional
feature/real PostgreSQL lanes; these run in candidate CI. No signed release,
clean-machine installer, cloud service, SWAG endpoint, third-party knowledge
service or benchmark was qualified. The empty opening candidate was canceled
and cannot be used as feature evidence.

## Installation clarification

During the package, the owner clarified few-step Windows/macOS/Linux setup
and Windows-first DevPulse bundling with guided optional Docker provisioning.
The PR brief was amended before updating README and the existing DevPulse
integration owner. It records host-runtime detection, explicit authorization,
restart/resume, verified readiness and clean-machine acceptance; it does not
claim an implemented DevPulse installer. Docker and Microsoft primary guidance
is linked in that owner. The intended DevPulse application checkout remains
to be identified; no other application's source was modified.

## Checklist and remaining boundaries

The pre-implementation brief binds outcome, baseline, scope, first oracle,
research, trace/resource decisions and stop conditions. New and changed paths
are derived by Git and the execution inventory. Navigation, changed source/test review,
complete diff review and the local package checks above passed before commit.
Candidate CI remains the required remote merge gate.

No roadmap completion box or POA&M gap is closed. Gate D explicitly brings
forward only this bounded native-query export prerequisite. Full Parquet import,
source binding, compression/decompression and streaming remain D/F work;
Connectome's export control remains H-06; cloud provisioning and bidirectional
sync remain H-07; SWAG execution remains the inference gates. Existing gaps
already own those missing capabilities; no new deficiency was found by the
focused tests. This receipt is not a turnkey installer or complete-product claim.

The PR's final exact-head CI and merge records are the remote effect evidence.
Neither its empty opening candidate nor a superseded run qualifies this change.
