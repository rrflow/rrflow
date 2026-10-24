# C-07c native crash boundary baseline

**Status:** active characterization evidence; deterministic native crash implementation pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-c/c-07c-native-crash-baseline`
**Owner:** OBJ-02 prerequisite, [C-07c](../../../roadmap/rrflow-1.0/gate-c-07.md)

This bounded package starts from public main
`51c5f7a6a20e1751383b19dc1af545e34bdabf94`, tree
`216b00db20010fc5f28d6700169dd405e6e0291b`, after PR 58's accepted installed
response-loss history. No unrelated changes were present. The pre-edit change
brief in `target/c07c-native-pr.md` defines scope, failure oracle and stop rules;
this seed opens the review before implementation.

The existing `rrd-lsm` unit-test I/O model distinguishes stable and volatile
bytes, and the error matrix returns injected failures. Neither proves a killed
native child reached a specific write, sync, publication or reclamation call.
The new package will extend that existing test boundary with a bounded one-use
parent/child rendezvous, independent requested/acknowledged values, compound
recovery and a repeated native reopen corpus. The facility will remain under
`cfg(test)`, absent from ordinary library and primary-binary builds.

The unchanged `cargo test -p rrd-lsm --lib fault_io --locked -- --nocapture`
baseline was started before implementation; its complete result belongs in the
successor journal. The existing README, objective, C-07/POAM-035 owners,
dependency spine, authoring procedure, installed fixture and history,
native-call wrappers, model and failure matrix were inspected. No source
adaptation is proposed. Test receipts provide bounded diagnostics; production
telemetry, defaults and formats remain untouched.

Native substrate evidence cannot close installed composition, resource limits,
native disk exhaustion or the 24-hour sustained profile. The final journal and
PR must retain those omissions and exact candidate results. No roadmap,
objective, POA&M or version status changes here.
