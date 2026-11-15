//! Opening and inspecting an installed engram and its operator key.

use super::*;

impl RrdEngine {
    pub fn open_installed(project: &Path, executable: &Path) -> Result<Self> {
        let clock = HostSystemClock;
        Self::open_installed_with_clock(project, executable, &clock)
    }

    pub(in super::super) fn open_installed_with_clock(
        project: &Path,
        executable: &Path,
        clock: &dyn ClockSource,
    ) -> Result<Self> {
        let project = canonical_project_root(project)?;
        Self::open_installed_anchored(&project, executable, clock)
    }

    pub(in super::super) fn open_installed_anchored(
        project: &EstateFilesystem,
        executable: &Path,
        clock: &dyn ClockSource,
    ) -> Result<Self> {
        let preflight =
            Self::with_installed_binding(project, executable, clock, "install.open.preflight", Ok)?;
        require_accepted(&preflight.clock)?;
        let (project, locator, _) = read_locator_anchored(project)?;
        let (_, profile_sha256) = DefaultInstallProfile::load(locator.profile_id.as_str())?;
        if sha256_file(executable)? != locator.executable_sha256
            || profile_sha256 != locator.profile_sha256
        {
            return Err(ServiceError::ProjectBindingMismatch);
        }
        let token_path = safe_join(&project, &locator.token_key)?;
        let storage_root = safe_join(&project, &locator.storage_root)?;
        let token_key = read_token_key(&token_path)
            .map_err(|error| ServiceError::Storage(error.to_string()))?;
        #[cfg(feature = "crash-test")]
        let _recovery_crash = rrd_store::crash_test::enter_recovery(storage_root.diagnostic_path());
        let mut engine = RrdEngine::open_existing_anchored_with_options(
            &storage_root,
            locator.identity.instance_id.clone(),
            token_key,
            super::super::core::installed_storage_options(&locator.storage)?,
        )?;
        let bytes = engine
            .storage
            .control()
            .get(INSTALLED_RECORD_KEY)?
            .ok_or_else(|| ServiceError::Storage("installed engram record is missing".into()))?;
        let installed: InstalledEstateRecord = decode_json(&bytes, "installed engram record")?;
        installed.validate()?;
        require_locator_record_match(&locator, &installed)?;
        bootstrap::verify(&project, &installed.installation)?;
        if locator.identity.instance_id != preflight.locator.identity.instance_id
            || locator.plan_sha256 != preflight.locator.plan_sha256
            || installed.clock_anchor != preflight.clock.anchor
        {
            return Err(ServiceError::ProjectBindingMismatch);
        }
        let configuration_bytes = engine
            .storage
            .control()
            .get(&engram_configuration_key(&locator.identity.instance_id))?
            .ok_or_else(|| ServiceError::Storage("engram configuration is missing".into()))?;
        let configuration: EstateConfiguration =
            decode_json(&configuration_bytes, "engram configuration")?;
        configuration
            .validate()
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        if configuration.storage != locator.storage
            || configuration.revision != locator.configuration_revision
            || configuration.configuration_sha256 != locator.configuration_sha256
        {
            return Err(ServiceError::ProjectBindingMismatch);
        }
        engine.bind_installed_state(
            locator.identity.clone(),
            installed.installation.deployment.clone(),
            configuration,
            safe_join(&project, STORAGE_DIRECTORY)?,
        )?;
        if !engine.security_enforced()? {
            return Err(ServiceError::Storage(
                "installed security authority is missing".into(),
            ));
        }
        engine.installed_project_root = Some(project.project().clone());
        engine.installed_estate_root = Some(project.clone());
        Ok(engine)
    }

    /// The installed operator credential, read when needed and never kept.
    pub(in crate::engine) fn installed_operator_key(&self) -> Result<ApiKeyDocument> {
        let project = self
            .installed_estate_root
            .as_ref()
            .ok_or(ServiceError::ProjectBindingMismatch)?;
        let (project, locator, _) = read_locator_anchored(project)?;
        let document = read_api_key_document(&safe_join(&project, &locator.operator_credential)?)
            .map_err(|error| ServiceError::Storage(error.to_string()))?;
        if document.plan_sha256 != locator.plan_sha256 {
            return Err(ServiceError::ProjectBindingMismatch);
        }
        Ok(document)
    }

    pub fn read_installed_api_key(project: &Path) -> Result<ApiKeyDocument> {
        let (project, locator, _) = read_locator(project)?;
        let document = read_api_key_document(&safe_join(&project, &locator.operator_credential)?)
            .map_err(|error| ServiceError::Storage(error.to_string()))?;
        if document.plan_sha256 != locator.plan_sha256 {
            return Err(ServiceError::ProjectBindingMismatch);
        }
        Ok(document)
    }

