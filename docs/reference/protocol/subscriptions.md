# RRD durable subscriptions

**Status:** active implementation reference; B-04 multiplexing is implemented and commit-impact delivery remains incomplete
**Coordinate:** `rrflow://rrflow-instance/data/reference/protocol/subscriptions`
**Owner:** durable changefeed and live-query subscription state, replay, acknowledgement, and current WebSocket delivery

RRD exposes changefeeds and semantic rrflowQL deltas through one durable
subscription authority composed by `RrdEngine`. A subscription does not own a
second event log, query executor, transaction coordinator, or client
lifecycle. Its delivery coordinates refer to the authoritative runtime commit
cursor.

The [server reference](server.md) owns HTTP/WebSocket process and transport
profiles. The [live-query reference](../query/live-query.md) owns current
semantic-delta computation. Roadmap
[B-04](../../roadmap/rrflow-1.0.md#gate-b--freeze-public-and-model-neutral-contracts)
owns the implemented multiplexed WebSocket protocol, and
[H-03](../../roadmap/rrflow-1.0.md#gate-h--prove-context-flow-feedback-live-delivery-and-connectome)
owns commit-impact live-query evaluation. This record describes implemented
subscription behavior without claiming H-03 complete.

## Public operations and stream definition

The generated endpoint catalogue defines two durable-subscription
administration operations and one generic WebSocket connection operation:

- `POST /v1/subscriptions/open` creates one immutable `changefeed` or
  `live_query` stream definition plus its delivery policy.
- `POST /v1/subscriptions/close` performs an idempotent administrative close.
- `GET /v1/ws` upgrades one authenticated session to a multiplexed RRFlow
  WebSocket. A client attaches any permitted durable stream with an exact
  `SubscriptionResume`; socket unsubscribe detaches it, while HTTP close is
  the only operation that closes the durable subscription.

An open request binds the subscription identity, stream and scope, starting
cursor, batch size, maximum in-flight deliveries, retention-cursor window,
lease duration, and heartbeat interval. A live-query stream additionally binds
the rrflowQL string, typed parameters, query budget, and maximum semantic-delta
rows. Contract validation rejects unknown fields and values outside the
declared bounds; `max_in_flight` is currently limited to 1 through 64.

The stream digest is stable for the accepted definition. Reusing the open
idempotency key with the same request returns the existing subscription;
changing the request conflicts. There is no update operation that silently
changes what an existing subscription means.

## Durable state, ordering, and replay

The runtime commit cursor is the only data-delivery order. Changefeed pages
retain the underlying change identity, commit identity and ordinal, previous
change digest, scope, actor, and event time. Live-query frames cover the range
from the durable acknowledged cursor through one captured runtime head.

Current subscription state includes:

- the owning session and immutable stream definition;
- the durable acknowledged cursor and configured retention window;
- the active connection generation and lease expiry;
- the next delivery sequence; and
- the bounded outstanding-delivery intervals for the active generation.

The engine currently encodes this pre-release state as a versioned JSON control
record and replaces it with compare-and-swap while appending a digest-chained
control-journal entry. That physical representation is implementation
inventory, not a public persistence format. A delivery interval is committed
before its frame is returned for transport.

An ACK must identify an exact outstanding delivery by connection generation,
delivery sequence, and through-cursor. It advances the durable cursor to that
delivery and cumulatively removes earlier outstanding deliveries. A dropped
connection does not acknowledge data. Reconnect increments the fencing
generation, clears the abandoned in-flight window, and resumes from the last
durable ACK; a frame from the replaced generation fails closed. The tested
rrflowKV path preserves this behavior across engine restart.

The configured retention floor is `head - retention_cursor_window`. Open,
connect, and delivery reject a resume cursor below that floor or beyond the
head. This is a logical replay bound, not evidence that runtime history has
been physically pruned. ACK and client heartbeat frames renew the durable
lease. An expired or closed subscription cannot reconnect or deliver.

## Multiplexing, backpressure, and WebSocket frames

The outstanding window cannot exceed `max_in_flight`. Once full,
`RrdEngine` returns subscription backpressure; the server stops requesting new
delivery while the same socket remains able to receive ACK, heartbeat, or
close frames. Changefeed `batch_size` uses the bounded changefeed page
contract. Live queries use their separate execution and delta-row budgets.

Every JSON text application message is one closed `WebSocketFrame` with the
`rrd` protocol identity, version, connection ID, contiguous direction-local
sequence, and exactly one typed payload. Clients may send `request`, `cancel`,
`subscribe`, `ack`, `heartbeat`, and `unsubscribe`. Servers may send
`connected`, `response`, `cancellation`, `subscribed`, `delivery`,
`acknowledged`, `heartbeat`, `unsubscribed`, `error`, and `backpressure`.
Unknown envelope or payload fields, wrong-direction variants, repeated
`connected`, foreign connections, sequence gaps, malformed JSON, and binary
application messages fail closed.

`WebSocketLimits` negotiates message, frame, read-buffer, write-buffer,
maximum-write-buffer, in-flight-request, subscription, send-timeout,
receive-timeout, and heartbeat bounds. The default maximum application
message and frame are one MiB; hard maxima admit at most 128 requests and 64
subscriptions, while the defaults are 64 and 32. Client and server configure
their WebSocket libraries from validated admission limits before the upgrade
or first application allocation. The server's `connected` frame may only
narrow those admitted limits, including timeouts and heartbeat interval, and
the client adopts the narrower values. Heartbeats may not be configured below
100 milliseconds, preventing a peer from negotiating an unbounded liveness
loop.
Application compression is not enabled.

Delivery is currently checked on a bounded polling cadence rather than
awakened directly by each commit. One socket can carry multiple independently
fenced subscriptions and continues to process ACK, heartbeat, unsubscribe,
and cancellation traffic while any stream is backpressured.
The socket handles inbound frames and delivery polls in one ordered loop, so
a delivery sent on a poll before the server reads an `unsubscribe` can reach
the client ahead of `unsubscribed`. That delivery continues the acknowledged
stream and stays within the subscription's `max_in_flight`. The server removes
the subscription before it answers, so no delivery for it follows
`unsubscribed`; a client waiting for `unsubscribed` must accept those earlier
deliveries rather than treat them as a protocol error.
`changes/follow` and live-query polling also remain callable as separate
bounded HTTP surfaces; H-03 owns their convergence with commit-impact
delivery.

B-04 freezes generic request and cancellation shapes without claiming H-04
dispatch. The server currently returns an exactly correlated
`failed_precondition` error for a request and an exactly correlated terminal
`not_found` cancellation result. H-04 binds every catalogued semantic
operation and real server cancellation through the shared dispatcher; a
socket close still cannot manufacture engine completion.

## Authentication and authorization

Opening, attaching, acknowledging, and closing use distinct deny-by-default
security actions. The generic upgrade requires `websocket_connect`; attaching
a durable stream separately requires `subscription_connect`. Open, attach,
delivery, ACK, and stream close also enforce the underlying
`changefeed_follow` or `query_live_poll` permission. The stream scope must
equal the configured instance, and current ownership is bound to the exact
session rather than merely its principal. A real-server denial test proves a
principal can establish the generic socket but cannot attach a subscription
without the separate grant.

The WebSocket upgrade uses the same session and bearer headers as ordinary RRD
requests. Loopback uses `ws://`. The real-process Rust client test proves that
the configured remote profile requires a client certificate, rejects a wrong
server name, and carries subscription delivery over `wss://` when the mutual
TLS identities and trust roots are valid. Certificate rotation, revocation,
external identity integration, and a complete untrusted-client-chain matrix
remain unqualified.

## Current semantic live-query cost

The delivery layer does not make live-query computation incremental.
`poll_live_query` currently captures a head, executes the same bound query at
the acknowledged and head cursors, materializes both result sets by stable row
identity, and computes deterministic added, updated, and removed rows. It
fails closed on fixed `KNOWN` queries, cursors beyond head, truncation,
duplicate identities, or delta-budget excess.

H-03 must replace this two-snapshot work with commit-impact evaluation over the
native graph, scalar, BM25, and vector access paths, while retaining cursor
ordering, exact predicate semantics, authorization, bounded resources, ACK
replay, and restart behavior.

## Characterization evidence and open qualification

| Boundary | Current evidence | Not established |
|---|---|---|
| Public contract | Seven B-04 codec/golden cases cover the closed envelope, directions, contiguous sequence, foreign connections, request/response/cancel correlation, exact resume coordinates, negotiated and hard limits, JSON bounds, and unknown-field rejection. | Cross-language carriage and H-04 operation dispatch. |
| Engine | Six focused tests cover durable open/reopen, exact resume validation, idempotent open, bounded delivery, cumulative ACK, generation fencing, retention, ownership, dual authorization, one global cursor chain, semantic live delta, and close. | Commit-triggered wakeup, principal-level transfer, or H-03 impact evaluation. |
| Query | Two tests prove deterministic semantic deltas and failure behavior across rrflowMX and rrflowKV. | Direct stamped access paths, incremental result maintenance, or bounded cost independent of full snapshots. |
| Rust client/server | Three real-process cases cover a generic authenticated upgrade, independent subscription authorization, two-stream multiplexing, ACK, detach, disconnect/reconnect replay, exact generation fencing, correlated fail-closed request/cancellation results, close, audit actions, and mutual-TLS WSS. Three malicious-server tests cover six foreign-connection, sequence-gap, unknown-field, binary, oversized-message, and mismatched-response faults. | H-04 operation execution/cancellation, generated-SDK parity, H-03 commit-impact delivery, or release deployment qualification. |

The focused characterization commands are:

```text
cargo test -p rrd-contract --test websocket_contract --locked
cargo test -p rrd-engine subscription --locked
cargo test -p rrd-query --test live_query --locked
cargo test -p rrd-client --test real_server --locked
cargo test -p rrd-client --test transport_faults --locked
```

## Implementation anchors

- Public stream contract: `crates/transport/rrd-contract/src/lib.rs`
- Multiplexed frame contract: `crates/transport/rrd-contract/src/websocket.rs`
- Durable subscription authority: `crates/authority/rrd-engine/src/engine/subscription.rs`
- Current semantic delta computation: `crates/compute/rrd-query/src/live.rs`
- WebSocket adapter: `crates/transport/rrd-server/src/http/websocket.rs`
- Supported Rust client: `crates/transport/rrd-client`
