//! Source, text, digest and router-artifact validation.

use super::*;

pub(super) fn validate_source(
    job: &EmbeddingJob,
    snapshot: &EmbeddingSourceSnapshot,
) -> Result<()> {
    snapshot.validate()?;
    if snapshot.source != job.source {
        return invalid("embedding source reader returned the wrong identity");
    }
    if snapshot.digest != job.expected_source_digest {
        return invalid("embedding source differs from the job's expected digest");
    }
    if snapshot.bytes.len() > job.model.maximum_input_bytes as usize {
        return invalid("embedding source exceeds the requested model byte limit");
    }
    match job.model.modality {
        EmbeddingModality::Text
            if !snapshot.media_type.starts_with("text/")
                && snapshot.media_type != "application/json" =>
        {
            invalid("text embedding job received a non-text source")
        }
        EmbeddingModality::Image if !snapshot.media_type.starts_with("image/") => {
            invalid("image embedding job received a non-image source")
        }
        _ => Ok(()),
    }
}

pub(super) fn execution_name(execution: &ExecutionTarget) -> String {
    match execution {
        ExecutionTarget::Cpu => "cpu".into(),
        ExecutionTarget::Gpu { platform, device } => format!("gpu:{platform}:{device}"),
        ExecutionTarget::Remote { provider } => format!("remote:{provider}"),
    }
}

pub(super) fn validate_text(label: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() || value.as_bytes().contains(&0) {
        return invalid(format!(
            "{label} must be non-empty and contain no NUL bytes"
        ));
    }
    Ok(())
}

pub(super) fn validate_digest(label: &str, value: &str) -> Result<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return invalid(format!("{label} digest must be lowercase SHA-256 hex"));
    }
    Ok(())
}

pub(super) fn validate_router_artifact_bytes(
    label: &str,
    bytes: &[u8],
    descriptor: &RouterArtifactDescriptor,
) -> Result<()> {
    let encoded_bytes = u64::try_from(bytes.len()).map_err(|_| Error::InvalidRuntime {
        reason: format!("router {label} artifact byte length exceeds u64"),
    })?;
    if encoded_bytes != descriptor.encoded_bytes {
        return invalid(format!(
            "router {label} artifact byte length differs from the manifest"
        ));
    }
    if digest::sha256_hex(bytes) != descriptor.sha256 {
        return invalid(format!(
            "router {label} artifact digest differs from the manifest"
        ));
    }
    Ok(())
}

pub(super) fn invalid<T>(reason: impl Into<String>) -> Result<T> {
    Err(Error::InvalidRuntime {
        reason: reason.into(),
    })
}
