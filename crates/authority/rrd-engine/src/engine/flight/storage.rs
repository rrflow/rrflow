//! Flight storage and typed property helpers.

use super::*;

pub(super) fn flight_storage(reason: &str) -> ServiceError {
    ServiceError::Storage(format!("prompt flight row is malformed: {reason}"))
}

pub(super) fn unsigned(properties: &RuntimeProperties, name: &str) -> Result<u64> {
    match properties.get(name) {
        Some(RuntimeValue::Unsigned(value)) => Ok(*value),
        _ => Err(flight_storage(&format!(
            "{name} is missing or not unsigned"
        ))),
    }
}

pub(super) fn optional_unsigned(properties: &RuntimeProperties, name: &str) -> Result<Option<u64>> {
    match properties.get(name) {
        None => Ok(None),
        Some(RuntimeValue::Unsigned(value)) => Ok(Some(*value)),
        _ => Err(flight_storage(&format!("{name} is not unsigned"))),
    }
}

pub(super) fn boolean(properties: &RuntimeProperties, name: &str) -> Result<bool> {
    match properties.get(name) {
        Some(RuntimeValue::Bool(value)) => Ok(*value),
        _ => Err(flight_storage(&format!("{name} is missing or not boolean"))),
    }
}

pub(super) fn text(properties: &RuntimeProperties, name: &str) -> Result<String> {
    match properties.get(name) {
        Some(RuntimeValue::String(value)) => Ok(value.clone()),
        _ => Err(flight_storage(&format!("{name} is missing or not text"))),
    }
}

pub(super) fn optional_text(properties: &RuntimeProperties, name: &str) -> Result<Option<String>> {
    match properties.get(name) {
        None => Ok(None),
        Some(RuntimeValue::String(value)) => Ok(Some(value.clone())),
        _ => Err(flight_storage(&format!("{name} is not text"))),
    }
}

pub(super) fn digest_text(properties: &RuntimeProperties, name: &str) -> Result<String> {
    match properties.get(name) {
        Some(RuntimeValue::Digest(value)) => Ok(value.clone()),
        _ => Err(flight_storage(&format!(
            "{name} is missing or not a digest"
        ))),
    }
}

pub(super) fn canonical_entry(
    entry: &BTreeMap<String, RuntimeValue>,
    name: &str,
) -> Result<CanonicalId> {
    match entry.get(name) {
        Some(RuntimeValue::String(value)) => {
            CanonicalId::new(value.as_str()).map_err(|error| flight_storage(&error.to_string()))
        }
        _ => Err(flight_storage(&format!(
            "context seed {name} is missing or not text"
        ))),
    }
}
