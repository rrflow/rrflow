//! Deriving the schema and lowering GraphQL operations to rrflowQL.

use super::*;

/// Derive the exact GraphQL-visible source/field catalogue for a historical
/// schema revision within `catalog`'s already captured read stamp.
pub fn derive_graphql_schema(catalog: &Catalog, known_at_cursor: u64) -> Result<GraphqlSchema> {
    if known_at_cursor > catalog.read.commit_cursor {
        return graphql_error(format!(
            "known cursor {known_at_cursor} exceeds captured head {}",
            catalog.read.commit_cursor
        ));
    }
    let registry = catalog.schema_at(known_at_cursor).ok_or_else(|| {
        Error::Graphql(format!(
            "no schema was visible at known cursor {known_at_cursor}"
        ))
    })?;
    let mut definitions = Vec::new();
    for kind in registry.records.keys() {
        definitions.push(source_definition(
            GraphqlSourceFamily::Record,
            Some(kind.as_str()),
            Source::Record { kind: kind.clone() },
            registry,
        )?);
    }
    for kind in registry.relations.keys() {
        definitions.push(source_definition(
            GraphqlSourceFamily::Relation,
            Some(kind.as_str()),
            Source::Relation { kind: kind.clone() },
            registry,
        )?);
    }
    for kind in registry.events.keys() {
        definitions.push(source_definition(
            GraphqlSourceFamily::Event,
            Some(kind.as_str()),
            Source::Event { kind: kind.clone() },
            registry,
        )?);
    }

    definitions.push(source_definition(
        GraphqlSourceFamily::Series,
        None,
        Source::Series {
            kind: RuntimeType::new("graphql-schema-series")
                .map_err(|error| Error::Graphql(error.to_string()))?,
        },
        registry,
    )?);
    definitions.push(source_definition(
        GraphqlSourceFamily::Geo,
        None,
        Source::Geo {
            kind: RuntimeType::new("graphql-schema-geo")
                .map_err(|error| Error::Graphql(error.to_string()))?,
        },
        registry,
    )?);
    definitions.push(source_definition(
        GraphqlSourceFamily::Claim,
        None,
        Source::Claim { predicate: None },
        registry,
    )?);

    for (relation, definition) in &registry.relations {
        let Some(start_kind) = definition.from.iter().chain(&definition.to).next() else {
            continue;
        };
        definitions.push(source_definition(
            GraphqlSourceFamily::Traversal,
            Some(relation.as_str()),
            Source::Traversal {
                relation: relation.clone(),
                start: RuntimeRef {
                    kind: start_kind.clone(),
                    id: RuntimeId::new("graphql-schema-start")
                        .map_err(|error| Error::Graphql(error.to_string()))?,
                },
                direction: TraversalDirection::Both,
                max_depth: 1,
            },
            registry,
        )?);
    }

    definitions.sort_by(|left, right| source_sort_key(left).cmp(&source_sort_key(right)));
    if definitions
        .windows(2)
        .any(|pair| source_sort_key(&pair[0]) == source_sort_key(&pair[1]))
    {
        return graphql_error("GraphQL schema contains a duplicate source coordinate");
    }
    let mut schema = GraphqlSchema {
        contract_version: GRAPHQL_ADAPTER_CONTRACT_VERSION,
        read: catalog.read.clone(),
        known_at_cursor,
        schema_revision: registry.revision,
        sources: definitions,
        digest: String::new(),
    };
    schema.digest = schema_digest(&schema)?;
    schema.validate()?;
    Ok(schema)
}

