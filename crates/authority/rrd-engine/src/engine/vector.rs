use super::*;

mod collection;
mod index;
mod points;
mod quantization;
mod search;

pub(in crate::engine) use collection::{
    internal_vector_metric, public_vector_collection, validate_collection_query,
    vector_collection_error, vector_matches_collection,
};
pub(in crate::engine) use points::public_data_ref;
pub(in crate::engine) use search::{internal_vector_filter, internal_vector_query};

pub(in crate::engine) struct DirectVectorRead {
    pub changes: Vec<RuntimeChange>,
    pub selected_versions: u64,
    pub read_evidence: rrd_contract::ReadEvidence,
}

pub(in crate::engine) fn read_vector_versions(
    engine: &RrdEngine,
    read: &ReadStamp,
    max_storage_keys: u64,
) -> Result<DirectVectorRead> {
    let direct = engine.storage.runtime().read_versioned(
        read,
        &[rrd_store::RuntimeVersionedSource::Vectors { kind: None }],
        rrd_store::RuntimeReadBudget::new(max_storage_keys)
            .map_err(|error| ServiceError::Vector(error.to_string()))?,
    )?;
    public_vector_read(direct)
}

pub(in crate::engine) fn read_vector_search_versions(
    engine: &RrdEngine,
    read: &ReadStamp,
    max_storage_keys: u64,
) -> Result<DirectVectorRead> {
    let direct = crate::runtime::vector_catalog::read_vector_catalogue_versions(
        &engine.storage,
        read,
        &[rrd_store::RuntimeVersionedSource::Vectors { kind: None }],
        rrd_store::RuntimeReadBudget::new(max_storage_keys)?,
    )?;
    public_vector_read(direct)
}

fn public_vector_read(direct: rrd_store::RuntimeVersionedRead) -> Result<DirectVectorRead> {
    let selected_versions = u64::try_from(direct.changes.len())
        .map_err(|_| ServiceError::Vector("selected vector versions exceed u64".into()))?;
    Ok(DirectVectorRead {
        changes: direct.changes,
        selected_versions,
        read_evidence: crate::runtime::public_read_evidence(direct.evidence)?,
    })
}

pub(in crate::engine) fn core_vector(error: rrd_core::Error) -> ServiceError {
    ServiceError::Vector(error.to_string())
}
