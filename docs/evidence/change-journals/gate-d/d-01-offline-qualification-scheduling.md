# D-01 offline qualification scheduling correction

**Status:** active bounded qualification correction; alpha remains open
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-d/d-01-offline-qualification-scheduling`
**Owner:** OBJ-07, D-01, POAM-006 and POAM-013

## Baseline, brief and failure oracle

Starting revision `648bf370b197ebf3d0f2e063090d9a649d30b5d5`.
[PR 43](https://github.com/rrflow/rrflow-development/pull/43) records this
correction brief before implementation. Candidate run `39527523573` passed
15 partitions; installed lifecycle and its dependent CI gate failed. The
ordinary Linux five-test corpus passed in 283.81 seconds. Under verified
mount/network/PID isolation, the fresh-project case passed in 282.5 seconds,
then the serial corpus expired at its 540-second deadline during the existing
project case. This is failed qualification, not an offline pass.

## Change and traceability

`scripts/ci/run_offline_lifecycle.py::inside` now bounds the existing independent
test fixtures to two concurrent test threads. No case, assertion, artifact,
isolation check or deadline changes. Thread-local staged distributions, unique
temporary project roots and ephemeral loopback listeners already isolate the
two fixtures in the ordinary parallel corpus. No runtime behavior is replaced.
The original evidence record stays immutable; navigation and file inventory
are regenerated to discover this correction.

Read the previous qualification journal, complete runner and runner-policy
tests, installed lifecycle fixture/command construction, candidate job log
`118985570566`, and current diff. Research: existing measured candidate timing
and fixture ownership suffice; no new algorithm or upstream adaptation.
Trace/resource decision: existing process assertions and namespace/hash receipt
remain the oracle; cap concurrency at two and retain all process deadlines.
Stop on failed isolation, missing corpus, unexpected shared fixture state or
any failed required candidate check. This does not close a new product gap.

## Verification and checklist

The six runner-policy tests and documentation, workflow, navigation, inventory
and diff checks are rerun before commit; exact results are recorded below.
Actual namespace execution requires Linux candidate CI and is not claimed
from this Windows worktree. PR 43 carries exact subsequent candidate and merge
receipts only after all 17 checks pass. No alpha objective, roadmap completion,
POA&M closure, product version or release maturity changes.

Change brief, owning outcome, exact scope, failure oracle, trace/resource and
research decisions, replacement map and stop conditions are above. Public
development remote/ref/revision is recorded before push. Acceptance remains
the complete candidate pipeline, including the previously failed offline case.

Local result: all six runner-policy tests passed; navigation and inventory
regeneration, documentation/workflow policy and `git diff --check` passed.
