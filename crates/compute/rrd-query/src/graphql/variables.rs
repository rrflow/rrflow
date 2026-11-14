//! Argument and variable validation and scalar resolution.

use super::*;

pub(super) fn validate_allowed_arguments<'a>(
    root: &str,
    names: impl Iterator<Item = &'a str>,
) -> Result<()> {
    let mut allowed = BTreeSet::from(["validAt", "knownAt", "where", "limit", "explain"]);
    match root {
        "record" | "relation" | "event" | "series" | "geo" => {
            allowed.extend(["kind", "join"]);
        }
        "claim" => {
            allowed.extend(["predicate", "join"]);
        }
        "traverse" => {
            allowed.extend(["relation", "startKind", "startId", "direction", "depth"]);
        }
        _ => return graphql_error(format!("unknown RRFlow GraphQL root field {root:?}")),
    }
    let unknown = names
        .filter(|name| !allowed.contains(name))
        .collect::<Vec<_>>();
    if unknown.is_empty() {
        Ok(())
    } else {
        graphql_error(format!("unknown GraphQL arguments: {}", unknown.join(", ")))
    }
}

pub(super) fn validate_variables(
    operation: &OperationDefinition,
    supplied: &Parameters,
) -> Result<Parameters> {
    if operation.variable_definitions.len() > MAX_GRAPHQL_VARIABLES {
        return graphql_error(format!(
            "GraphQL operation exceeds {MAX_GRAPHQL_VARIABLES} variable definitions"
        ));
    }
    let mut definitions = BTreeMap::new();
    for definition in &operation.variable_definitions {
        let name = definition.node.name.node.to_string();
        validate_graphql_name(&name, "variable definition")?;
        validate_variable_type(&definition.node.var_type.node)?;
        if definitions.insert(name.clone(), definition).is_some() {
            return graphql_error(format!("duplicate GraphQL variable definition ${name}"));
        }
    }
    let mut referenced = BTreeSet::new();
    for selection in &operation.selection_set.node.items {
        collect_selection_variables(&selection.node, &mut referenced);
    }
    for name in &referenced {
        if !definitions.contains_key(name) {
            return graphql_error(format!("GraphQL variable ${name} is not defined"));
        }
    }
    for name in definitions.keys() {
        if !referenced.contains(name) {
            return graphql_error(format!("GraphQL variable ${name} is never used"));
        }
    }
    for name in supplied.keys() {
        if !definitions.contains_key(name) {
            return graphql_error(format!("GraphQL variable ${name} was not declared"));
        }
    }

    let mut effective = supplied.clone();
    for (name, definition) in definitions {
        if !effective.contains_key(&name) {
            if let Some(default) = &definition.node.default_value {
                effective.insert(
                    name.clone(),
                    const_scalar(&default.node, "variable default")?,
                );
            } else if !definition.node.var_type.node.nullable {
                return graphql_error(format!("required GraphQL variable ${name} is missing"));
            } else {
                effective.insert(name.clone(), RuntimeValue::Null);
            }
        }
        validate_variable_value(
            &name,
            &definition.node.var_type.node,
            effective
                .get(&name)
                .expect("effective variable was inserted or supplied"),
        )?;
    }
    Ok(effective)
}

pub(super) fn validate_variable_type(value: &Type) -> Result<()> {
    let BaseType::Named(name) = &value.base else {
        return graphql_error("list-valued GraphQL variables are not lowerable");
    };
    if matches!(
        name.as_str(),
        "Boolean" | "ID" | "Int" | "String" | "Unsigned"
    ) {
        Ok(())
    } else {
        graphql_error(format!(
            "GraphQL variable type {:?} is not in the lowering contract",
            name.as_str()
        ))
    }
}

