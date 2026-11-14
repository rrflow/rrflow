//! The flight schema and public record and event projections.

use super::*;

/// Installs or repairs the canonical flight tables in a schema under
/// construction. The caller owns the revision bump because that
/// compare-and-swap belongs to its exact read stamp.
pub(super) fn register_flight_schema(
    registry: &mut RuntimeSchemaRegistry,
    used_kinds: &BTreeSet<RuntimeType>,
) -> Result<bool> {
    let record_kind = record_kind()?;
    let event_kind = event_kind()?;
    let required = |value_type: RuntimeValueType| RuntimePropertySchema::required(value_type);
    let optional = |value_type: RuntimeValueType| RuntimePropertySchema::optional(value_type);
    let record_schema = RuntimeRecordSchema {
        properties: BTreeMap::from([
            (
                "format_version".into(),
                required(RuntimeValueType::Unsigned),
            ),
            ("cohort_sha256".into(), required(RuntimeValueType::Digest)),
            ("prompt".into(), required(RuntimeValueType::String)),
            ("provider".into(), required(RuntimeValueType::String)),
            ("context_mode".into(), required(RuntimeValueType::String)),
            (
                "context_valid_at_unix_ms".into(),
                required(RuntimeValueType::Unsigned),
            ),
            ("context_seeds".into(), required(RuntimeValueType::List)),
            (
                "context_max_graph_depth".into(),
                required(RuntimeValueType::Unsigned),
            ),
            (
                "context_max_items".into(),
                required(RuntimeValueType::Unsigned),
            ),
            (
                "context_max_output_bytes".into(),
                required(RuntimeValueType::Unsigned),
            ),
            (
                "context_max_storage_keys".into(),
                required(RuntimeValueType::Unsigned),
            ),
            (
                "acceptance_marker".into(),
                optional(RuntimeValueType::String),
            ),
            ("task_id".into(), optional(RuntimeValueType::String)),
            ("status".into(), required(RuntimeValueType::String)),
            (
                "created_at_unix_ms".into(),
                required(RuntimeValueType::Unsigned),
            ),
            ("idempotency_key".into(), required(RuntimeValueType::String)),
            (
                "operation_sha256".into(),
                required(RuntimeValueType::Digest),
            ),
            (
                "read_runtime_manifest_sha256".into(),
                required(RuntimeValueType::Digest),
            ),
            (
                "read_runtime_cursor".into(),
                required(RuntimeValueType::Unsigned),
            ),
            (
                "read_schema_revision".into(),
                optional(RuntimeValueType::Unsigned),
            ),
            (
                "read_catalogue_revision".into(),
                required(RuntimeValueType::Unsigned),
            ),
            (
                "context_request_sha256".into(),
                optional(RuntimeValueType::Digest),
            ),
            (
                "context_plan_sha256".into(),
                optional(RuntimeValueType::Digest),
            ),
            (
                "context_packet_sha256".into(),
                optional(RuntimeValueType::Digest),
            ),
            ("context_items".into(), required(RuntimeValueType::Unsigned)),
            (
                "context_output_bytes".into(),
                required(RuntimeValueType::Unsigned),
            ),
            ("context_truncated".into(), required(RuntimeValueType::Bool)),
            (
                "context_selected_versions".into(),
                required(RuntimeValueType::Unsigned),
            ),
            ("elapsed_ms".into(), required(RuntimeValueType::Unsigned)),
            ("use_count".into(), required(RuntimeValueType::Unsigned)),
            ("event_count".into(), required(RuntimeValueType::Unsigned)),
        ]),
        allow_additional_properties: false,
        unique_properties: BTreeSet::new(),
    };
    let event_schema = RuntimeEventSchema {
        subject_required: true,
        subject_types: BTreeSet::from([record_kind.clone()]),
        properties: BTreeMap::from([
            ("ordinal".into(), required(RuntimeValueType::Unsigned)),
            ("at_unix_ms".into(), required(RuntimeValueType::Unsigned)),
            ("elapsed_ms".into(), required(RuntimeValueType::Unsigned)),
            ("stage".into(), required(RuntimeValueType::String)),
            ("kind".into(), required(RuntimeValueType::String)),
            ("label".into(), required(RuntimeValueType::String)),
            ("detail".into(), required(RuntimeValueType::String)),
            ("data".into(), required(RuntimeValueType::Map)),
        ]),
        allow_additional_properties: false,
    };
    let record_changed = registry
        .define_record_table(
            record_kind.clone(),
            RuntimeLogicalModel::LifecycleRecord,
            record_schema,
        )
        .map_err(core_contract)?;
    let event_changed = registry
        .define_event_table(
            event_kind,
            RuntimeLogicalModel::LifecycleEvent,
            event_schema,
        )
        .map_err(core_contract)?;
    // The edge table's target set grows with every record kind a flight has
    // used, so an edge is never written to an undeclared endpoint kind.
    let use_kind = use_kind()?;
    let mut targets = registry
        .relations
        .get(&use_kind)
        .map(|schema| schema.to.clone())
        .unwrap_or_default();
    targets.extend(used_kinds.iter().cloned());
    let use_changed = if targets.is_empty() {
        false
    } else {
        registry
            .define_relation_table(
                use_kind,
                RuntimeRelationSchema {
                    from: BTreeSet::from([record_kind]),
                    to: targets,
                    properties: BTreeMap::from([
                        ("rank".into(), required(RuntimeValueType::Unsigned)),
                        ("score".into(), required(RuntimeValueType::Decimal)),
                        ("evidence".into(), required(RuntimeValueType::List)),
                    ]),
                    allow_additional_properties: false,
                    // One edge per distinct packet record is constructed here
                    // and validated by the contract. A schema pair constraint
                    // would make every later commit in the scope load every
                    // relation in it (POAM-034).
                    unique_pair: false,
                    max_outgoing: None,
                    max_incoming: None,
                },
            )
            .map_err(core_contract)?
    };
    Ok(record_changed || event_changed || use_changed)
}

