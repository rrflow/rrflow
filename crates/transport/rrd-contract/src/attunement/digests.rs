//! Contract digests and shared validation.

use super::*;

/// Adopted entries are single, sorted, unique top-level names below `.rrflow`.
pub fn validate_adopted_estate_entries(entries: &[String]) -> Result<()> {
    if entries.len() > MAX_ADOPTED_ESTATE_ENTRIES {
        return invalid("the existing .rrflow folder has too many entries to adopt");
    }
    for entry in entries {
        validate_bounded_text(entry, "adopted .rrflow entry", 255)?;
        if matches!(entry.as_str(), "." | ".." | ADOPTED_ESTATE_DIRECTORY)
            || entry.contains(['/', '\\', ':'])
        {
            return invalid("adopted .rrflow entries must be plain top-level names");
        }
    }
    if entries.windows(2).any(|pair| pair[0] >= pair[1]) {
        return invalid("adopted .rrflow entries must be sorted and unique");
    }
    Ok(())
}

pub fn installation_plan_sha256(plan: &InstallationPlan) -> Result<String> {
    let base = installation_plan_base_sha256(plan)?;
    match &plan.adopted_estate_entries {
        // Plans without adoption keep their established digest exactly.
        None => Ok(base),
        Some(entries) => digest_json(b"rrflow-installation-plan-v4-adoption", &(base, entries)),
    }
}

pub(super) fn installation_plan_base_sha256(plan: &InstallationPlan) -> Result<String> {
    digest_json(
        b"rrflow-installation-plan-v4",
        &(
            (
                plan.contract_version,
                &plan.installation_identity,
                &plan.project_root_sha256,
                &plan.id,
                &plan.target,
                plan.target_kind,
                &plan.deployment,
                &plan.product_version,
                &plan.executable_sha256,
                &plan.profile_id,
                &plan.profile_sha256,
                &plan.project_precondition_sha256,
                &plan.storage_root_id,
            ),
            (
                &plan.configuration,
                &plan.bootstrap,
                &plan.initial_seat,
                &plan.initial_principal_id,
                &plan.initial_grants,
                plan.credential_bytes,
                &plan.inactive_capabilities,
                &plan.managed_paths,
                &plan.attunement_plan_id,
                &plan.attunement_plan_sha256,
                &plan.actions,
            ),
        ),
    )
}

pub fn installation_result_sha256(result: &InstallationResult) -> Result<String> {
    digest_json(
        b"rrflow-installation-result-v1",
        &(
            result.contract_version,
            &result.plan_id,
            &result.plan_sha256,
            &result.attunement_job_id,
            &result.action_results,
            &result.runtime_manifest_sha256,
            result.runtime_cursor,
            result.control_journal_sequence,
            &result.installed_record_sha256,
            &result.credential_sha256,
            &result.locator_sha256,
            result.applied_at_unix_ms,
            result.idempotent_replay,
        ),
    )
}

pub fn attunement_plan_sha256(plan: &AttunementPlan) -> Result<String> {
    digest_json(
        b"rrflow-attunement-plan-v1",
        &(
            plan.contract_version,
            &plan.id,
            &plan.target,
            &plan.source_sha256,
            &plan.phases,
            plan.estimated_min_duration_ms,
            plan.estimated_max_duration_ms,
        ),
    )
}

pub fn attunement_checkpoint_sha256(checkpoint: &AttunementPhaseCheckpoint) -> Result<String> {
    digest_json(
        b"rrflow-attunement-checkpoint-v1",
        &(
            checkpoint.contract_version,
            &checkpoint.job_id,
            &checkpoint.plan_sha256,
            checkpoint.phase,
            checkpoint.sequence,
            &checkpoint.configuration_sha256,
            &checkpoint.input_sha256,
            &checkpoint.output_sha256,
            &checkpoint.coordinates,
            checkpoint.committed_at_unix_ms,
        ),
    )
}

pub fn attunement_verification_sha256(verification: &AttunementVerification) -> Result<String> {
    digest_json(
        b"rrflow-attunement-verification-v1",
        &(
            verification.contract_version,
            &verification.job_id,
            &verification.plan_sha256,
            verification.job_revision,
            &verification.last_checkpoint_sha256,
            &verification.runtime_manifest_sha256,
            verification.status,
            &verification.checks,
            verification.verified_at_unix_ms,
        ),
    )
}

pub(super) fn validate_attunement_contract_version(version: u16) -> Result<()> {
    if version != ATTUNEMENT_CONTRACT_VERSION {
        return invalid(format!("unsupported attunement contract version {version}; expected {ATTUNEMENT_CONTRACT_VERSION}"));
    }
    Ok(())
}

pub(crate) fn validate_contract_version(version: u16) -> Result<()> {
    if version != INSTALL_ATTUNEMENT_CONTRACT_VERSION {
        return invalid(format!(
            "unsupported install/attunement contract version {version}; expected {INSTALL_ATTUNEMENT_CONTRACT_VERSION}"
        ));
    }
    Ok(())
}

pub(crate) fn validate_project_relative_path(value: &str) -> Result<()> {
    validate_bounded_text(value, "installation managed relative_path", 4_096)?;
    if value.starts_with('/') || value.contains('\\') {
        return invalid("installation managed path must use portable project-relative syntax");
    }
    let segments = value.split('/').collect::<Vec<_>>();
    if segments.first().copied() != Some(".rrflow")
        || segments
            .iter()
            .any(|segment| segment.is_empty() || matches!(*segment, "." | ".."))
    {
        return invalid("installation managed path must remain below .rrflow");
    }
    Ok(())
}

pub(super) fn validate_sorted_unique<T: Ord>(values: &[T], message: &str) -> Result<()> {
    if values.windows(2).any(|pair| pair[0] >= pair[1]) {
        return invalid(message);
    }
    Ok(())
}

pub(super) fn validate_job_command_binding(
    job_id: &CanonicalId,
    plan_sha256: &str,
    expected_job_revision: u64,
    last_checkpoint_sha256: Option<&str>,
    requested_at_unix_ms: u64,
    job: &AttunementJob,
) -> Result<()> {
    validate_sha256(plan_sha256, "attunement command plan_sha256")?;
    if job_id != &job.id
        || plan_sha256 != job.plan_sha256
        || expected_job_revision != job.revision
        || last_checkpoint_sha256 != job.last_checkpoint_sha256()
    {
        return invalid("attunement command does not bind the current job revision and checkpoint");
    }
    if requested_at_unix_ms < job.updated_at_unix_ms {
        return invalid("attunement command predates the job snapshot");
    }
    Ok(())
}

pub(crate) fn validate_bounded_text(value: &str, field: &str, maximum: usize) -> Result<()> {
    if value.trim().is_empty() || value.len() > maximum || value.as_bytes().contains(&0) {
        return invalid(format!(
            "{field} must be non-empty, contain no NUL, and use at most {maximum} bytes"
        ));
    }
    Ok(())
}

pub(crate) fn digest_json<T: Serialize>(domain: &[u8], value: &T) -> Result<String> {
    let encoded = serde_json::to_vec(value).map_err(|error| {
        crate::ContractError(format!("contract digest encoding failed: {error}"))
    })?;
    let mut bytes = Vec::with_capacity(domain.len() + 1 + encoded.len());
    bytes.extend_from_slice(domain);
    bytes.push(0);
    bytes.extend_from_slice(&encoded);
    Ok(sha256_bytes(&bytes))
}
