//! Digests, public mappings and validation helpers.

use super::*;

pub(super) fn desired_request_sha256(request: &SetDesired) -> String {
    let bytes = serde_json::to_vec(&(
        &request.instance_id,
        request.target.phase,
        &request.target.deployment_ref,
        &request.target.version,
        &request.target.configuration_sha256,
    ))
    .expect("desired request fields serialize");
    digest::sha256_hex(&bytes)
}

pub(super) fn public_authority_kind(
    kind: AuthorityResourceKind,
) -> rrd_contract::EstateAuthorityResourceKind {
    match kind {
        AuthorityResourceKind::Organisation => {
            rrd_contract::EstateAuthorityResourceKind::Organisation
        }
        AuthorityResourceKind::Account => rrd_contract::EstateAuthorityResourceKind::Account,
        AuthorityResourceKind::Entitlement => {
            rrd_contract::EstateAuthorityResourceKind::Entitlement
        }
        AuthorityResourceKind::Project => rrd_contract::EstateAuthorityResourceKind::Project,
        AuthorityResourceKind::Environment => {
            rrd_contract::EstateAuthorityResourceKind::Environment
        }
        AuthorityResourceKind::Instance => rrd_contract::EstateAuthorityResourceKind::Instance,
        AuthorityResourceKind::Node => rrd_contract::EstateAuthorityResourceKind::Node,
        AuthorityResourceKind::Shard => rrd_contract::EstateAuthorityResourceKind::Shard,
        AuthorityResourceKind::Job => rrd_contract::EstateAuthorityResourceKind::Job,
        AuthorityResourceKind::Assignment => rrd_contract::EstateAuthorityResourceKind::Assignment,
        AuthorityResourceKind::Health => rrd_contract::EstateAuthorityResourceKind::Health,
        AuthorityResourceKind::SecretReference => {
            rrd_contract::EstateAuthorityResourceKind::SecretReference
        }
    }
}

pub(super) fn public_authority_status(
    status: AuthorityStatus,
) -> rrd_contract::EstateAuthorityStatus {
    match status {
        AuthorityStatus::Pending => rrd_contract::EstateAuthorityStatus::Pending,
        AuthorityStatus::Ready => rrd_contract::EstateAuthorityStatus::Ready,
        AuthorityStatus::Degraded => rrd_contract::EstateAuthorityStatus::Degraded,
        AuthorityStatus::Failed => rrd_contract::EstateAuthorityStatus::Failed,
        AuthorityStatus::Retired => rrd_contract::EstateAuthorityStatus::Retired,
    }
}

pub(super) fn public_authority_receipt_boundary(
    boundary: AuthorityReceiptBoundary,
) -> rrd_contract::EstateAuthorityReceiptBoundary {
    match boundary {
        AuthorityReceiptBoundary::DesiredAccepted => {
            rrd_contract::EstateAuthorityReceiptBoundary::DesiredAccepted
        }
        AuthorityReceiptBoundary::Assigned => {
            rrd_contract::EstateAuthorityReceiptBoundary::Assigned
        }
        AuthorityReceiptBoundary::Applied => rrd_contract::EstateAuthorityReceiptBoundary::Applied,
        AuthorityReceiptBoundary::Observed => {
            rrd_contract::EstateAuthorityReceiptBoundary::Observed
        }
        AuthorityReceiptBoundary::Completed => {
            rrd_contract::EstateAuthorityReceiptBoundary::Completed
        }
        AuthorityReceiptBoundary::Failed => rrd_contract::EstateAuthorityReceiptBoundary::Failed,
    }
}

pub(super) fn operation_kind(
    previous: Option<&ManagedInstance>,
    target: &DesiredTarget,
) -> OperationKind {
    if target.phase == DesiredPhase::Absent {
        return OperationKind::Delete;
    }
    let Some(previous) = previous else {
        return OperationKind::Provision;
    };
    if previous.desired.version != target.version
        || previous.desired.configuration_sha256 != target.configuration_sha256
        || previous.desired.deployment_ref != target.deployment_ref
    {
        return OperationKind::Upgrade;
    }
    match target.phase {
        DesiredPhase::Running if previous.desired.phase == DesiredPhase::Running => {
            OperationKind::Restart
        }
        DesiredPhase::Running => OperationKind::Start,
        DesiredPhase::Stopped => OperationKind::Stop,
        DesiredPhase::Absent => OperationKind::Delete,
    }
}

pub(super) fn target_matches(desired: &DesiredInstance, target: &DesiredTarget) -> bool {
    desired.phase == target.phase
        && desired.deployment_ref == target.deployment_ref
        && desired.version == target.version
        && desired.configuration_sha256 == target.configuration_sha256
}

