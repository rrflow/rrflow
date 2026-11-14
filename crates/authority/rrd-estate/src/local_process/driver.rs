//! The local process driver and its estate driver implementation.

use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ProcessRecord {
    pub(super) format: u16,
    pub(super) instance_id: CanonicalId,
    pub(super) operation_id: CanonicalId,
    pub(super) deployment_ref: CanonicalId,
    pub(super) version: String,
    pub(super) configuration_sha256: String,
    pub(super) executable: PathBuf,
    pub(super) executable_sha256: String,
    pub(super) pid: u32,
    pub(super) process_start_time_unix_s: u64,
    #[serde(default)]
    pub(super) shutdown: LocalShutdown,
}

pub struct LocalProcessDriver {
    state_root: PathBuf,
    catalog: LocalDeploymentCatalog,
}

impl LocalProcessDriver {
    pub fn new(
        state_root: impl Into<PathBuf>,
        catalog: LocalDeploymentCatalog,
    ) -> std::result::Result<Self, String> {
        catalog.validate()?;
        let state_root = state_root.into();
        if !state_root.is_absolute() {
            return Err("local process state root must be absolute".into());
        }
        std::fs::create_dir_all(state_root.join("instances"))
            .map_err(|error| format!("cannot create local instance root: {error}"))?;
        std::fs::create_dir_all(state_root.join("processes"))
            .map_err(|error| format!("cannot create local process root: {error}"))?;
        Ok(Self {
            state_root,
            catalog,
        })
    }

    pub fn process_record_path(&self, instance: &CanonicalId) -> PathBuf {
        self.state_root
            .join("processes")
            .join(format!("{instance}.json"))
    }

