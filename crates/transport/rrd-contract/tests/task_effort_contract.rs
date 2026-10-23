use rrd_contract::{
    ActivateBuiltinEmbedding, ConfigureTaskBudget, ExecuteTaskEmbedding, TaskBudgetView,
};
use serde_json::json;

#[test]
fn repetition_policy_rejects_invalid_limits_counts_and_unbounded_history() {
    use rrd_contract::TaskReasoningBudget;
    let config = json!({"scope":"instance:test","task_id":"task","limits":{"max_run_elapsed_ms":1000,"max_step_elapsed_ms":500,"max_steps":200},"repetition_policy":{"max_equivalent_attempts":2}});
    serde_json::from_value::<ConfigureTaskBudget>(config.clone())
        .unwrap()
        .validate()
        .unwrap();
    for maximum in [0, 201, u64::MAX] {
        let mut invalid = config.clone();
        invalid["repetition_policy"]["max_equivalent_attempts"] = json!(maximum);
        assert!(serde_json::from_value::<ConfigureTaskBudget>(invalid)
            .unwrap()
            .validate()
            .is_err());
    }
    let original = json!({"format_version":1,"instance_id":"test","scope":"instance:test","task_id":"task","configuration_sha256":"a".repeat(64),"limits":config["limits"],"started_at_unix_ms":100,"deadline_unix_ms":1100,"last_observed_at_unix_ms":100,"charged_elapsed_ms":0,"reserved_steps":0,"active_attempt":null,"execution_may_continue":false,"stopped":null,"stop_expected_sha256":null});
    let legacy: TaskReasoningBudget = serde_json::from_value(original.clone()).unwrap();
    legacy.validate_repetition().unwrap();
    assert_eq!(serde_json::to_value(&legacy).unwrap(), original);
    let mut budget = legacy;
    budget.repetition_policy = Some(rrd_contract::TaskRepetitionPolicy {
        max_equivalent_attempts: 2,
    });
    budget.equivalent_attempts.insert("b".repeat(64), 1);
    assert!(budget.validate_repetition().is_err());
    budget.reserved_steps = 1;
    budget.validate_repetition().unwrap();
    for count in [0, 3, u64::MAX] {
        let mut invalid = budget.clone();
        invalid.equivalent_attempts.insert("b".repeat(64), count);
        invalid.reserved_steps = count;
        assert!(invalid.validate_repetition().is_err());
    }
    budget.equivalent_attempts = (0..129).map(|n| (format!("{n:064x}"), 1)).collect();
    budget.reserved_steps = 129;
    assert!(budget.validate_repetition().is_err());
    budget.equivalent_attempts.clear();
    budget.reserved_steps = 1;
    budget
        .equivalent_attempts
        .insert("invalid-digest".into(), 1);
    assert!(budget.validate_repetition().is_err());
}

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