    /// Reads the public, project-local installed-engine locator. The locator
    /// contains identities and paths but never credential material.
    pub fn read_project_locator(project: &Path) -> Result<ProjectLocator> {
        read_locator(project).map(|(_, locator, _)| locator)
    }

    pub fn inspect_installed(
        project: &Path,
        executable: &Path,
    ) -> Result<InstallationVerificationReport> {
        let clock = HostSystemClock;
        Self::inspect_installed_with_clock(project, executable, &clock)
    }

    pub(in super::super) fn inspect_installed_with_clock(
        project: &Path,
        executable: &Path,
        clock: &dyn ClockSource,
    ) -> Result<InstallationVerificationReport> {
        Self::inspect_installed_with_clock_at_boundary(
            project,
            executable,
            clock,
            "install.inspect",
        )
    }

    pub(super) fn inspect_installed_with_clock_at_boundary(
        project: &Path,
        executable: &Path,
        clock: &dyn ClockSource,
        boundary: &'static str,
    ) -> Result<InstallationVerificationReport> {
        let project = canonical_project(project, InstallationTargetKind::ExistingProject)?;
        Self::inspect_installed_anchored(&project, executable, clock, boundary)
    }

    pub(in super::super) fn inspect_installed_anchored(
        project: &EstateFilesystem,
        executable: &Path,
        clock: &dyn ClockSource,
        boundary: &'static str,
    ) -> Result<InstallationVerificationReport> {
        Self::with_installed_binding(project, executable, clock, boundary, |mut binding| {
            let project_inventory_sha256 =
                project_inventory_sha256(project.project(), &binding.profile)?;
            let attunement_source_current =
                project_inventory_sha256 == binding.source_precondition_sha256;
            // Preserve the public report's evidence order and actual digest.
            binding.checks.insert(
                3,
                InstallationVerificationCheck {
                    id: CanonicalId::new("project-inventory")
                        .map_err(|error| ServiceError::Contract(error.to_string()))?,
                    passed: true,
                    evidence_sha256: project_inventory_sha256.clone(),
                },
            );
            Ok(InstallationVerificationReport {
                status: if binding.clock.passed() {
                    InstallationVerificationStatus::Passed
                } else {
                    InstallationVerificationStatus::Failed
                },
                product_version: binding.locator.product_version,
                instance_id: binding.locator.identity.instance_id,
                plan_sha256: binding.locator.plan_sha256,
                project_inventory_sha256,
                attunement_source_current,
                runtime_manifest_sha256: binding.runtime_manifest_sha256,
                runtime_cursor: binding.physical.runtime_cursor,
                control_journal_sequence: binding.physical.control_journal_sequence,
                clock: binding.clock,
                physical: binding.physical,
                checks: binding.checks,
            })
        })
    }