pub(super) fn record_properties(record: &PromptFlightRecord) -> Result<RuntimeProperties> {
    let mut properties = RuntimeProperties::from([
        (
            "format_version".to_owned(),
            RuntimeValue::Unsigned(u64::from(record.format_version)),
        ),
        (
            "cohort_sha256".to_owned(),
            RuntimeValue::Digest(record.cohort_sha256.clone()),
        ),
        (
            "prompt".to_owned(),
            RuntimeValue::String(record.prompt.clone()),
        ),
        (
            "provider".to_owned(),
            RuntimeValue::String(record.provider.as_str().into()),
        ),
        (
            "context_mode".to_owned(),
            RuntimeValue::String(record.context_mode.as_str().into()),
        ),
        (
            "context_valid_at_unix_ms".to_owned(),
            RuntimeValue::Unsigned(record.context_request.valid_at),
        ),
        (
            "context_seeds".to_owned(),
            RuntimeValue::List(
                record
                    .context_request
                    .seeds
                    .iter()
                    .map(|seed| {
                        RuntimeValue::Map(BTreeMap::from([
                            (
                                "kind".to_owned(),
                                RuntimeValue::String(seed.kind.as_str().into()),
                            ),
                            (
                                "id".to_owned(),
                                RuntimeValue::String(seed.id.as_str().into()),
                            ),
                        ]))
                    })
                    .collect(),
            ),
        ),
        (
            "context_max_graph_depth".to_owned(),
            RuntimeValue::Unsigned(u64::from(record.context_request.max_graph_depth)),
        ),
        (
            "context_max_items".to_owned(),
            RuntimeValue::Unsigned(record.context_request.max_items),
        ),
        (
            "context_max_output_bytes".to_owned(),
            RuntimeValue::Unsigned(record.context_request.max_output_bytes),
        ),
        (
            "context_max_storage_keys".to_owned(),
            RuntimeValue::Unsigned(record.context_request.max_storage_keys),
        ),
        (
            "status".to_owned(),
            RuntimeValue::String(record.status.as_str().into()),
        ),
        (
            "created_at_unix_ms".to_owned(),
            RuntimeValue::Unsigned(record.created_at_unix_ms),
        ),
        (
            "idempotency_key".to_owned(),
            RuntimeValue::String(record.idempotency_key.as_str().into()),
        ),
        (
            "operation_sha256".to_owned(),
            RuntimeValue::Digest(record.operation_sha256.clone()),
        ),
        (
            "read_runtime_manifest_sha256".to_owned(),
            RuntimeValue::Digest(record.read.runtime_manifest_sha256.clone()),
        ),
        (
            "read_runtime_cursor".to_owned(),
            RuntimeValue::Unsigned(record.read.runtime_cursor),
        ),
        (
            "read_catalogue_revision".to_owned(),
            RuntimeValue::Unsigned(record.read.catalogue_revision),
        ),
        (
            "context_items".to_owned(),
            RuntimeValue::Unsigned(record.metrics.context_items),
        ),
        (
            "context_output_bytes".to_owned(),
            RuntimeValue::Unsigned(record.metrics.context_output_bytes),
        ),
        (
            "context_truncated".to_owned(),
            RuntimeValue::Bool(
                record
                    .context
                    .as_ref()
                    .is_some_and(|context| context.truncated),
            ),
        ),
        (
            "context_selected_versions".to_owned(),
            RuntimeValue::Unsigned(
                record
                    .context
                    .as_ref()
                    .map_or(0, |context| context.selected_versions),
            ),
        ),
        (
            "elapsed_ms".to_owned(),
            RuntimeValue::Unsigned(record.metrics.elapsed_ms),
        ),
        (
            "use_count".to_owned(),
            RuntimeValue::Unsigned(record.use_count),
        ),
        (
            "event_count".to_owned(),
            RuntimeValue::Unsigned(record.event_count),
        ),
    ]);
    if let Some(marker) = &record.acceptance_marker {
        properties.insert(
            "acceptance_marker".into(),
            RuntimeValue::String(marker.clone()),
        );
    }
    if let Some(task_id) = &record.task_id {
        properties.insert(
            "task_id".into(),
            RuntimeValue::String(task_id.as_str().into()),
        );
    }
    if let Some(revision) = record.read.schema_revision {
        properties.insert(
            "read_schema_revision".into(),
            RuntimeValue::Unsigned(revision),
        );
    }
    if let Some(context) = &record.context {
        properties.insert(
            "context_request_sha256".into(),
            RuntimeValue::Digest(context.request_sha256.clone()),
        );
        properties.insert(
            "context_plan_sha256".into(),
            RuntimeValue::Digest(context.plan_sha256.clone()),
        );
        properties.insert(
            "context_packet_sha256".into(),
            RuntimeValue::Digest(context.packet_sha256.clone()),
        );
    }
    Ok(properties)
}

