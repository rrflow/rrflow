//! Grant, role, binding, issuer and policy validation.

use super::*;

pub(super) fn validate_grant(grant: &ResourceGrant) -> Result<()> {
    grant
        .resource_prefix
        .validate()
        .map_err(|error| Error::Invalid(error.to_string()))?;
    if let Some(policy) = &grant.data_policy {
        policy.validate()?;
    }
    Ok(())
}

pub(super) fn validate_roles(state: &SecurityState) -> Result<()> {
    for (id, role) in &state.roles {
        if id != &role.id
            || (role.inherits.is_empty() && role.grants.is_empty())
            || role.grants.len() > MAX_GRANTS_PER_PRINCIPAL
        {
            return Err(Error::Invalid(
                "role identity or grant bounds are invalid".into(),
            ));
        }
        let mut grants = BTreeSet::new();
        for grant in &role.grants {
            validate_grant(grant)?;
            let identity = serde_json::to_vec(grant).map_err(json_error)?;
            if !grants.insert(identity) {
                return Err(Error::Invalid("role grants must be unique".into()));
            }
        }
        if role
            .inherits
            .iter()
            .any(|inherited| inherited == id || !state.roles.contains_key(inherited))
        {
            return Err(Error::Invalid("role inheritance target is invalid".into()));
        }
        let mut visiting = BTreeSet::new();
        validate_role_depth(state, id, &mut visiting, 0)?;
    }
    Ok(())
}

pub(super) fn validate_role_depth(
    state: &SecurityState,
    role_id: &CanonicalId,
    visiting: &mut BTreeSet<CanonicalId>,
    depth: usize,
) -> Result<()> {
    if depth >= MAX_ROLE_DEPTH || !visiting.insert(role_id.clone()) {
        return Err(Error::Invalid(
            "role inheritance is cyclic or exceeds its depth bound".into(),
        ));
    }
    let role = state
        .roles
        .get(role_id)
        .ok_or_else(|| Error::Invalid("role inheritance target is missing".into()))?;
    for inherited in &role.inherits {
        validate_role_depth(state, inherited, visiting, depth + 1)?;
    }
    visiting.remove(role_id);
    Ok(())
}

pub(super) fn validate_identity_bindings(state: &SecurityState) -> Result<()> {
    let mut tuples = BTreeSet::new();
    for (id, binding) in &state.identity_bindings {
        if id != &binding.id
            || !state.principals.contains_key(&binding.principal_id)
            || !bounded_identity(&binding.issuer)
            || !bounded_identity(&binding.subject)
            || !bounded_identity(&binding.audience)
            || !tuples.insert((
                binding.issuer.as_str(),
                binding.subject.as_str(),
                binding.audience.as_str(),
            ))
        {
            return Err(Error::Invalid(
                "third-party identity binding is invalid".into(),
            ));
        }
    }
    Ok(())
}

pub(super) fn validate_jwt_issuers(state: &SecurityState) -> Result<()> {
    let mut key_ids = BTreeSet::new();
    for (id, issuer) in &state.jwt_issuers {
        validate_sha256(&issuer.signing_key_sha256)?;
        if id != &issuer.id
            || !bounded_identity(&issuer.issuer)
            || !bounded_identity(&issuer.audience)
            || issuer.not_before_unix_ms == 0
            || issuer.not_before_unix_ms >= issuer.expires_at_unix_ms
            || !key_ids.insert(&issuer.key_id)
        {
            return Err(Error::Invalid("JWT issuer is invalid".into()));
        }
    }
    Ok(())
}

pub(super) fn validate_jwt_window(issuer: &JwtIssuer, request: &JwtIssueRequest) -> Result<()> {
    if issuer.disabled
        || request.issued_at_unix_ms < issuer.not_before_unix_ms
        || request.issued_at_unix_ms >= issuer.expires_at_unix_ms
        || request.not_before_unix_ms < request.issued_at_unix_ms
        || request.expires_at_unix_ms <= request.not_before_unix_ms
        || request.expires_at_unix_ms > issuer.expires_at_unix_ms
        || request
            .expires_at_unix_ms
            .saturating_sub(request.issued_at_unix_ms)
            > MAX_JWT_LIFETIME_MS
    {
        return Err(Error::Invalid("JWT validity window is invalid".into()));
    }
    Ok(())
}

pub(super) fn validate_policy_field(field: &str) -> Result<()> {
    if field.is_empty()
        || field.len() > 256
        || !field
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '.'))
    {
        return Err(Error::Invalid("security policy field is invalid".into()));
    }
    Ok(())
}

pub(super) fn validate_policy_value(value: &RuntimeValue) -> Result<()> {
    if matches!(
        value,
        RuntimeValue::Null
            | RuntimeValue::Bool(_)
            | RuntimeValue::Integer(_)
            | RuntimeValue::Unsigned(_)
            | RuntimeValue::String(_)
    ) {
        Ok(())
    } else {
        Err(Error::Invalid(
            "security predicates require scalar query values".into(),
        ))
    }
}

pub(super) fn bounded_identity(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= 1_024 && !value.as_bytes().contains(&0)
}

pub(super) fn initial_credential_revision() -> u64 {
    1
}
