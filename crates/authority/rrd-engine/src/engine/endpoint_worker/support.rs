//! Task reads, read stamps, worker source resources and work digests.

use super::*;

pub(super) fn worker_task_read(request: &TaskWorkerRequest) -> ReadTask {
    ReadTask {
        scope: request.scope.clone(),
        task_id: request.task_id.clone(),
        max_storage_keys: request.max_storage_keys,
    }
}

pub(in super::super) fn context_read_stamp(read: &ReadStamp) -> ContextReadStamp {
    ContextReadStamp {
        runtime_manifest_sha256: read.manifest_id.clone(),
        runtime_cursor: read.commit_cursor,
        schema_revision: read.schema_revision,
        catalogue_revision: read.catalog_revision,
    }
}

pub(in super::super) fn worker_source_resource(
    instance: &CanonicalId,
    project: &CanonicalId,
    entry_identity: &str,
) -> Result<ResourcePath> {
    Ok(ResourcePath {
        segments: vec![
            ResourceId::new(ResourceKind::Instance, instance.as_str().to_owned())
                .map_err(worker_contract)?,
            ResourceId::new(ResourceKind::Project, project.as_str().to_owned())
                .map_err(worker_contract)?,
            ResourceId::new(ResourceKind::Record, entry_identity.to_owned())
                .map_err(worker_contract)?,
        ],
    })
}

pub(in super::super) fn equivalent_worker_source(
    captured: &TaskWorkerSourceProvenance,
    validated: &TaskWorkerSourceProvenance,
) -> bool {
    captured.project_id == validated.project_id
        && captured.entry_identity == validated.entry_identity
        && captured.path_sha256 == validated.path_sha256
        && captured.content_sha256 == validated.content_sha256
        && captured.profile_sha256 == validated.profile_sha256
}

pub(super) fn equivalent_worker_work_digest(
    packet: &WorkerPacket,
    request: &TaskWorkerRequest,
    source: Option<&WorkerSourceCapture>,
) -> Result<String> {
    let context_identity = packet
        .context
        .as_ref()
        .map(|context| (&context.items, context.truncated));
    if let Some(source) = source {
        let provenance = source.provenance();
        operation_digest(&(
            "task-worker-source-v1",
            &packet.task,
            &packet.configuration_sha256,
            &packet.choice,
            &request.context,
            (
                &provenance.project_id,
                &provenance.entry_identity,
                &provenance.path_sha256,
                &provenance.content_sha256,
                &provenance.profile_sha256,
            ),
            context_identity,
        ))
    } else {
        // Preserve the exact legacy serialization and repetition identity for
        // no-source worker calls. Existing budget records use this digest.
        operation_digest(&(
            "task-worker",
            &packet.task,
            &packet.configuration_sha256,
            &packet.choice,
            &request.context,
            context_identity,
        ))
    }
}

pub(super) fn worker_contract(error: impl std::fmt::Display) -> ServiceError {
    ServiceError::Contract(error.to_string())
}
