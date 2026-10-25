# Task effort: accepted storage-main integration

**Status:** active integration evidence; shared native capacity repair pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/task-effort-accepted-main`
**Owner:** OBJ-06 / G-01a, G-02a, G-03 prerequisites / POAM-009 / PR105

Starting head `aa2f7b87828d4b8ceaa15da855b2f5779634a423` integrates public
main `f3c04e955fd8851c0fc466c45f008975e9d4c1a1` by ordinary merge.
The PR brief was updated before editing, naming PR129's native capacity repair
as a landing dependency. The [prior integration](task-effort-current-main-integration.md)
and original PR100-104 behavior/tests stay in their canonical inference,
engine reasoning-budget, public contract/client and SDK boundaries.

The only conflict is the generated inventory. No task-control implementation
conflicted with main. Regenerate inventory/navigation, preserving main's Gate G
evidence/usage paragraph. Read the complete prior integration and FeatureHash
repair records, current inference control entry points, merged source/conflict
diff and this journal. No implementation is moved, removed or replaced.
The failure oracle remains public execute/Stop/replay across reopen plus the
owning budget, cancellation, repetition and retained-history suites.

The first focused build failed with missing task-contract imports despite the
current source exporting them. The shared target retained a contract artifact
from another worktree. `cargo clean -p rrd-contract` removed that derived
artifact and rebuilt the actual dependency; the focused public flow then passed.
This is a local build-cache failure, not a source repair or accepted test result.
Future native evidence remains bound to its exact binary/source identities.

Validation after the cache repair:

- `cargo test --locked -p rrd-client --test task_effort`: one public execute/Stop/reopen/replay flow passed.
- `cargo test --locked -p rrd-engine --all-targets`: passed; library 188 passed and two existing child/privileged fixtures ignored; all remaining integration/example targets passed.
- `cargo test --locked -p rrd-inference`: 26 passed, none failed.
- `cargo clippy --locked -p rrd-engine -p rrd-inference -p rrd-client --all-targets -- -D warnings`: passed.
- Documentation, navigation, execution inventory and whitespace passed; formatting passed before commit.

Six-language conformance and installed native capacity are not claimed from
these local tests; final candidate CI owns them.
Research, runtime trace, budgets and resource decisions remain those of the
original package. No new runtime/model/soak or performance claim is made.
The earlier macOS candidate failure is retained at job 120899277583, log SHA256
`ea3c2c97a17d5af797b4d6ade2bc1fc5356e5df39208c985e3cc8906265f3af4`.
No unrelated green job clears that failure; final integrated candidate and
merge-group CI are required after the shared native repair.

Checklist: original outcomes and pre-edit brief retained, main synchronized,
source/test mapping preserved, generated conflict resolved mechanically,
smallest flow then owning suites, surfaced failures and omissions, complete
new diff review. No alpha, roadmap or POAM status advances. PR100-104 remain
open until their combined implementation actually lands through PR105.
