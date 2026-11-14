//! Session summaries, validation, manifests and durable publication helpers.

use super::*;

pub(super) fn distinct_objects(manifest: &ArtifactTransferManifest) -> BTreeMap<String, u64> {
    manifest
        .objects
        .iter()
        .map(|object| (object.sha256.clone(), object.length))
        .collect()
}

pub(super) fn summarize_sessions(
    entries: &[SessionInventoryEntry],
) -> Result<ArtifactTransferSessionInventory> {
    let mut active_sessions = 0usize;
    let mut reserved_bytes = 0u64;
    let mut partial_bytes = 0u64;
    let mut retained_receipts = 0usize;
    for entry in entries {
        if entry.complete {
            retained_receipts = retained_receipts.checked_add(1).ok_or_else(|| {
                ClusterError::Unavailable("artifact receipt count overflowed".into())
            })?;
        } else {
            active_sessions = active_sessions.checked_add(1).ok_or_else(|| {
                ClusterError::Unavailable("artifact session count overflowed".into())
            })?;
            reserved_bytes = reserved_bytes
                .checked_add(entry.reserved_bytes)
                .ok_or_else(|| {
                    ClusterError::Unavailable("artifact reserved byte count overflowed".into())
                })?;
        }
        partial_bytes = partial_bytes
            .checked_add(entry.partial_bytes)
            .ok_or_else(|| {
                ClusterError::Unavailable("artifact partial byte count overflowed".into())
            })?;
    }
    Ok(ArtifactTransferSessionInventory {
        active_sessions,
        reserved_bytes,
        partial_bytes,
        retained_receipts,
    })
}

pub(super) fn remove_session_directory(root: &Path, path: &Path, digest: &str) -> Result<()> {
    if !is_sha256(digest)
        || path.parent() != Some(root)
        || path.file_name() != Some(digest.as_ref())
    {
        return Err(ClusterError::Denied(
            "artifact GC target is outside the session root".into(),
        ));
    }
    let metadata = fs::symlink_metadata(path).map_err(cluster_io_error)?;
    if !metadata.file_type().is_dir() || metadata.file_type().is_symlink() {
        return Err(ClusterError::Denied(
            "artifact GC target is not a session directory".into(),
        ));
    }
    fs::remove_dir_all(path).map_err(cluster_io_error)
}

pub(super) fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .as_bytes()
            .iter()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

pub(super) fn receiver_now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

pub(super) fn lock_error<T>(error: std::sync::PoisonError<T>) -> ClusterError {
    ClusterError::Unavailable(format!("artifact session lock was poisoned: {error}"))
}

pub(super) fn telemetry_add(counter: &mut u64, value: u64, overflowed: &mut bool) {
    match counter.checked_add(value) {
        Some(next) => *counter = next,
        None => {
            *counter = u64::MAX;
            *overflowed = true;
        }
    }
}

pub(super) fn validate_peers(
    manifest: &ArtifactTransferManifest,
    source: &NodeId,
    target: &NodeId,
) -> Result<()> {
    manifest.validate()?;
    if &manifest.plan.source != source || &manifest.plan.target != target {
        return Err(ClusterError::Denied(
            "authenticated artifact peers differ from the transfer manifest".into(),
        ));
    }
    Ok(())
}

pub(super) fn read_manifest(path: &Path) -> Result<ArtifactTransferManifest> {
    let manifest: ArtifactTransferManifest = read_json(path, "artifact manifest")?;
    manifest.validate()?;
    Ok(manifest)
}

pub(super) fn read_json<T: serde::de::DeserializeOwned>(path: &Path, label: &str) -> Result<T> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            ClusterError::NotFound(format!("{label} session is absent"))
        } else {
            cluster_io_error(error)
        }
    })?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > 16 * 1024 * 1024 {
        return Err(ClusterError::Denied(format!(
            "{label} file is outside its bounded contract"
        )));
    }
    let bytes = fs::read(path).map_err(cluster_io_error)?;
    serde_json::from_slice(&bytes)
        .map_err(|error| ClusterError::Invalid(format!("decode {label}: {error}")))
}

pub(super) fn publish_json(path: &Path, value: &impl serde::Serialize) -> Result<()> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| ClusterError::Invalid(format!("encode artifact session: {error}")))?;
    if bytes.is_empty() || bytes.len() > 16 * 1024 * 1024 {
        return Err(ClusterError::Invalid(
            "artifact session JSON is outside its bounded contract".into(),
        ));
    }
    publish_bytes(path, &bytes)
}

pub(super) fn publish_bytes(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| ClusterError::Invalid("artifact session path has no parent".into()))?;
    let filename = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| ClusterError::Invalid("artifact session filename is invalid".into()))?;
    let (pending, mut file) = loop {
        let ordinal = PUBLISH_ORDINAL.fetch_add(1, Ordering::Relaxed);
        let candidate = parent.join(format!(
            ".{filename}-{}-{ordinal}.pending",
            std::process::id()
        ));
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(file) => break (candidate, file),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(cluster_io_error(error)),
        }
    };
    let published = (|| -> Result<()> {
        file.write_all(bytes).map_err(cluster_io_error)?;
        file.sync_all().map_err(cluster_io_error)?;
        drop(file);
        publish_durable_rename(parent, &pending, path).map_err(cluster_io_error)
    })();
    if published.is_err() {
        let _ = fs::remove_file(&pending);
    }
    published
}

pub(super) fn sync_directory(path: &Path) -> Result<()> {
    sync_directory_metadata(path).map_err(cluster_io_error)
}

pub(super) fn cluster_io_error(error: std::io::Error) -> ClusterError {
    ClusterError::Unavailable(format!("artifact session I/O: {error}"))
}

pub(super) fn cluster_store_error(error: StoreError) -> ClusterError {
    match error {
        StoreError::ObjectMissing(digest) => ClusterError::NotFound(digest),
        error => ClusterError::Unavailable(error.to_string()),
    }
}