    fn deployment<'a>(
        &'a self,
        request: &DriverRequest,
    ) -> std::result::Result<&'a LocalDeployment, DriverError> {
        let deployment = self
            .catalog
            .deployments
            .get(request.desired.deployment_ref.as_str())
            .ok_or_else(|| {
                permanent(
                    request,
                    "desired deployment reference is not in the trusted catalogue",
                )
            })?;
        if deployment.version != request.desired.version {
            return Err(permanent(
                request,
                "desired version does not match the trusted deployment catalogue",
            ));
        }
        let executable = std::fs::canonicalize(&deployment.executable).map_err(|error| {
            permanent(
                request,
                format!("cannot resolve trusted executable: {error}"),
            )
        })?;
        if executable != deployment.executable {
            return Err(permanent(
                request,
                "trusted executable path must already be canonical",
            ));
        }
        let observed_sha256 = file_sha256(&executable).map_err(|error| {
            permanent(
                request,
                format!("cannot authenticate trusted executable: {error}"),
            )
        })?;
        if observed_sha256 != deployment.executable_sha256 {
            return Err(permanent(request, "trusted executable digest changed"));
        }
        Ok(deployment)
    }

    fn load_record(
        &self,
        instance: &CanonicalId,
    ) -> std::result::Result<Option<ProcessRecord>, DriverError> {
        let path = self.process_record_path(instance);
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => {
                return Err(DriverError::retryable(
                    format!("cannot read process record: {error}"),
                    digest::sha256_hex(format!("process-record-read:{instance}").as_bytes()),
                ));
            }
        };
        let record: ProcessRecord = serde_json::from_slice(&bytes).map_err(|error| {
            DriverError::permanent(
                format!("process record is corrupt: {error}"),
                digest::sha256_hex(&bytes),
            )
        })?;
        if record.format != PROCESS_RECORD_FORMAT || record.instance_id != *instance {
            return Err(DriverError::permanent(
                "process record identity is invalid",
                digest::sha256_hex(&bytes),
            ));
        }
        Ok(Some(record))
    }

    fn inspect_record(
        &self,
        record: &ProcessRecord,
    ) -> std::result::Result<ProcessIdentity, DriverError> {
        let system = system_for(record.pid);
        let Some(process) = system.process(Pid::from_u32(record.pid)) else {
            return Ok(ProcessIdentity::Exited);
        };
        let executable = process
            .exe()
            .and_then(|path| std::fs::canonicalize(path).ok());
        if process.start_time() != record.process_start_time_unix_s
            || executable
                .as_deref()
                .is_none_or(|path| !same_executable_file(path, &record.executable))
        {
            return Ok(ProcessIdentity::Foreign);
        }
        Ok(ProcessIdentity::Owned)
    }

    fn spawn(
        &self,
        request: &DriverRequest,
        deployment: &LocalDeployment,
    ) -> std::result::Result<ProcessRecord, DriverError> {
        let instance_root = self
            .state_root
            .join("instances")
            .join(request.instance_id.as_str());
        std::fs::create_dir_all(&instance_root).map_err(|error| {
            retryable(
                request,
                format!("cannot create instance directory: {error}"),
            )
        })?;
        let stdout_path = instance_root.join(PROCESS_STDOUT_LOG);
        let stdout = open_process_log(&stdout_path).map_err(|error| {
            retryable(
                request,
                format!("cannot open managed process stdout log: {error}"),
            )
        })?;
        let stderr_path = instance_root.join(PROCESS_STDERR_LOG);
        let stderr = open_process_log(&stderr_path).map_err(|error| {
            retryable(
                request,
                format!("cannot open managed process stderr log: {error}"),
            )
        })?;
        run_preparation(
            request,
            deployment,
            &instance_root,
            &stdout,
            &stderr,
            &stderr_path,
        )?;
        prepare_shutdown_artifacts(&instance_root, &deployment.shutdown).map_err(|error| {
            retryable(
                request,
                format!("cannot prepare graceful shutdown artifacts: {error}"),
            )
        })?;
        prepare_readiness_artifact(&instance_root, &deployment.readiness).map_err(|error| {
            retryable(
                request,
                format!("cannot prepare readiness artifact: {error}"),
            )
        })?;
        let arguments = deployment
            .arguments
            .iter()
            .map(|argument| resolve_argument(argument, request, &instance_root))
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let mut command = Command::new(&deployment.executable);
        command
            .args(&arguments)
            .current_dir(&instance_root)
            .env_clear()
            .envs(&deployment.environment)
            .stdin(Stdio::null())
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr));
        configure_platform_environment(&mut command, request)?;
        let launch = rrd_core::child_launch_lock();
        let spawned = command.spawn();
        drop(launch);
        let mut child = spawned
            .map_err(|error| retryable(request, format!("cannot spawn instance: {error}")))?;
        let pid = child.id();

        let discovery_deadline = Instant::now() + PROCESS_DISCOVERY_TIMEOUT;
        let mut candidate: Option<(u64, PathBuf)> = None;
        let mut candidate_since = None;
        let mut mismatched_executable = None;
        let (process_start_time_unix_s, executable) = loop {
            if let Some(status) = child.try_wait().map_err(|error| {
                retryable(request, format!("cannot inspect spawned process: {error}"))
            })? {
                return Err(retryable(
                    request,
                    spawned_exit_message(status, &stderr_path),
                ));
            }
            let system = system_for(pid);
            if let Some(process) = system.process(Pid::from_u32(pid)) {
                if let Some(executable) = process
                    .exe()
                    .and_then(|path| std::fs::canonicalize(path).ok())
                {
                    if !same_executable_file(&executable, &deployment.executable) {
                        if let Some(status) = child.try_wait().map_err(|error| {
                            retryable(request, format!("cannot inspect spawned process: {error}"))
                        })? {
                            return Err(retryable(
                                request,
                                spawned_exit_message(status, &stderr_path),
                            ));
                        }
                        // Process observers can briefly expose the pre-exec
                        // parent image. Never authenticate or persist it.
                        mismatched_executable = Some(executable);
                        candidate = None;
                        candidate_since = None;
                    } else {
                        mismatched_executable = None;
                        let observed = (process.start_time(), executable);
                        if candidate.as_ref() != Some(&observed) {
                            candidate = Some(observed);
                            candidate_since = Some(Instant::now());
                        } else if candidate_since
                            .is_some_and(|since| since.elapsed() >= PROCESS_START_STABILITY)
                        {
                            break candidate
                                .take()
                                .expect("a stable process identity is present");
                        }
                    }
                }
            }
            if Instant::now() >= discovery_deadline {
                let _ = child.kill();
                let _ = child.wait();
                if let Some(executable) = mismatched_executable {
                    return Err(permanent(
                        request,
                        format!(
                            "live spawned process executable does not match the trusted catalogue: expected {}, observed {}",
                            deployment.executable.display(),
                            executable.display()
                        ),
                    ));
                }
                return Err(retryable(
                    request,
                    "spawned process identity did not remain stable",
                ));
            }
            thread::sleep(Duration::from_millis(10));
        };
        debug_assert!(same_executable_file(&executable, &deployment.executable));
        wait_for_readiness(
            request,
            deployment,
            &instance_root,
            &stderr_path,
            &mut child,
        )?;
        let record = ProcessRecord {
            format: PROCESS_RECORD_FORMAT,
            instance_id: request.instance_id.clone(),
            operation_id: request.operation_id.clone(),
            deployment_ref: request.desired.deployment_ref.clone(),
            version: request.desired.version.clone(),
            configuration_sha256: request.desired.configuration_sha256.clone(),
            executable,
            executable_sha256: deployment.executable_sha256.clone(),
            pid,
            process_start_time_unix_s,
            shutdown: deployment.shutdown.clone(),
        };
        if let Err(error) = write_record(&self.process_record_path(&request.instance_id), &record) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(retryable(
                request,
                format!("cannot persist process record; spawned child was terminated: {error}"),
            ));
        }
        drop(child);
        Ok(record)
    }

    fn stop(
        &self,
        request: &DriverRequest,
        record: &ProcessRecord,
    ) -> std::result::Result<(), DriverError> {
        match self.inspect_record(record)? {
            ProcessIdentity::Exited => {}
            ProcessIdentity::Foreign => {
                return Err(permanent(
                    request,
                    "refusing to signal a PID whose executable/start identity changed",
                ));
            }
            ProcessIdentity::Owned => {
                if let LocalShutdown::RequestFile {
                    request: request_path,
                    complete,
                    timeout_ms,
                } = &record.shutdown
                {
                    let instance_root = self
                        .state_root
                        .join("instances")
                        .join(request.instance_id.as_str());
                    write_shutdown_request(&instance_root.join(request_path), request, record)
                        .map_err(|error| {
                            retryable(
                                request,
                                format!("cannot persist graceful shutdown request: {error}"),
                            )
                        })?;
                    if wait_for_owned_exit(record, Duration::from_millis(*timeout_ms), request)? {
                        if !instance_root.join(complete).is_file() {
                            return Err(permanent(
                                request,
                                "managed process exited without graceful shutdown confirmation",
                            ));
                        }
                        return self.remove_record(request);
                    }
                }
                kill_owned_process(record, request)?;
            }
        }
        self.remove_record(request)
    }

    fn remove_record(&self, request: &DriverRequest) -> std::result::Result<(), DriverError> {
        let path = self.process_record_path(&request.instance_id);
        match std::fs::remove_file(&path) {
            Ok(()) => sync_parent(&path).map_err(|error| {
                retryable(
                    request,
                    format!("cannot sync stopped process record removal: {error}"),
                )
            }),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(retryable(
                request,
                format!("cannot remove stopped process record: {error}"),
            )),
        }
    }

    fn ensure_running(
        &self,
        request: &DriverRequest,
        deployment: &LocalDeployment,
    ) -> std::result::Result<(), DriverError> {
        if let Some(record) = self.load_record(&request.instance_id)? {
            match self.inspect_record(&record)? {
                ProcessIdentity::Foreign => {
                    return Err(permanent(
                        request,
                        "process record now identifies a foreign process",
                    ));
                }
                ProcessIdentity::Exited => {
                    let record_path = self.process_record_path(&request.instance_id);
                    match std::fs::remove_file(&record_path) {
                        Ok(()) => sync_parent(&record_path).map_err(|error| {
                            retryable(
                                request,
                                format!("cannot sync stale process record removal: {error}"),
                            )
                        })?,
                        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                        Err(error) => {
                            return Err(retryable(
                                request,
                                format!("cannot remove stale process record: {error}"),
                            ));
                        }
                    }
                }
                ProcessIdentity::Owned => {
                    let target_matches = record.deployment_ref == request.desired.deployment_ref
                        && record.version == request.desired.version
                        && record.configuration_sha256 == request.desired.configuration_sha256;
                    if record.operation_id == request.operation_id
                        || (target_matches
                            && !matches!(
                                request.kind,
                                OperationKind::Restart | OperationKind::Upgrade
                            ))
                    {
                        return Ok(());
                    }
                    self.stop(request, &record)?;
                }
            }
        }
        self.spawn(request, deployment).map(|_| ())
    }

    fn ensure_stopped(&self, request: &DriverRequest) -> std::result::Result<(), DriverError> {
        if let Some(record) = self.load_record(&request.instance_id)? {
            self.stop(request, &record)?;
        }
        Ok(())
    }
}