pub(super) fn operation_mut<'a>(
    document: &'a mut EstateDocument,
    id: &CanonicalId,
) -> Result<&'a mut EstateOperation> {
    document
        .operations
        .get_mut(id.as_str())
        .ok_or_else(|| Error::NotFound(id.to_string()))
}

pub(super) fn validate_lease(
    operation: &EstateOperation,
    worker: &CanonicalId,
    epoch: u64,
    at: u64,
) -> Result<()> {
    let lease = operation
        .lease
        .as_ref()
        .ok_or_else(|| Error::StaleLease(operation.id.to_string()))?;
    if lease.owner != *worker || lease.epoch != epoch || lease.expires_at <= at {
        return Err(Error::StaleLease(operation.id.to_string()));
    }
    Ok(())
}

pub(super) fn validate_state_advance(
    current: OperationState,
    target: OperationState,
) -> Result<()> {
    let valid = matches!(
        (current, target),
        (OperationState::Leased, OperationState::Prepared)
            | (OperationState::Prepared, OperationState::Applied)
            | (OperationState::Applied, OperationState::Succeeded)
            | (OperationState::Leased, OperationState::Failed)
            | (OperationState::Prepared, OperationState::Failed)
            | (OperationState::Applied, OperationState::Failed)
    );
    if !valid {
        return Err(Error::Invalid(format!(
            "operation state cannot advance from {current:?} to {target:?}"
        )));
    }
    Ok(())
}

pub(super) fn validate_context(context: &MutationContext) -> Result<()> {
    require_time(context.at)?;
    validate_ascii_key(&context.actor, "actor")?;
    validate_ascii_key(&context.request_id, "request id")?;
    Ok(())
}

pub(super) fn validate_target(target: &DesiredTarget) -> Result<()> {
    validate_version(Some(&target.version))?;
    validate_sha256(&target.configuration_sha256, "configuration")
}

pub(super) fn validate_desired(desired: &DesiredInstance) -> Result<()> {
    if desired.generation == 0 {
        return Err(Error::Invalid("desired generation must be non-zero".into()));
    }
    require_time(desired.updated_at)?;
    validate_version(Some(&desired.version))?;
    validate_sha256(&desired.configuration_sha256, "configuration")
}

pub(super) fn validate_observed(
    observed: &ObservedInstance,
    desired_generation: u64,
) -> Result<()> {
    if observed.generation > desired_generation {
        return Err(Error::Invalid(
            "observed generation exceeds desired generation".into(),
        ));
    }
    require_time(observed.observed_at)?;
    validate_version(observed.version.as_deref())?;
    if let Some(digest) = &observed.evidence_sha256 {
        validate_sha256(digest, "observation evidence")?;
    }
    validate_error(observed.error.as_deref())
}

pub(super) fn validate_version(version: Option<&str>) -> Result<()> {
    if version.is_some_and(|value| {
        value.is_empty()
            || value.len() > 256
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_graphic() && byte != b'\\')
    }) {
        return Err(Error::Invalid("version is invalid".into()));
    }
    Ok(())
}

pub(super) fn validate_error(error: Option<&str>) -> Result<()> {
    if error.is_some_and(|value| value.len() > 4_096 || value.as_bytes().contains(&0)) {
        return Err(Error::Invalid("operation error is invalid".into()));
    }
    Ok(())
}

pub(super) fn validate_id_key(key: &str, id: &CanonicalId) -> Result<()> {
    if key != id.as_str() {
        return Err(Error::Invalid(format!(
            "map key {key} does not match id {id}"
        )));
    }
    Ok(())
}

pub(super) fn validate_ascii_key(value: &str, name: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > 256
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
    {
        return Err(Error::Invalid(format!("{name} is invalid")));
    }
    Ok(())
}

pub(super) fn validate_sha256(value: &str, name: &str) -> Result<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        return Err(Error::Invalid(format!("{name} SHA-256 is invalid")));
    }
    Ok(())
}

pub(super) fn require_time(at: u64) -> Result<()> {
    if at == 0 {
        return Err(Error::Invalid("timestamp must be non-zero".into()));
    }
    Ok(())
}

pub(super) fn max_option(left: Option<u64>, right: Option<u64>) -> Option<u64> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.max(right)),
        (left, right) => left.or(right),
    }
}

pub(super) fn encode(document: &EstateDocument) -> Result<Vec<u8>> {
    document.validate()?;
    serde_json::to_vec(document).map_err(|error| Error::Invalid(error.to_string()))
}

pub(super) fn decode(bytes: &[u8]) -> Result<EstateDocument> {
    let document: EstateDocument =
        serde_json::from_slice(bytes).map_err(|error| Error::Invalid(error.to_string()))?;
    document.validate()?;
    Ok(document)
}
