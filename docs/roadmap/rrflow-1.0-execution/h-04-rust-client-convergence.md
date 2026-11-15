# H-04 Rust client convergence

**Status:** active package design; it changes no gate or POA&M status until its acceptance evidence passes
**Coordinate:** `rrflow://rrflow-instance/data/execution-design/h-04-rust-client-convergence`
**Owner:** the design for making every catalogue operation callable through one generated, validated Rust client path, as part of [H-04](../rrflow-1.0/gate-h.md)

Owner direction, 2026-11-14: "we are in pre-release and need to get this fully planned out and then implemented". One current implementation; no workaround through raw HTTP.

## Current behavior

Surveyed on `development/main` at `a4e32942`:

- **Coverage.** The endpoint catalogue has 83 HTTP operations. `rrd-client/build.rs` generates a marker for each.
  - Only 9 can be called through `call_operation`, because each needs a hand-written `ValidatedOperation` impl.
  - 72 have hand-written typed methods.
  - 4 cannot be called from Rust at all: `health-live`, `query-index-ensure`, `query-index-list` and `query-live-poll`.
- **Three dispatch paths:** `session_call`, `generated_attunement_call` and `call_operation`. Some methods also look paths up from the catalogue at runtime.
- **Retry disagrees.** `session_call` retries every call, mutations included (`retry_safe: true`, default 2 attempts). `call_operation` never retries.
- **Validation is inconsistent.**
  - Validators are inherent methods with different names and signatures.
  - 28 response types have none.
  - About 19 of the 72 typed methods call `request.validate()`, and some skip response validators that exist (`begin_transaction`, `commit_transaction`).
- **Shared response types.** Validation cannot be keyed by type, because several types serve more than one operation:
  - `AttunementControlResponse` serves five operations;
  - `TaskBudgetView` four;
  - `ObservationDeliverySnapshot` two;
  - `SessionLease` two;
  - `ReadAttunementReceipt` is the request for two operations.

## Target design

1. **The contract owns operation markers and their validation.**
   - Today each catalogue row names its request and response types as strings. Each row becomes one typed declaration, `operations! { query_index_ensure: QueryIndexEnsure = "query-index-ensure", Post "/v1/query/indexes/ensure", SessionBearer, mutation, QueryIndexEnsure action, EnsureQueryIndex => EnsureQueryIndexResult, keyed_mutation; … }`.
   - The macro emits both the `EndpointDescriptor`, with type names from `stringify!`, and a marker type whose associated `Request` and `Response` are the real types. A wrong type name becomes a compile error instead of a string mismatch.
   - The client and, later, the server's dispatch use these markers, so no second registry exists. (A build script cannot call the crate it is building, so the markers cannot be generated from `endpoint_catalogue()` at build time.)
   - The contract hand-writes one `OperationContract` impl per marker, beside its family's types:
     - request and response type;
     - `Context`: instance, principal and, for plan-bound attunement, the plan;
     - `validate_request`;
     - `validate_response(request, response, context)`.
   - The impls adapt the existing inherent validators. The 28 responses without one get real structural validators: bounds, echoed identity, and digest shape.
   - A generated assertion requires an impl for every marker, so a missing validator fails to compile.
2. **The catalogue declares retry semantics.** `EndpointDescriptor` gains `retry: EndpointRetry`, with retry happening at one layer only (the client):
   - `read`: retried;
   - `keyed_mutation`: retried with the same idempotency key, which the engine already deduplicates and replays;
   - `external_effect`: never retried. These are model and worker dispatch, assessment and embedding execution. The caller reads the attempt.

   This follows AWS's "Making retries safe with idempotent APIs" and Stripe's idempotent-request guidance, and it satisfies H-04's "one explicit operation-semantic retry, cancellation, and uncertain-outcome policy".
3. **The client has one call path.**
   - `call_operation::<O>` handles every method, typed path parameters (`{session}`, `{transaction}`, `{engram}`) and the declared retry class.
   - `session_call`, `generated_attunement_call` and the runtime catalogue lookups are removed.
   - The typed methods stay as one-line wrappers, because the CLI, MCP server and Lab call them.
4. **Special cases are explicit.** A test pins an allow-list of `session-create`, `session-renew`, the five public GETs and the WebSocket connect. The test requires generated callable operations plus allow-listed ones to equal the catalogue exactly: nothing missing, nothing extra.

## Acceptance

- Every catalogue operation is callable from Rust exactly once, through `call_operation` or the pinned allow-list.
- The `ValidatedOperation`-equivalent coverage test equals the catalogue.
- Every new response validator has a unit test that rejects a malformed response.
- The compile-fail doctest proves an operation without a contract impl cannot be called.
- Existing suites pass unchanged: `rrd-client` (including `real_server`, `generated_catalogue` and `sdk_conformance`), and the callers in `rrflow-cli`, `rrflow-mcp` and the Lab.
- The generated OpenAPI and SDK projections are regenerated and `--check` clean, since the descriptor gains `retry`.

## Progress

- **Landed, 2026-11-14:** the typed catalogue rows, the `rrd_contract::operations` markers, and the retry classes with their evidence ([journal](../../evidence/change-journals/gate-h/h-04-typed-catalogue-retry-classes-20261114.md)).
- **Open:**
  - the `OperationContract` validation impls;
  - validators for the 38 request and response types that have none;
  - the client's single call path;
  - retry by class.

  The owner put the prototype first on 2026-11-14 (Connectome reaches every operation through the TypeScript SDK), so these wait behind the development loop and the second brain.

## Not in this package

These are recorded so they stay open under H-04:

- The shared conformance corpus covers about 21 of the 83 operations; extending it to all of them across the six SDKs comes next.
- The server's hand-written `http_dispatch_catalogue!` is not yet generated from the same markers.
- The TypeScript and Python SDKs validate envelopes only, not request and response semantics.