impl EstateDriver for LocalProcessDriver {
    fn apply(&mut self, request: &DriverRequest) -> std::result::Result<DriverEffect, DriverError> {
        match request.desired.phase {
            crate::DesiredPhase::Running => {
                let deployment = self.deployment(request)?;
                self.ensure_running(request, deployment)?;
            }
            crate::DesiredPhase::Stopped | crate::DesiredPhase::Absent => {
                self.ensure_stopped(request)?;
            }
        }
        Ok(DriverEffect {
            evidence_sha256: effect_digest(request),
        })
    }

    fn observe(
        &mut self,
        request: &DriverRequest,
    ) -> std::result::Result<DriverObservation, DriverError> {
        let record = self.load_record(&request.instance_id)?;
        let (phase, version, process_id, error) = match (request.desired.phase, record) {
            (crate::DesiredPhase::Running, Some(record)) => match self.inspect_record(&record)? {
                ProcessIdentity::Owned => (
                    ObservedPhase::Running,
                    Some(record.version),
                    Some(record.pid),
                    None,
                ),
                ProcessIdentity::Exited => (
                    ObservedPhase::Failed,
                    Some(record.version),
                    None,
                    Some("managed process exited after apply".into()),
                ),
                ProcessIdentity::Foreign => (
                    ObservedPhase::Failed,
                    Some(record.version),
                    None,
                    Some("managed PID identity changed".into()),
                ),
            },
            (crate::DesiredPhase::Running, None) => (
                ObservedPhase::Failed,
                None,
                None,
                Some("managed process record is absent after apply".into()),
            ),
            (crate::DesiredPhase::Stopped, None) => (ObservedPhase::Stopped, None, None, None),
            (crate::DesiredPhase::Absent, None) => (ObservedPhase::Absent, None, None, None),
            (_, Some(record)) => match self.inspect_record(&record)? {
                ProcessIdentity::Owned => (
                    ObservedPhase::Failed,
                    Some(record.version),
                    Some(record.pid),
                    Some("managed process is still running after stop".into()),
                ),
                ProcessIdentity::Exited => match request.desired.phase {
                    crate::DesiredPhase::Stopped => (ObservedPhase::Stopped, None, None, None),
                    _ => (ObservedPhase::Absent, None, None, None),
                },
                ProcessIdentity::Foreign => (
                    ObservedPhase::Failed,
                    Some(record.version),
                    None,
                    Some("refusing to treat a foreign PID as stopped".into()),
                ),
            },
        };
        let evidence_sha256 = digest::sha256_hex(
            &serde_json::to_vec(&(&request.operation_id, phase, &version, process_id, &error))
                .expect("local observation fields serialize"),
        );
        Ok(DriverObservation {
            phase,
            version,
            process_id,
            evidence_sha256,
            error,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ProcessIdentity {
    Owned,
    Exited,
    Foreign,
}
