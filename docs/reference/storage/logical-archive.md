# RRFlow logical archive and application backup

**Status:** active implementation reference; resumable logical recovery exists, but this is not the rrflowKV physical format or the live reasoning/recall path
**Coordinate:** `rrflow://rrflow-instance/data/reference/storage/logical-archive`
**Owner:** logical archive cut, replay, application-backup closure, and explicit recovery exclusions

RRFlow uses a logical archive to recover authoritative semantic operations across
physical storage-format changes. The archive is not a copy of the rrflowKV WAL,
manifest, immutable segments, Arrow-compatible pages, or index artifacts. It is
also not an alternate database: a restore publishes a new rrflowKV root, after
which ordinary access returns to `RrdEngine`.

Export accepts the common `StorageEngine` read contract. Restore is deliberately
narrower: the current implementation can publish only a new `RrflowKvStore`
root. The recovery functions are not exposed as ordinary mutation operations
through HTTP, WebSocket, MCP, or an SDK.

## Stable logical cut

An export captures three source coordinates:

- the append-ordered claim sequence;
- the typed-runtime commit cursor; and
- the runtime audit-chain head.

It validates the cut once, streams the same cut, then verifies that the source
watermarks did not advance. Claims and runtime changes are read in fixed
1,024-entry pages. The exporter retains at most one page and one encoded action;
one action is bounded to 64 MiB. It does not reconstruct the complete source
log in memory.

The validator checks runtime cursor continuity, mutation ordinals, commit and
change digests, previous-change links, exact audit envelopes, previous-audit
links, and the final audit head. Claim mutations in a commit must correspond to
one contiguous interval in the append-ordered claim log. Standalone claim
appends are emitted in their original order around commit-owned claims.

If the source advances, no requested archive is published. An interrupted
private export remains bound to its original coordinates. Resume parses and
authenticates the durable prefix, compares every retained action with the fixed
source cut, and reconciles the crash window in which an action reached storage
before its receipt. A stale cut is removed rather than combined with a newer
observation.

## Current framed format

The implemented format is a binary frame stream containing:

1. the exact current `RRDLAR01` magic, logical-archive version, internal
   contract version, and source watermarks;
2. length-delimited JSON actions, each either a standalone claim or a complete
   runtime commit with its accepted audit envelope; and
3. a footer containing action and mutation counts plus SHA-256 over every
   preceding byte.

The current `.rrd-archive` suffix, `RRDLAR01` magic, and internal `rrd` receipt
names are implementation facts, not accepted RRFlow 1.0 naming or a
compatibility promise. A-07 and J-01 must converge or replace those identities
directly; RRFlow will not retain an alias reader or migration shim merely to
preserve this pre-release spelling.

The digest detects changed bytes. It does not authenticate the producer,
authorize creation or restore, encrypt content, manage keys, or prove
confidentiality. Those properties require an explicitly authorized and signed
release/operations design and must not be inferred from the code's older use of
the word "authenticated."

## Restore and publication

Restore requires an absent destination and validates the complete archive
before applying an action. It replays into an archive-addressed private sibling
root and records a checksummed progress receipt. On resume, it compares the
receipt with authoritative staging watermarks and verifies every already
applied claim, commit outcome, and audit envelope before continuing.

Standalone claims use the storage append operation. Runtime commits use a
crate-private recovery path that performs the normal schema, reference, cursor,
change-chain, projection-outbox, and idempotent-outcome planning, then requires
the reconstructed audit envelope to equal the archived envelope. This path
cannot be invoked by a client as a way around `RrdEngine` authorization.

After replay, restore verifies the claim, runtime, and audit heads; flushes;
closes; reopens; verifies again; applies an optional catalogue/object closure;
then atomically publishes the staging directory. Corruption or divergent
staging state leaves the requested target absent.

## Exact coverage