/// Parse and lower GraphQL without touching storage. The returned parameters
/// include validated request values and operation defaults.
pub fn lower_graphql(request: &GraphqlRequest, catalog: &Catalog) -> Result<LoweredGraphqlQuery> {
    request.validate_envelope()?;
    let document = parse_query(&request.document)
        .map_err(|error| Error::Graphql(format!("GraphQL syntax error: {error}")))?;
    if !document.fragments.is_empty() {
        return graphql_error(
            "GraphQL fragments are outside the 1.0 lowering contract; inline the selection",
        );
    }
    let operation = select_operation(&document.operations, request.operation_name.as_deref())?;
    if operation.node.ty != OperationType::Query {
        return graphql_error("GraphQL mutation and subscription operations are not query ingress");
    }
    if !operation.node.directives.is_empty() {
        return graphql_error("GraphQL operation directives are not lowerable");
    }
    let parameters = validate_variables(&operation.node, &request.variables)?;
    let root = single_root_field(&operation.node)?;
    if !root.node.directives.is_empty() {
        return graphql_error("GraphQL root-field directives are not lowerable");
    }
    let response_key = root.node.response_key().node.to_string();
    let arguments = argument_map(&root.node.arguments)?;
    let source = lower_source(root.node.name.node.as_str(), &arguments, &parameters)?;
    let temporal = TemporalSelector {
        valid_at: lower_valid_at(required_argument(&arguments, "validAt")?, &parameters)?,
        known_at: lower_known_at(required_argument(&arguments, "knownAt")?, &parameters)?,
    };
    let known_at_cursor = resolve_known_cursor(&temporal.known_at, &parameters, &catalog.read)?;
    let schema = derive_graphql_schema(catalog, known_at_cursor)?;
    if !schema.admits(&source) {
        return graphql_error("GraphQL source is not present in the captured schema");
    }
    let join = arguments
        .get("join")
        .map(|value| lower_join(value, &parameters))
        .transpose()?;
    if matches!(source, Source::Traversal { .. }) && join.is_some() {
        return graphql_error("GraphQL traversal queries cannot also declare a join");
    }
    let projection = lower_projection(&root.node.selection_set.node.items, join.is_some())?;
    let filters = arguments
        .get("where")
        .map(|value| lower_filters(value, &parameters))
        .transpose()?
        .unwrap_or_default();
    let limit = arguments
        .get("limit")
        .map(|value| resolve_usize(value, &parameters, "limit"))
        .transpose()?;
    let (explain_contract, explain_analyze) = arguments
        .get("explain")
        .map(|value| lower_explain(value, &parameters))
        .transpose()?
        .unwrap_or((false, false));
    validate_allowed_arguments(root.node.name.node.as_str(), arguments.keys().copied())?;

    let query = Query {
        contract_version: QUERY_CONTRACT_VERSION,
        source,
        join,
        temporal,
        filters,
        projection,
        limit,
        explain_contract,
        explain_analyze,
    };
    query
        .validate()
        .map_err(|error| Error::Graphql(error.to_string()))?;
    Ok(LoweredGraphqlQuery {
        schema_digest: schema.digest,
        response_key,
        query,
        parameters,
    })
}

/// Lower GraphQL and invoke the existing rrflowQL binder. This is the only
/// B-05 entry point that produces a catalogue-bound query representation.
pub fn bind_graphql(request: &GraphqlRequest, catalog: &Catalog) -> Result<BoundQuery> {
    let lowered = lower_graphql(request, catalog)?;
    bind(&lowered.query, &lowered.parameters, catalog)
}

