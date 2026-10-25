# Knowledge trees with endpoint profiles and task controls

**Status:** active integration evidence; combined candidate qualification pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/knowledge-trees-public-api-integration`
**Owner:** OBJ-07 / H-06 prerequisite / D-06 / POAM-037 / PR16

Starting head `bf70261c91160ae90d3735a72ee1b55d1899c3c3` integrates endpoint
head `1e7b7f01c34a1cfde2f5fc02d2c2459561628567` by ordinary merge, after the
PR description bound the combined public API scope. PR113 already includes
PR105 task controls and PR129 shared native changes. Landing order remains
PR129, PR105, PR113, then PR16; this does not transfer those package owners.
The [prior native integration](knowledge-trees-native-integration.md) retains
its own revision and evidence.

No runtime implementation conflict occurred. Existing engine composition,
invocation/HTTP dispatch and typed client additions retain all three features.
Knowledge authority, contract and both engine/HTTP fixtures were verified
byte-for-byte against bf70261c. The incoming deterministic evaluation renderer
repair stays at its existing test/refresher boundary. No provider, persistence
authority, runtime dependency, source deletion or alternate controller is added.

The canonical catalogue exports 55 HTTP operations, including two knowledge,
three endpoint-configuration and six task-control operations. All remain in the
OpenAPI document, SHA256
`a36ace4e6572c85913cee78d147555688ba2d87684cc45cf7d19f08de536fa07`.
Five SDK endpoint projections and seven signal projections regenerate from
that catalogue. Explicit count assertions become 55; none are discarded.
The Rust SDK reference now includes knowledge configuration (50 typed HTTP
operations, five existing unsupported operations). Both knowledge and endpoint
configuration sections survive their shared reference-file conflict. Synthetic
storage fixtures refresh against the current catalogue; no original native
campaign receipt or immutable journal is rewritten.

Reviewed all conflict hunks, composed invocation/router/client changes, the
existing knowledge source/contract and public HTTP fixture, combined API export,
SDK reference and canonical generators. The first oracle remains real HTTP
knowledge CAS/replay/restart alongside the inherited task Stop/replay and
installed endpoint restart/revocation tests. No new upstream algorithm or
resource trace is needed for this integration. Stop on missing operations,
changed authorization/durability, relaxed budgets or undeclared effects.

## Validation

All eleven integrated operations were independently checked in the generated
OpenAPI export. Endpoint/signal parity passed. TypeScript lint/typecheck and six
tests passed; Go tests passed. The focused `cargo test --locked -p rrd-client
--test knowledge_trees --test task_effort` passed both real HTTP flows; the
installed endpoint restart/revocation case passed in `rrd-server --test
http_process`. The full `cargo test --locked -p rrd-contract -p rrd-client
-p rrd-server -p rrd-engine -p rrflow-eval --all-targets` passed, including the
previously repaired exact storage golden under combined dependency features.
Strict all-target Clippy passed for those packages plus rrd-inference, and
Rust formatting passed. No new runtime/test failure surfaced in this merge.

The final SDK reference also replaces a stale H-04 milestone count with its
executable-catalogue owner rather than duplicating another changing number.
Final documentation/workflow, navigation/inventory and whitespace checks are
required before commit; the local outputs retain their exact commands.

## Limits and checklist

The three public-flow fixtures are separate temporary estates; this is not a
claim of full knowledge ingestion, graph/vector reasoning, provider execution
or a unified production workload. Current Connectome's knowledge editor remains
reserved for the owner's Slice D rebuild; the submodule pin and original
salvage branch are unchanged. Python 3.15 is unavailable locally; generation
uses Python 3.11 and the declared Ruff version without changing requirements.
Full Python/Java/.NET daemon conformance, native capacity on this exact tree,
external UI and 24-hour soak remain unrun locally. C-07 descriptor exhaustion
and all wider alpha/connector gaps remain open.

Checklist: pre-effect dependency brief, exact input identities, preserved
canonical source/test ownership, regenerated overlap rather than discarded
behavior, focused real HTTP oracles, linked journal, final diff and generated
navigation/inventory review. Final candidate and merge-group CI remain required.
No objective, roadmap, POA&M completion or version status advances.
