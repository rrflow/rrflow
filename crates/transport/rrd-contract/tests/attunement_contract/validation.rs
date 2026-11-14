//! Golden JSON, state machine, plan validation and control command checks.

use super::*;

#[test]
pub(super) fn attunement_read_binds_plan_and_status_and_rejects_unknown_claims() {
    let fixture = golden_contract();
    let result = rrd_contract::AttunementReadResult {
        plan: fixture.attunement_plan,
        status: fixture.status,
        source_verification: rrd_contract::AttunementSourceVerification::NotChecked,
    };
    result.validate().unwrap();
    let json = serde_json::to_value(&result).unwrap();
    assert_eq!(json["source_verification"], "not_checked");
    let decoded: rrd_contract::AttunementReadResult = serde_json::from_value(json.clone()).unwrap();
    assert_eq!(decoded, result);
    let mut unknown = json.clone();
    unknown["source_verification"] = serde_json::json!("verified");
    assert!(serde_json::from_value::<rrd_contract::AttunementReadResult>(unknown).is_err());
    let mut extra = json;
    extra["fresh"] = serde_json::json!(true);
    assert!(serde_json::from_value::<rrd_contract::AttunementReadResult>(extra).is_err());
    let mut mismatch = result;
    mismatch.status.job.plan_sha256 = digest(999);
    assert!(mismatch.validate().is_err());
    assert!(rrd_contract::ReadAttunement {
        contract_version: 0,
        job_id: mismatch.status.job.id,
    }
    .validate()
    .is_err());
}

#[test]
pub(super) fn install_and_attunement_contract_matches_golden_json() {
    let fixture = golden_contract();
    fixture.attunement_plan.validate().unwrap();
    fixture
        .installation_plan
        .validate_attunement_plan(&fixture.attunement_plan)
        .unwrap();
    fixture
        .installation_result
        .validate_for(&fixture.installation_plan)
        .unwrap();
    fixture.status.validate(&fixture.attunement_plan).unwrap();
    let paused = job(
        &fixture.attunement_plan,
        AttunementJobState::Paused,
        1,
        3,
        CREATED_AT + 3_000,
    );
    let succeeded = job(
        &fixture.attunement_plan,
        AttunementJobState::Succeeded,
        ATTUNEMENT_PHASES.len(),
        20,
        CREATED_AT + 20_000,
    );
    fixture
        .resume
        .validate_for(&paused, &fixture.attunement_plan)
        .unwrap();
    fixture
        .cancel
        .validate_for(&paused, &fixture.attunement_plan)
        .unwrap();
    fixture
        .verification
        .validate_for(&succeeded, &fixture.attunement_plan)
        .unwrap();

    let actual = serde_json::to_value(&fixture).unwrap();
    if std::env::var_os("RRFLOW_UPDATE_GOLDEN").is_some() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/install-attunement-v2.json");
        let mut encoded = serde_json::to_string_pretty(&actual).unwrap();
        encoded.push('\n');
        std::fs::write(path, encoded).unwrap();
        return;
    }
    let expected: serde_json::Value =
        serde_json::from_str(include_str!("../../fixtures/install-attunement-v2.json")).unwrap();
    assert_eq!(
        actual,
        expected,
        "{}",
        serde_json::to_string_pretty(&actual).unwrap()
    );
    let reopened: GoldenAttunementContract = serde_json::from_value(expected).unwrap();
    assert_eq!(reopened, fixture);
}

