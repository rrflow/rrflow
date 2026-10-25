use rrd_contract::{
    attunement_checkpoint_sha256, attunement_plan_sha256, attunement_verification_sha256,
    installation_plan_sha256, installation_result_sha256, AttunementFailure, AttunementJob,
    AttunementJobState, AttunementLease, AttunementPhase, AttunementPhaseCheckpoint,
    AttunementPhasePlan, AttunementPlan, AttunementRuntimeCoordinates, AttunementStatus,
    AttunementVerification, AttunementVerificationCheck, AttunementVerificationStatus,
    CancelAttunement, CanonicalId, DeploymentForm, DeploymentProfile, EndpointPresentation,
    EstateConfiguration, InstallationActionDisposition, InstallationActionKind,
    InstallationActionResult, InstallationBootstrap, InstallationBootstrapPolicy,
    InstallationManagedPath, InstallationManagedPathKind, InstallationPlan, InstallationPlanAction,
    InstallationRecordKind, InstallationRemovalRule, InstallationResult,
    InstallationSpecialization, InstallationStorageGeneration, InstallationTargetKind,
    InstalledEstateIdentity, MemorySeatDefinition, ResumeAttunement, SecurityAction,
    StorageProfileKind, ATTUNEMENT_PHASES, INSTALLATION_PLAN_CONTRACT_VERSION,
    INSTALL_ATTUNEMENT_CONTRACT_VERSION,
};
use schemars::{schema_for, JsonSchema};
use serde::{Deserialize, Serialize};

