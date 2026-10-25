# Endpoint profiles and task controls: public-contract integration

**Status:** active integration evidence; combined candidate qualification pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/endpoint-profile-task-control-integration`
**Owner:** D-06 / POAM-006 and POAM-017 / PR113, dependent on PR105

Endpoint head `25a123e1bcadab0c63491a246a7598e16ec428b0` integrates task-control
head `e9d5d20f366994cf70e97af64d068fe87d4f92e3` by ordinary merge. The PR brief
bound this overlap before effects. Both already include native candidate
`d0e7f54c`; PR105 owns the incoming task controls and must land before PR113.
The [native integration](endpoint-profile-native-integration.md) retains its
original checks; this record qualifies the combined public contract separately.

The public catalogue now contains 53 HTTP operations: both branches' new
operations remain present. The only implementation conflict was the HTTP
handler module declaration; preserve both endpoint_configuration and
reasoning_budget modules. Engine composition, router dispatch, typed client
methods, request/response schema mappings and their tests merged with both
behaviors. No task or endpoint implementation is moved, deleted or replaced.
Existing engine authentication, storage admission, control CAS and durable
receipts remain the authority.

Digest/count, current reference, synthetic storage-fixture and generated SDK
conflicts are rebuilt from the combined catalogue. Explicit endpoint-count
oracles become 53, not removed. The Rust reference retains both endpoint and
task-flow sections (48 typed operations, five previously unsupported HTTP
operations). The combined OpenAPI SHA256 is
`71921d69bf89de1530532adb98d5d3d1c5784c78726434facf41173d312c800b`.
Synthetic fixture refresh is not a native workload or a rewritten historical
campaign. Original journals and retained native evidence stay unchanged.

Reviewed every conflict hunk, the combined engine/contract/handler/router/client
diffs, Rust SDK reference, canonical surface generators and complete synthetic
storage refresher. The first failure oracle requires both actual HTTP flows:
task execution/Stop/replay after reopen and endpoint configure/restart/revoke.
No new upstream algorithm, provider activation or runtime trace is required;
this package preserves existing semantics rather than adding another controller.
Unexpected behavior loss, weakened limits or unmapped source changes stop work.

## Validation

Both focused real HTTP flows passed. The five-package all-target command
(`cargo test --locked -p rrd-contract -p rrd-client -p rrd-server -p rrd-engine
-p rrflow-eval --all-targets`) passed client, contract, engine and server
suites; engine library had 192 passes and two existing child/privileged ignores.
It then exposed a storage golden failure under unified serde_json features:
parsed schemas were identical (36,608 bytes each), but preserve_order changed
object-key rendering. The standalone refresher had used sorted-map rendering.

Before repair, the PR brief explicitly added the existing evaluation test's
`golden` helper and synthetic refresher's `write` helper. Both now call
`Value::sort_all_objects` before rendering. The exact byte assertion remains;
array ordering, values, runtime hashing and historical native evidence are
unchanged. The original failing test then passed under the same five-package
feature unification. `cargo test --locked -p rrflow-eval --all-targets` then passed (41 library,
three provider compatibility and 11 storage tests). Strict all-target Clippy
passed for contract/client/server/engine/inference/evaluation; Rust formatting
passed. Other package suites are not repeated after the evaluation-only repair.
Final generated navigation/inventory, documentation/workflow and whitespace
checks are required before commit.

All five SDK endpoint projections and seven signal projections regenerated
and passed canonical parity. TypeScript lint/typecheck and six tests passed;
Go `go test ./...` passed. The OpenAPI export contains all six task-control
operations and all three endpoint-configuration operations. Eight task-control
implementation/HTTP oracle files and all five endpoint-specific modules/tests
were checked byte-for-byte against their owning input revisions.

## Limits and checklist

No current native sustained campaign, external-provider operation, Connectome
editor or 24-hour soak is claimed. C-07/POAM-035's descriptor exhaustion remains
open. Python 3.15 is unavailable locally; the SDK generator uses installed
Python 3.11 with declared Ruff 0.17.1, without changing package requirements.
Full Python, Java and .NET execution remains for candidate CI.

Checklist: pre-effect dependency brief, exact input revisions, preserved
canonical source/tests, generator-owned schema/SDK/fixture conflict resolution,
meaningful combined HTTP oracles, linked journal, navigation/inventory
regeneration and final diff review. Candidate and normal merge-queue checks
remain required; no roadmap, POA&M completion or version status advances.