pub(super) fn validate_variable_value(
    name: &str,
    value_type: &Type,
    value: &RuntimeValue,
) -> Result<()> {
    if matches!(value, RuntimeValue::Null) {
        return if value_type.nullable {
            Ok(())
        } else {
            graphql_error(format!("non-null GraphQL variable ${name} cannot be null"))
        };
    }
    let BaseType::Named(expected) = &value_type.base else {
        return graphql_error("list-valued GraphQL variables are not lowerable");
    };
    let valid = match expected.as_str() {
        "Boolean" => matches!(value, RuntimeValue::Bool(_)),
        "ID" | "String" => matches!(value, RuntimeValue::String(_)),
        "Int" => match value {
            RuntimeValue::Integer(value) => i32::try_from(*value).is_ok(),
            RuntimeValue::Unsigned(value) => i32::try_from(*value).is_ok(),
            _ => false,
        },
        "Unsigned" => matches!(value, RuntimeValue::Unsigned(_)),
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        graphql_error(format!(
            "GraphQL variable ${name} does not match type {expected}"
        ))
    }
}

pub(super) fn collect_selection_variables(selection: &Selection, output: &mut BTreeSet<String>) {
    match selection {
        Selection::Field(field) => {
            for (_, value) in &field.node.arguments {
                collect_value_variables(&value.node, output);
            }
            for nested in &field.node.selection_set.node.items {
                collect_selection_variables(&nested.node, output);
            }
        }
        Selection::FragmentSpread(_) => {}
        Selection::InlineFragment(fragment) => {
            for nested in &fragment.node.selection_set.node.items {
                collect_selection_variables(&nested.node, output);
            }
        }
    }
}

pub(super) fn collect_value_variables(value: &Value, output: &mut BTreeSet<String>) {
    match value {
        Value::Variable(name) => {
            output.insert(name.to_string());
        }
        Value::List(values) => {
            for value in values {
                collect_value_variables(value, output);
            }
        }
        Value::Object(values) => {
            for value in values.values() {
                collect_value_variables(value, output);
            }
        }
        Value::Null
        | Value::Number(_)
        | Value::String(_)
        | Value::Boolean(_)
        | Value::Binary(_)
        | Value::Enum(_) => {}
    }
}

pub(super) fn resolve_scalar(
    value: &Value,
    parameters: &Parameters,
    label: &str,
) -> Result<RuntimeValue> {
    match value {
        Value::Variable(name) => parameters
            .get(name.as_str())
            .cloned()
            .ok_or_else(|| Error::Graphql(format!("missing GraphQL variable ${}", name.as_str()))),
        Value::Null => Ok(RuntimeValue::Null),
        Value::Boolean(value) => Ok(RuntimeValue::Bool(*value)),
        Value::String(value) => Ok(RuntimeValue::String(value.clone())),
        Value::Enum(value) => Ok(RuntimeValue::String(value.to_string())),
        Value::Number(value) => number_scalar(value, label),
        Value::Binary(_) | Value::List(_) | Value::Object(_) => {
            graphql_error(format!("GraphQL {label} must be a scalar"))
        }
    }
}

pub(super) fn const_scalar(value: &ConstValue, label: &str) -> Result<RuntimeValue> {
    match value {
        ConstValue::Null => Ok(RuntimeValue::Null),
        ConstValue::Boolean(value) => Ok(RuntimeValue::Bool(*value)),
        ConstValue::String(value) => Ok(RuntimeValue::String(value.clone())),
        ConstValue::Enum(value) => Ok(RuntimeValue::String(value.to_string())),
        ConstValue::Number(value) => number_scalar(value, label),
        ConstValue::Binary(_) | ConstValue::List(_) | ConstValue::Object(_) => {
            graphql_error(format!("GraphQL {label} must be a scalar"))
        }
    }
}

pub(super) fn number_scalar(value: &serde_json::Number, label: &str) -> Result<RuntimeValue> {
    if let Some(value) = value.as_u64() {
        Ok(RuntimeValue::Unsigned(value))
    } else if let Some(value) = value.as_i64() {
        Ok(RuntimeValue::Integer(value))
    } else {
        graphql_error(format!("GraphQL {label} must be an integer"))
    }
}

