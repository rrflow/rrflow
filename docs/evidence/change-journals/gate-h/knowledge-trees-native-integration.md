# Knowledge trees: shared native candidate integration

**Status:** active integration evidence; final candidate qualification pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/knowledge-trees-native-integration`
**Owner:** OBJ-07 / H-06 prerequisite / D-06 / POAM-037 / PR16

Starting head `78dfcb911bdb9dd3e097fa6354760ed89ee3d476` integrates PR129
`d0e7f54c1496bf55c1a0932c6f4d574cae21545f` by ordinary merge.
The PR description declared this dependency and effect scope before edits.
The [previous integration](knowledge-trees-accepted-main.md) retains its exact-source
checks. This record does not rebind them to the new tree.

The merge has no implementation conflict. Its only conflict is the generated
execution inventory, regenerated from the combined tracked source. Reusable
behavior stays in its existing canonical module: the segment-v6 encoder uses
one row-group builder and reusable compression scratch; the database example
binds task/relationship reads to one captured cursor; the native capacity
fixture records bounded WAL allocation and synthetic API errors; the receipt
validator requires the repeated APFS WAL step. Their original journals and
characterization tests are retained. Feature contracts, SDK projections and
transaction authority are unchanged by this integration.

Reviewed the incoming implementation diff, complete added encoder test and WAL
observation module, native workflow/acceptance deltas, database-view delta,
previous feature integration, C-07 owner and pre-effect PR brief. No code is
moved, deleted or substituted. The smallest cross-boundary oracle is authenticated project knowledge configuration, CAS, replay and restart.
No additional upstream research or runtime tracing is needed for this ordinary
integration; inherited native diagnostics and all time/resource/failure bounds
remain intact. Unexpected source conflicts or lost semantics stop the package.

## Validation

`cargo test --locked -p rrd-client --test knowledge_trees` passed its one real
HTTP authentication/CAS/replay/restart test. `cargo clippy --locked
-p rrd-contract -p rrd-engine -p rrd-server -p rrd-client --all-targets
-- -D warnings` passed. No new source failure occurred. Full owning suites
retain their prior exact-source evidence; final candidate CI reruns them on
this tree. The generated navigation/inventory, documentation/workflow, Rust
formatting and whitespace checks must pass before committing this journal.

## Limits and checklist

Candidate CI and normal merge-queue CI are required before landing. This branch
stays draft while PR129 qualifies. Its three native crash/capacity jobs passed
on run 40154271269; the 100-cycle lanes remain pending at this record's review.
That other candidate's evidence is not evidence for this feature tree.
The retained descriptor-exhaustion failure remains owned by C-07/POAM-035;
no 24-hour soak is running or accepted. Full workspace/native capacity, live
providers and external UI were not exercised by this local integration.

Checklist: pre-effect brief, preserved source/test ownership, only generated
conflict, focused cross-boundary check, linked immutable journal, regenerated
navigation/inventory and complete integration-diff review. No objective,
roadmap, POA&M completion or product-version change is made.

The Connectome owner explicitly closed old PR10 as salvage rather than merge:
[owner decision](https://github.com/rrflow/connectome-development/pull/10#issuecomment-6391824222).
Its knowledge-tree behavior is reserved for rebuilding in the existing Slice D
plan, with original branch retained. This engine integration does not reopen
that branch, change the UI pin or claim the editor is available.
