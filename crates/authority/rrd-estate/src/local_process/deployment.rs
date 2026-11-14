//! Local deployment arguments, shutdown and readiness modes, and the deployment catalogue.

use super::*;

pub const LOCAL_DEPLOYMENT_FORMAT: u16 = 2;
pub(super) const PROCESS_RECORD_FORMAT: u16 = 1;
pub(super) const PROCESS_DISCOVERY_TIMEOUT: Duration = Duration::from_secs(3);
pub(super) const PROCESS_START_STABILITY: Duration = Duration::from_millis(250);
pub(super) const PROCESS_PREPARATION_TIMEOUT: Duration = Duration::from_secs(10);
pub(super) const PROCESS_STOP_TIMEOUT: Duration = Duration::from_secs(5);
pub(super) const PROCESS_STDOUT_LOG: &str = "RRD.PROCESS.STDOUT.LOG";
pub(super) const PROCESS_STDERR_LOG: &str = "RRD.PROCESS.STDERR.LOG";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "source", content = "value")]
pub enum LocalArgument {
    Literal(String),
    InstanceId,
    InstanceRoot,
    InstancePath(PathBuf),
    DesiredVersion,
    ConfigurationSha256,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case", tag = "strategy")]
pub enum LocalShutdown {
    #[default]
    Immediate,
    RequestFile {
        request: PathBuf,
        complete: PathBuf,
        timeout_ms: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case", tag = "strategy")]
pub enum LocalReadiness {
    #[default]
    ProcessStable,
    File {
        path: PathBuf,
        timeout_ms: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalDeployment {
    pub id: CanonicalId,
    pub version: String,
    pub executable: PathBuf,
    pub executable_sha256: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub preparation_arguments: Vec<LocalArgument>,
    pub arguments: Vec<LocalArgument>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub environment: BTreeMap<String, String>,
    #[serde(default)]
    pub readiness: LocalReadiness,
    #[serde(default)]
    pub shutdown: LocalShutdown,
}

impl LocalDeployment {
    pub fn authenticate(
        id: CanonicalId,
        version: impl Into<String>,
        executable: impl AsRef<Path>,
        preparation_arguments: Vec<LocalArgument>,
        arguments: Vec<LocalArgument>,
        environment: BTreeMap<String, String>,
        shutdown: LocalShutdown,
    ) -> std::result::Result<Self, String> {
        let executable = std::fs::canonicalize(executable.as_ref())
            .map_err(|error| format!("cannot resolve deployment executable: {error}"))?;
        if !executable.is_file() {
            return Err("deployment executable is not a regular file".into());
        }
        let executable_sha256 = file_sha256(&executable)
            .map_err(|error| format!("cannot authenticate deployment executable: {error}"))?;
        let deployment = Self {
            id,
            version: version.into(),
            executable,
            executable_sha256,
            preparation_arguments,
            arguments,
            environment,
            readiness: LocalReadiness::default(),
            shutdown,
        };
        LocalDeploymentCatalog::single(deployment.clone())?;
        Ok(deployment)
    }

    pub fn with_readiness(
        mut self,
        readiness: LocalReadiness,
    ) -> std::result::Result<Self, String> {
        self.readiness = readiness;
        LocalDeploymentCatalog::single(self.clone())?;
        Ok(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalDeploymentCatalog {
    pub format: u16,
    pub deployments: BTreeMap<String, LocalDeployment>,
}

impl LocalDeploymentCatalog {
    pub fn single(deployment: LocalDeployment) -> std::result::Result<Self, String> {
        let key = deployment.id.to_string();
        let catalog = Self {
            format: LOCAL_DEPLOYMENT_FORMAT,
            deployments: BTreeMap::from([(key, deployment)]),
        };
        catalog.validate()?;
        Ok(catalog)
    }

    pub fn load_json(path: &Path) -> std::result::Result<Self, String> {
        let metadata = std::fs::metadata(path)
            .map_err(|error| format!("cannot inspect local deployment catalogue: {error}"))?;
        if metadata.len() > 1024 * 1024 {
            return Err("local deployment catalogue exceeds one MiB".into());
        }
        let bytes = std::fs::read(path)
            .map_err(|error| format!("cannot read local deployment catalogue: {error}"))?;
        let catalog: Self = serde_json::from_slice(&bytes)
            .map_err(|error| format!("cannot decode local deployment catalogue: {error}"))?;
        catalog.validate()?;
        Ok(catalog)
    }

    pub fn sha256(&self) -> String {
        digest::sha256_hex(
            &serde_json::to_vec(self).expect("validated deployment catalogue fields serialize"),
        )
    }

    pub fn validate(&self) -> std::result::Result<(), String> {
        if self.format != LOCAL_DEPLOYMENT_FORMAT || self.deployments.len() > 256 {
            return Err("unsupported or oversized local deployment catalogue".into());
        }
        for (key, deployment) in &self.deployments {
            if key != deployment.id.as_str() {
                return Err(format!("deployment key {key} does not match its id"));
            }
            if deployment.version.is_empty()
                || deployment.version.len() > 256
                || !deployment
                    .version
                    .bytes()
                    .all(|byte| byte.is_ascii_graphic())
            {
                return Err(format!("deployment {key} has an invalid version"));
            }
            if !deployment.executable.is_absolute() {
                return Err(format!("deployment {key} executable must be absolute"));
            }
            validate_sha256(&deployment.executable_sha256)?;
            #[cfg(windows)]
            if deployment
                .executable
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| {
                    extension.eq_ignore_ascii_case("bat") || extension.eq_ignore_ascii_case("cmd")
                })
            {
                return Err(format!(
                    "deployment {key} cannot use a Windows batch or cmd target"
                ));
            }
            if deployment.preparation_arguments.len() > 256
                || deployment.arguments.len() > 256
                || deployment.environment.len() > 128
            {
                return Err(format!(
                    "deployment {key} launch configuration is oversized"
                ));
            }
            for argument in deployment
                .preparation_arguments
                .iter()
                .chain(&deployment.arguments)
            {
                match argument {
                    LocalArgument::Literal(value) => validate_argument(value)?,
                    LocalArgument::InstancePath(path) => validate_relative_path(path)?,
                    _ => {}
                }
            }
            for (name, value) in &deployment.environment {
                if name.is_empty()
                    || name.len() > 128
                    || name.contains('=')
                    || name.as_bytes().contains(&0)
                    || value.len() > 4_096
                    || value.as_bytes().contains(&0)
                {
                    return Err(format!("deployment {key} environment is invalid"));
                }
            }
            if let LocalShutdown::RequestFile {
                request,
                complete,
                timeout_ms,
            } = &deployment.shutdown
            {
                validate_control_filename(request)?;
                validate_control_filename(complete)?;
                if request == complete || !(100..=30_000).contains(timeout_ms) {
                    return Err(format!(
                        "deployment {key} graceful shutdown contract is invalid"
                    ));
                }
            }
            if let LocalReadiness::File { path, timeout_ms } = &deployment.readiness {
                validate_control_filename(path)?;
                if !(100..=120_000).contains(timeout_ms) {
                    return Err(format!("deployment {key} readiness contract is invalid"));
                }
                if let LocalShutdown::RequestFile {
                    request, complete, ..
                } = &deployment.shutdown
                {
                    if path == request || path == complete {
                        return Err(format!(
                            "deployment {key} readiness and shutdown files must be distinct"
                        ));
                    }
                }
            }
        }
        Ok(())
    }
}
