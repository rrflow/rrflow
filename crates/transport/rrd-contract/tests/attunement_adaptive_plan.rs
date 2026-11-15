use rrd_contract::{
    attunement_checkpoint_sha256, attunement_plan_sha256, attunement_verification_sha256,
    AttunementJob, AttunementJobState, AttunementPhase, AttunementPhaseCheckpoint,
    AttunementPhasePlan, AttunementPlan, AttunementRuntimeCoordinates, AttunementVerification,
    AttunementVerificationCheck, AttunementVerificationStatus, CanonicalId,
    InstalledEstateIdentity, ATTUNEMENT_CONTRACT_VERSION,
};

fn id(value: &str) -> CanonicalId {
    CanonicalId::new(value).unwrap()
}

fn plan(selected: &[AttunementPhase]) -> AttunementPlan {
    let phases = selected
        .iter()
        .enumerate()
        .map(|(index, phase)| AttunementPhasePlan {
            phase: *phase,
            sequence: (index + 1) as u16,
            description: format!("{phase:?}"),
            configuration_sha256: "a".repeat(64),
            estimated_items: 0,
            estimated_input_bytes: 0,
            parse_policy: None,
        })
        .collect();
    let mut plan = AttunementPlan {
        contract_version: ATTUNEMENT_CONTRACT_VERSION,
        id: id("adaptive-plan"),
        target: InstalledEstateIdentity {
            project_id: id("project"),
            estate_id: id("estate"),
            instance_id: id("instance"),
        },
        source_sha256: "b".repeat(64),
        phases,
        estimated_min_duration_ms: 0,
        estimated_max_duration_ms: 0,
        plan_sha256: "0".repeat(64),
    };
    plan.plan_sha256 = attunement_plan_sha256(&plan).unwrap();
    plan
}

#[test]
fn selected_effects_need_not_end_in_an_unavailable_verification_phase() {
    for selected in [
        vec![AttunementPhase::Connect],
        vec![AttunementPhase::Inventory],
        vec![AttunementPhase::Connect, AttunementPhase::Inventory],
        vec![AttunementPhase::Inventory, AttunementPhase::Connect],
    ] {
        let candidate = plan(&selected);
        assert!(
            candidate.validate().is_ok(),
            "a plan must describe its selected effects, not a mandatory final phase: {selected:?}"
        );
    }
}

#[test]
fn adaptive_plans_still_reject_empty_bad_sequence_and_changed_content() {
    assert!(plan(&[]).validate().is_err());
    let mut bad_sequence = plan(&[AttunementPhase::Connect, AttunementPhase::Inventory]);
    bad_sequence.phases[1].sequence = 1;
    bad_sequence.plan_sha256 = attunement_plan_sha256(&bad_sequence).unwrap();
    assert!(bad_sequence.validate().is_err());
    let mut tampered = plan(&[AttunementPhase::Connect]);
    tampered.phases[0].configuration_sha256 = "c".repeat(64);
    assert!(tampered.validate().is_err());
}

#[test]
fn completed_non_verification_effect_cannot_forge_a_passed_verification() {
    let plan = plan(&[AttunementPhase::Connect]);
    let mut checkpoint = AttunementPhaseCheckpoint {
        contract_version: ATTUNEMENT_CONTRACT_VERSION,
        job_id: id("job"),
        plan_sha256: plan.plan_sha256.clone(),
        phase: AttunementPhase::Connect,
        sequence: 1,
        configuration_sha256: plan.phases[0].configuration_sha256.clone(),
        input_sha256: plan.source_sha256.clone(),
        output_sha256: "c".repeat(64),
        coordinates: AttunementRuntimeCoordinates {
            runtime_manifest_sha256: "d".repeat(64),
            runtime_cursor: 1,
            schema_revision: None,
            catalogue_revision: 1,
        },
        committed_at_unix_ms: 2,
        checkpoint_sha256: "0".repeat(64),
    };
    checkpoint.checkpoint_sha256 = attunement_checkpoint_sha256(&checkpoint).unwrap();
    let job = AttunementJob {
        contract_version: ATTUNEMENT_CONTRACT_VERSION,
        id: id("job"),
        plan_id: plan.id.clone(),
        plan_sha256: plan.plan_sha256.clone(),
        revision: 3,
        state: AttunementJobState::Succeeded,
        current_phase: None,
        checkpoints: vec![checkpoint.clone()],
        lease: None,
        failure: None,
        created_at_unix_ms: 1,
        updated_at_unix_ms: 2,
    };
    assert!(job.validate(&plan).is_ok());
    let mut verification = AttunementVerification {
        contract_version: ATTUNEMENT_CONTRACT_VERSION,
        job_id: job.id.clone(),
        plan_sha256: plan.plan_sha256.clone(),
        job_revision: job.revision,
        last_checkpoint_sha256: checkpoint.checkpoint_sha256,
        runtime_manifest_sha256: checkpoint.coordinates.runtime_manifest_sha256,
        status: AttunementVerificationStatus::Passed,
        checks: vec![AttunementVerificationCheck {
            id: id("check"),
            passed: true,
            evidence_sha256: "e".repeat(64),
        }],
        verified_at_unix_ms: 3,
        verification_sha256: "0".repeat(64),
    };
    verification.verification_sha256 = attunement_verification_sha256(&verification).unwrap();
    assert!(verification.validate_for(&job, &plan).is_err());
}