const CREATED_AT: u64 = 1_800_000_000_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct TransitionVector {
    from: AttunementJobState,
    to: AttunementJobState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct GoldenJobStateSnapshot {
    state: AttunementJobState,
    current_phase: Option<AttunementPhase>,
    checkpoint_count: usize,
    leased: bool,
    retryable_failure: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct GoldenAttunementContract {
    attunement_plan: AttunementPlan,
    installation_plan: InstallationPlan,
    installation_result: InstallationResult,
    status: AttunementStatus,
    state_snapshots: Vec<GoldenJobStateSnapshot>,
    resume: ResumeAttunement,
    cancel: CancelAttunement,
    verification: AttunementVerification,
    allowed_transitions: Vec<TransitionVector>,
}

fn canonical_id(value: &str) -> CanonicalId {
    CanonicalId::new(value).unwrap()
}

fn digest(value: u64) -> String {
    format!("{value:064x}")
}

fn target() -> InstalledEstateIdentity {
    InstalledEstateIdentity {
        project_id: canonical_id("rrflow"),
        estate_id: canonical_id("developer-estate"),
        instance_id: canonical_id("rrflow-dev"),
    }
}

fn attunement_plan() -> AttunementPlan {
    let mut plan = AttunementPlan {
        contract_version: INSTALL_ATTUNEMENT_CONTRACT_VERSION,
        id: canonical_id("attune-01"),
        target: target(),
        source_sha256: digest(1),
        phases: ATTUNEMENT_PHASES
            .iter()
            .copied()
            .map(|phase| AttunementPhasePlan {
                phase,
                sequence: phase.sequence(),
                configuration_sha256: digest(100 + u64::from(phase.sequence())),
                estimated_items: u64::from(phase.sequence()) * 10,
                estimated_input_bytes: u64::from(phase.sequence()) * 1_024,
            })
            .collect(),
        estimated_min_duration_ms: 1_800_000,
        estimated_max_duration_ms: 2_700_000,
        plan_sha256: digest(0),
    };
    plan.plan_sha256 = attunement_plan_sha256(&plan).unwrap();
    plan
}

fn bootstrap() -> InstallationBootstrap {
    let specialization = InstallationSpecialization {
        format_version: 1,
        id: canonical_id("generic"),
        revision: 1,
        seat: MemorySeatDefinition {
            id: canonical_id("rrflow-local-seat"),
            display_name: "Local RRFlow".into(),
            purpose: "Provider-neutral reasoning and recall for this project.".into(),
        },
        instructions: "Use explicitly authorized public capabilities.".into(),
        endpoint_profile: None,
    };
    let mut result = InstallationBootstrap {
        format_version: 1,
        template_id: canonical_id("generic-project"),
        template_revision: 1,
        template_text: "# Project instructions\n".into(),
        template_sha256: rrd_core::digest::sha256_hex(b"# Project instructions\n"),
        specialization_sha256: specialization.digest().unwrap(),
        specialization,
        instructions_sha256: digest(0),
        project_policy: InstallationBootstrapPolicy::PreserveProjectOwnedState,
        initial_records: InstallationRecordKind::INITIAL.to_vec(),
        max_record_bytes: 1_048_576,
        storage_generation: InstallationStorageGeneration {
            application_format: 1,
            manifest_format: 3,
            wal_format: 1,
            batch_format: 2,
            max_initial_transaction_bytes: 4_194_304,
        },
        token_key_bytes: 32,
        credential_entropy_bytes: 32,
        private_directories: [
            ".rrflow",
            ".rrflow/credentials",
            ".rrflow/rrd",
            ".rrflow/rrd/roots",
        ]
        .map(String::from)
        .to_vec(),
        private_files: [
            ".rrflow-install-install-01.json",
            ".rrflow-install.lock",
            ".rrflow/credentials/local-operator.json",
            ".rrflow/rrd/roots/install-01/RRD.TOKEN",
        ]
        .map(String::from)
        .to_vec(),
        process_invocations: Vec::new(),
        network_endpoints: Vec::new(),
        activated_provider_bindings: Vec::new(),
    };
    result.instructions_sha256 =
        rrd_core::digest::sha256_hex(result.rendered_instructions().as_bytes());
    result
}

fn installation_plan(attunement: &AttunementPlan) -> InstallationPlan {
    let mut plan = InstallationPlan {
        contract_version: INSTALLATION_PLAN_CONTRACT_VERSION,
        installation_identity: digest(46),
        project_root_sha256: digest(47),
        id: canonical_id("install-01"),
        target: target(),
        target_kind: InstallationTargetKind::ExistingProject,
        deployment: DeploymentProfile {
            contract_version: 1,
            deployment_form: DeploymentForm::SingleNodeServer,
            storage_profile: StorageProfileKind::RrflowKv,
            endpoint_presentation: EndpointPresentation::LoopbackHttpWebsocket,
        },
        product_version: "1.0.0".into(),
        executable_sha256: digest(40),
        profile_id: canonical_id("default"),
        profile_sha256: digest(41),
        project_precondition_sha256: digest(42),
        storage_root_id: canonical_id("install-01"),
        configuration: EstateConfiguration::default(),
        bootstrap: bootstrap(),
        initial_seat: MemorySeatDefinition {
            id: canonical_id("rrflow-local-seat"),
            display_name: "Local RRFlow".into(),
            purpose: "Provider-neutral reasoning and recall for this project.".into(),
        },
        initial_principal_id: canonical_id("local-operator"),
        initial_grants: vec![
            SecurityAction::ServiceInspect,
            SecurityAction::SessionCreate,
            SecurityAction::SessionClose,
            SecurityAction::QueryExecute,
        ],
        credential_bytes: 32,
        inactive_capabilities: vec![
            canonical_id("external-model-provider"),
            canonical_id("project-generator"),
        ],
        managed_paths: vec![
            InstallationManagedPath {
                kind: InstallationManagedPathKind::EstateDirectory,
                relative_path: ".rrflow".into(),
                precondition_sha256: digest(43),
                removal_rule: InstallationRemovalRule::RemoveIfOwnedAndEmpty,
            },
            InstallationManagedPath {
                kind: InstallationManagedPathKind::StorageDirectory,
                relative_path: ".rrflow/rrd".into(),
                precondition_sha256: digest(44),
                removal_rule: InstallationRemovalRule::RemoveIfOwnedAndEmpty,
            },
            InstallationManagedPath {
                kind: InstallationManagedPathKind::StorageRootsDirectory,
                relative_path: ".rrflow/rrd/roots".into(),
                precondition_sha256: digest(45),
                removal_rule: InstallationRemovalRule::RemoveIfOwnedAndEmpty,
            },
            InstallationManagedPath {
                kind: InstallationManagedPathKind::CredentialDirectory,
                relative_path: ".rrflow/credentials".into(),
                precondition_sha256: digest(46),
                removal_rule: InstallationRemovalRule::RemoveIfOwnedAndEmpty,
            },
            InstallationManagedPath {
                kind: InstallationManagedPathKind::StorageRoot,
                relative_path: ".rrflow/rrd/roots/install-01".into(),
                precondition_sha256: digest(47),
                removal_rule: InstallationRemovalRule::RemoveIfOwnedTreeDigestMatches,
            },
            InstallationManagedPath {
                kind: InstallationManagedPathKind::TokenKey,
                relative_path: ".rrflow/rrd/roots/install-01/RRD.TOKEN".into(),
                precondition_sha256: digest(48),
                removal_rule: InstallationRemovalRule::RemoveIfOwnedDigestMatches,
            },
            InstallationManagedPath {
                kind: InstallationManagedPathKind::OperatorCredential,
                relative_path: ".rrflow/credentials/local-operator.json".into(),
                precondition_sha256: digest(49),
                removal_rule: InstallationRemovalRule::RemoveIfOwnedDigestMatches,
            },
            InstallationManagedPath {
                kind: InstallationManagedPathKind::ProjectLocator,
                relative_path: ".rrflow/config.toml".into(),
                precondition_sha256: digest(50),
                removal_rule: InstallationRemovalRule::RemoveIfOwnedDigestMatches,
            },
            InstallationManagedPath {
                kind: InstallationManagedPathKind::InstallationLease,
                relative_path: rrd_contract::INSTALLATION_LEASE_PATH.into(),
                precondition_sha256: rrd_core::digest::sha256_hex(
                    rrd_contract::INSTALLATION_LEASE_PRECONDITION.as_bytes(),
                ),
                removal_rule: InstallationRemovalRule::RetainForExclusion,
            },
            InstallationManagedPath {
                kind: InstallationManagedPathKind::InstallationIntent,
                relative_path: ".rrflow-install-install-01.json".into(),
                precondition_sha256: digest(43),
                removal_rule: InstallationRemovalRule::RemoveIfOwnedDigestMatches,
            },
            InstallationManagedPath {
                kind: InstallationManagedPathKind::LocatorPending,
                relative_path: ".rrflow/.config.install-01.pending".into(),
                precondition_sha256: digest(43),
                removal_rule: InstallationRemovalRule::RemoveIfOwnedDigestMatches,
            },
            InstallationManagedPath {
                kind: InstallationManagedPathKind::ProjectBootstrap,
                relative_path: ".rrflow/bootstrap.md".into(),
                precondition_sha256: digest(43),
                removal_rule: InstallationRemovalRule::RemoveIfOwnedDigestMatches,
            },
            InstallationManagedPath {
                kind: InstallationManagedPathKind::BootstrapPending,
                relative_path: ".rrflow/.bootstrap.install-01.pending".into(),
                precondition_sha256: digest(43),
                removal_rule: InstallationRemovalRule::RemoveIfOwnedDigestMatches,
            },
        ],
        attunement_plan_id: attunement.id.clone(),
        attunement_plan_sha256: attunement.plan_sha256.clone(),
        actions: vec![
            InstallationPlanAction {
                kind: InstallationActionKind::ValidateProject,
                disposition: InstallationActionDisposition::Unchanged,
                input_sha256: digest(3),
                estimated_write_bytes: 0,
            },
            InstallationPlanAction {
                kind: InstallationActionKind::AcquireInstallationLease,
                disposition: InstallationActionDisposition::Create,
                input_sha256: digest(51),
                estimated_write_bytes: 0,
            },
            InstallationPlanAction {
                kind: InstallationActionKind::PrepareCredentials,
                disposition: InstallationActionDisposition::Create,
                input_sha256: digest(52),
                estimated_write_bytes: 65_536,
            },
            InstallationPlanAction {
                kind: InstallationActionKind::CreateEstateDirectories,
                disposition: InstallationActionDisposition::Create,
                input_sha256: digest(4),
                estimated_write_bytes: 0,
            },
            InstallationPlanAction {
                kind: InstallationActionKind::MaterializeBootstrap,
                disposition: InstallationActionDisposition::Create,
                input_sha256: digest(52),
                estimated_write_bytes: 65_536,
            },
            InstallationPlanAction {
                kind: InstallationActionKind::CreateStorageRoot,
                disposition: InstallationActionDisposition::Create,
                input_sha256: digest(5),
                estimated_write_bytes: 4_096,
            },
            InstallationPlanAction {
                kind: InstallationActionKind::DeliverCredentials,
                disposition: InstallationActionDisposition::Create,
                input_sha256: digest(6),
                estimated_write_bytes: 96,
            },
            InstallationPlanAction {
                kind: InstallationActionKind::InitializeInstance,
                disposition: InstallationActionDisposition::Create,
                input_sha256: digest(7),
                estimated_write_bytes: 4_096,
            },
            InstallationPlanAction {
                kind: InstallationActionKind::CreateAttunementJob,
                disposition: InstallationActionDisposition::Create,
                input_sha256: digest(8),
                estimated_write_bytes: 1_024,
            },
            InstallationPlanAction {
                kind: InstallationActionKind::PublishProjectLocator,
                disposition: InstallationActionDisposition::Create,
                input_sha256: digest(9),
                estimated_write_bytes: 256,
            },
        ],
        plan_sha256: digest(0),
    };
    plan.plan_sha256 = installation_plan_sha256(&plan).unwrap();
    plan
}

fn installation_result(plan: &InstallationPlan) -> InstallationResult {
    let mut result = InstallationResult {
        contract_version: INSTALL_ATTUNEMENT_CONTRACT_VERSION,
        plan_id: plan.id.clone(),
        plan_sha256: plan.plan_sha256.clone(),
        attunement_job_id: canonical_id("job-01"),
        action_results: plan
            .actions
            .iter()
            .enumerate()
            .map(|(index, action)| InstallationActionResult {
                kind: action.kind,
                output_sha256: digest(20 + index as u64),
            })
            .collect(),
        runtime_manifest_sha256: digest(30),
        runtime_cursor: 1,
        control_journal_sequence: 1,
        installed_record_sha256: digest(31),
        credential_sha256: digest(32),
        locator_sha256: digest(33),
        applied_at_unix_ms: CREATED_AT + 10,
        idempotent_replay: false,
        result_sha256: digest(0),
    };
    result.result_sha256 = installation_result_sha256(&result).unwrap();
    result
}

fn checkpoints(plan: &AttunementPlan, count: usize) -> Vec<AttunementPhaseCheckpoint> {
    let mut output = Vec::new();
    let mut input_sha256 = plan.source_sha256.clone();
    for phase in ATTUNEMENT_PHASES.iter().copied().take(count) {
        let phase_output = digest(200 + u64::from(phase.sequence()));
        let mut checkpoint = AttunementPhaseCheckpoint {
            contract_version: INSTALL_ATTUNEMENT_CONTRACT_VERSION,
            job_id: canonical_id("job-01"),
            plan_sha256: plan.plan_sha256.clone(),
            phase,
            sequence: phase.sequence(),
            configuration_sha256: plan.phases[usize::from(phase.sequence() - 1)]
                .configuration_sha256
                .clone(),
            input_sha256,
            output_sha256: phase_output.clone(),
            coordinates: AttunementRuntimeCoordinates {
                runtime_manifest_sha256: digest(300 + u64::from(phase.sequence())),
                runtime_cursor: u64::from(phase.sequence()),
                schema_revision: Some(1),
                catalogue_revision: u64::from(phase.sequence()),
            },
            committed_at_unix_ms: CREATED_AT + 100 + u64::from(phase.sequence()),
            checkpoint_sha256: digest(0),
        };
        checkpoint.checkpoint_sha256 = attunement_checkpoint_sha256(&checkpoint).unwrap();
        output.push(checkpoint);
        input_sha256 = phase_output;
    }
    output
}

fn job(
    plan: &AttunementPlan,
    state: AttunementJobState,
    checkpoint_count: usize,
    revision: u64,
    updated_at_unix_ms: u64,
) -> AttunementJob {
    let phase = ATTUNEMENT_PHASES.get(checkpoint_count).copied();
    let current_phase = match state {
        AttunementJobState::Pending
        | AttunementJobState::Cancelled
        | AttunementJobState::Succeeded => None,
        _ => phase,
    };
    let lease = (state == AttunementJobState::Running).then(|| AttunementLease {
        id: canonical_id("lease-01"),
        holder: canonical_id("rrd-engine-worker"),
        generation: revision,
        acquired_at_unix_ms: updated_at_unix_ms - 10,
        expires_at_unix_ms: updated_at_unix_ms + 30_000,
    });
    let failure = (state == AttunementJobState::Failed).then(|| AttunementFailure {
        phase: phase.unwrap(),
        code: canonical_id("phase-execution-failed"),
        message: "phase execution failed before commit".into(),
        retryable: true,
        evidence_sha256: digest(400 + u64::from(phase.unwrap().sequence())),
        observed_at_unix_ms: updated_at_unix_ms,
    });
    AttunementJob {
        contract_version: INSTALL_ATTUNEMENT_CONTRACT_VERSION,
        id: canonical_id("job-01"),
        plan_id: plan.id.clone(),
        plan_sha256: plan.plan_sha256.clone(),
        revision,
        state,
        current_phase,
        checkpoints: checkpoints(plan, checkpoint_count),
        lease,
        failure,
        created_at_unix_ms: CREATED_AT + 10,
        updated_at_unix_ms,
    }
}

fn command_checkpoint(job: &AttunementJob) -> Option<String> {
    job.checkpoints
        .last()
        .map(|checkpoint| checkpoint.checkpoint_sha256.clone())
}

fn verification(
    job: &AttunementJob,
    status: AttunementVerificationStatus,
) -> AttunementVerification {
    let last = job.checkpoints.last().unwrap();
    let mut verification = AttunementVerification {
        contract_version: INSTALL_ATTUNEMENT_CONTRACT_VERSION,
        job_id: job.id.clone(),
        plan_sha256: job.plan_sha256.clone(),
        job_revision: job.revision,
        last_checkpoint_sha256: last.checkpoint_sha256.clone(),
        runtime_manifest_sha256: last.coordinates.runtime_manifest_sha256.clone(),
        status,
        checks: vec![AttunementVerificationCheck {
            id: canonical_id("runtime-reopen"),
            passed: status == AttunementVerificationStatus::Passed,
            evidence_sha256: digest(500),
        }],
        verified_at_unix_ms: job.updated_at_unix_ms + 1,
        verification_sha256: digest(0),
    };
    verification.verification_sha256 = attunement_verification_sha256(&verification).unwrap();
    verification
}

fn all_states() -> [AttunementJobState; 7] {
    [
        AttunementJobState::Pending,
        AttunementJobState::Running,
        AttunementJobState::Paused,
        AttunementJobState::CancelRequested,
        AttunementJobState::Cancelled,
        AttunementJobState::Failed,
        AttunementJobState::Succeeded,
    ]
}

fn allowed_transitions() -> Vec<TransitionVector> {
    all_states()
        .into_iter()
        .flat_map(|from| {
            all_states()
                .into_iter()
                .filter(move |to| from.permits(*to))
                .map(move |to| TransitionVector { from, to })
        })
        .collect()
}

fn golden_contract() -> GoldenAttunementContract {
    let attunement_plan = attunement_plan();
    let installation_plan = installation_plan(&attunement_plan);
    let installation_result = installation_result(&installation_plan);
    let running = job(
        &attunement_plan,
        AttunementJobState::Running,
        1,
        2,
        CREATED_AT + 2_000,
    );
    let paused = job(
        &attunement_plan,
        AttunementJobState::Paused,
        1,
        3,
        CREATED_AT + 3_000,
    );
    let succeeded = job(
        &attunement_plan,
        AttunementJobState::Succeeded,
        ATTUNEMENT_PHASES.len(),
        20,
        CREATED_AT + 20_000,
    );
    let state_snapshots = all_states()
        .into_iter()
        .map(|state| {
            let checkpoint_count =
                usize::from(state == AttunementJobState::Succeeded) * ATTUNEMENT_PHASES.len();
            let revision = if state == AttunementJobState::Pending {
                1
            } else {
                30
            };
            let snapshot = job(
                &attunement_plan,
                state,
                checkpoint_count,
                revision,
                CREATED_AT + 30_000,
            );
            snapshot.validate(&attunement_plan).unwrap();
            GoldenJobStateSnapshot {
                state,
                current_phase: snapshot.current_phase,
                checkpoint_count: snapshot.checkpoints.len(),
                leased: snapshot.lease.is_some(),
                retryable_failure: snapshot
                    .failure
                    .as_ref()
                    .is_some_and(|failure| failure.retryable),
            }
        })
        .collect();
    GoldenAttunementContract {
        attunement_plan,
        installation_plan,
        installation_result,
        status: AttunementStatus {
            contract_version: INSTALL_ATTUNEMENT_CONTRACT_VERSION,
            observed_at_unix_ms: running.updated_at_unix_ms + 1,
            job: running,
        },
        state_snapshots,
        resume: ResumeAttunement {
            contract_version: INSTALL_ATTUNEMENT_CONTRACT_VERSION,
            job_id: paused.id.clone(),
            plan_sha256: paused.plan_sha256.clone(),
            expected_job_revision: paused.revision,
            last_checkpoint_sha256: command_checkpoint(&paused),
            resume_from_phase: paused.next_phase().unwrap(),
            requested_at_unix_ms: paused.updated_at_unix_ms + 1,
        },
        cancel: CancelAttunement {
            contract_version: INSTALL_ATTUNEMENT_CONTRACT_VERSION,
            job_id: paused.id.clone(),
            plan_sha256: paused.plan_sha256.clone(),
            expected_job_revision: paused.revision,
            last_checkpoint_sha256: command_checkpoint(&paused),
            reason: "operator requested cancellation".into(),
            requested_at_unix_ms: paused.updated_at_unix_ms + 1,
        },
        verification: verification(&succeeded, AttunementVerificationStatus::Passed),
        allowed_transitions: allowed_transitions(),
    }
}

#[test]
fn install_and_attunement_contract_matches_golden_json() {
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
            .join("fixtures/install-attunement-v1.json");
        let mut encoded = serde_json::to_string_pretty(&actual).unwrap();
        encoded.push('\n');
        std::fs::write(path, encoded).unwrap();
        return;
    }
    let expected: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/install-attunement-v1.json")).unwrap();
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
fn installation_plan_requires_versioned_identity_and_root_binding() {
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
fn installation_lease_cannot_change_path_precondition_or_retention_rule() {
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
fn generated_schema_is_closed_and_contains_every_envelope() {
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
fn state_machine_accepts_every_declared_transition_and_rejects_every_other_pair() {
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
fn validation_rejects_skipped_phases_and_mismatched_digests() {
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
        contract_version: INSTALL_ATTUNEMENT_CONTRACT_VERSION,
        job_id: paused.id.clone(),
        plan_sha256: paused.plan_sha256.clone(),
        expected_job_revision: paused.revision,
        last_checkpoint_sha256: command_checkpoint(&paused),
        resume_from_phase: paused.next_phase().unwrap(),
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
fn installation_and_attunement_plans_are_clock_free_deterministic_values() {
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
fn canonical_security_action_catalogue_matches_the_generated_schema() {
    let schema = serde_json::to_value(schema_for!(SecurityAction)).unwrap();
    let variants = schema["enum"].as_array().unwrap();
    let projected = SecurityAction::ALL
        .into_iter()
        .map(|action| serde_json::to_value(action).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(projected, *variants);
}

#[test]
fn installation_plan_rejects_escaping_or_reordered_managed_effects() {
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
fn failed_verification_binds_the_verify_phase_without_committing_it() {
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
fn bootstrap_rejects_unknown_effects_versions_and_unbounded_generation() {
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
