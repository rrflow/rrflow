//! Applying an installation plan: staging, publication, recovery and the result.

use super::*;

impl RrdEngine {
    pub fn apply_installation(
        project: &Path,
        target_kind: InstallationTargetKind,
        preview: &InstallationPreview,
        expected_plan_sha256: &str,
        executable: &Path,
    ) -> Result<InstallationResult> {
        let clock = HostSystemClock;
        Self::apply_installation_with_observer(
            project,
            target_kind,
            preview,
            expected_plan_sha256,
            executable,
            &clock,
            &mut |_, _| Ok(()),
        )
    }

    #[cfg(test)]
    #[allow(clippy::too_many_arguments)]
    pub(in super::super) fn apply_installation_at(
        project: &Path,
        target_kind: InstallationTargetKind,
        preview: &InstallationPreview,
        expected_plan_sha256: &str,
        at_unix_ms: u64,
        executable: &Path,
    ) -> Result<InstallationResult> {
        let clock = FixedClock::at(at_unix_ms);
        Self::apply_installation_with_observer(
            project,
            target_kind,
            preview,
            expected_plan_sha256,
            executable,
            &clock,
            &mut |_, _| Ok(()),
        )
    }

    #[cfg(test)]
    pub(in super::super) fn apply_installation_with_test_clock(
        project: &Path,
        target_kind: InstallationTargetKind,
        preview: &InstallationPreview,
        expected_plan_sha256: &str,
        executable: &Path,
        clock: &dyn ClockSource,
    ) -> Result<InstallationResult> {
        Self::apply_installation_with_observer(
            project,
            target_kind,
            preview,
            expected_plan_sha256,
            executable,
            clock,
            &mut |_, _| Ok(()),
        )
    }

    #[cfg(test)]
    #[allow(clippy::too_many_arguments)]
    pub(in super::super) fn apply_installation_with_failure(
        project: &Path,
        target_kind: InstallationTargetKind,
        preview: &InstallationPreview,
        expected_plan_sha256: &str,
        at_unix_ms: u64,
        executable: &Path,
        fail_after: InstallApplyStage,
    ) -> Result<InstallationResult> {
        let mut injected = false;
        Self::apply_installation_with_observer(
            project,
            target_kind,
            preview,
            expected_plan_sha256,
            executable,
            &FixedClock::at(at_unix_ms),
            &mut |stage, _| {
                if stage == fail_after && !injected {
                    injected = true;
                    return Err(ServiceError::Storage(format!(
                        "test installation failpoint after {}",
                        stage.as_str()
                    )));
                }
                Ok(())
            },
        )
    }