#[test]
pub(super) fn adopted_estate_entries_are_bound_only_when_present() {
    let plan = installation_plan(&attunement_plan());
    let value = serde_json::to_value(&plan).unwrap();
    assert!(value.get("adopted_estate_entries").is_none());

    let mut adopted = plan.clone();
    adopted.adopted_estate_entries = Some(vec!["dev".into(), "instance.toml".into()]);
    adopted.plan_sha256 = installation_plan_sha256(&adopted).unwrap();
    assert_ne!(adopted.plan_sha256, plan.plan_sha256);
    adopted.validate().unwrap();
    let reopened: InstallationPlan =
        serde_json::from_value(serde_json::to_value(&adopted).unwrap()).unwrap();
    assert_eq!(reopened, adopted);

    let mut empty = plan.clone();
    empty.adopted_estate_entries = Some(Vec::new());
    empty.plan_sha256 = installation_plan_sha256(&empty).unwrap();
    assert_ne!(empty.plan_sha256, plan.plan_sha256);
    empty.validate().unwrap();

    let mut changed = adopted.clone();
    changed.adopted_estate_entries = Some(vec!["dev".into()]);
    assert!(changed.validate().is_err());

    for entries in [
        vec!["previous".to_owned()],
        vec!["..".to_owned()],
        vec!["a/b".to_owned()],
        vec!["b".to_owned(), "a".to_owned()],
        vec!["a".to_owned(), "a".to_owned()],
    ] {
        let mut invalid = plan.clone();
        invalid.adopted_estate_entries = Some(entries);
        invalid.plan_sha256 = installation_plan_sha256(&invalid).unwrap();
        assert!(invalid.validate().is_err());
    }
}

#[test]
pub(super) fn installation_plan_requires_versioned_identity_and_root_binding() {
    let plan = installation_plan(&attunement_plan());
    for field in ["installation_identity", "project_root_sha256"] {
        let mut value = serde_json::to_value(&plan).unwrap();
        value.as_object_mut().unwrap().remove(field);
        assert!(serde_json::from_value::<InstallationPlan>(value).is_err());
    }
    let mut old = plan.clone();
    old.contract_version = 1;
    old.plan_sha256 = installation_plan_sha256(&old).unwrap();
    assert!(old.validate().is_err());
    for identity in [true, false] {
        let mut changed = plan.clone();
        if identity {
            changed.installation_identity = digest(999);
        } else {
            changed.project_root_sha256 = digest(999);
        }
        assert_ne!(
            installation_plan_sha256(&changed).unwrap(),
            plan.plan_sha256
        );
        assert!(changed.validate().is_err());
    }
}

#[test]
pub(super) fn installation_lease_cannot_change_path_precondition_or_retention_rule() {
    for mutation in ["path", "precondition", "retention", "missing"] {
        let mut plan = installation_plan(&attunement_plan());
        let lease = plan
            .managed_paths
            .iter_mut()
            .find(|path| path.kind == InstallationManagedPathKind::InstallationLease)
            .unwrap();
        match mutation {
            "path" => lease.relative_path = ".rrflow/other.lock".into(),
            "precondition" => lease.precondition_sha256 = digest(999),
            "retention" => lease.removal_rule = InstallationRemovalRule::RemoveIfOwnedDigestMatches,
            _ => {
                plan.managed_paths
                    .retain(|path| path.kind != InstallationManagedPathKind::InstallationLease);
            }
        }
        plan.plan_sha256 = installation_plan_sha256(&plan).unwrap();
        assert!(plan.validate().is_err(), "{mutation}");
    }
}

#[test]
pub(super) fn generated_schema_is_closed_and_contains_every_envelope() {
    let schema = serde_json::to_value(schema_for!(GoldenAttunementContract)).unwrap();
    assert_eq!(schema["additionalProperties"], serde_json::json!(false));
    let definitions = schema["$defs"].as_object().unwrap();
    for name in [
        "AttunementJob",
        "AttunementPhaseCheckpoint",
        "AttunementPlan",
        "AttunementStatus",
        "AttunementVerification",
        "CancelAttunement",
        "InstallationPlan",
        "InstallationResult",
        "ResumeAttunement",
    ] {
        assert!(
            definitions.contains_key(name),
            "missing generated schema {name}"
        );
        assert_eq!(
            definitions[name]["additionalProperties"],
            serde_json::json!(false),
            "schema {name} must reject unknown fields"
        );
    }

    let mut unknown = serde_json::to_value(attunement_plan()).unwrap();
    unknown["provider"] = serde_json::json!("claude");
    assert!(serde_json::from_value::<AttunementPlan>(unknown).is_err());
}

