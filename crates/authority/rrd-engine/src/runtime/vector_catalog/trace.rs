//! Trace attributes for publication and quantization.

use super::*;

pub(super) fn publication_attributes(
    artifact: &VectorArtifact,
    expected_revision: u64,
) -> RuntimeProperties {
    let descriptor = artifact.descriptor();
    let stamp = descriptor.stamp();
    RuntimeProperties::from([
        (
            "projection_id".into(),
            RuntimeValue::String(stamp.id.to_string()),
        ),
        (
            "projection_kind".into(),
            RuntimeValue::String(artifact.kind().as_str().into()),
        ),
        (
            "generation".into(),
            RuntimeValue::Unsigned(stamp.generation),
        ),
        (
            "source_cursor".into(),
            RuntimeValue::Unsigned(stamp.source_cursor),
        ),
        (
            "config_digest".into(),
            RuntimeValue::Digest(stamp.config_digest.clone()),
        ),
        (
            "artifact_digest".into(),
            RuntimeValue::Digest(stamp.artifact_digest.clone()),
        ),
        (
            "expected_catalog_revision".into(),
            RuntimeValue::Unsigned(expected_revision),
        ),
    ])
}

pub(super) fn quantization_trace_attributes(
    stamp: &rrd_core::ProjectionStamp,
    kind: rrd_vector::VectorArtifactKind,
    action: &str,
) -> RuntimeProperties {
    RuntimeProperties::from([
        (
            "projection_id".into(),
            RuntimeValue::String(stamp.id.to_string()),
        ),
        (
            "generation".into(),
            RuntimeValue::Unsigned(stamp.generation),
        ),
        (
            "projection_kind".into(),
            RuntimeValue::String(kind.as_str().into()),
        ),
        (
            "lifecycle_action".into(),
            RuntimeValue::String(action.into()),
        ),
        (
            "config_digest".into(),
            RuntimeValue::Digest(stamp.config_digest.clone()),
        ),
        (
            "artifact_digest".into(),
            RuntimeValue::Digest(stamp.artifact_digest.clone()),
        ),
    ])
}

pub(super) fn quantization_finish_attributes(
    lifecycle_revision: u64,
    state: QuantizationArtifactState,
    entry: &QuantizationArtifactEntry,
    commit: &RuntimeCommitOutcome,
) -> RuntimeProperties {
    let state = match state {
        QuantizationArtifactState::Ready => "ready",
        QuantizationArtifactState::Active => "active",
        QuantizationArtifactState::Retired => "retired",
    };
    RuntimeProperties::from([
        (
            "lifecycle_revision".into(),
            RuntimeValue::Unsigned(lifecycle_revision),
        ),
        ("lifecycle_state".into(), RuntimeValue::String(state.into())),
        (
            "catalog_entry_digest".into(),
            RuntimeValue::Digest(entry.entry_digest.clone()),
        ),
        (
            "object_sha256".into(),
            RuntimeValue::Digest(entry.object.sha256.clone()),
        ),
        (
            "object_length".into(),
            RuntimeValue::Unsigned(entry.object.length),
        ),
        (
            "commit_id".into(),
            RuntimeValue::Digest(commit.commit_id.clone()),
        ),
    ])
}

pub(super) fn finish_publication_error<E: StorageEngine, T>(
    store: &E,
    span: DurableTraceSpan,
    stage: &str,
    error: Box<dyn std::error::Error>,
) -> Result<T, Box<dyn std::error::Error>> {
    let rendered = error.to_string();
    let attributes = RuntimeProperties::from([
        ("failed_stage".into(), RuntimeValue::String(stage.into())),
        (
            "error_digest".into(),
            RuntimeValue::Digest(rrd_core::digest::sha256_hex(rendered.as_bytes())),
        ),
    ]);
    if let Err(trace_error) = span.finish(store, TraceOutcome::Error, Vec::new(), attributes) {
        return Err(
            format!("{rendered}; authoritative trace finish also failed: {trace_error}").into(),
        );
    }
    Err(error)
}
