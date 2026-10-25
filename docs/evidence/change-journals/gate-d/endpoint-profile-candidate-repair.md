# D-06 endpoint configuration candidate repair

**Status:** active integration evidence; candidate CI remains required
**Coordinate:** `rrflow://rrflow-instance/data/evidence/endpoint-profile-candidate-repair`
**Owner:** D-06 prerequisite / OBJ-07 / PR113

Starting head `37229a812423acfae8d9c807743ea5eab241be9f`; integrate accepted
main `66464e35` after PR70 passed 23 candidate checks and merged. PR113's
repair brief preceded fixture changes. This continues its canonical public
contract projection scope, not a second configuration implementation.

Candidate run 40059654095 failed product-operations and service-protocol:
`public catalogue identity differs from validator`. The storage fixture still
bound the earlier endpoint catalogue. Its closed validator correctly rejected
it before the intended failure-provenance assertions. The Linux capacity lane
also exceeded an installed child deadline; that separate C-07 failure remains
unresolved and is not attributed to this identity mismatch.

Read the full canonical refresh_storage_contract example, catalogue/manifest
digest definitions, synthetic manifest/result/report, failing campaign fixture
construction and failed_campaign_preserves_closed_cause_and_readback_provenance,
plus their candidate log contexts. Reviewed README, alpha outcomes, dependency
spine, C-07, Gate J comparison requirements and change-authoring procedure.
No external algorithm research or added tracing is needed for this deterministic
projection repair. Existing closed diagnostics and limits remain unchanged.

`cargo run --locked -p rrflow-eval --example refresh_storage_contract`
regenerated only the synthetic storage manifest/result/report. Public catalogue
identity is now `2017f7b903debd2981d691f784b36c5d0f19fbe4783e09229bf8d2a3a69ea9f4`.
The native campaign source/binary identities, raw results, thresholds and
qualification state were not changed. Main's knowledge-path definition and
this PR's endpoint-configuration section both survive the authored merge;
the conflicting generated file inventory is regenerated from combined source.

`cargo test --locked -p rrflow-eval` passed 47 tests (33 unit, three CLI,
11 storage contract). `cargo test --locked -p rrflow-cli --test installed_reopen
failed_campaign_preserves_closed_cause_and_readback_provenance -- --nocapture`
passed the previously failing provenance test. The complete newly generated
fixture diff has only catalogue identity and dependent manifest/report digests;
no assertion or validator was weakened. Documentation/navigation/inventory and
staged whitespace are checked before commit. Broader native candidate lanes
are not local evidence and must pass before merge.

Checklist: owner/outcome/brief bound; affected code and failed logs inspected;
canonical generator used; focused then owning package checks executed; failure
and omissions retained; full authored diff reviewed; no implementation removed,
no source adaptation, new authority, version or gate-status change. The real
alpha remains blocked by C-07 qualification and downstream D/E/F/G/H/I/J work.
OBJ-10 requires same-corpus, same-quality, pinned competitor comparisons with
raw correctness/recall, latency distributions, resource and recovery evidence;
this fixture repair is not a benchmark result.