#[test]
pub(super) fn state_machine_accepts_every_declared_transition_and_rejects_every_other_pair() {
    let plan = attunement_plan();
    let timestamp = CREATED_AT + 10_000;
    let cases = [
        (
            job(&plan, AttunementJobState::Pending, 0, 1, timestamp),
            job(&plan, AttunementJobState::Running, 0, 2, timestamp + 1),
        ),
        (
            job(&plan, AttunementJobState::Pending, 0, 1, timestamp),
            job(
                &plan,
                AttunementJobState::CancelRequested,
                0,
                2,
                timestamp + 1,
            ),
        ),
        (
            job(&plan, AttunementJobState::Running, 0, 2, timestamp),
            job(&plan, AttunementJobState::Running, 1, 3, timestamp + 1),
        ),
        (
            job(&plan, AttunementJobState::Running, 0, 2, timestamp),
            job(&plan, AttunementJobState::Paused, 0, 3, timestamp + 1),
        ),
        (
            job(&plan, AttunementJobState::Running, 0, 2, timestamp),
            job(
                &plan,
                AttunementJobState::CancelRequested,
                0,
                3,
                timestamp + 1,
            ),
        ),
        (
            job(&plan, AttunementJobState::Running, 0, 2, timestamp),
            job(&plan, AttunementJobState::Failed, 0, 3, timestamp + 1),
        ),
        (
            job(&plan, AttunementJobState::Running, 10, 20, timestamp),
            job(&plan, AttunementJobState::Succeeded, 11, 21, timestamp + 1),
        ),
        (
            job(&plan, AttunementJobState::Paused, 0, 3, timestamp),
            job(&plan, AttunementJobState::Running, 0, 4, timestamp + 1),
        ),
        (
            job(&plan, AttunementJobState::Paused, 0, 3, timestamp),
            job(
                &plan,
                AttunementJobState::CancelRequested,
                0,
                4,
                timestamp + 1,
            ),
        ),
        (
            job(&plan, AttunementJobState::Failed, 0, 3, timestamp),
            job(&plan, AttunementJobState::Running, 0, 4, timestamp + 1),
        ),
        (
            job(&plan, AttunementJobState::Failed, 0, 3, timestamp),
            job(
                &plan,
                AttunementJobState::CancelRequested,
                0,
                4,
                timestamp + 1,
            ),
        ),
        (
            job(&plan, AttunementJobState::CancelRequested, 0, 4, timestamp),
            job(&plan, AttunementJobState::Cancelled, 0, 5, timestamp + 1),
        ),
    ];
    assert_eq!(cases.len(), allowed_transitions().len());
    for (before, after) in cases {
        assert!(before.state.permits(after.state));
        before.validate_transition(&after, &plan).unwrap();
    }

    for from in all_states() {
        for to in all_states() {
            if from.permits(to) {
                continue;
            }
            let before_count =
                usize::from(from == AttunementJobState::Succeeded) * ATTUNEMENT_PHASES.len();
            let after_count =
                usize::from(to == AttunementJobState::Succeeded) * ATTUNEMENT_PHASES.len();
            let before = job(&plan, from, before_count, 30, timestamp);
            let after = job(&plan, to, after_count, 31, timestamp + 1);
            assert!(before.validate_transition(&after, &plan).is_err());
        }
    }
}

