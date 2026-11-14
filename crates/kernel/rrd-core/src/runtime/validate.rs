//! Text, digest, window and property validation.

use super::*;

pub(super) fn validate_text(kind: &'static str, value: &str) -> Result<()> {
    check_text(kind, value)
}

pub(super) fn validate_digest(kind: &'static str, value: &str) -> Result<()> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(Error::InvalidRuntime {
            reason: format!("{kind} must be a 64-character hexadecimal SHA-256 digest"),
        });
    }
    Ok(())
}

pub(super) fn validate_window(valid_from: Millis, valid_to: Option<Millis>) -> Result<()> {
    if let Some(valid_to) = valid_to {
        if valid_to <= valid_from {
            return Err(Error::InvalidValidityWindow {
                valid_from,
                valid_to,
            });
        }
    }
    Ok(())
}

pub(super) fn valid_at_window(valid_from: Millis, valid_to: Option<Millis>, at: Millis) -> bool {
    valid_from <= at && valid_to.is_none_or(|valid_to| at < valid_to)
}

pub(super) fn validate_properties(properties: &RuntimeProperties) -> Result<()> {
    for key in properties.keys() {
        check_text("runtime property", key)?;
    }
    Ok(())
}

pub(super) fn text(out: &mut Vec<u8>, value: &str) {
    out.extend_from_slice(&(value.len() as u64).to_be_bytes());
    out.extend_from_slice(value.as_bytes());
}

pub(super) fn optional_text(out: &mut Vec<u8>, value: Option<&str>) {
    out.push(u8::from(value.is_some()));
    if let Some(value) = value {
        text(out, value);
    }
}
