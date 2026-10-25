# C-07g: safe attribution of internal storage read pressure

**Status:** active harness implementation evidence; native attribution pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/c-07g-read-pressure-attribution`
**Owner:** OBJ-02, C-07g, POAM-035
**Pull request:** [109](https://github.com/rrflow/rrflow-development/pull/109)

Baseline `c38ae7aa2dfaa5fd33130167b28e4d3540364c23`, tree
`f24f5cd784acd5ba1654b2aa5b6d3b95c99a5b6a`, integrates fetched main with
PR108 `9607c07203489821e729c5c454f81e52295e3797`. The draft PR bound scope
and oracle before edits. Linux19/20 both failed setup at ordinal 608 with
`rejected-api-internal`; existing closed labels did not identify the owner.

The engine deliberately converts some typed read/transaction pressure into
Internal because it can arise after publication. The harness now recognizes
Storage-wrapped projected-read resources, source-page and transaction limits
and fixed storage error families. `engine_error_label` returns only constant
labels and retains generic `storage` for unknown Storage details. Exact prefix
boundaries prevent similarly named private strings from matching a resource.
No raw messages, values, credentials or numeric text are copied. The original
API code, error digest, successful-abort proof and uncertain-outcome handling
remain unchanged. A resource label is never evidence of rejection on its own.

Read the owner chain and authoring procedure, Linux19/20 receipts, complete
affected classifier/client-error/failed-commit functions, client Display,
server API conversion and engine/store error conversions, canonical resource
names and the existing privacy tests. Changed only installed harness
`tests/support/sustained/workload.rs`, this linked journal, POAM-035 and generated
navigation/inventory. The changed functions, new tests and diff were reviewed.
Local source and retained failures suffice; no external adaptation or runtime
instrumentation. Stop for changed effect semantics or disclosure of server text.

The new test constructs real typed errors and converts them through
`ServiceError`, requiring Internal to remain unchanged while labels identify
all twelve projected resources and source/transaction pressure. Secret suffixes,
unknown resources and near-miss prefixes test the disclosure boundary. Existing
maintenance classifications and bounded evidence length remain covered.

- Before implementation, `cargo test --locked -p rrflow-cli --test installed_reopen diagnostic_labels_attribute -- --nocapture`
  failed as intended: `None` versus `read-active-views`; 77.22 seconds including build.
- After implementation, `cargo test --locked -p rrflow-cli --test installed_reopen sustained::workload:: -- --nocapture`
  passed both diagnostic tests; 14.98 seconds including build.

`cargo clippy --locked -p rrflow-cli --test installed_reopen -- -D warnings`
passed in 20.67 seconds. Formatting, navigation, inventory, documentation
(484 statuses, 482 coordinates) and diff checks passed. Raw logs use
`target/c07g-attribution-*.log`. Full installed
campaign execution remains necessary for actual attribution; component tests
do not identify Linux20's cause. Unaffected engine/storage suites are not rerun
for a harness-only classifier. No runtime policy, threshold, production API,
gate status, version or alpha claim changes. The existing POAM remains open.