#[test]
pub(super) fn validation_rejects_skipped_phases_and_mismatched_digests() {
    let plan = attunement_plan();

    let mut tampered_plan = plan.clone();
    tampered_plan.source_sha256 = digest(997);
    assert!(tampered_plan.validate().is_err());

    let mut skipped_plan = plan.clone();
    skipped_plan.phases.swap(2, 3);
    skipped_plan.plan_sha256 = attunement_plan_sha256(&skipped_plan).unwrap();
    assert!(skipped_plan.validate().is_err());

    let mut broken_chain = job(&plan, AttunementJobState::Running, 2, 3, CREATED_AT + 3_000);
    broken_chain.checkpoints[1].input_sha256 = digest(999);
    broken_chain.checkpoints[1].checkpoint_sha256 =
        attunement_checkpoint_sha256(&broken_chain.checkpoints[1]).unwrap();
    assert!(broken_chain.validate(&plan).is_err());

    let mut mismatched_configuration =
        job(&plan, AttunementJobState::Running, 1, 3, CREATED_AT + 3_000);
    mismatched_configuration.checkpoints[0].configuration_sha256 = digest(996);
    mismatched_configuration.checkpoints[0].checkpoint_sha256 =
        attunement_checkpoint_sha256(&mismatched_configuration.checkpoints[0]).unwrap();
    assert!(mismatched_configuration.validate(&plan).is_err());

    let paused = job(&plan, AttunementJobState::Paused, 1, 3, CREATED_AT + 3_000);
    let mut stale_resume = ResumeAttunement {
        contract_version: ATTUNEMENT_CONTRACT_VERSION,
        job_id: paused.id.clone(),
        plan_sha256: paused.plan_sha256.clone(),
        expected_job_revision: paused.revision,
        last_checkpoint_sha256: command_checkpoint(&paused),
        resume_from_phase: paused.next_phase(&plan).unwrap(),
        requested_at_unix_ms: paused.updated_at_unix_ms + 1,
    };
    stale_resume.expected_job_revision -= 1;
    assert!(stale_resume.validate_for(&paused, &plan).is_err());

    let succeeded = job(
        &plan,
        AttunementJobState::Succeeded,
        ATTUNEMENT_PHASES.len(),
        20,
        CREATED_AT + 20_000,
    );
    let mut mismatched_verification =
        verification(&succeeded, AttunementVerificationStatus::Passed);
    mismatched_verification.last_checkpoint_sha256 = digest(998);
    mismatched_verification.verification_sha256 =
        attunement_verification_sha256(&mismatched_verification).unwrap();
    assert!(mismatched_verification
        .validate_for(&succeeded, &plan)
        .is_err());

    let mut non_retryable = job(&plan, AttunementJobState::Failed, 0, 3, CREATED_AT + 3_000);
    non_retryable.failure.as_mut().unwrap().retryable = false;
    let resumed = job(&plan, AttunementJobState::Running, 0, 4, CREATED_AT + 4_000);
    assert!(non_retryable.validate_transition(&resumed, &plan).is_err());
}

#[test]
pub(super) fn installation_and_attunement_plans_are_clock_free_deterministic_values() {
    let first_attunement = attunement_plan();
    let second_attunement = attunement_plan();
    assert_eq!(first_attunement, second_attunement);
    assert_eq!(
        serde_json::to_vec(&first_attunement).unwrap(),
        serde_json::to_vec(&second_attunement).unwrap()
    );

    let first_installation = installation_plan(&first_attunement);
    let second_installation = installation_plan(&second_attunement);
    assert_eq!(first_installation, second_installation);
    assert_eq!(
        serde_json::to_vec(&first_installation).unwrap(),
        serde_json::to_vec(&second_installation).unwrap()
    );
    assert!(serde_json::to_value(first_installation)
        .unwrap()
        .get("planned_at_unix_ms")
        .is_none());
}

