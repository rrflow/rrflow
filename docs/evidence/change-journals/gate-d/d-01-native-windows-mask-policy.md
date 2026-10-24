# D-01.5 Windows mask admission correction

**Status:** active qualification correction; final native candidate pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-d/d-01-native-windows-mask-policy`
**Owner:** OBJ-07 prerequisite, D-01.5, POAM-006

Starting revision `5970656aefa19a4786bf4cccd5c3300fa0701916`, tree
`45aba2a1438a61d12437c214fec06b778c30b1cc`; [PR 52](https://github.com/rrflow/rrflow-development/pull/52)
brief amended before edits. This follows the immutable
[coverage completion record](d-01-native-coverage-completion.md).

## Failure, boundary and correction

[Run 39664876136](https://github.com/rrflow/rrflow-development/actions/runs/39664876136)
passed Windows installation (including the repaired SDK anchor), privileged
foreign-owner and bounded capture checks. Its offline runner failed before any
account, ACL or WFP effects: mask_paths rejected D:/a/rrflow-development as an
operating-system root. The resolved Windows current-drive root differed from
the unresolved Path("/") exclusion and therefore protected the entire drive.
Earlier local mask tests used a different drive and did not expose the defect.

The new current-drive source test reproduced exactly that RuntimeError locally
before repair. The runner now chooses protected system directories by native
platform. Broad-root rejection, system ancestors/descendants, payload overlap
and nested-root deduplication remain enforced. Windows also explicitly protects
ProgramFiles(x86) and ProgramData. No source/network denial is weakened, and
no production path-admission policy changes.

The same failed CI step produced no artifact because it failed before receipt
creation. record_outer_failure now writes a bounded preflight failure when no
receipt exists and changes any prior success to failure on later cleanup error,
retaining already-established hashes. Tests cover both paths and the 1024-character
structural diagnostic ceiling. No product output or estate content is added.

Full reads and diff review cover run_offline_lifecycle.py and its complete
policy-test file, with the owning README/Gate D/POAM-006/CI records and existing
change-authoring routine retained from this package. Changed implementation
paths are only those two Python files; supporting paths are this journal,
POAM-006, CI operations and generated navigation/inventory. No implementation
is moved or deleted. Native path policy remains in the same runner, with no
new runtime authority, dependency or product format. No external research is
needed: native pathlib behavior, the CI failure and the local reproducer give
the failure oracle. Production tracing is unchanged; diagnostics remain bounded.

## Validation and limits

The focused current-drive regression failed before the correction. After it,
`python -X utf8 scripts/ci/test_offline_lifecycle.py` passes all twelve tests in
0.319 seconds. Python compilation and diff whitespace checks pass. Navigation renders 17
indexes, 368 nodes and 1598 edges; inventory renders 1324 records; documentation
checks 369 statuses and 367 coordinates; workflow policy passes. No Rust code
changed, so previously passing focused Rust suites are not rerun for this Python
repair. The native Windows principal/WFP launch has not yet been qualified;
its setup was never reached by this failing candidate. The new candidate must
run all native jobs and the complete CI pipeline before merge.

The preceding candidate run 39662303299 retained passing Linux/ext4 and
macOS/APFS isolated five-case receipts, each with IPv4/IPv6 denial and masked
checkout/toolchain roots. That run was superseded and cancelled after its
Windows anchor failure; it is not accepted CI. Run 39664876136 additionally
passed the new macOS mounted-volume step before this Windows preflight failure.
These observations are retained as bounded evidence, not merged-candidate or
D-01 acceptance. The exact final candidate will be reconciled by D-01.6.

## Checklist and stop conditions

Baseline/owners/diff reconciled; pre-edit PR brief updated; exact failure and
first local oracle retained; canonical replacement and unchanged enforcement
mapped; research, trace and resource decisions recorded; relevant tests run;
full files/diff reviewed; linked journal and generated projections included.
Missing enforcement/privilege/cases, unrelated host effects, escaped fixtures,
secret-bearing diagnostics or a production-boundary defect stop acceptance.
D-01.5, D-01.6, parent D-01, OBJ-07 and POAM-006 remain open. No alpha claim.