    pub(super) fn with_installed_binding<T>(
        project: &EstateFilesystem,
        executable: &Path,
        clock: &dyn ClockSource,
        boundary: &'static str,
        finish: impl FnOnce(InstalledBindingInspection) -> Result<T>,
    ) -> Result<T> {
        let (project, locator, locator_bytes) = read_locator_anchored(project)?;

        let executable_sha256 = sha256_file(executable)?;
        if executable_sha256 != locator.executable_sha256 {
            return Err(ServiceError::ProjectBindingMismatch);
        }
        let (profile, profile_sha256) = DefaultInstallProfile::load(locator.profile_id.as_str())?;
        if profile_sha256 != locator.profile_sha256 {
            return Err(ServiceError::ProjectBindingMismatch);
        }
        let token_key = read_token_key(&safe_join(&project, &locator.token_key)?)
            .map_err(|error| ServiceError::Storage(error.to_string()))?;
        let credential = read_api_key_document(&safe_join(&project, &locator.operator_credential)?)
            .map_err(|error| ServiceError::Storage(error.to_string()))?;
        let storage_root = safe_join(&project, &locator.storage_root)?;
        super::super::core::validate_existing_object_layout(&storage_root)?;
        let inspector = RrflowKvInspector::open_anchored_with_options(
            &storage_root,
            super::super::core::installed_storage_options(&locator.storage)?,
        )?;
        let installed: InstalledEstateRecord =
            decode_control(&inspector, INSTALLED_RECORD_KEY, "installed engram record")?;
        installed.validate()?;
        require_locator_record_match(&locator, &installed)?;
        bootstrap::verify(&project, &installed.installation)?;
        let layout = EstateLayout::new(
            &installed.installation.storage_root_id,
            &installed.installation.initial_principal_id,
        )?;
        if installed.installation.bootstrap != bootstrap::build(&profile, &layout)? {
            return Err(ServiceError::ProjectBindingMismatch);
        }
        let configuration: EstateConfiguration = decode_control(
            &inspector,
            &engram_configuration_key(&locator.identity.instance_id),
            "engram configuration",
        )?;
        configuration
            .validate()
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        if configuration.storage != locator.storage
            || configuration.revision != locator.configuration_revision
            || configuration.configuration_sha256 != locator.configuration_sha256
        {
            return Err(ServiceError::ProjectBindingMismatch);
        }
        let clock_assessment = assess(
            clock.observe(),
            &installed.clock_anchor,
            &configuration.clock,
            boundary,
        )?;
        if credential.plan_sha256 != installed.installation.plan_sha256
            || credential.principal_id != installed.installation.initial_principal_id
            || digest::sha256_hex(credential.credential().as_bytes()) != installed.credential_sha256
            || digest::sha256_hex(&token_key) != installed.token_key_sha256
            || inspector.inspection().runtime_cursor < installed.expected_runtime_cursor
            || inspector.inspection().control_journal_sequence
                < installed.bootstrap_control_sequence
        {
            return Err(ServiceError::ProjectBindingMismatch);
        }
        let security_key = format!(
            "server/state/{}/security/policy",
            locator.identity.instance_id
        );
        let security: SecurityState = decode_control(&inspector, &security_key, "security state")?;
        security.validate()?;
        let principal = security
            .principals
            .get(&installed.installation.initial_principal_id)
            .ok_or(ServiceError::Unauthenticated)?;
        if principal.credential_sha256 != installed.credential_sha256
            || principal
                .grants
                .iter()
                .map(|grant| grant.action)
                .collect::<Vec<_>>()
                != installed.installation.initial_grants
        {
            return Err(ServiceError::ProjectBindingMismatch);
        }
        let attunement: AttunementPlan = decode_control(
            &inspector,
            &attunement_plan_key(
                &locator.identity.instance_id,
                &installed.installation.attunement_plan_id,
            ),
            "attunement plan",
        )?;
        installed
            .installation
            .validate_attunement_plan(&attunement)
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        let job_id = canonical_prefixed("job", &attunement.plan_sha256)?;
        let job: AttunementJob = decode_control(
            &inspector,
            &attunement_job_key(&locator.identity.instance_id, &job_id),
            "attunement job",
        )?;
        job.validate(&attunement)
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        let checkpoint: InstallationCheckpoint = decode_control(
            &inspector,
            INSTALL_CHECKPOINT_KEY,
            "installation checkpoint",
        )?;
        checkpoint.validate()?;
        if checkpoint.installed_record_sha256 != installed.record_sha256
            || checkpoint.runtime_cursor != installed.expected_runtime_cursor
            || checkpoint.control_sequence != installed.bootstrap_control_sequence
            || checkpoint.clock_anchor_unix_ms != installed.clock_anchor.observed_at_unix_ms
        {
            return Err(ServiceError::ProjectBindingMismatch);
        }

        let runtime_manifest_sha256 = runtime_manifest_from_inspection(&inspector, &locator)?;
        let evidence = [
            ("locator", digest::sha256_hex(&locator_bytes)),
            ("profile", profile_sha256),
            ("executable", executable_sha256),
            ("physical", json_sha256(&inspector.inspection())?),
            ("installed-record", installed.record_sha256.clone()),
            ("configuration", configuration.configuration_sha256.clone()),
            ("credentials", installed.credential_sha256.clone()),
            ("security", json_sha256(&security)?),
            ("runtime", runtime_manifest_sha256.clone()),
            ("attunement", attunement.plan_sha256),
            ("checkpoint", checkpoint.checkpoint_sha256),
        ];
        let mut checks = evidence
            .into_iter()
            .map(|(id, evidence_sha256)| {
                Ok(InstallationVerificationCheck {
                    id: CanonicalId::new(id)
                        .map_err(|error| ServiceError::Contract(error.to_string()))?,
                    passed: true,
                    evidence_sha256,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        checks.push(InstallationVerificationCheck {
            id: CanonicalId::new("clock")
                .map_err(|error| ServiceError::Contract(error.to_string()))?,
            passed: clock_assessment.passed(),
            evidence_sha256: json_sha256(&clock_assessment)?,
        });
        // Keep the authenticated physical snapshot alive through the caller's
        // inspection work; opening releases it before acquiring the writer.
        finish(InstalledBindingInspection {
            profile,
            locator,
            source_precondition_sha256: installed.installation.project_precondition_sha256,
            runtime_manifest_sha256,
            clock: clock_assessment,
            physical: inspector.inspection().clone(),
            checks,
        })
    }
}
