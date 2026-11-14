//! JWT, state and audit decoding, keys and digest helpers.

use super::*;

pub(super) fn decode_jwt_segment<T>(segment: &str) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    let bytes = URL_SAFE_NO_PAD
        .decode(segment)
        .map_err(|_| Error::Unauthenticated)?;
    serde_json::from_slice(&bytes).map_err(|_| Error::Unauthenticated)
}

pub(super) fn jwt_signature(key: &[u8], value: &[u8]) -> Result<Vec<u8>> {
    let mut mac = <Hmac<Sha256> as KeyInit>::new_from_slice(key)
        .map_err(|_| Error::Invalid("JWT signing key is invalid".into()))?;
    mac.update(value);
    Ok(mac.finalize().into_bytes().to_vec())
}

pub(super) fn verify_jwt_signature(key: &[u8], value: &[u8], signature: &[u8]) -> Result<()> {
    let expected = jwt_signature(key, value)?;
    if constant_time_equal(&expected, signature) {
        Ok(())
    } else {
        Err(Error::Unauthenticated)
    }
}

pub(super) fn decode_state(bytes: &[u8]) -> Result<SecurityState> {
    let state: SecurityState = serde_json::from_slice(bytes).map_err(json_error)?;
    state.validate()?;
    Ok(state)
}

pub(super) fn decode_audit(entry: &ControlJournalEntry) -> Result<AuditRecord> {
    let bytes = entry
        .replacement
        .as_deref()
        .ok_or_else(|| Error::Invalid("audit journal entry deleted its record".into()))?;
    decode_audit_bytes(bytes)
}

pub(super) fn decode_audit_bytes(bytes: &[u8]) -> Result<AuditRecord> {
    let record: AuditRecord = serde_json::from_slice(bytes).map_err(json_error)?;
    record.validate()?;
    Ok(record)
}

pub(super) fn decode_audit_head(bytes: &[u8]) -> Result<AuditHead> {
    let head: AuditHead = serde_json::from_slice(bytes).map_err(json_error)?;
    validate_sha256(&head.audit_sha256)?;
    if head.record_count == 0 {
        return Err(Error::Invalid("audit head record count is zero".into()));
    }
    Ok(head)
}

pub(super) fn exact_audit_replay(existing: &[u8], event: &AuditEvent) -> Result<()> {
    let record = decode_audit_bytes(existing)?;
    if record.as_event() == *event {
        Ok(())
    } else {
        Err(Error::IdempotencyConflict)
    }
}

pub(super) fn security_key(instance: &CanonicalId) -> String {
    format!("server/state/{instance}/security/policy")
}

pub(super) fn audit_key(instance: &CanonicalId, audit_id: &CanonicalId) -> String {
    format!("server/state/{instance}/audit/{audit_id}")
}

pub(super) fn audit_head_key(instance: &CanonicalId) -> String {
    format!("server/state/{instance}/audit-head")
}

pub(super) fn validate_sha256(value: &str) -> Result<()> {
    if value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        Ok(())
    } else {
        Err(Error::Invalid(
            "SHA-256 must be lowercase hexadecimal".into(),
        ))
    }
}

pub(super) fn constant_time_equal(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0_u8, |difference, (left, right)| {
            difference | (left ^ right)
        })
        == 0
}

pub(super) fn json_error(error: serde_json::Error) -> Error {
    Error::Invalid(error.to_string())
}