/// Projects one stored row. A flight row carries no scope property: the scope
/// is the runtime scope it was committed in, which every reader names.
pub(super) fn public_record(
    record: &RuntimeRecord,
    scope: &str,
    flight_cursor: u64,
    known_at_cursor: u64,
) -> Result<PromptFlightRecord> {
    let properties = &record.properties;
    let format_version = u16::try_from(unsigned(properties, "format_version")?)
        .map_err(|_| flight_storage("format_version exceeds u16"))?;
    if format_version != PROMPT_FLIGHT_FORMAT_VERSION {
        return Err(flight_storage(&format!(
            "unsupported prompt flight format version {format_version}"
        )));
    }
    let flight_id = CanonicalId::new(record.reference.id.as_str())
        .map_err(|error| flight_storage(&error.to_string()))?;
    let context_mode = PromptFlightContextMode::parse(&text(properties, "context_mode")?)
        .map_err(|error| flight_storage(&error.to_string()))?;
    let context = match context_mode {
        PromptFlightContextMode::Fresh => None,
        PromptFlightContextMode::Pruned => Some(PromptFlightContextSummary {
            request_sha256: digest_text(properties, "context_request_sha256")?,
            plan_sha256: digest_text(properties, "context_plan_sha256")?,
            packet_sha256: digest_text(properties, "context_packet_sha256")?,
            items: unsigned(properties, "context_items")?,
            output_bytes: unsigned(properties, "context_output_bytes")?,
            truncated: boolean(properties, "context_truncated")?,
            selected_versions: unsigned(properties, "context_selected_versions")?,
        }),
    };
    let seeds = match properties.get("context_seeds") {
        Some(RuntimeValue::List(values)) => values
            .iter()
            .map(|value| match value {
                RuntimeValue::Map(entry) => Ok(DataReference {
                    kind: canonical_entry(entry, "kind")?,
                    id: canonical_entry(entry, "id")?,
                }),
                _ => Err(flight_storage("context seed is not a reference map")),
            })
            .collect::<Result<Vec<_>>>()?,
        _ => return Err(flight_storage("context_seeds is missing or not a list")),
    };
    Ok(PromptFlightRecord {
        format_version,
        flight_id,
        scope: scope.to_owned(),
        cohort_sha256: digest_text(properties, "cohort_sha256")?,
        prompt: text(properties, "prompt")?,
        provider: PromptFlightProvider::parse(&text(properties, "provider")?)
            .map_err(|error| flight_storage(&error.to_string()))?,
        context_mode,
        context_request: PromptFlightContextRequest {
            valid_at: unsigned(properties, "context_valid_at_unix_ms")?,
            seeds,
            max_graph_depth: u8::try_from(unsigned(properties, "context_max_graph_depth")?)
                .map_err(|_| flight_storage("context_max_graph_depth exceeds u8"))?,
            max_items: unsigned(properties, "context_max_items")?,
            max_output_bytes: unsigned(properties, "context_max_output_bytes")?,
            max_storage_keys: unsigned(properties, "context_max_storage_keys")?,
        },
        acceptance_marker: optional_text(properties, "acceptance_marker")?,
        task_id: optional_text(properties, "task_id")?
            .map(CanonicalId::new)
            .transpose()
            .map_err(|error| flight_storage(&error.to_string()))?,
        status: PromptFlightStatus::parse(&text(properties, "status")?)
            .map_err(|error| flight_storage(&error.to_string()))?,
        created_at_unix_ms: unsigned(properties, "created_at_unix_ms")?,
        idempotency_key: CorrelationId::new(text(properties, "idempotency_key")?)
            .map_err(|error| flight_storage(&error.to_string()))?,
        operation_sha256: digest_text(properties, "operation_sha256")?,
        read: ContextReadStamp {
            runtime_manifest_sha256: digest_text(properties, "read_runtime_manifest_sha256")?,
            runtime_cursor: unsigned(properties, "read_runtime_cursor")?,
            schema_revision: optional_unsigned(properties, "read_schema_revision")?,
            catalogue_revision: unsigned(properties, "read_catalogue_revision")?,
        },
        context,
        metrics: PromptFlightMetrics {
            context_items: unsigned(properties, "context_items")?,
            context_output_bytes: unsigned(properties, "context_output_bytes")?,
            elapsed_ms: unsigned(properties, "elapsed_ms")?,
        },
        flight_cursor,
        known_at_cursor,
        use_count: unsigned(properties, "use_count")?,
        event_count: unsigned(properties, "event_count")?,
    })
}

