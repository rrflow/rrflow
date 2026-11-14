//! Initialization, administrative audit and compiling authorization.

use super::*;

/// Builds the initial security authority and its genesis administrative audit
/// as pure control transitions. The caller owns the physical transaction and
/// may compose these effects with the initial runtime commit.
pub fn prepare_security_initialization(
    instance: &CanonicalId,
    state: &SecurityState,
    at: u64,
    actor: &str,
    request_id: &str,
    operation_id: &str,
) -> Result<Vec<ControlTransition>> {
    state.validate()?;
    let state_bytes = serde_json::to_vec(state).map_err(json_error)?;
    let audit = administrative_audit_for_instance(
        instance,
        at,
        request_id,
        operation_id,
        &digest::sha256_hex(&state_bytes),
        "security-authority-initialized",
    )?;
    let record = AuditRecord::seal(audit.clone(), None)?;
    let head = AuditHead {
        audit_id: record.audit_id.clone(),
        audit_sha256: record.audit_sha256.clone(),
        record_count: 1,
    };
    let transitions = vec![
        ControlTransition {
            key: security_key(instance),
            expected: None,
            replacement: Some(state_bytes),
            at,
            actor: actor.into(),
            action: "security.initialized".into(),
            request_id: request_id.into(),
            operation_id: operation_id.into(),
        },
        ControlTransition {
            key: audit_key(instance, &record.audit_id),
            expected: None,
            replacement: Some(serde_json::to_vec(&record).map_err(json_error)?),
            at,
            actor: "anonymous".into(),
            action: "security.audit".into(),
            request_id: request_id.into(),
            operation_id: operation_id.into(),
        },
        ControlTransition {
            key: audit_head_key(instance),
            expected: None,
            replacement: Some(serde_json::to_vec(&head).map_err(json_error)?),
            at,
            actor: "anonymous".into(),
            action: "security.audit.head".into(),
            request_id: request_id.into(),
            operation_id: operation_id.into(),
        },
    ];
    for transition in &transitions {
        transition.validate()?;
    }
    Ok(transitions)
}

pub(super) fn administrative_audit_for_instance(
    instance: &CanonicalId,
    at_unix_ms: u64,
    request_id: &str,
    operation_id: &str,
    request_sha256: &str,
    outcome: &str,
) -> Result<AuditEvent> {
    let identity = digest::sha256_hex(
        &serde_json::to_vec(&(
            request_id,
            operation_id,
            Action::SecurityAdmin,
            AuditPhase::Completed,
            request_sha256,
            outcome,
        ))
        .map_err(json_error)?,
    );
    Ok(AuditEvent {
        audit_id: CanonicalId::new(format!("audit-{identity}"))
            .map_err(|error| Error::Invalid(error.to_string()))?,
        at_unix_ms,
        principal_id: None,
        action: Action::SecurityAdmin,
        resource: ResourcePath {
            segments: vec![
                ResourceId::new(ResourceKind::Instance, instance.as_str().to_owned())
                    .map_err(|error| Error::Invalid(error.to_string()))?,
            ],
        },
        request_id: request_id.into(),
        operation_id: operation_id.into(),
        phase: AuditPhase::Completed,
        decision: AuditDecision::Allowed,
        status_code: 200,
        request_sha256: request_sha256.into(),
        response_sha256: digest::sha256_hex(outcome.as_bytes()),
    })
}

pub(super) fn compile_authorization(
    state: &SecurityState,
    principal_id: &CanonicalId,
    action: Action,
    resource: &ResourcePath,
    at: u64,
) -> Result<Authorization> {
    let principal = state
        .principals
        .get(principal_id)
        .ok_or(Error::PrincipalNotFound)?;
    require_active_principal(principal, at)?;
    resource
        .validate()
        .map_err(|error| Error::Invalid(error.to_string()))?;
    let mut grants = principal.grants.iter().collect::<Vec<_>>();
    let mut visited = BTreeSet::new();
    for role in &principal.role_ids {
        collect_role_grants(state, role, &mut visited, &mut grants)?;
    }
    let specificity = grants
        .iter()
        .filter(|grant| {
            grant.action == action && resource_has_prefix(resource, &grant.resource_prefix)
        })
        .map(|grant| grant.resource_prefix.segments.len())
        .max()
        .ok_or(Error::PermissionDenied)?;
    let selected = grants
        .into_iter()
        .filter(|grant| {
            grant.action == action
                && grant.resource_prefix.segments.len() == specificity
                && resource_has_prefix(resource, &grant.resource_prefix)
        })
        .collect::<Vec<_>>();
    let data_policy = selected[0].data_policy.clone();
    if selected
        .iter()
        .any(|grant| grant.data_policy != data_policy)
    {
        return Err(Error::Invalid(
            "equally specific grants have ambiguous data policies".into(),
        ));
    }
    let authorization_sha256 = digest::sha256_hex(
        &serde_json::to_vec(&(
            state.revision,
            principal.id.as_str(),
            principal.credential_revision,
            action,
            resource,
            &data_policy,
        ))
        .map_err(json_error)?,
    );
    Ok(Authorization {
        principal_id: principal.id.clone(),
        principal_kind: principal.kind,
        action,
        resource: resource.clone(),
        policy_revision: state.revision,
        credential_revision: principal.credential_revision,
        data_policy,
        authorization_sha256,
    })
}

pub(super) fn require_active_principal(principal: &Principal, at: u64) -> Result<()> {
    if principal.disabled || at < principal.not_before_unix_ms || at >= principal.expires_at_unix_ms
    {
        return Err(Error::PermissionDenied);
    }
    Ok(())
}

pub(super) fn collect_role_grants<'a>(
    state: &'a SecurityState,
    role_id: &CanonicalId,
    visited: &mut BTreeSet<CanonicalId>,
    grants: &mut Vec<&'a ResourceGrant>,
) -> Result<()> {
    if !visited.insert(role_id.clone()) {
        return Ok(());
    }
    let role = state
        .roles
        .get(role_id)
        .ok_or_else(|| Error::Invalid("principal references an unknown role".into()))?;
    grants.extend(&role.grants);
    for inherited in &role.inherits {
        collect_role_grants(state, inherited, visited, grants)?;
    }
    Ok(())
}

pub(super) fn resource_has_prefix(resource: &ResourcePath, prefix: &ResourcePath) -> bool {
    prefix.segments.len() <= resource.segments.len()
        && resource.segments[..prefix.segments.len()] == prefix.segments
}
