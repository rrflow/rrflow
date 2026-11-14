//! Local recovery authorization, directories, idempotency and control commits.

use super::*;

pub(super) fn ensure_direct_directory(root: &Path, child: &str) -> std::result::Result<(), String> {
    let path = root.join(child);
    create_direct_directory(&path)
        .map_err(|error| format!("cannot create local {child} root: {error}"))?;
    let canonical = fs::canonicalize(&path)
        .map_err(|error| format!("cannot resolve local {child} root: {error}"))?;
    if canonical != path || !canonical.is_dir() {
        return Err(format!("local {child} root must be a direct directory"));
    }
    Ok(())
}

pub(super) fn create_direct_directory(path: &Path) -> std::io::Result<()> {
    match fs::create_dir(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            let metadata = fs::symlink_metadata(path)?;
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err(std::io::Error::other("path is not a direct directory"));
            }
            Ok(())
        }
        Err(error) => Err(error),
    }
}

pub(super) fn authorize_local_recovery(
    policy_path: &Path,
    key_path: &Path,
    estate_id: CanonicalId,
    permission: rrd_estate::LocalEstatePermission,
    at: u64,
) -> Result<rrd_estate::LocalOperatorAuthorization> {
    rrd_estate::LocalOperatorPolicy::load_json(policy_path)
        .map_err(ServiceError::Contract)?
        .authorize_key_file(key_path, estate_id, permission, at)
        .map_err(ServiceError::Contract)
}

pub(super) fn canonical_local_state_root(supplied: &Path) -> Result<PathBuf> {
    if !supplied.is_absolute() {
        return Err(ServiceError::Contract(
            "local recovery state root must be absolute".into(),
        ));
    }
    let root = fs::canonicalize(supplied).map_err(|error| {
        ServiceError::Contract(format!("cannot resolve local recovery state root: {error}"))
    })?;
    if root != supplied || !root.is_dir() {
        return Err(ServiceError::Contract(
            "local recovery state root must be a direct canonical directory".into(),
        ));
    }
    Ok(root)
}

pub(super) fn require_direct_directory(
    path: &(impl rrd_store::filesystem::AnchorInput + ?Sized),
    name: &str,
) -> Result<()> {
    let path = path
        .anchor()
        .map_err(|e| ServiceError::Contract(e.to_string()))?;
    let metadata = path
        .metadata()
        .map_err(|error| ServiceError::Contract(format!("cannot inspect {name}: {error}")))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(ServiceError::Contract(format!(
            "{name} must be a direct canonical directory"
        )));
    }
    Ok(())
}

pub(super) fn validate_local_recovery_request(at: u64, idempotency_key: &str) -> Result<()> {
    if at == 0
        || idempotency_key.is_empty()
        || idempotency_key.len() > 128
        || !idempotency_key.is_ascii()
        || idempotency_key.as_bytes().contains(&0)
    {
        return Err(ServiceError::Contract(
            "local recovery request time or idempotency key is invalid".into(),
        ));
    }
    Ok(())
}

pub(super) fn validate_sha256_contract(value: &str, name: &str) -> Result<()> {
    if value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        Ok(())
    } else {
        Err(ServiceError::Contract(format!("{name} is not SHA-256")))
    }
}

pub(super) fn local_recovery_operation_key(
    kind: &str,
    estate_id: &CanonicalId,
    instance_id: &CanonicalId,
    idempotency_key: &str,
) -> String {
    let identity = digest::sha256_hex(
        format!("estate-recovery\0{kind}\0{estate_id}\0{instance_id}\0{idempotency_key}")
            .as_bytes(),
    );
    format!("server/state/estate-recovery/{estate_id}/{kind}/{identity}")
}

pub(super) fn local_recovery_idempotency_key(kind: &str, idempotency_key: &str) -> String {
    let identity = digest::sha256_hex(format!("{kind}\0{idempotency_key}").as_bytes());
    format!("{kind}-{}", &identity[..32])
}

pub(super) fn derived_recovery_operation_id(
    kind: &str,
    estate_id: &CanonicalId,
    instance_id: &CanonicalId,
    idempotency_key: &str,
) -> Result<CanonicalId> {
    let digest = digest::sha256_hex(
        format!("{kind}\0{estate_id}\0{instance_id}\0{idempotency_key}").as_bytes(),
    );
    CanonicalId::new(format!("{kind}-{}", &digest[..32]))
        .map_err(|error| ServiceError::Contract(error.to_string()))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn commit_local_recovery_control(
    engine: &impl StorageEngine,
    key: String,
    expected: Option<Vec<u8>>,
    state: &impl Serialize,
    at: u64,
    actor: &str,
    action: &str,
    request_id: &str,
    operation_id: &str,
) -> Result<()> {
    engine.control().commit(&ControlTransition {
        key,
        expected,
        replacement: Some(serde_json::to_vec(state).map_err(contract_json)?),
        at,
        actor: actor.into(),
        action: action.into(),
        request_id: request_id.into(),
        operation_id: operation_id.into(),
    })?;
    Ok(())
}

pub(super) fn permanent(
    request: &rrd_estate::BackupDriverRequest,
    message: impl Into<String>,
) -> rrd_estate::DriverError {
    let message = message.into();
    rrd_estate::DriverError::permanent(
        message.clone(),
        digest::sha256_hex(format!("backup-permanent:{}:{message}", request.job_id).as_bytes()),
    )
}

pub(super) fn retryable(
    request: &rrd_estate::BackupDriverRequest,
    message: impl Into<String>,
) -> rrd_estate::DriverError {
    let message = message.into();
    rrd_estate::DriverError::retryable(
        message.clone(),
        digest::sha256_hex(format!("backup-retryable:{}:{message}", request.job_id).as_bytes()),
    )
}

#[cfg(debug_assertions)]
pub(super) fn maybe_test_hold(marker: Option<&Path>, contents: &str) -> std::io::Result<()> {
    let Some(path) = marker else {
        return Ok(());
    };
    let temporary = path.with_extension("new");
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    file.write_all(contents.as_bytes())?;
    file.sync_all()?;
    fs::rename(temporary, path)?;
    loop {
        std::thread::park_timeout(Duration::from_secs(60));
    }
}

#[cfg(not(debug_assertions))]
pub(super) fn maybe_test_hold(marker: Option<&Path>, _contents: &str) -> std::io::Result<()> {
    if marker.is_some() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "test hold failpoints are unavailable in release builds",
        ));
    }
    Ok(())
}