| State family | Logical archive | Application backup | Restore behavior |
|---|---|---|---|
| Claims and typed runtime mutations | Included | Included | Replayed with original ordering and commit boundaries |
| Schema, records, relations, events, vectors, series, and geo values | Included as runtime mutations | Included | Replayed into rrflowKV |
| Runtime audit envelopes and outcomes | Included | Included | Verified against reconstructed commits and cursors |
| Immutable-object references | Included | Included | References are replayed with runtime state |
| Immutable-object payload bytes | Referenced only | Included through a content-addressed manifest | Verified and copied to the restored `immutable` store before publication |
| Vector-collection and index catalogue records | Excluded | Included from their two current control-journal keyspaces | Values and per-scope catalogue revisions are reconstructed before publication |
| Estate control state (installed record and checkpoint, active configuration, security policy and audit chain, attunement plan and job, estate documents) | Excluded | Included as the ordered control-journal entries of the engine-declared families, in a content-addressed `RRDCJA01` control-journal archive | Replayed one transition per entry after the catalogues and objects, each committed with a replay-progress marker; verified contiguous and field-identical before publication |
| Session leases, subscriptions, backup/restore and estate-recovery receipts | Excluded | Excluded | No recovery claim; they belong to the instance that performed the operation |
| Token key and operator credential | Excluded | Excluded | Files outside the store; publication carries the token key, never a backup |
| Scalar, BM25, vector, HNSW, and analytical projections | Not retained | Not retained | Must be rebuilt from authoritative state |
| Invocation telemetry | Excluded | Excluded | No recovery claim |
| Snapshot leases | Excluded | Excluded | No recovery claim |
| rrflowKV WAL, manifest, row segments, or future Arrow pages | Excluded | Excluded | A fresh physical rrflowKV representation is produced by replay |

"Application backup" is the current API term for the bounded closure in this
table. It does not mean every future application or control-state family is
implicitly covered. Adding a family requires an explicit coverage value,
bounded manifest representation, corrupt-input denial, and restore test.

The backup catalogue is local JSON at `catalogue.json`, format version 2; a
version 1 catalogue, whose application-complete entries carried no control
state, is refused rather than read as complete. Archives, object manifests,
catalogue manifests, control-journal archives, and payloads are
content-addressed beneath the catalogue root. Entries bind caller-supplied label/time to component digests,
are canonically ordered, and are published through a synced temporary file and
rename. Pruning requires a complete retained/pruned partition bound to the
current catalogue digest; replay is idempotent, and shared artifacts remain
until no retained entry references them.

## Relationship to persistent reasoning and recall

Logical recovery protects the authoritative mutations from which reasoning and
recall structures can be rebuilt. It does not implement their online access
path. In particular:

- it does not make the current row-segment rrflowKV format Arrow-native;
- it does not expose a stamped `TableProvider` or stream rrflowKV pages into
  rrflowQL/DataFusion;
- it does not provide native adjacency, BM25 posting, vector, HNSW, or RRF
  reads; and
- it does not persist or advance a reasoning tree by itself.

Those dependencies remain ordered: C-01 through C-05 establish one
transactional key/version and multi-model access spine; C-06 and C-07 establish
Arrow-compatible immutable pages and safe recovery/lifetimes; E establishes
native graph and recall indexes; F binds stamped streams into DataFusion; and G
and H persist and deliver governed reasoning/context decisions. Projections
remain rebuildable derivatives, while their source mutations and rebuild work
coordinates must commit atomically through `RrdEngine` and rrflowKV.

## Executable evidence

The focused `rrd-store` suites cover:

- exact claim/runtime/audit coordinates after close and reopen;
- 2,100 claims across source pages and one 1,100-mutation atomic commit across
  runtime pages;
- every current typed runtime family, including vector, series, geo, and
  immutable-object references;
- interruption after action, receipt, and finalized-footer durability;
- receipt tampering, source advancement, truncation, corrupt archives, and
  existing-target denial;
- logical-only versus application-backup coverage;
- immutable payload and catalogue closure restoration; and
- digest-bound, partition-complete catalogue pruning with shared-artifact
  retention.

The implemented CLI exposes logical-only archive and catalogue operations:

```text
rrflow --db SOURCE storage archive-export --archive FILE
rrflow --db SOURCE storage archive-inspect --archive FILE
rrflow --db ABSENT_TARGET storage archive-restore --archive FILE
rrflow --db SOURCE storage backup-create --catalogue DIR --label LABEL
rrflow --db SOURCE storage backup-list --catalogue DIR
rrflow --db ABSENT_TARGET storage backup-restore --catalogue DIR --backup-id ID
```

`backup-create` currently calls `create_logical_backup`; it does not create the
application payload/catalogue closure. Application-backup creation exists only
as a Rust storage API and is not yet a complete authorized operator workflow.
Gate J must rehearse the accepted backup and restore path from a release
candidate before it can qualify RRFlow 1.0.