    #[cfg(test)]
    #[allow(clippy::too_many_arguments)]
    pub(in super::super) fn apply_installation_with_test_observer(
        project: &Path,
        preview: &InstallationPreview,
        executable: &Path,
        observer: &mut dyn FnMut(InstallApplyStage, InstallRecoveryState) -> Result<()>,
    ) -> Result<InstallationResult> {
        Self::apply_installation_with_observer(
            project,
            preview.installation.target_kind,
            preview,
            &preview.installation.plan_sha256,
            executable,
            &FixedClock::at(1_800_000_000_000),
            observer,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn apply_installation_with_observer(
        project: &Path,
        target_kind: InstallationTargetKind,
        preview: &InstallationPreview,
        expected_plan_sha256: &str,
        executable: &Path,
        clock: &dyn ClockSource,
        observer: &mut dyn FnMut(InstallApplyStage, InstallRecoveryState) -> Result<()>,
    ) -> Result<InstallationResult> {
        preview.validate()?;
        if preview.installation.target_kind != target_kind
            || preview.installation.plan_sha256 != expected_plan_sha256
        {
            return Err(ServiceError::Contract(
                "installation apply mode or expected digest does not match the preview".into(),
            ));
        }
        let mut project = canonical_project_root(project, true)?;
        validate_engram_directories(&project, true)?;
        let executable_sha256 = sha256_file(executable)?;
        if executable_sha256 != preview.installation.executable_sha256 {
            return Err(ServiceError::ProjectBindingMismatch);
        }
        let (profile, profile_sha256) =
            DefaultInstallProfile::load(preview.installation.profile_id.as_str())?;
        if profile_sha256 != preview.installation.profile_sha256 {
            return Err(ServiceError::ProjectBindingMismatch);
        }

        let started = Instant::now();
        let layout = EngramLayout::for_plan(&preview.installation)?;
        validate_sealed_preview(preview, &profile, &profile_sha256, &executable_sha256)?;

        // Refused preconditions must not create even an empty admission file.
        // Existing leases are locked before reading mutable recovery state.
        let _lease = match lease::InstallationLease::existing(project.project())? {
            Some(lease) => lease,
            None => {
                admission::inspect(&project, &layout, preview, &profile, executable, clock)?;
                lease::InstallationLease::create_or_acquire(project.project())?
            }
        };
        // Recheck every mutable precondition under the retained native owner.
        // Re-admit private descendants in case another owner completed between
        // the initial read-only preflight and this successful acquisition.
        project.admit_missing_private_directories(true)?;
        let admitted = admission::inspect(&project, &layout, preview, &profile, executable, clock)?;
        observe_install_stage(
            preview,
            started,
            InstallApplyStage::LeaseAcquired,
            InstallRecoveryState::Admitted,
            None,
            observer,
        )?;

        let intent = match admitted {
            admission::Admission::Published { intent, result } => {
                if let Some(intent) = intent {
                    let (_, _, locator_bytes) = read_locator_anchored(&project)?;
                    validate_recovery_engram(&project, &layout, &preview.installation)?;
                    observe_install_stage(
                        preview,
                        started,
                        InstallApplyStage::LocatorPublished,
                        InstallRecoveryState::Published,
                        Some(result.idempotent_replay),
                        observer,
                    )?;
                    cleanup_locator_pending(&project, &layout, &locator_bytes)?;
                    acknowledge_intent(&project, &layout, &intent)?;
                    observe_install_stage(
                        preview,
                        started,
                        InstallApplyStage::IntentAcknowledged,
                        InstallRecoveryState::Complete,
                        Some(result.idempotent_replay),
                        observer,
                    )?;
                }
                return Ok(*result);
            }
            admission::Admission::Recover(intent) => {
                observe_install_stage(
                    preview,
                    started,
                    InstallApplyStage::IntentPublished,
                    InstallRecoveryState::OwnedIntent,
                    None,
                    observer,
                )?;
                *intent
            }
            admission::Admission::New(clock_anchor) => {
                let (intent, recovery_state) = load_or_create_intent(
                    &project,
                    &layout,
                    preview,
                    &clock_anchor,
                    &executable_sha256,
                )?;
                if project_inventory_sha256_excluding(
                    project.project(),
                    &profile,
                    Some(Path::new(&layout.install_intent)),
                )? != preview.installation.project_precondition_sha256
                {
                    return Err(ServiceError::ProjectBindingMismatch);
                }
                validate_recovery_engram(&project, &layout, &preview.installation)?;
                observe_install_stage(
                    preview,
                    started,
                    InstallApplyStage::IntentPublished,
                    recovery_state,
                    None,
                    observer,
                )?;
                intent
            }
        };

        let storage_relative = preview
            .installation
            .managed_path(InstallationManagedPathKind::StorageRoot)
            .relative_path
            .clone();
        let token_relative = preview
            .installation
            .managed_path(InstallationManagedPathKind::TokenKey)
            .relative_path
            .clone();
        let credential_relative = preview
            .installation
            .managed_path(InstallationManagedPathKind::OperatorCredential)
            .relative_path
            .clone();
        create_engram_directories(&mut project, &preview.installation)?;
        validate_recovery_engram(&project, &layout, &preview.installation)?;
        let locator_path = safe_join(&project, LOCATOR_RELATIVE_PATH)?;
        observe_install_stage(
            preview,
            started,
            InstallApplyStage::EngramDirectoriesReady,
            InstallRecoveryState::OwnedIntent,
            None,
            observer,
        )?;
        bootstrap::materialize(&project, &layout, preview, &mut || {
            observe_install_stage(
                preview,
                started,
                InstallApplyStage::BootstrapStaged,
                InstallRecoveryState::OwnedIntent,
                None,
                observer,
            )
        })?;
        observe_install_stage(
            preview,
            started,
            InstallApplyStage::BootstrapReady,
            InstallRecoveryState::OwnedIntent,
            None,
            observer,
        )?;
        let storage_root = safe_join(&project, &storage_relative)?;
        let token_path = safe_join(&project, &token_relative)?;
        let credential_path = safe_join(&project, &credential_relative)?;
        let instance_id = preview.installation.target.instance_id.clone();
        let token_key = intent.token_key()?;
        let engine = match storage_root.metadata() {
            Ok(metadata) if !metadata.file_type().is_symlink() && metadata.is_dir() => {
                RrdEngine::open_existing_anchored_with_options(
                    &storage_root,
                    instance_id.clone(),
                    token_key,
                    super::super::core::installed_storage_options(
                        &preview.installation.configuration.storage,
                    )?,
                )?
            }
            Ok(_) => return Err(ServiceError::ProjectBindingMismatch),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                RrdEngine::create_new_anchored_with_options(
                    &storage_root,
                    instance_id.clone(),
                    token_key,
                    super::super::core::installed_storage_options(
                        &preview.installation.configuration.storage,
                    )?,
                )?
            }
            Err(error) => return Err(ServiceError::Storage(error.to_string())),
        };
        let store_state = classify_installation_store(&engine)?;
        observe_install_stage(
            preview,
            started,
            InstallApplyStage::StorageReady,
            store_state,
            Some(store_state == InstallRecoveryState::Committed),
            observer,
        )?;
        materialize_secret(
            &token_path,
            &token_key,
            store_state == InstallRecoveryState::EmptyStore,
        )?;
        observe_install_stage(
            preview,
            started,
            InstallApplyStage::TokenReady,
            store_state,
            Some(store_state == InstallRecoveryState::Committed),
            observer,
        )?;
        materialize_secret(
            &credential_path,
            intent.credential_bytes(),
            store_state == InstallRecoveryState::EmptyStore,
        )?;
        observe_install_stage(
            preview,
            started,
            InstallApplyStage::CredentialReady,
            store_state,
            Some(store_state == InstallRecoveryState::Committed),
            observer,
        )?;
        let credential_sha256 = intent.credential_sha256().to_owned();
        let token_key_sha256 = intent.token_key_sha256().to_owned();
        let installed_at_unix_ms = intent.clock_anchor().observed_at_unix_ms;

        let memory_plan = build_memory_engram_plan(
            &PersistMemoryEngram {
                scope: format!("instance:{instance_id}"),
                seat: preview.installation.initial_seat.clone(),
                representations: Vec::new(),
                valid_from: installed_at_unix_ms,
            },
            None,
        )?;
        let bootstrap_session = CorrelationId::new("installation-bootstrap")
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        let runtime_mutations = memory_plan
            .mutations
            .iter()
            .map(|mutation| public_runtime_mutation(mutation, &bootstrap_session))
            .collect::<Result<Vec<_>>>()?;
        let runtime_commit = RuntimeCommit {
            scope: ScopeId::new(format!("instance:{instance_id}")).map_err(core_contract)?,
            at: installed_at_unix_ms,
            actor: "rrflow:installer".into(),
            expected_cursor: 0,
            mutations: runtime_mutations,
        };
        runtime_commit.validate().map_err(core_contract)?;
        let runtime_commit_sha256 = runtime_commit.digest();
        let expected_runtime_cursor = runtime_commit.mutations.len() as u64;

        let resource = ResourcePath {
            segments: vec![
                ResourceId::new(ResourceKind::Instance, instance_id.as_str())
                    .map_err(|error| ServiceError::Contract(error.to_string()))?,
            ],
        };
        let grants = preview
            .installation
            .initial_grants
            .iter()
            .copied()
            .map(|action| ResourceGrant {
                action,
                resource_prefix: resource.clone(),
                data_policy: None,
            })
            .collect();
        let principal = Principal {
            id: preview.installation.initial_principal_id.clone(),
            kind: PrincipalKind::User,
            credential_sha256: credential_sha256.clone(),
            credential_revision: 1,
            not_before_unix_ms: installed_at_unix_ms,
            expires_at_unix_ms: u64::MAX,
            disabled: false,
            role_ids: BTreeSet::new(),
            grants,
        };
        let security = SecurityState {
            format_version: SECURITY_FORMAT,
            revision: 1,
            principals: BTreeMap::from([(principal.id.clone(), principal)]),
            roles: BTreeMap::new(),
            identity_bindings: BTreeMap::new(),
            jwt_issuers: BTreeMap::new(),
        };
        let request_id = format!(
            "install-request-{}",
            &preview.installation.plan_sha256[..16]
        );
        let operation_id = format!(
            "install-operation-{}",
            &preview.installation.plan_sha256[..16]
        );
        let mut transitions = rrd_security::prepare_security_initialization(
            &instance_id,
            &security,
            installed_at_unix_ms,
            "rrflow:installer",
            &request_id,
            &operation_id,
        )?;
        let bootstrap_control_sequence = transitions.len() as u64 + 5;
        let mut installed = InstalledEngramRecord {
            format_version: INSTALLED_RECORD_FORMAT,
            installation: preview.installation.clone(),
            attunement_plan_sha256: preview.attunement.plan_sha256.clone(),
            credential_sha256: credential_sha256.clone(),
            token_key_sha256,
            runtime_commit_sha256: runtime_commit_sha256.clone(),
            expected_runtime_cursor,
            bootstrap_control_sequence,
            clock_anchor: intent.clock_anchor().clone(),
            record_sha256: zero_digest(),
        };
        installed.record_sha256 = installed_record_sha256(&installed)?;
        installed.validate()?;
        let mut checkpoint = InstallationCheckpoint {
            format_version: INSTALL_CHECKPOINT_FORMAT,
            plan_sha256: preview.installation.plan_sha256.clone(),
            runtime_commit_sha256: runtime_commit_sha256.clone(),
            runtime_cursor: expected_runtime_cursor,
            control_sequence: bootstrap_control_sequence,
            installed_record_sha256: installed.record_sha256.clone(),
            clock_anchor_unix_ms: installed_at_unix_ms,
            checkpoint_sha256: zero_digest(),
        };
        checkpoint.checkpoint_sha256 = installation_checkpoint_sha256(&checkpoint)?;
        checkpoint.validate()?;
        let job = AttunementJob {
            contract_version: ATTUNEMENT_CONTRACT_VERSION,
            id: canonical_prefixed("job", &preview.attunement.plan_sha256)?,
            plan_id: preview.attunement.id.clone(),
            plan_sha256: preview.attunement.plan_sha256.clone(),
            revision: 1,
            state: AttunementJobState::Pending,
            current_phase: None,
            checkpoints: Vec::new(),
            lease: None,
            failure: None,
            created_at_unix_ms: installed_at_unix_ms,
            updated_at_unix_ms: installed_at_unix_ms,
        };
        job.validate(&preview.attunement)
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        transitions.extend([
            control_create(
                &engram_configuration_key(&instance_id),
                &preview.installation.configuration,
                installed_at_unix_ms,
                &request_id,
                &operation_id,
                "configuration.installed",
            )?,
            control_create(
                INSTALLED_RECORD_KEY,
                &installed,
                installed_at_unix_ms,
                &request_id,
                &operation_id,
                "install.record",
            )?,
            control_create(
                INSTALL_CHECKPOINT_KEY,
                &checkpoint,
                installed_at_unix_ms,
                &request_id,
                &operation_id,
                "install.checkpoint",
            )?,
            control_create(
                &attunement_plan_key(&instance_id, &preview.attunement.id),
                &preview.attunement,
                installed_at_unix_ms,
                &request_id,
                &operation_id,
                "attunement.plan.created",
            )?,
            control_create(
                &attunement_job_key(&instance_id, &job.id),
                &job,
                installed_at_unix_ms,
                &request_id,
                &operation_id,
                "attunement.job.created",
            )?,
        ]);
        bootstrap::validate_transaction(preview, &runtime_commit, &transitions)?;
        let committed_before = store_state == InstallRecoveryState::Committed;
        if !committed_before {
            require_empty_installation(&engine, &transitions)?;
            let (outcome, entries) = engine
                .storage
                .runtime()
                .commit_with_control_transitions(&runtime_commit, &transitions)?;
            if outcome.commit_id != runtime_commit_sha256
                || outcome.last_cursor != expected_runtime_cursor
                || entries.last().map(|entry| entry.sequence) != Some(bootstrap_control_sequence)
            {
                return Err(ServiceError::StorageConflict(
                    "installation commit outcome differs from the sealed bootstrap".into(),
                ));
            }
        }
        drop(engine);
        let read = require_committed_installation(
            &storage_root,
            &runtime_commit,
            &transitions,
            expected_runtime_cursor,
            bootstrap_control_sequence,
            super::super::core::installed_storage_options(
                &preview.installation.configuration.storage,
            )?,
        )?;
        observe_install_stage(
            preview,
            started,
            InstallApplyStage::EngineCommitted,
            InstallRecoveryState::Committed,
            Some(committed_before),
            observer,
        )?;
        let locator = ProjectLocator {
            format_version: LOCATOR_FORMAT,
            product_version: env!("CARGO_PKG_VERSION").into(),
            identity: preview.installation.target.clone(),
            storage_root: storage_relative,
            token_key: token_relative,
            operator_credential: credential_relative,
            plan_sha256: preview.installation.plan_sha256.clone(),
            profile_id: preview.installation.profile_id.clone(),
            profile_sha256: preview.installation.profile_sha256.clone(),
            executable_sha256: preview.installation.executable_sha256.clone(),
            installed_record_sha256: installed.record_sha256.clone(),
            configuration_revision: preview.installation.configuration.revision,
            storage: preview.installation.configuration.storage.clone(),
            configuration_sha256: preview
                .installation
                .configuration
                .configuration_sha256
                .clone(),
        };
        locator.validate()?;
        let locator_bytes = toml::to_string(&locator)
            .map_err(|error| ServiceError::Contract(error.to_string()))?
            .into_bytes();
        let locator_sha256 = digest::sha256_hex(&locator_bytes);
        let locator_pending = safe_join(&project, &layout.locator_pending)?;
        if let Some(proof) = &layout.locator_proof {
            publish_into_retained_parent(
                &locator_path,
                &locator_pending,
                &safe_join(&project, proof)?,
                &locator_bytes,
                &preview.installation.plan_sha256,
                &mut || Ok(()),
            )?;
        } else {
            publish_new_file(&locator_path, &locator_pending, &locator_bytes)?;
        }
        observe_install_stage(
            preview,
            started,
            InstallApplyStage::LocatorPublished,
            InstallRecoveryState::Published,
            Some(committed_before),
            observer,
        )?;
        let result = build_installation_result(
            preview,
            &job.id,
            &installed,
            &read.manifest_id,
            read.commit_cursor,
            bootstrap_control_sequence,
            &credential_sha256,
            &locator_sha256,
            committed_before,
        )?;
        acknowledge_intent(&project, &layout, &intent)?;
        observe_install_stage(
            preview,
            started,
            InstallApplyStage::IntentAcknowledged,
            InstallRecoveryState::Complete,
            Some(committed_before),
            observer,
        )?;
        Ok(result)
    }
}
