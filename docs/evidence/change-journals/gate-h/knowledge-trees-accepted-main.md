# Knowledge trees: current-engine integration and bounded admission

**Status:** active integration evidence; candidate and UI qualification pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/knowledge-trees-accepted-main`
**Owner:** OBJ-07 / H-06 prerequisite / D-06 / PR16 / POAM-037

## Baseline and scope

PR16 head `5871e80f8d9e52c45165e23cfea4c9c660f3f817` contained unlanded
knowledge-tree contracts, engine methods, HTTP handlers and tests. Accepted main
`f3c04e955fd8851c0fc466c45f008975e9d4c1a1` did not contain those implementations.
The PR brief bound ordinary main integration before edits. This package keeps
that behavior at its original engine/public-contract boundaries while adopting
current transaction admission, anchored installation and schema generation.
The version remains 1.0.0; alpha and C-07 status do not advance.

The first compile exposed a retired `transaction_gate.lock()` call. Knowledge
replacement now obtains `admit_transaction(request, request_id, operation_id)`
before reading/replacing the catalogue and receipt. Existing count, byte,
deadline and cancellation admission therefore applies to its serialized durable
effect. The added saturation test proves a denied write persists no knowledge
catalogue or receipt; releasing capacity permits the same request and exact
replay. This is the existing gate, not another lock or admission authority.

The embedded fixture now constructs the installation container through
`rrd_store::filesystem::AnchoredPath`. The real HTTP fixture supplies an explicit
installation identity and uses the current engine-clock apply signature. Its
authenticated denial, CAS, replay and restart assertions remain unchanged.

Strict Clippy identified the 328-byte external binding beside the 48-byte native
variant. Boxing the external `EndpointBinding` reduces inline enum storage;
Serde/JSON Schema remain transparent. The OpenAPI digest remains
`ddbcfd660f2c5d708396229a1ebe176ca7dc8343bbe47f4ad3d3a734194b4c65` after that
representation change. No throughput or measured process-memory claim is made.

## Conflict and implementation traceability

The canonical destinations remain `rrd-contract::knowledge_tree`,
`RrdEngine` knowledge methods/control-journal CAS, the existing invocation and
HTTP catalogues, and typed `rrd-client` operations. Their characterization
tests move with the original behavior. All five SDK projections are regenerated
from the merged catalogue: 46 HTTP operations and 53 security actions. The
kernel signal fingerprint is unchanged. Authored count/digest assertions are
updated to include the two new operations/actions, not removed.

Current main's README, AGENTS, HTTPS submodule policy, workspace architecture
tests, inventory tooling and native workflow are preserved. The older fixed
Connectome-branch restriction is not restored. Existing main tests characterize
pinned public source and optional UI exclusion; four GitlinkInventoryTests and
31 workspace architecture tests passed. Generated conflicts are rebuilt from
their owners rather than hand-merged. Existing immutable journals remain intact.

The old branch and main independently assigned POAM-032. Main's prompt-flight
record is retained exactly; the unlanded provider/sync gap moves to unused
POAM-037 with explicit lineage. The obsolete inference-extension document is a
historical pointer to current LFG/SWAG and learning owners. Knowledge-tree
configuration links the advanced connector contract without a second execution
authority. Current Connectome pin `03b716b4` is preserved; PR114 owns its update.

Inspection also found the old authored knowledge editor/native bridge/tests at
Connectome `8a9eb5e5` absent from both current pin and PR114's `38142b77`. Child
PR10 is closed without merge, with its original source branch still retained.
POAM-037 records the missing adaptation into current native sessions/project
workspaces. This package does not claim UI absorption, editor availability or
external provider execution. Neither old UI source nor its tests were deleted.

## Review, failures and checks

Reviewed owner procedure, current knowledge connector/client contracts, original
knowledge journal, complete knowledge engine/contract methods, their test cases,
current transaction admission, installation signatures, invocation/HTTP/client
deltas, generator sources and every changed conflict. Full diff review preserves
current main's policy/runtime behavior. No new external research or runtime trace
is needed for this integration; the existing bounded audit/receipt paths remain.
No external provider, original user estate or private cloud account is invoked.

Initial failures were the retired mutex API, PathBuf versus AnchoredPath,
incorrect direct `rrd_lsm` import (corrected to the store's exported boundary),
changed installation signatures, a stale 51-action assertion and the large-enum
Clippy warning. Documentation checks required a historical supersession link and
parent-index entry; both were supplied. A read-only console print needed explicit
UTF-8. These failures are retained as integration findings, not reported as passes.

The 13 knowledge contract tests, new admission test, knowledge engine cases and
real authenticated HTTP restart test passed. The owning client all-target suite
passed; contract/engine/server all-target suites passed after correcting the
action count. Post-boxing focused contract/OpenAPI tests preserve the exact
wire digest. Final strict all-target Clippy passed for contract/engine/server/
client; 15 filtered engine cases and the real HTTP restart test passed again. TypeScript lint/typecheck and six tests passed; Go tests passed.
All five generated endpoint projections and seven signal projections passed the
canonical parity check. The default installation projection refresher produced
no change relative to main.

Exact acceptance commands: `cargo test --locked -p rrd-contract knowledge_tree`,
`cargo test --locked -p rrd-engine --lib knowledge`,
`cargo test --locked -p rrd-client --test knowledge_trees`, and the owning
`cargo test --locked -p rrd-contract -p rrd-engine -p rrd-server --all-targets`.
The initial four-package all-target command completed all client targets before
finding the documented contract count error; the corrected three-package run
completed the remaining packages. `cargo clippy --locked -p rrd-contract
-p rrd-engine -p rrd-server -p rrd-client --all-targets -- -D warnings` passed.
`python scripts/ci/check_generated_surfaces.py --check`, TypeScript
`pnpm lint`, `pnpm typecheck`, `pnpm test`, and Go `go test ./...` passed.

The local Python SDK environment could not sync because Python 3.15 is absent.
Its generator ran with Python 3.11 and the declared Ruff 0.17.1 without changing
project requirements; full Python, Java and .NET SDK tests remain for candidate
CI. Native Connectome GUI, live provider integrations, full capacity and soak
were not run by this package. Final navigation/inventory/documentation/workflow
and whitespace checks passed after this final journal text.

Checklist: pre-edit and refined briefs, exact input revisions, mapped reusable
behavior and current-policy destinations, first compile failures corrected at
canonical boundaries, admission denial/retry proof, real HTTP restart evidence,
generated public projections, explicit UI/toolchain gaps and immutable record.
Candidate CI and normal merge-queue checks remain required before landing.
