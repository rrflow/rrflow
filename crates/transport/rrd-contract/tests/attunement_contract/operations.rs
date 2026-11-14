//! Attunement schema, replan, status and cancel operations, parse policy and plan bounds.

use super::*;

#[test]
pub(super) fn attunement_schema_projects_version_and_plan_bounds() {
    let plan = serde_json::to_value(schema_for!(AttunementPlan)).unwrap();
    assert_eq!(
        plan["properties"]["contract_version"]["const"],
        serde_json::json!(ATTUNEMENT_CONTRACT_VERSION)
    );
    assert_eq!(plan["properties"]["phases"]["minItems"], 1);
    assert_eq!(
        plan["properties"]["phases"]["maxItems"],
        serde_json::json!(rrd_contract::MAX_ATTUNEMENT_PHASES)
    );
    let installation = serde_json::to_value(schema_for!(InstallationResult)).unwrap();
    assert!(installation["properties"]["contract_version"]
        .get("const")
        .is_none());
}

#[test]
pub(super) fn public_replan_operations_have_exact_authentication_and_mutation_boundaries() {
    let catalogue = rrd_contract::endpoint_catalogue();
    for (id, path, mutation, action) in [
        (
            "attunement-pause",
            "/v1/attunement/pause",
            true,
            SecurityAction::EstateAdmin,
        ),
        (
            "attunement-recover-expired",
            "/v1/attunement/recover-expired",
            true,
            SecurityAction::EstateAdmin,
        ),
        (
            "attunement-resume",
            "/v1/attunement/resume",
            true,
            SecurityAction::EstateAdmin,
        ),
        (
            "attunement-start",
            "/v1/attunement/start",
            true,
            SecurityAction::EstateAdmin,
        ),
        (
            "attunement-step",
            "/v1/attunement/step",
            true,
            SecurityAction::EstateAdmin,
        ),
        (
            "attunement-replan-preview",
            "/v1/attunement/replan/preview",
            false,
            SecurityAction::EstateAdmin,
        ),
        (
            "attunement-replan-accept",
            "/v1/attunement/replan/accept",
            true,
            SecurityAction::EstateAdmin,
        ),
        (
            "attunement-replan-receipt-read",
            "/v1/attunement/replan/receipt/read",
            false,
            SecurityAction::EstateRead,
        ),
    ] {
        let endpoint = catalogue
            .endpoints
            .iter()
            .find(|endpoint| endpoint.operation.as_str() == id)
            .unwrap();
        assert_eq!(endpoint.path, path);
        assert_eq!(endpoint.mutation, mutation);
        assert_eq!(endpoint.action.fixed_action(), Some(action));
        assert_eq!(
            endpoint.authentication,
            rrd_contract::EndpointAuthentication::SessionBearer
        );
    }
    assert!(serde_json::from_value::<rrd_contract::StartAttunement>(serde_json::json!({
        "contract_version": 1, "job_id": "job-a", "plan_sha256": "bad", "expected_revision": 1, "lease_ms": 100,
        "unknown_effect": true
    })).is_err());
    let invalid: rrd_contract::StartAttunement = serde_json::from_value(serde_json::json!({
        "contract_version": 2, "job_id": "job-a", "plan_sha256": "bad", "expected_revision": 1, "lease_ms": 100
    })).unwrap();
    assert!(invalid.validate().is_err());
}

#[test]
pub(super) fn attunement_status_is_an_authenticated_read_operation() {
    let catalogue = rrd_contract::endpoint_catalogue();
    let endpoint = catalogue
        .endpoints
        .iter()
        .find(|endpoint| endpoint.operation.as_str() == "attunement-read")
        .expect("installed job status must have a public operation");
    assert_eq!(endpoint.path, "/v1/attunement/read");
    assert!(!endpoint.mutation);
    assert_eq!(
        endpoint.action.fixed_action(),
        Some(SecurityAction::EstateRead)
    );
    assert_eq!(endpoint.request_type, "ReadAttunement");
    assert_eq!(endpoint.response_type, "AttunementReadResult");
}

