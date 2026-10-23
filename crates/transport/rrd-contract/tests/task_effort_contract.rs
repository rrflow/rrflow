use rrd_contract::{
    ActivateBuiltinEmbedding, ConfigureTaskBudget, ExecuteTaskEmbedding, TaskBudgetView,
};
use serde_json::json;

#[test]
fn task_effort_wire_contract_rejects_unbounded_or_unknown_input() {
    for dimensions in [0, 1, 7, 1025, u32::MAX] {
        assert!(ActivateBuiltinEmbedding {
            scope: "instance:test".into(),
            dimensions,
            seed: 42
        }
        .validate()
        .is_err());
    }
    for dimensions in [8, 1024] {
        ActivateBuiltinEmbedding {
            scope: "instance:test".into(),
            dimensions,
            seed: 42,
        }
        .validate()
        .unwrap();
    }
    let valid = json!({"scope":"instance:test","task_id":"task-a","limits":{"max_run_elapsed_ms":1000,"max_step_elapsed_ms":500,"max_steps":3}});
    serde_json::from_value::<ConfigureTaskBudget>(valid.clone())
        .unwrap()
        .validate()
        .unwrap();
    for field in ["max_run_elapsed_ms", "max_step_elapsed_ms", "max_steps"] {
        let mut invalid = valid.clone();
        invalid["limits"][field] = json!(0);
        assert!(serde_json::from_value::<ConfigureTaskBudget>(invalid)
            .unwrap()
            .validate()
            .is_err());
    }
    let mut unknown = valid;
    unknown["automatic_extension"] = json!(true);
    assert!(serde_json::from_value::<ConfigureTaskBudget>(unknown).is_err());
    assert!(TaskBudgetView {
        budget: None,
        budget_sha256: Some("a".repeat(64))
    }
    .validate()
    .is_err());
    let oversized = json!({"task_id":"task-a","attempt_id":"attempt-a","request":{"scope":"instance:test","backend_id":"test:cpu:v1","network_policy":"deny","inputs":[{"id":"input","media_type":"text/plain","bytes":vec![0;65537]}]}});
    assert!(serde_json::from_value::<ExecuteTaskEmbedding>(oversized)
        .unwrap()
        .validate()
        .is_err());
}