pub(super) fn resolve_string(
    value: &Value,
    parameters: &Parameters,
    label: &str,
) -> Result<String> {
    match resolve_scalar(value, parameters, label)? {
        RuntimeValue::String(value) => Ok(value),
        _ => graphql_error(format!("GraphQL {label} must be a string")),
    }
}

pub(super) fn resolve_token(value: &Value, parameters: &Parameters, label: &str) -> Result<String> {
    match value {
        Value::Enum(value) => Ok(value.to_string()),
        _ => resolve_string(value, parameters, label),
    }
}

pub(super) fn resolve_u64(value: &Value, parameters: &Parameters, label: &str) -> Result<u64> {
    match resolve_scalar(value, parameters, label)? {
        RuntimeValue::Unsigned(value) => Ok(value),
        _ => graphql_error(format!("GraphQL {label} must be unsigned")),
    }
}

pub(super) fn resolve_usize(value: &Value, parameters: &Parameters, label: &str) -> Result<usize> {
    usize::try_from(resolve_u64(value, parameters, label)?)
        .map_err(|_| Error::Graphql(format!("GraphQL {label} exceeds platform capacity")))
}

pub(super) fn require_unsigned_parameter(
    parameters: &Parameters,
    name: &str,
    label: &str,
) -> Result<u64> {
    match parameters.get(name) {
        Some(RuntimeValue::Unsigned(value)) => Ok(*value),
        Some(_) => graphql_error(format!("GraphQL {label} variable ${name} must be unsigned")),
        None => graphql_error(format!("missing GraphQL variable ${name}")),
    }
}

pub(super) fn runtime_type(
    value: &Value,
    parameters: &Parameters,
    label: &str,
) -> Result<RuntimeType> {
    RuntimeType::new(resolve_string(value, parameters, label)?)
        .map_err(|error| Error::Graphql(error.to_string()))
}

pub(super) fn resolve_object<'a>(
    value: &'a Value,
    label: &str,
) -> Result<&'a async_graphql_value::indexmap::IndexMap<async_graphql_value::Name, Value>> {
    let Value::Object(value) = value else {
        return graphql_error(format!("GraphQL {label} must be an input object"));
    };
    Ok(value)
}

pub(super) fn object_value<'a>(
    object: &'a async_graphql_value::indexmap::IndexMap<async_graphql_value::Name, Value>,
    name: &str,
) -> Result<&'a Value> {
    object
        .get(name)
        .ok_or_else(|| Error::Graphql(format!("GraphQL input field {name:?} is required")))
}

pub(super) fn reject_unknown_object_fields(
    object: &async_graphql_value::indexmap::IndexMap<async_graphql_value::Name, Value>,
    allowed: &BTreeSet<&str>,
    label: &str,
) -> Result<()> {
    let unknown = object
        .keys()
        .filter(|name| !allowed.contains(name.as_str()))
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    if unknown.is_empty() {
        Ok(())
    } else {
        graphql_error(format!(
            "GraphQL {label} contains unknown fields: {}",
            unknown.join(", ")
        ))
    }
}

pub(super) fn validate_graphql_name(value: &str, label: &str) -> Result<()> {
    let mut characters = value.chars();
    let Some(first) = characters.next() else {
        return graphql_error(format!("GraphQL {label} must not be empty"));
    };
    if !(first == '_' || first.is_ascii_alphabetic())
        || !characters.all(|character| character == '_' || character.is_ascii_alphanumeric())
        || value.starts_with("__")
    {
        return graphql_error(format!(
            "GraphQL {label} {value:?} is not a permitted GraphQL name"
        ));
    }
    Ok(())
}

pub(super) fn graphql_error<T>(message: impl Into<String>) -> Result<T> {
    Err(Error::Graphql(message.into()))
}
