# RRFlow deployment reference

**Status:** active stable-reference index
**Coordinate:** `rrflow://rrflow-instance/data/reference-index/deployment`
**Owner:** deployment-profile classification, executable packaging, and deployment contracts; linked from `docs/reference/README.md`

Deployment records describe how the same RRFlow authority is composed,
presented, and packaged. A profile or artifact cannot introduce another
engine, database, query planner, or configuration authority.

| Subject | Durable warp | Checkout record | State |
|---|---|---|---|
| Installed product lifecycle | [`rrflow://rrflow-instance/data/reference/deployment/installed-lifecycle`](rrflow://rrflow-instance/data/reference/deployment/installed-lifecycle) | [`installed-lifecycle.md`](installed-lifecycle.md) | target single-executable acquisition, install, readiness, verification, repair, restore, salvage, uninstall, and platform-artifact contract; implementation remains open |
| Deployment profiles | [`rrflow://rrflow-instance/data/reference/deployment/modes`](rrflow://rrflow-instance/data/reference/deployment/modes) | [`modes.md`](modes.md) | accepted independent deployment-form, storage-profile, and endpoint-presentation classification; implementation convergence remains open |
| Endpoint bindings, profiles, and hardware inventory | [`rrflow://rrflow-instance/data/reference/deployment/endpoint-bindings`](rrflow://rrflow-instance/data/reference/deployment/endpoint-bindings) | [`endpoint-bindings.md`](endpoint-bindings.md) | contract and validation for the configured endpoints every workload class reaches; runtime resolution and governed activation remain open |
| Offline edge artifact CLI | [`rrflow://rrflow-instance/data/reference/deployment/edge`](rrflow://rrflow-instance/data/reference/deployment/edge) | [`edge.md`](edge.md) | narrow executable artifact proof; not a persistent project estate or release installer |
| Local RRD process adapter | [`rrflow://rrflow-instance/data/reference/deployment/local-process-driver`](rrflow://rrflow-instance/data/reference/deployment/local-process-driver) | [`local-process-driver.md`](local-process-driver.md) | target launch/readiness/shutdown contract; current implementation requires direct convergence |
| Kubernetes deployment adapter | [`rrflow://rrflow-instance/data/reference/deployment/kubernetes-operator`](rrflow://rrflow-instance/data/reference/deployment/kubernetes-operator) | [`kubernetes-operator.md`](kubernetes-operator.md) | target projection/reconciliation contract; current controller is non-conforming inventory and is not alpha-qualified |
