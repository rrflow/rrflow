//! Value, revision-lineage and schema-shape validation.

use super::*;

pub fn validate_function_value(value: &QueryValue) -> Result<()> {
    let mut stack = vec![(value, 0_usize)];
    let mut items = 0_usize;
    while let Some((value, depth)) = stack.pop() {
        if depth > MAX_FUNCTION_VALUE_DEPTH {
            return invalid("function value exceeds the v1 nesting-depth bound");
        }
        items = items
            .checked_add(1)
            .ok_or_else(|| crate::ContractError("function value item count overflow".into()))?;
        if items > MAX_FUNCTION_VALUE_ITEMS {
            return invalid("function value exceeds the v1 item-count bound");
        }
        match value {
            QueryValue::List(values) => {
                stack.extend(values.iter().map(|value| (value, depth + 1)));
            }
            QueryValue::Map(values) => {
                stack.extend(values.values().map(|value| (value, depth + 1)));
            }
            QueryValue::Null
            | QueryValue::Bool(_)
            | QueryValue::Integer(_)
            | QueryValue::Unsigned(_)
            | QueryValue::Decimal(_)
            | QueryValue::String(_)
            | QueryValue::Digest(_) => {}
        }
    }
    Ok(())
}

pub(super) fn validate_revision_lineage(
    revision: u64,
    predecessor: Option<&str>,
    kind: &str,
) -> Result<()> {
    match (revision, predecessor) {
        (0, _) => invalid(format!("{kind} revision must be non-zero")),
        (1, None) => Ok(()),
        (1, Some(_)) => invalid(format!("first {kind} revision cannot have a predecessor")),
        (_, None) => invalid(format!("later {kind} revision requires a predecessor")),
        (_, Some(predecessor)) => validate_sha256(predecessor, &format!("{kind} predecessor")),
    }
}

pub(super) fn validate_schema_shape(
    shape: &FunctionValueShape,
    depth: usize,
    items: &mut usize,
) -> Result<()> {
    if depth > MAX_FUNCTION_SCHEMA_DEPTH {
        return invalid("function schema exceeds its nesting-depth bound");
    }
    *items = items
        .checked_add(1)
        .ok_or_else(|| crate::ContractError("function schema item count overflow".into()))?;
    if *items > MAX_FUNCTION_SCHEMA_ITEMS {
        return invalid("function schema exceeds its item-count bound");
    }
    match shape {
        FunctionValueShape::String { max_bytes } if *max_bytes == 0 => {
            invalid("function string schema max_bytes must be non-zero")
        }
        FunctionValueShape::List {
            items: item,
            max_items,
        } => {
            if *max_items == 0 || *max_items as usize > MAX_FUNCTION_VALUE_ITEMS {
                return invalid("function list schema max_items is outside the v1 bound");
            }
            validate_schema_shape(item, depth + 1, items)
        }
        FunctionValueShape::Object {
            properties,
            required,
            ..
        } => {
            if properties.len() > MAX_FUNCTION_SCHEMA_ITEMS
                || !required.iter().all(|name| properties.contains_key(name))
            {
                return invalid("function object schema properties or required set are invalid");
            }
            for (name, property) in properties {
                if name.is_empty() || name.len() > 256 || name.as_bytes().contains(&0) {
                    return invalid("function object schema property name is invalid");
                }
                validate_schema_shape(property, depth + 1, items)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

pub(super) fn validate_value_shape(
    shape: &FunctionValueShape,
    value: &QueryValue,
    schema: &str,
) -> Result<()> {
    let valid = match (shape, value) {
        (FunctionValueShape::CanonicalValueV1, _) => true,
        (FunctionValueShape::Null, QueryValue::Null) => true,
        (FunctionValueShape::Boolean, QueryValue::Bool(_)) => true,
        (FunctionValueShape::Integer, QueryValue::Integer(_)) => true,
        (FunctionValueShape::Unsigned, QueryValue::Unsigned(_)) => true,
        (FunctionValueShape::Decimal, QueryValue::Decimal(_)) => true,
        (FunctionValueShape::String { max_bytes }, QueryValue::String(value)) => {
            value.len() <= *max_bytes as usize
        }
        (FunctionValueShape::Digest, QueryValue::Digest(value)) => {
            validate_sha256(value, "function schema digest value").is_ok()
        }
        (FunctionValueShape::List { items, max_items }, QueryValue::List(values)) => {
            values.len() <= *max_items as usize
                && values
                    .iter()
                    .all(|value| validate_value_shape(items, value, schema).is_ok())
        }
        (
            FunctionValueShape::Object {
                properties,
                required,
                allow_additional_properties,
            },
            QueryValue::Map(values),
        ) => {
            required.iter().all(|name| values.contains_key(name))
                && (*allow_additional_properties
                    || values.keys().all(|name| properties.contains_key(name)))
                && values.iter().all(|(name, value)| {
                    properties
                        .get(name)
                        .is_none_or(|shape| validate_value_shape(shape, value, schema).is_ok())
                })
        }
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        invalid(format!("function value does not satisfy schema {schema}"))
    }
}

pub(super) fn sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        write!(&mut output, "{byte:02x}").expect("writing to string cannot fail");
    }
    output
}
