//! Golden installation and attunement contract builders.

use super::*;

pub(super) fn bootstrap() -> InstallationBootstrap {
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

pub(super) fn installation_plan(attunement: &AttunementPlan) -> InstallationPlan {
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
        adopted_estate_entries: None,
        plan_sha256: digest(0),
    };
    plan.plan_sha256 = installation_plan_sha256(&plan).unwrap();
    plan
}

pub(super) fn installation_result(plan: &InstallationPlan) -> InstallationResult {
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

pub(super) fn checkpoints(plan: &AttunementPlan, count: usize) -> Vec<AttunementPhaseCheckpoint> {
    let mut output = Vec::new();
    let mut input_sha256 = plan.source_sha256.clone();
    for planned in plan.phases.iter().take(count) {
        let phase = planned.phase;
        let sequence = planned.sequence;
        let phase_output = digest(200 + u64::from(sequence));
        let mut checkpoint = AttunementPhaseCheckpoint {
            contract_version: ATTUNEMENT_CONTRACT_VERSION,
            job_id: canonical_id("job-01"),
            plan_sha256: plan.plan_sha256.clone(),
            phase,
            sequence,
            configuration_sha256: planned.configuration_sha256.clone(),
            input_sha256,
            output_sha256: phase_output.clone(),
            coordinates: AttunementRuntimeCoordinates {
                runtime_manifest_sha256: digest(300 + u64::from(sequence)),
                runtime_cursor: u64::from(sequence),
                schema_revision: Some(1),
                catalogue_revision: u64::from(sequence),
            },
            committed_at_unix_ms: CREATED_AT + 100 + u64::from(sequence),
            checkpoint_sha256: digest(0),
        };
        checkpoint.checkpoint_sha256 = attunement_checkpoint_sha256(&checkpoint).unwrap();
        output.push(checkpoint);
        input_sha256 = phase_output;
    }
    output
}

pub(super) fn job(
    plan: &AttunementPlan,
    state: AttunementJobState,
    checkpoint_count: usize,
    revision: u64,
    updated_at_unix_ms: u64,
) -> AttunementJob {
    let phase = plan.phases.get(checkpoint_count).map(|step| step.phase);
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
        evidence_sha256: digest(400 + u64::try_from(checkpoint_count + 1).unwrap()),
        observed_at_unix_ms: updated_at_unix_ms,
    });
    AttunementJob {
        contract_version: ATTUNEMENT_CONTRACT_VERSION,
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

pub(super) fn command_checkpoint(job: &AttunementJob) -> Option<String> {
    job.checkpoints
        .last()
        .map(|checkpoint| checkpoint.checkpoint_sha256.clone())
}

pub(super) fn verification(
    job: &AttunementJob,
    status: AttunementVerificationStatus,
) -> AttunementVerification {
    let last = job.checkpoints.last().unwrap();
    let mut verification = AttunementVerification {
        contract_version: ATTUNEMENT_CONTRACT_VERSION,
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

pub(super) fn all_states() -> [AttunementJobState; 7] {
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

pub(super) fn allowed_transitions() -> Vec<TransitionVector> {
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

pub(super) fn golden_contract() -> GoldenAttunementContract {
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
            contract_version: ATTUNEMENT_CONTRACT_VERSION,
            observed_at_unix_ms: running.updated_at_unix_ms + 1,
            job: running,
        },
        state_snapshots,
        resume: ResumeAttunement {
            contract_version: ATTUNEMENT_CONTRACT_VERSION,
            job_id: paused.id.clone(),
            plan_sha256: paused.plan_sha256.clone(),
            expected_job_revision: paused.revision,
            last_checkpoint_sha256: command_checkpoint(&paused),
            resume_from_phase: paused.current_phase.unwrap(),
            requested_at_unix_ms: paused.updated_at_unix_ms + 1,
        },
        cancel: CancelAttunement {
            contract_version: ATTUNEMENT_CONTRACT_VERSION,
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