pub(super) fn source_definition(
    family: GraphqlSourceFamily,
    kind: Option<&str>,
    source: Source,
    registry: &rrd_core::RuntimeSchemaRegistry,
) -> Result<GraphqlSourceDefinition> {
    let fields = fields_for_source(&source, registry)?
        .into_iter()
        .map(|(name, accepted_types)| {
            validate_graphql_name(&name, "schema field")?;
            Ok(GraphqlFieldDefinition {
                name,
                accepted_types,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(GraphqlSourceDefinition {
        family,
        kind: kind.map(str::to_owned),
        fields,
    })
}

pub(super) fn schema_digest(schema: &GraphqlSchema) -> Result<String> {
    let bytes = serde_json::to_vec(&(
        schema.contract_version,
        &schema.read,
        schema.known_at_cursor,
        schema.schema_revision,
        &schema.sources,
    ))?;
    Ok(digest::sha256_hex(&bytes))
}

pub(super) fn source_sort_key(source: &GraphqlSourceDefinition) -> (GraphqlSourceFamily, &str) {
    (source.family, source.kind.as_deref().unwrap_or(""))
}

pub(super) fn source_coordinate(source: &Source) -> (GraphqlSourceFamily, Option<&str>) {
    match source {
        Source::Record { kind } => (GraphqlSourceFamily::Record, Some(kind.as_str())),
        Source::Relation { kind } => (GraphqlSourceFamily::Relation, Some(kind.as_str())),
        Source::Event { kind } => (GraphqlSourceFamily::Event, Some(kind.as_str())),
        Source::Series { .. } => (GraphqlSourceFamily::Series, None),
        Source::Geo { .. } => (GraphqlSourceFamily::Geo, None),
        Source::Traversal { relation, .. } => {
            (GraphqlSourceFamily::Traversal, Some(relation.as_str()))
        }
        Source::Claim { .. } => (GraphqlSourceFamily::Claim, None),
    }
}

pub(super) fn select_operation<'a>(
    operations: &'a DocumentOperations,
    requested: Option<&str>,
) -> Result<&'a Positioned<OperationDefinition>> {
    match operations {
        DocumentOperations::Single(operation) => {
            if requested.is_some() {
                graphql_error("operationName cannot select an anonymous GraphQL operation")
            } else {
                Ok(operation)
            }
        }
        DocumentOperations::Multiple(operations) => match requested {
            Some(name) => operations
                .get(name)
                .ok_or_else(|| Error::Graphql(format!("GraphQL operation {name:?} was not found"))),
            None if operations.len() == 1 => Ok(operations
                .values()
                .next()
                .expect("one named operation was checked")),
            None => graphql_error(
                "operationName is required when a GraphQL document has multiple operations",
            ),
        },
    }
}

pub(super) fn single_root_field(
    operation: &OperationDefinition,
) -> Result<&Positioned<async_graphql_parser::types::Field>> {
    if operation.selection_set.node.items.len() != 1 {
        return graphql_error("GraphQL query must select exactly one RRFlow root field");
    }
    match &operation.selection_set.node.items[0].node {
        Selection::Field(field) => Ok(field),
        Selection::FragmentSpread(_) | Selection::InlineFragment(_) => {
            graphql_error("GraphQL root selection must be a field")
        }
    }
}

pub(super) fn argument_map(
    arguments: &[(Positioned<async_graphql_value::Name>, Positioned<Value>)],
) -> Result<BTreeMap<&str, &Value>> {
    if arguments.len() > MAX_GRAPHQL_ARGUMENTS {
        return graphql_error(format!(
            "GraphQL root exceeds {MAX_GRAPHQL_ARGUMENTS} arguments"
        ));
    }
    let mut out = BTreeMap::new();
    for (name, value) in arguments {
        if out.insert(name.node.as_str(), &value.node).is_some() {
            return graphql_error(format!("duplicate GraphQL argument {:?}", name.node));
        }
    }
    Ok(out)
}

pub(super) fn required_argument<'a>(
    arguments: &BTreeMap<&str, &'a Value>,
    name: &str,
) -> Result<&'a Value> {
    arguments
        .get(name)
        .copied()
        .ok_or_else(|| Error::Graphql(format!("GraphQL argument {name:?} is required")))
}

pub(super) fn lower_source(
    root: &str,
    arguments: &BTreeMap<&str, &Value>,
    parameters: &Parameters,
) -> Result<Source> {
    match root {
        "record" => Ok(Source::Record {
            kind: runtime_type(required_argument(arguments, "kind")?, parameters, "kind")?,
        }),
        "relation" => Ok(Source::Relation {
            kind: runtime_type(required_argument(arguments, "kind")?, parameters, "kind")?,
        }),
        "event" => Ok(Source::Event {
            kind: runtime_type(required_argument(arguments, "kind")?, parameters, "kind")?,
        }),
        "series" => Ok(Source::Series {
            kind: runtime_type(required_argument(arguments, "kind")?, parameters, "kind")?,
        }),
        "geo" => Ok(Source::Geo {
            kind: runtime_type(required_argument(arguments, "kind")?, parameters, "kind")?,
        }),
        "claim" => Ok(Source::Claim {
            predicate: arguments
                .get("predicate")
                .map(|value| resolve_string(value, parameters, "predicate"))
                .transpose()?
                .map(Predicate::new)
                .transpose()
                .map_err(|error| Error::Graphql(error.to_string()))?,
        }),
        "traverse" => {
            let relation = runtime_type(
                required_argument(arguments, "relation")?,
                parameters,
                "relation",
            )?;
            let start_kind = resolve_string(
                required_argument(arguments, "startKind")?,
                parameters,
                "startKind",
            )?;
            let start_id = resolve_string(
                required_argument(arguments, "startId")?,
                parameters,
                "startId",
            )?;
            let direction = match resolve_token(
                required_argument(arguments, "direction")?,
                parameters,
                "direction",
            )?
            .as_str()
            {
                "OUTGOING" => TraversalDirection::Outgoing,
                "INCOMING" => TraversalDirection::Incoming,
                "BOTH" => TraversalDirection::Both,
                value => {
                    return graphql_error(format!(
                        "GraphQL direction {value:?} must be OUTGOING, INCOMING, or BOTH"
                    ))
                }
            };
            let depth = resolve_u64(required_argument(arguments, "depth")?, parameters, "depth")?;
            Ok(Source::Traversal {
                relation,
                start: RuntimeRef::parse(start_kind, start_id)
                    .map_err(|error| Error::Graphql(error.to_string()))?,
                direction,
                max_depth: u16::try_from(depth)
                    .map_err(|_| Error::Graphql("GraphQL depth exceeds u16".into()))?,
            })
        }
        _ => graphql_error(format!("unknown RRFlow GraphQL root field {root:?}")),
    }
}