#[test]
pub(super) fn attunement_cancel_is_an_authenticated_mutation_operation() {
    let catalogue = rrd_contract::endpoint_catalogue();
    let endpoint = catalogue
        .endpoints
        .iter()
        .find(|endpoint| endpoint.operation.as_str() == "attunement-cancel")
        .expect("cancellation must have a public operation");
    assert_eq!(endpoint.path, "/v1/attunement/cancel");
    assert!(endpoint.mutation);
    assert_eq!(
        endpoint.action.fixed_action(),
        Some(SecurityAction::EstateAdmin)
    );
    assert_eq!(endpoint.request_type, "CancelAttunement");
    assert_eq!(endpoint.response_type, "AttunementCancelResult");
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct TransitionVector {
    pub(super) from: AttunementJobState,
    pub(super) to: AttunementJobState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct GoldenJobStateSnapshot {
    pub(super) state: AttunementJobState,
    pub(super) current_phase: Option<AttunementPhase>,
    pub(super) checkpoint_count: usize,
    pub(super) leased: bool,
    pub(super) retryable_failure: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct GoldenAttunementContract {
    pub(super) attunement_plan: AttunementPlan,
    pub(super) installation_plan: InstallationPlan,
    pub(super) installation_result: InstallationResult,
    pub(super) status: AttunementStatus,
    pub(super) state_snapshots: Vec<GoldenJobStateSnapshot>,
    pub(super) resume: ResumeAttunement,
    pub(super) cancel: CancelAttunement,
    pub(super) verification: AttunementVerification,
    pub(super) allowed_transitions: Vec<TransitionVector>,
}

pub(super) fn canonical_id(value: &str) -> CanonicalId {
    CanonicalId::new(value).unwrap()
}

pub(super) fn digest(value: u64) -> String {
    format!("{value:064x}")
}

pub(super) fn target() -> InstalledEstateIdentity {
    InstalledEstateIdentity {
        project_id: canonical_id("rrflow"),
        estate_id: canonical_id("developer-estate"),
        instance_id: canonical_id("rrflow-dev"),
    }
}

pub(super) fn attunement_plan() -> AttunementPlan {
    let mut plan = AttunementPlan {
        contract_version: ATTUNEMENT_CONTRACT_VERSION,
        id: canonical_id("attune-01"),
        target: target(),
        source_sha256: digest(1),
        phases: ATTUNEMENT_PHASES
            .iter()
            .copied()
            .enumerate()
            .map(|(index, phase)| {
                let sequence = u16::try_from(index + 1).unwrap();
                AttunementPhasePlan {
                    phase,
                    sequence,
                    configuration_sha256: digest(100 + u64::from(sequence)),
                    estimated_items: u64::from(sequence) * 10,
                    estimated_input_bytes: u64::from(sequence) * 1_024,
                    parse_policy: None,
                }
            })
            .collect(),
        estimated_min_duration_ms: 1_800_000,
        estimated_max_duration_ms: 2_700_000,
        plan_sha256: digest(0),
    };
    plan.plan_sha256 = attunement_plan_sha256(&plan).unwrap();
    plan
}

#[test]
pub(super) fn parse_policy_is_optional_versioned_and_bound_to_the_selected_plan() {
    use rrd_contract::{SourceParseBinding, SourceParsePolicy};

    let original = attunement_plan();
    let original_wire = serde_json::to_value(&original).unwrap();
    let restored: AttunementPlan = serde_json::from_value(original_wire.clone()).unwrap();
    assert_eq!(restored.plan_sha256, original.plan_sha256);
    assert_eq!(serde_json::to_value(&restored).unwrap(), original_wire);
    assert!(restored
        .phases
        .iter()
        .all(|step| step.parse_policy.is_none()));

    let policy = SourceParsePolicy {
        format_version: 1,
        bindings: vec![SourceParseBinding {
            extension: "rs".into(),
            language: canonical_id("rust"),
        }],
        max_files: 64,
        max_file_bytes: 1024,
        max_input_bytes: 4096,
        max_nodes: 512,
        max_depth: 64,
        max_output_bytes: 65536,
        max_commit_bytes: 32768,
        max_elapsed_ms: 5000,
        max_memory_bytes: 1048576,
    };
    let mut selected = original.clone();
    let index = selected
        .phases
        .iter()
        .position(|step| step.phase == AttunementPhase::Parse)
        .unwrap();
    selected.phases[index].parse_policy = Some(policy.clone());
    assert!(
        selected.validate().is_err(),
        "old digest cannot approve new policy"
    );
    selected.plan_sha256 = attunement_plan_sha256(&selected).unwrap();
    assert_ne!(selected.plan_sha256, original.plan_sha256);
    selected.validate().unwrap();
    let mut streamed = selected.clone();
    let streamed_policy = streamed.phases[index].parse_policy.as_mut().unwrap();
    streamed_policy.max_output_bytes = streamed_policy.max_memory_bytes + 1;
    streamed_policy.max_input_bytes = u64::from(u32::MAX) + 1;
    streamed_policy.max_memory_bytes = u64::from(u32::MAX) + 1;
    streamed.plan_sha256 = attunement_plan_sha256(&streamed).unwrap();
    streamed.validate().unwrap();
    let mut max_wire = streamed.clone();
    max_wire.phases[index]
        .parse_policy
        .as_mut()
        .unwrap()
        .max_memory_bytes = (1_u64 << 53) - 1;
    max_wire.plan_sha256 = attunement_plan_sha256(&max_wire).unwrap();
    max_wire.validate().unwrap();
    max_wire.phases[index]
        .parse_policy
        .as_mut()
        .unwrap()
        .max_memory_bytes = 1_u64 << 53;
    max_wire.plan_sha256 = attunement_plan_sha256(&max_wire).unwrap();
    assert!(
        max_wire.validate().is_err(),
        "wire integer must be exact in JavaScript"
    );
    let mut invalid_file = streamed.clone();
    invalid_file.phases[index]
        .parse_policy
        .as_mut()
        .unwrap()
        .max_file_bytes = u64::from(u32::MAX) + 1;
    invalid_file.plan_sha256 = attunement_plan_sha256(&invalid_file).unwrap();
    assert!(
        invalid_file.validate().is_err(),
        "parser byte offsets are u32"
    );
    let mut invalid = selected.clone();
    invalid.phases[index]
        .parse_policy
        .as_mut()
        .unwrap()
        .bindings
        .push(policy.bindings[0].clone());
    invalid.plan_sha256 = attunement_plan_sha256(&invalid).unwrap();
    assert!(
        invalid.validate().is_err(),
        "duplicate extension must be rejected"
    );
    let mut invalid_limit = selected.clone();
    invalid_limit.phases[index]
        .parse_policy
        .as_mut()
        .unwrap()
        .max_nodes = 0;
    invalid_limit.plan_sha256 = attunement_plan_sha256(&invalid_limit).unwrap();
    assert!(
        invalid_limit.validate().is_err(),
        "zero node budget must be rejected"
    );
    let mut invalid_commit = selected.clone();
    let bounds = invalid_commit.phases[index].parse_policy.as_mut().unwrap();
    bounds.max_commit_bytes = bounds.max_output_bytes + 1;
    invalid_commit.plan_sha256 = attunement_plan_sha256(&invalid_commit).unwrap();
    assert!(
        invalid_commit.validate().is_err(),
        "a transaction group cannot exceed total output"
    );
    let mut invalid_extension = selected.clone();
    invalid_extension.phases[index]
        .parse_policy
        .as_mut()
        .unwrap()
        .bindings[0]
        .extension = "../rs".into();
    invalid_extension.plan_sha256 = attunement_plan_sha256(&invalid_extension).unwrap();
    assert!(
        invalid_extension.validate().is_err(),
        "path-like extension must be rejected"
    );
    let mut too_many = selected.clone();
    too_many.phases[index]
        .parse_policy
        .as_mut()
        .unwrap()
        .bindings = (0..65)
        .map(|number| SourceParseBinding {
            extension: format!("x{number:02}"),
            language: canonical_id("rust"),
        })
        .collect();
    too_many.plan_sha256 = attunement_plan_sha256(&too_many).unwrap();
    assert!(
        too_many.validate().is_err(),
        "binding count must be bounded"
    );
    let mut misplaced = original;
    misplaced.phases[0].parse_policy = Some(policy);
    misplaced.plan_sha256 = attunement_plan_sha256(&misplaced).unwrap();
    assert!(
        misplaced.validate().is_err(),
        "a non-Parse step cannot carry a parse policy"
    );
}

#[test]
pub(super) fn parse_policy_schema_exposes_structural_admission_bounds() {
    let policy = serde_json::to_value(schema_for!(rrd_contract::SourceParsePolicy)).unwrap();
    assert_eq!(policy["properties"]["format_version"]["const"], 1);
    assert_eq!(policy["properties"]["bindings"]["minItems"], 1);
    assert_eq!(policy["properties"]["bindings"]["maxItems"], 64);
    assert_eq!(policy["properties"]["max_nodes"]["minimum"], 1);
    assert_eq!(
        policy["properties"]["max_nodes"]["maximum"],
        (1_u64 << 53) - 1
    );
    assert_eq!(policy["properties"]["max_file_bytes"]["maximum"], u32::MAX);
    let binding = serde_json::to_value(schema_for!(rrd_contract::SourceParseBinding)).unwrap();
    assert_eq!(binding["properties"]["extension"]["minLength"], 1);
    assert_eq!(binding["properties"]["extension"]["maxLength"], 32);
    assert_eq!(binding["additionalProperties"], false);
}

#[test]
pub(super) fn bound_plan_accepts_a_project_specific_sequence() {
    let mut plan = attunement_plan();
    let mut connect_again = plan.phases[0].clone();
    connect_again.sequence = 2;
    let mut verify = plan.phases.last().unwrap().clone();
    verify.sequence = 3;
    plan.phases = vec![plan.phases[0].clone(), connect_again, verify];
    plan.plan_sha256 = attunement_plan_sha256(&plan).unwrap();
    assert!(plan.validate().is_ok());
    let mut wrong_version = plan.clone();
    wrong_version.contract_version = INSTALL_ATTUNEMENT_CONTRACT_VERSION;
    wrong_version.plan_sha256 = attunement_plan_sha256(&wrong_version).unwrap();
    assert!(wrong_version.validate().is_err());

    let running = job(&plan, AttunementJobState::Running, 2, 3, CREATED_AT + 3_000);
    assert_eq!(running.current_phase, Some(AttunementPhase::Verify));
    running.validate(&plan).unwrap();
    let mut changed = running.clone();
    changed.checkpoints[1].configuration_sha256 = digest(999);
    changed.checkpoints[1].checkpoint_sha256 =
        attunement_checkpoint_sha256(&changed.checkpoints[1]).unwrap();
    assert!(changed.validate(&plan).is_err());
    let mut skipped = running.clone();
    skipped.checkpoints.remove(0);
    assert!(skipped.validate(&plan).is_err());
    let mut early_success = running.clone();
    early_success.state = AttunementJobState::Succeeded;
    early_success.current_phase = None;
    early_success.lease = None;
    assert!(early_success.validate(&plan).is_err());
    let mut no_verification = plan.clone();
    no_verification.phases.pop();
    no_verification.plan_sha256 = attunement_plan_sha256(&no_verification).unwrap();
    assert!(no_verification.validate().is_ok());
    let mut foreign = plan.clone();
    foreign.source_sha256 = digest(999);
    foreign.plan_sha256 = attunement_plan_sha256(&foreign).unwrap();
    assert!(running.validate(&foreign).is_err());
    assert_eq!(running.next_phase(&foreign), None);
    let mut tampered = plan.clone();
    tampered.phases[2].phase = AttunementPhase::Inventory;
    assert_eq!(running.next_phase(&tampered), None);
}

#[test]
pub(super) fn completed_short_plan_does_not_forge_verification_evidence() {
    let mut plan = attunement_plan();
    plan.phases.truncate(1);
    plan.plan_sha256 = attunement_plan_sha256(&plan).unwrap();
    let completed = job(
        &plan,
        AttunementJobState::Succeeded,
        1,
        3,
        CREATED_AT + 3_000,
    );
    completed.validate(&plan).unwrap();
    assert!(
        verification(&completed, AttunementVerificationStatus::Passed)
            .validate_for(&completed, &plan)
            .is_err()
    );
}