pub(super) fn event_properties(event: &PromptFlightEvent) -> Result<RuntimeProperties> {
    Ok(RuntimeProperties::from([
        ("ordinal".to_owned(), RuntimeValue::Unsigned(event.ordinal)),
        (
            "at_unix_ms".to_owned(),
            RuntimeValue::Unsigned(event.at_unix_ms),
        ),
        (
            "elapsed_ms".to_owned(),
            RuntimeValue::Unsigned(event.elapsed_ms),
        ),
        (
            "stage".to_owned(),
            RuntimeValue::String(event.stage.as_str().into()),
        ),
        (
            "kind".to_owned(),
            RuntimeValue::String(event.kind.as_str().into()),
        ),
        (
            "label".to_owned(),
            RuntimeValue::String(event.label.clone()),
        ),
        (
            "detail".to_owned(),
            RuntimeValue::String(event.detail.clone()),
        ),
        (
            "data".to_owned(),
            RuntimeValue::Map(runtime_properties(&event.data)?),
        ),
    ]))
}

pub(super) fn public_event(properties: &RuntimeProperties) -> Result<PromptFlightEvent> {
    let data = match properties.get("data") {
        Some(RuntimeValue::Map(values)) => public_properties(values)?,
        _ => return Err(flight_storage("event data is missing or not a map")),
    };
    Ok(PromptFlightEvent {
        ordinal: unsigned(properties, "ordinal")?,
        at_unix_ms: unsigned(properties, "at_unix_ms")?,
        elapsed_ms: unsigned(properties, "elapsed_ms")?,
        stage: PromptFlightStage::parse(&text(properties, "stage")?)
            .map_err(|error| flight_storage(&error.to_string()))?,
        kind: CanonicalId::new(text(properties, "kind")?)
            .map_err(|error| flight_storage(&error.to_string()))?,
        label: text(properties, "label")?,
        detail: text(properties, "detail")?,
        data,
    })
}