#[test]
pub(super) fn canonical_security_action_catalogue_matches_the_generated_schema() {
    let schema = serde_json::to_value(schema_for!(SecurityAction)).unwrap();
    let variants = schema["enum"].as_array().unwrap();
    let projected = SecurityAction::ALL
        .into_iter()
        .map(|action| serde_json::to_value(action).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(projected, *variants);
}

#[test]
pub(super) fn installation_plan_rejects_escaping_or_reordered_managed_effects() {
    let attunement = attunement_plan();
    let mut escaping = installation_plan(&attunement);
    escaping.managed_paths[2].relative_path = "../operator.json".into();
    escaping.plan_sha256 = installation_plan_sha256(&escaping).unwrap();
    assert!(escaping.validate().is_err());

    let mut reordered = installation_plan(&attunement);
    reordered.actions.swap(0, 1);
    reordered.plan_sha256 = installation_plan_sha256(&reordered).unwrap();
    assert!(reordered.validate().is_err());

    let mut duplicate_grant = installation_plan(&attunement);
    duplicate_grant
        .initial_grants
        .push(SecurityAction::QueryExecute);
    duplicate_grant.plan_sha256 = installation_plan_sha256(&duplicate_grant).unwrap();
    assert!(duplicate_grant.validate().is_err());
}

#[test]
pub(super) fn failed_verification_binds_the_verify_phase_without_committing_it() {
    let plan = attunement_plan();
    let failed = job(
        &plan,
        AttunementJobState::Failed,
        ATTUNEMENT_PHASES.len() - 1,
        20,
        CREATED_AT + 20_000,
    );
    assert_eq!(failed.current_phase, Some(AttunementPhase::Verify));
    verification(&failed, AttunementVerificationStatus::Failed)
        .validate_for(&failed, &plan)
        .unwrap();
}

#[test]
pub(super) fn bootstrap_rejects_unknown_effects_versions_and_unbounded_generation() {
    for case in 0..10 {
        let mut plan = installation_plan(&attunement_plan());
        match case {
            0 => plan.bootstrap.format_version += 1,
            1 => plan.bootstrap.specialization.format_version += 1,
            2 => plan.bootstrap.initial_records.pop().map(|_| ()).unwrap(),
            3 => plan
                .bootstrap
                .process_invocations
                .push(canonical_id("command")),
            4 => plan
                .bootstrap
                .network_endpoints
                .push("https://example.invalid".into()),
            5 => plan
                .bootstrap
                .activated_provider_bindings
                .push(canonical_id("provider")),
            6 => plan.bootstrap.private_files.clear(),
            7 => plan.bootstrap.token_key_bytes = 0,
            8 => plan.bootstrap.max_record_bytes = 1_048_577,
            9 => {
                plan.bootstrap
                    .storage_generation
                    .max_initial_transaction_bytes = u64::MAX
            }
            _ => unreachable!(),
        }
        plan.plan_sha256 = installation_plan_sha256(&plan).unwrap();
        assert!(plan.validate().is_err(), "case {case}");
    }
    let plan = installation_plan(&attunement_plan());
    let mut json = serde_json::to_value(&plan).unwrap();
    json["bootstrap"]["undeclared_effect"] = serde_json::json!("execute");
    assert!(serde_json::from_value::<InstallationPlan>(json).is_err());
    let mut json = serde_json::to_value(&plan).unwrap();
    json["bootstrap"]["initial_records"][0] = serde_json::json!("unknown_record");
    assert!(serde_json::from_value::<InstallationPlan>(json).is_err());
}

#[test]
pub(super) fn replan_seed_preserves_the_existing_engine_wire_digest() {
    let digest = rrd_contract::attunement_replan_seed(
        &canonical_id("job-test"),
        5,
        &"a".repeat(64),
        2,
        &"b".repeat(64),
    )
    .unwrap();
    assert_eq!(
        digest,
        "a2b55f6708f6bd425c4ffea1a6002bcff1a49568d2a5d54619b9497908196e8f"
    );
}

#[test]
pub(super) fn public_attunement_control_commands_require_the_plan_contract_version() {
    for version in [
        INSTALL_ATTUNEMENT_CONTRACT_VERSION,
        ATTUNEMENT_CONTRACT_VERSION,
    ] {
        let job_id = CanonicalId::new("versioned-job").unwrap();
        let digest = "0".repeat(64);
        let preview = rrd_contract::AttunementReplanPreview {
            predecessor_job_id: job_id.clone(),
            predecessor_revision: 1,
            generation: 1,
            plan: attunement_plan(),
            proposal_sha256: digest.clone(),
            evidence: None,
        };
        let outcomes = [
            rrd_contract::StartAttunement {
                contract_version: version,
                job_id: job_id.clone(),
                plan_sha256: digest.clone(),
                expected_revision: 1,
                lease_ms: 100,
            }
            .validate()
            .is_ok(),
            rrd_contract::StepAttunement {
                contract_version: version,
                job_id: job_id.clone(),
                expected_revision: 1,
                lease_generation: 1,
            }
            .validate()
            .is_ok(),
            rrd_contract::PauseAttunement {
                contract_version: version,
                job_id: job_id.clone(),
                expected_revision: 1,
                lease_generation: 1,
            }
            .validate()
            .is_ok(),
            rrd_contract::RecoverExpiredAttunement {
                contract_version: version,
                job_id: job_id.clone(),
                expected_revision: 1,
            }
            .validate()
            .is_ok(),
            rrd_contract::PreviewAttunementReplan {
                contract_version: version,
                job_id,
                expected_revision: 1,
            }
            .validate()
            .is_ok(),
            rrd_contract::AcceptAttunementReplan {
                contract_version: version,
                preview,
                proposal_sha256: digest,
            }
            .validate()
            .is_ok(),
        ];
        assert_eq!(outcomes, [version == ATTUNEMENT_CONTRACT_VERSION; 6]);
    }
}