pub(super) fn lower_join(value: &Value, parameters: &Parameters) -> Result<Join> {
    let object = resolve_object(value, "join")?;
    let family = resolve_token(object_value(object, "family")?, parameters, "join.family")?;
    let kind = runtime_type(object_value(object, "kind")?, parameters, "join.kind")?;
    let source = match family.as_str() {
        "RECORD" => Source::Record { kind },
        "RELATION" => Source::Relation { kind },
        "EVENT" => Source::Event { kind },
        "SERIES" => Source::Series { kind },
        "GEO" => Source::Geo { kind },
        _ => {
            return graphql_error(format!(
                "GraphQL join family {family:?} is not a lowerable source"
            ))
        }
    };
    let allowed = BTreeSet::from(["family", "kind", "leftField", "rightField"]);
    reject_unknown_object_fields(object, &allowed, "join")?;
    Ok(Join {
        source,
        left_field: resolve_string(
            object_value(object, "leftField")?,
            parameters,
            "join.leftField",
        )?,
        right_field: resolve_string(
            object_value(object, "rightField")?,
            parameters,
            "join.rightField",
        )?,
    })
}

pub(super) fn lower_valid_at(value: &Value, parameters: &Parameters) -> Result<TimeExpr> {
    match value {
        Value::Variable(name) => {
            require_unsigned_parameter(parameters, name.as_str(), "validAt")?;
            Ok(TimeExpr::Parameter(name.to_string()))
        }
        _ => Ok(TimeExpr::Literal(resolve_u64(
            value, parameters, "validAt",
        )?)),
    }
}

pub(super) fn lower_known_at(value: &Value, parameters: &Parameters) -> Result<CursorExpr> {
    match value {
        Value::Variable(name) => {
            require_unsigned_parameter(parameters, name.as_str(), "knownAt")?;
            Ok(CursorExpr::Parameter(name.to_string()))
        }
        Value::Enum(name) if name == "HEAD" => Ok(CursorExpr::Head),
        _ => Ok(CursorExpr::Literal(resolve_u64(
            value, parameters, "knownAt",
        )?)),
    }
}

pub(super) fn resolve_known_cursor(
    value: &CursorExpr,
    parameters: &Parameters,
    read: &ReadStamp,
) -> Result<u64> {
    let cursor = match value {
        CursorExpr::Head => read.commit_cursor,
        CursorExpr::Literal(value) => *value,
        CursorExpr::Parameter(name) => require_unsigned_parameter(parameters, name, "knownAt")?,
    };
    if cursor > read.commit_cursor {
        return graphql_error(format!(
            "known cursor {cursor} exceeds captured head {}",
            read.commit_cursor
        ));
    }
    Ok(cursor)
}

