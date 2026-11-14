//! Opening, cleaning and ensuring the adapter database.

use super::*;

/// Canonical state is rrflowKV truth applied through rrd-store, so its
/// database carries the rrflowKV application format; the local Raft log and
/// vote database is adapter-private and has none.
pub(super) fn open_adapter_database(
    root: &AnchoredPath,
    domain: AdapterDomain,
) -> ClusterResult<Database> {
    if root
        .join("CURRENT")
        .try_exists()
        .map_err(|error| ClusterError::Unavailable(error.to_string()))?
    {
        let database = Database::open_anchored(root, Default::default())
            .map_err(|error| ClusterError::Unavailable(error.to_string()))?;
        if domain == AdapterDomain::CanonicalState && !is_rrflow_kv_database(&database) {
            return Err(ClusterError::Denied(format!(
                "canonical Raft state database {} is not in the rrflowKV application format",
                root.display()
            )));
        }
        return Ok(database);
    }
    if root
        .try_exists()
        .map_err(|error| ClusterError::Unavailable(error.to_string()))?
        && root
            .entries()
            .map_err(|error| ClusterError::Unavailable(error.to_string()))?
            .next()
            .is_some()
    {
        return Err(ClusterError::Denied(format!(
            "adapter database {} has content but no authenticated RRD LSM CURRENT pointer",
            root.display()
        )));
    }
    match domain {
        AdapterDomain::CanonicalState => create_rrflow_kv_database(root)
            .map_err(|error| ClusterError::Unavailable(error.to_string())),
        AdapterDomain::LocalRaft => Database::create_anchored(root, Default::default(), None)
            .map_err(|error| ClusterError::Unavailable(error.to_string())),
    }
}

pub(super) fn clean_snapshot_spool(path: &AnchoredPath) -> ClusterResult<()> {
    path.create_dir_all()
        .map_err(|error| ClusterError::Unavailable(error.to_string()))?;
    for entry in path
        .entries()
        .map_err(|error| ClusterError::Unavailable(error.to_string()))?
    {
        let entry = entry.map_err(|error| ClusterError::Unavailable(error.to_string()))?;
        let file_type = entry
            .metadata()
            .map_err(|error| ClusterError::Unavailable(error.to_string()))?;
        if !file_type.is_file() {
            return Err(ClusterError::Denied(format!(
                "snapshot spool contains unexpected non-file entry {}",
                entry.display()
            )));
        }
        entry
            .remove_file()
            .map_err(|error| ClusterError::Unavailable(error.to_string()))?;
    }
    path.sync_directory()
        .map_err(|error| ClusterError::Unavailable(error.to_string()))
}

pub(super) fn snapshot_spool_path(root: &AnchoredPath, purpose: &str) -> AnchoredPath {
    root.join(format!(
        "{purpose}-{}-{}.spool",
        std::process::id(),
        SNAPSHOT_SPOOL_ORDINAL.fetch_add(1, Ordering::Relaxed)
    ))
}

pub(super) fn ensure_adapter_database(
    database: &mut Database,
    config_key: &[u8],
    expected: AdapterConfig,
    initial: Option<Mutation>,
) -> ClusterResult<()> {
    let snapshot = database.snapshot();
    if let Some(bytes) = database
        .get(config_key, snapshot)
        .map_err(|error| ClusterError::Unavailable(error.to_string()))?
    {
        let actual: AdapterConfig = serde_json::from_slice(&bytes)
            .map_err(|error| ClusterError::Denied(error.to_string()))?;
        if actual != expected {
            return Err(ClusterError::Denied(
                "raft adapter format, shard binding, or storage domain does not match".into(),
            ));
        }
        if expected.domain == AdapterDomain::CanonicalState
            && database
                .get(KEY_STATE, snapshot)
                .map_err(|error| ClusterError::Unavailable(error.to_string()))?
                .is_none()
        {
            return Err(ClusterError::Denied(
                "canonical Raft state database has no state-machine record".into(),
            ));
        }
        return Ok(());
    }
    if snapshot.sequence != 0 {
        return Err(ClusterError::Denied(
            "existing database is not an RRD OpenRaft v4 storage domain".into(),
        ));
    }
    let mut operations = vec![put_json(config_key, &expected)?];
    operations.extend(initial);
    database
        .write_owned(
            WriteBatch::new(operations)
                .map_err(|error| ClusterError::Unavailable(error.to_string()))?,
            Durability::Authoritative,
        )
        .map_err(|error| ClusterError::Unavailable(error.to_string()))?;
    Ok(())
}
