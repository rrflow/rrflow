# C-07g: bounded stamped query capture and execution

**Status:** active evidence record; implementation complete, native scale qualification pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/c-07g-stamped-query-admission`
**Owner:** [C-07](../../../roadmap/rrflow-1.0/gate-c-07.md), OBJ-02 and POAM-035
**Pull request:** [85](https://github.com/rrflow/rrflow-development/pull/85)

## Brief, baseline and canonical replacement

Starting revision `6355e90a89585bad77b1e91fc543aafd5dceefb1` integrates
PR84. The pre-edit brief binds one installed scale prerequisite, exact source
scope, failure oracles, resource decisions and stop conditions. Version stays
1.0.0; C-07 and alpha status do not change.

The public query pipeline previously captured whole source families before
planning addressed reads. Capture and execution also received independent
copies of the caller's key budget, while the returned evidence reported only
execution. The pipeline now captures authenticated schema metadata, resolves
the existing canonical binding and its required coverage, and acquires family
history only for paths that require family watermarks/index/corpus semantics.
Direct identity, interval and endpoint reads need schema coverage and use the
requested known prefix as conservative freshness, like existing traversal.
They never reuse a family index artifact. Indexed/corpus queries retain their
actual family watermarks and temporal eligibility.

`plan.rs::prepare_binding` is the factored implementation of the existing
binder; both standalone binding and pipeline preparation enforce its declared
coverage. There is no second binding or lifecycle authority. The pipeline's
combined `run` subtracts actual semantic read/proof keys before each later
phase and composes checked evidence. It reports capture plus execution paths,
counts and stamp validation under the original key limit. Separate bind/plan/
execute calls retain explicit per-call contracts. This does not claim that
index-catalogue/control reads, RSS or total elapsed work are metered by the
semantic key budget.

The public oracle also reproduced a conversion defect: `rrd_query::Error::Budget`
became InvalidArgument. The existing engine conversion now uses the existing
ConfigurationLimit/ResourceExhausted surface. Syntax and binding errors remain
InvalidArgument. The PR scope was updated before this conversion edit. No
public operation/model, disk format, configuration or dependency was added.

## Scope, review and decisions

Production: query `pipeline.rs`, `pipeline/evidence.rs`, `plan.rs` and catalogue
coverage helpers; engine `engine/error.rs` query-budget conversion. Tests:
`query/stamped_admission.rs`, existing manual/temporal/traversal assertions,
and engine `tests/query_admission.rs` registered by the existing test owner.
`reference/query/multi-model.md` documents the freshness and measurement
boundaries. This journal, POAM-035 and generated navigation/inventory carry
the record. No implementation was moved or deleted.

Read the README owner chain, C-07, POAM-035/034/011, authoring procedure,
query pipeline/catalogue/binder/executor, source-watermark histories and index
selection, store read-evidence validation and stamp methods, engine public
query/error/transaction paths, native test fixtures and the query reference.
Reviewed the complete changed source diff, new helpers/tests and reference
text. Existing declarations and generated inventory retain exact paths.

Research decision: reuse existing local authenticated reads and index contracts;
no upstream source or new algorithm dependency. Resource/debug decision:
report actual phase work, reject arithmetic overflow and incompatible evidence,
retain existing bounded counters and avoid payload logging. Stop conditions:
underreported work, unsupported index reuse, changed temporal/security results,
relaxed limits or unsupported C-07 completion.

## Failures, corrections and acceptance

Dedicated build directory `target/c07g-addressed-build` remains at the same
worktree path. All commands below use the locked workspace.

| Command | Actual result |
|---|---|
| `cargo test --locked -p rrd-query --test query stamped_addressed_queries` before implementation | Failed in catalogue capture: physical key 129 exceeded budget 128 |
| Same focused command after repair | Passed across memory, native storage and reopen |
| `cargo test --locked -p rrd-query --test query stamped_` | Four passed |
| `cargo test --locked -p rrd-engine --lib public_query_uses_one_key_budget` | Final public commit/query/reopen history passed; exact budget succeeds, one less returns ResourceExhausted |
| `cargo test --locked -p rrd-query` | 70 passed |
| `cargo test --locked -p rrd-engine --lib` | 163 passed; two existing ignored tests remain ignored |
| `cargo clippy --locked -p rrd-query -p rrd-engine --all-targets -- -D warnings` | Passed |
| `cargo fmt --all` | Passed |

The first engine fixture used the wrong existing commit method signature;
compilation exposed it and the fixture was corrected. Its initial schemaless
schema then reproduced the already-owned [POAM-011 query limitation](../../../poam/rrflow-1.0-alpha/poam-011.md).
The admission fixture now explicitly declares its strict record properties;
it does not claim to fix schemaless queries. The subsequent public denial
classification failure was fixed in production, not weakened in the test.

Two existing assertions exposed changed evidence semantics. The traversal test
retains its original executor bound below 100 keys and now proves the pipeline
total equals that execution plus separately measured schema capture. The manual
pipeline test proves exact aggregate keys/decoded counts and all other result
fields. The addressed historical source coordinate changes from family-last
cursor 3 to requested known prefix 10 as specified in the brief; known/valid
rows, reopened results and index freshness tests remain unchanged and pass.

Additional histories cover source/schema changes after a captured stamp,
retirement, literal historical reads, parameterized binding, family and lexical
fallback, aggregate cap/cap-minus-one, forged totals, overflow, and current-to-
historical stamp-validation composition. No workload or native threshold changed.
Logs: `target/c07g-pipeline-*.log`; failed suite outputs are retained separately
as `suite-before-aggregate-oracle` and `suite-before-source-coordinate`.

## Checklist and remaining acceptance

Pre-edit brief, baseline, source replacement, red/green oracles, owning suites,
resource/research/debug choices, changed-file review and remaining limitations
are recorded. Regenerate navigation/inventory and check documentation before
commit. Exact candidate CI and ordinary PR merge remain required.

Native Linux campaign05 at `d209218d` remains failed: all 1,000 corpus writes
were acknowledged, 950 were verified before the 30-minute setup deadline,
zero measured segments completed and launcher cleanup was confirmed. Its
campaign digest is `c38bd351562a0086b6e24b73bd272b34bcbe34174f03274b88fbcf8ed17e7e04`;
raw evidence is retained at `target/c07-linux-scale-evidence-05`. The next
native comparison must preserve its corpus, allocation and deadlines while
binding this candidate's actual source/binary/configuration identities.

Windows campaign14 remains frozen at `d209218d`; passing the earlier segment255
failure point is progress, not completed 24-hour acceptance. Full capacity
profiles, soak, current-candidate native CI and C-07h reconciliation remain
open. POAM-034 write-constraint indexing and downstream reasoning/UI/connector
outcomes are not completed by this query repair.