pub(super) fn lower_filters(value: &Value, parameters: &Parameters) -> Result<Vec<Filter>> {
    let Value::List(values) = value else {
        return graphql_error("GraphQL where argument must be a list");
    };
    if values.len() > MAX_GRAPHQL_PROJECTION_FIELDS {
        return graphql_error("GraphQL where argument is too large");
    }
    values
        .iter()
        .map(|value| {
            let object = resolve_object(value, "where item")?;
            let allowed = BTreeSet::from(["field", "comparison", "value"]);
            reject_unknown_object_fields(object, &allowed, "where item")?;
            let field = resolve_string(object_value(object, "field")?, parameters, "where.field")?;
            let comparison = match resolve_token(
                object_value(object, "comparison")?,
                parameters,
                "where.comparison",
            )?
            .as_str()
            {
                "EQ" => ComparisonOperator::Equal,
                "NE" => ComparisonOperator::NotEqual,
                "LT" => ComparisonOperator::LessThan,
                "LTE" => ComparisonOperator::LessThanOrEqual,
                "GT" => ComparisonOperator::GreaterThan,
                "GTE" => ComparisonOperator::GreaterThanOrEqual,
                "MATCH" => ComparisonOperator::Match,
                token => {
                    return graphql_error(format!("GraphQL comparison {token:?} is not supported"))
                }
            };
            let value = object_value(object, "value")?;
            let value = match value {
                Value::Variable(name) => {
                    if !parameters.contains_key(name.as_str()) {
                        return graphql_error(format!(
                            "missing GraphQL variable ${}",
                            name.as_str()
                        ));
                    }
                    ValueExpr::Parameter(name.to_string())
                }
                _ => ValueExpr::Literal(resolve_scalar(value, parameters, "where.value")?),
            };
            Ok(Filter {
                field,
                comparison,
                value,
            })
        })
        .collect()
}

pub(super) fn lower_projection(
    selections: &[Positioned<Selection>],
    joined: bool,
) -> Result<Projection> {
    if selections.is_empty() || selections.len() > MAX_GRAPHQL_PROJECTION_FIELDS {
        return graphql_error(format!(
            "GraphQL projection requires 1..={MAX_GRAPHQL_PROJECTION_FIELDS} fields"
        ));
    }
    let mut fields = Vec::new();
    for selection in selections {
        let Selection::Field(field) = &selection.node else {
            return graphql_error("GraphQL projection fragments are not lowerable");
        };
        if field.node.alias.is_some()
            || !field.node.arguments.is_empty()
            || !field.node.directives.is_empty()
        {
            return graphql_error(
                "GraphQL projection aliases, arguments, and directives are not lowerable",
            );
        }
        let name = field.node.name.node.as_str();
        if joined {
            if !matches!(name, "left" | "right") || field.node.selection_set.node.items.is_empty() {
                return graphql_error(
                    "joined GraphQL projections must use non-empty left and right selections",
                );
            }
            for nested in &field.node.selection_set.node.items {
                let Selection::Field(nested) = &nested.node else {
                    return graphql_error("joined GraphQL projection cannot contain fragments");
                };
                if nested.node.alias.is_some()
                    || !nested.node.arguments.is_empty()
                    || !nested.node.directives.is_empty()
                    || !nested.node.selection_set.node.items.is_empty()
                {
                    return graphql_error("joined GraphQL projection leaves must be plain fields");
                }
                fields.push(format!("{name}.{}", nested.node.name.node));
            }
        } else {
            if !field.node.selection_set.node.items.is_empty() {
                return graphql_error("GraphQL projection fields must be scalar leaves");
            }
            fields.push(name.to_owned());
        }
    }
    if fields.len() > MAX_GRAPHQL_PROJECTION_FIELDS {
        return graphql_error("GraphQL projection is too large");
    }
    let unique = fields.iter().collect::<BTreeSet<_>>();
    if unique.len() != fields.len() {
        return graphql_error("GraphQL projection fields must be unique");
    }
    Ok(Projection::Fields(fields))
}

pub(super) fn lower_explain(value: &Value, parameters: &Parameters) -> Result<(bool, bool)> {
    match resolve_token(value, parameters, "explain")?.as_str() {
        "CONTRACT" => Ok((true, false)),
        "ANALYZE" => Ok((false, true)),
        token => graphql_error(format!(
            "GraphQL explain {token:?} must be CONTRACT or ANALYZE"
        )),
    }
}
