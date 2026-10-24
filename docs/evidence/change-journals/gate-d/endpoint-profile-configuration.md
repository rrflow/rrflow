# Durable endpoint profile configuration

**Status:** active implementation evidence; candidate CI required
**Coordinate:** `rrflow://rrflow-instance/data/evidence/endpoint-profile-configuration`
**Owner:** D-06 configuration prerequisite; alpha configurable engine

Starting revision: `335c9b67d8913f548de119378929a7aba8062a5a`.

## Research and implementation decision

The existing endpoint profile has bounded, revisioned types but no engine-owned
configuration publication. Use the existing control compare-and-swap boundary;
keep one current head and a revocation revision. Preview and read cannot probe
providers or resolve credentials. Configuration alone cannot authorize ingestion.

[Qdrant v1.20 scroll](https://api.qdrant.tech/api-reference/points/scroll-points)
requires a collection and offers filters, shard selection, pagination and read
consistency. [Pinecone 2025-04 list](https://docs.pinecone.io/reference/api/2025-04/data-plane/list)
uses an index host, namespace and pagination token. These different source
identities must be preserved by a future qualified adapter; a generic URL is not
proof of source isolation or snapshot consistency. Neither provider's remote
cursor can be substituted for an RRFlow read stamp. Reviewed 2026-10-24; no
upstream implementation was copied and no live vendor qualification is claimed.

This prerequisite makes existing profiles configurable through the canonical
engine before later source-specific knowledge-path activation. It does not
complete D-06, change C-07 status, or add a client-owned registry. The current
[knowledge connector owner](../../../reference/data/knowledge-connectors.md)
continues to govern activation, provenance and ingestion.

The change brief was written before this research deliverable. GitHub needs a
branch diff to open the draft PR; runtime implementation follows that PR.

## Change brief and implementation

[PR113](https://github.com/rrflow/rrflow-development/pull/113) binds this package
before runtime implementation. Starting source is the revision above; the
research bootstrap commit is `db938b22fc538780de15443bb875520955119f4f`.
D-06 remains open. This owner-requested bounded prerequisite advances the
existing profile publication boundary without moving the dependency spine or
claiming qualified discovery, activation, source ingestion, training, or sync.

The engine now reads, previews and applies one endpoint profile per installed
project/estate/instance through the existing control compare-and-swap owner.
Expected revision and prior-state digest reject stale plans. Exact approved
plan digests bind content and five-minute expiry. A null profile publishes a
revisioned removal; it does not reset the sequence. A lost-response retry may
return the same current head; an older plan cannot resurrect a removed profile.
The existing bounded transaction admission covers publication. Profile and
field limits remain canonical; a 1-MiB wire/state ceiling bounds serialization.
Digest shape is checked before serializing a submitted plan.

`EstateRead` gates reads; `EstateAdmin` gates preview/apply. Preview has a
non-mutating invocation identity, so transport parsing does not demand a
mutation key or grant write admission to preview. Ordinary authentication and
audit still run. No external network, secret resolution, process launch or
provider availability is implied. Configuring a binding does not change the
immutable installed estate resource configuration or installation locator.

The executable catalogue owns three new HTTP descriptors and their schemas.
The server, Rust methods and generated TypeScript/Python/Go/Java/.NET projections
consume that boundary. Rust responses validate instance, revision, profile and
applied-plan identity. OpenAPI review intentionally advances its fingerprint to
`af89ecd555361c5f35d36265efd4c3f0b1800210039a5e77f7da4735fbcc2100` and
operation count from 44 to 47; protocol v1 and product 1.0.0 are unchanged.

## Read, scope and traceability record

Followed README, alpha outcome/roadmap/POA&M ownership, D-06 and the executable
dependency spine, change-authoring procedure, and the knowledge-connector owner.
Read the complete new contract/engine/server modules and focused test modules,
the control publication helper, endpoint profile definitions/validation, and
relevant source sections of deployment, installation, core, invocation/security,
function catalogue, operator knowledge, server envelope/router and client calls.
Reviewed the capability projection, canonical exporter and SDK generators,
public fingerprint/count fixtures and the affected SDK reference sections.
These are focused boundary reads, not a claim of a full repository audit.

Created: `rrd-contract/src/endpoint_configuration.rs`, its contract test;
`rrd-engine/src/engine/endpoint_configuration.rs`, its engine test module; and
`rrd-server/src/http/handlers/endpoint_configuration.rs`. Changed the existing
module registrations, invocation/capability projection, endpoint/schema
catalogue, HTTP dispatch, typed Rust methods and installed HTTP process test.
Generated SDK/schema/signal projections, explicit operation-count fixtures,
reference fingerprints, the connector owner, bounded D-06 roadmap row, journal
index and execution inventory changed. No implementation was moved, deleted,
renamed or replaced; no parallel persistence store or provider registry was
introduced. Git and the regenerated inventory own the exact path/digest list.

Preserved authenticated invocation audit and control transition evidence with
`endpoint.configuration.applied` / `endpoint.configuration.revoked` actions.
No profile, credential or raw endpoint was added to diagnostic logs. No new
metric or general tracing framework was necessary; tests observe canonical
head content and exact failure results. No upstream source was adapted.

## Verification and surfaced failures

- `cargo test -p rrd-engine endpoint_configuration --lib --locked`: four
  focused tests passed. MX and rrflowKV exercise preview/no publication,
  competing revisions, lost response, reopen, removal, stale replay and
  replacement. Additional cases reject foreign/tampered/unapproved/expired
  plans, bad tokens and a reader's preview/apply; concurrent writers have one
  winner.
- `cargo test -p rrd-contract --test endpoint_configuration_contract --locked`:
  two tests passed. The final full contract run below also includes the bounded
  digest-shape case and generated authenticated operation contracts.
- `cargo test -p rrd-contract --locked`: 114 passed, zero failed/ignored.
- `cargo test -p rrd-engine --all-targets --locked`: 232 passed, zero failed,
  two ignored (subprocess fixture invoked by its parent and the privileged
  foreign-owner fixture delegated to native CI).
- `cargo test -p rrd-server --test http_process
  installed_endpoint_configuration_http_flow --locked -- --nocapture`: the
  real installed HTTP flow passed. It installs a fresh project, authenticates,
  previews/applies, reopens, removes and rejects the previous profile. Preview
  has no mutation key; anonymous access is denied. Unreachable provider/secret
  references prove setup does not require contacting either system.
- `cargo test -p rrd-server --lib --locked`: four passed, including complete
  catalogue-to-dispatch parity.
- `cargo clippy -p rrd-contract -p rrd-engine -p rrd-server -p rrd-client
  --all-targets --locked -- -D warnings`: passed.
- Canonical SDK generators and `python scripts/ci/check_generated_surfaces.py
  --write` / `--check`: passed, 47 operations. TypeScript generator ran through
  its pinned `tsx`; Python generator ran with the pinned ruff 0.17.1 under an
  isolated Python 3.11 tool environment. That is generator execution, not
  Python 3.15 SDK runtime qualification.
- TypeScript client tests: five passed; `tsc --noEmit` passed. `go test ./...`
  passed. These check the changed descriptor projections and existing clients.

Initial compilation caught the baseline's payload-bearing `StorageConflict`
variant; the engine and assertions were corrected. Full contract tests then
caught the deliberately changed fingerprint and 44-operation expectations;
canonical generation and explicit review corrected them. The first broad
engine run rejected the new test target while it was still untracked; staging
the new repository source and rerunning the full package passed. Review also
separated preview's non-mutating invocation from the existing mutating estate
administration identity and tightened digest validation before serialization.

The Windows pnpm shim failed; the installed pinned pnpm entry point successfully
performed the frozen install. Python SDK's normal `uv --frozen` invocation could
not find Python 3.15; project requirements were not downgraded. Java/Maven is
also absent locally. Full six-language daemon conformance, privileged ownership,
Linux/macOS execution and candidate CI remain unrun locally and are required
before merge. No live vendor qualification or C-07 soak is claimed. A mistaken
local verification output directory was stopped and moved under ignored target
storage before rerunning with the intended build cache; unrelated worktrees and
processes were preserved.

Final formatting, documentation, navigation, generated-surface, execution-
inventory and whitespace checks are recorded before the implementation commit.
The whitespace check caught an extra terminal blank line in this journal;
it was trimmed and the check rerun. The staged diff additionally caught CRLF
line endings in the new HTTP handler, which the earlier unstaged-only check did
not inspect. The handler was normalized to LF before publication. Changed boundary code and the complete
package diff were reviewed; generated
projections were checked against the canonical exporter. Existing unrelated
implementations remain intact. There is no new product deficiency beyond the
already-open D-06/D-05/H-04 obligations; roadmap completion and POA&M status are
unchanged. Candidate CI and merge evidence remain attached to PR113.

## Integration after PR109 and PR110

Merged public main `2255c2bee9c2e4c2f7b44f053dc73a92feebc7b6` into this
branch after both prerequisite candidates passed all 23 checks. The only
conflict was the generated execution inventory; regenerate it from the combined
tracked source. Engine module/test registrations merged both endpoint
configuration and retained transaction modules. No implementation was discarded
or history rewritten. Endpoint configuration keeps using the same control
transaction authority; terminal session records remain independently addressed.
The prior candidate does not qualify this combined tree: rerun the focused
endpoint and session histories and require the new candidate pipeline.
The first inventory regeneration rejected the still-unmerged index entry.
Resolved that generated entry to its prior projection as an intermediate index
state, then regenerated it from the complete combined tree before validation.
Combined-tree checks passed: `cargo test -p rrd-engine --lib --locked`
(176 passed, two existing ignored child/privileged fixtures; includes all four
endpoint configuration and six retained-session histories), and
`cargo test -p rrd-server --test http_process
installed_endpoint_configuration_http_flow --locked -- --nocapture`
(one actual installed HTTP restart/revocation test passed). Documentation,
navigation, execution inventory, formatting and staged whitespace checks are
repeated before the merge commit. Other package-wide checks above remain bound
to their earlier revision; candidate CI must validate the combined revision.
