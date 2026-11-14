//! Executing a traced rrflowQL query and linking its storage evidence.

use super::*;

pub(super) const MAX_QUERY_BYTES: usize = 64 * 1024;
pub(super) const MAX_QUERY_PARAMETERS: usize = 128;
pub(super) const MAX_PARAMETER_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TracedQueryExecution {
    pub canonical: String,
    pub query: Query,
    pub bound: BoundQuery,
    pub plan: PhysicalPlan,
    pub execution: QueryExecution,
}

/// Converts a JSON object of scalar values into the typed binder contract.
/// Positive JSON integers become unsigned; negative integers remain signed.
pub fn query_parameters_from_json(value: &Value) -> Result<Parameters, Box<dyn std::error::Error>> {
    let object = value
        .as_object()
        .ok_or("query parameters must be a JSON object")?;
    if object.len() > MAX_QUERY_PARAMETERS {
        return Err(format!("query parameters exceed {MAX_QUERY_PARAMETERS} entries").into());
    }
    if serde_json::to_vec(value)?.len() > MAX_PARAMETER_BYTES {
        return Err(format!("query parameters exceed {MAX_PARAMETER_BYTES} encoded bytes").into());
    }
    object
        .iter()
        .map(|(name, value)| Ok((name.clone(), scalar_parameter(name, value)?)))
        .collect()
}

pub(super) fn scalar_parameter(
    name: &str,
    value: &Value,
) -> Result<RuntimeValue, Box<dyn std::error::Error>> {
    match value {
        Value::Null => Ok(RuntimeValue::Null),
        Value::Bool(value) => Ok(RuntimeValue::Bool(*value)),
        Value::Number(value) if value.as_u64().is_some() => {
            Ok(RuntimeValue::Unsigned(value.as_u64().expect("checked")))
        }
        Value::Number(value) if value.as_i64().is_some() => {
            Ok(RuntimeValue::Integer(value.as_i64().expect("checked")))
        }
        Value::String(value) => Ok(RuntimeValue::String(value.clone())),
        _ => Err(format!(
            "query parameter {name:?} must be null, boolean, integer, unsigned integer, or string"
        )
        .into()),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn execute_traced_query<E: StorageEngine>(
    store: &E,
    scope: ScopeId,
    source: &str,
    parameters: &Parameters,
    budget: &ExecutionBudget,
    actor: &str,
    at: Millis,
) -> Result<TracedQueryExecution, Box<dyn std::error::Error>> {
    let read = store.runtime().read_stamp(&scope)?;
    let pipeline = StampedQueryPipeline::new(store, read.clone())?;
    let query_digest = digest::sha256_hex(source.as_bytes());
    let parameter_bytes = serde_json::to_vec(parameters)?;
    let parameter_digest = digest::sha256_hex(&parameter_bytes);
    let at_bytes = at.to_be_bytes();
    let cursor_bytes = read.commit_cursor.to_be_bytes();
    let identity = TraceIdentity::derive(&[
        scope.as_str().as_bytes(),
        query_digest.as_bytes(),
        parameter_digest.as_bytes(),
        &at_bytes,
        &cursor_bytes,
    ])?;
    let links = vec![TraceLink::Read {
        stamp: read.clone(),
    }];
    let root_attributes = RuntimeProperties::from([
        ("query_digest".into(), RuntimeValue::Digest(query_digest)),
        (
            "query_bytes".into(),
            RuntimeValue::Unsigned(source.len() as u64),
        ),
        (
            "parameter_digest".into(),
            RuntimeValue::Digest(parameter_digest),
        ),
        (
            "parameter_count".into(),
            RuntimeValue::Unsigned(parameters.len() as u64),
        ),
        (
            "contract_version".into(),
            RuntimeValue::Unsigned(u64::from(QUERY_CONTRACT_VERSION)),
        ),
        (
            TraceAttribute::MaxStorageKeys.into(),
            RuntimeValue::Unsigned(budget.max_storage_keys),
        ),
        (
            "max_rows".into(),
            RuntimeValue::Unsigned(budget.max_rows as u64),
        ),
        (
            "max_output_bytes".into(),
            RuntimeValue::Unsigned(budget.max_output_bytes as u64),
        ),
    ]);
    let root_identity = identity.clone();
    let root = DurableTraceSpan::start(
        store,
        scope.clone(),
        actor,
        identity,
        None,
        TraceBoundary::Ql,
        "rrflow.query.run",
        at,
        TraceDataClass::Control,
        links.clone(),
        root_attributes,
    )?;

    if source.trim().is_empty() {
        return fail_query(
            store,
            None,
            root,
            "parse_bind",
            "empty_query",
            TraceOutcome::Denied,
            "rrflowQL query must not be empty".into(),
        );
    }
    if source.len() > MAX_QUERY_BYTES {
        return fail_query(
            store,
            None,
            root,
            "parse_bind",
            "contract_limit",
            TraceOutcome::Denied,
            format!("rrflowQL query exceeds {MAX_QUERY_BYTES} bytes").into(),
        );
    }

    let prepare_identity = root_identity.child(&[b"rrflow.query.parse_bind"])?;
    let prepare = match DurableTraceSpan::start(
        store,
        scope.clone(),
        actor,
        prepare_identity,
        Some(root_identity.span_id.clone()),
        TraceBoundary::Ql,
        "rrflow.query.parse_bind",
        root.observed_at(),
        TraceDataClass::Control,
        links.clone(),
        RuntimeProperties::new(),
    ) {
        Ok(span) => span,
        Err(error) => {
            return fail_query(
                store,
                None,
                root,
                "parse_bind",
                "trace_start",
                TraceOutcome::Error,
                error,
            )
        }
    };
    let query = match rrd_query::parse(source) {
        Ok(query) => query,
        Err(error) => {
            return fail_query(
                store,
                Some(prepare),
                root,
                "parse_bind",
                "parse",
                TraceOutcome::Error,
                error.into(),
            )
        }
    };
    let bound = match pipeline.bind(&query, parameters, budget) {
        Ok(bound) => bound,
        Err(error) => {
            return fail_query(
                store,
                Some(prepare),
                root,
                "parse_bind",
                query_error_class(&error),
                TraceOutcome::Error,
                error.into(),
            )
        }
    };
    let canonical = query.canonical();
    let (source_family, source_type) = source_identity(&query.source);
    let prepare_attributes = RuntimeProperties::from([
        (
            "canonical_digest".into(),
            RuntimeValue::Digest(digest::sha256_hex(canonical.as_bytes())),
        ),
        (
            "source_family".into(),
            RuntimeValue::String(source_family.into()),
        ),
        ("source_type".into(), RuntimeValue::String(source_type)),
        (
            "schema_revision".into(),
            RuntimeValue::Unsigned(bound.schema_revision),
        ),
        (
            "filter_count".into(),
            RuntimeValue::Unsigned(bound.filters.len() as u64),
        ),
    ]);
    if let Err(error) = prepare.finish(store, TraceOutcome::Ok, Vec::new(), prepare_attributes) {
        return fail_query(
            store,
            None,
            root,
            "parse_bind",
            "trace_finish",
            TraceOutcome::Error,
            error,
        );
    }

    let planning_identity = root_identity.child(&[b"rrflow.query.plan"])?;
    let planning = match DurableTraceSpan::start(
        store,
        scope.clone(),
        actor,
        planning_identity,
        Some(root_identity.span_id.clone()),
        TraceBoundary::Ql,
        "rrflow.query.plan",
        root.observed_at(),
        TraceDataClass::Control,
        links.clone(),
        RuntimeProperties::new(),
    ) {
        Ok(span) => span,
        Err(error) => {
            return fail_query(
                store,
                None,
                root,
                "planning",
                "trace_start",
                TraceOutcome::Error,
                error,
            )
        }
    };
    let plan = match pipeline.plan(&bound) {
        Ok(plan) => plan,
        Err(error) => {
            return fail_query(
                store,
                Some(planning),
                root,
                "planning",
                query_error_class(&error),
                TraceOutcome::Error,
                error.into(),
            )
        }
    };
    let selected = plan
        .explanation
        .candidates
        .iter()
        .filter(|candidate| candidate.selected)
        .map(|candidate| RuntimeValue::String(candidate.name.clone()))
        .collect::<Vec<_>>();
    let rejected = plan
        .explanation
        .candidates
        .iter()
        .filter(|candidate| !candidate.selected)
        .map(|candidate| RuntimeValue::String(candidate.name.clone()))
        .collect::<Vec<_>>();
    let planning_attributes = RuntimeProperties::from([
        (
            "exact".into(),
            RuntimeValue::Bool(plan.explanation.contract.exact),
        ),
        (
            "logical_operator_count".into(),
            RuntimeValue::Unsigned(plan.logical.operators.len() as u64),
        ),
        (
            "physical_operator_count".into(),
            RuntimeValue::Unsigned(plan.operators.len() as u64),
        ),
        ("selected_paths".into(), RuntimeValue::List(selected)),
        ("rejected_paths".into(), RuntimeValue::List(rejected)),
    ]);
    if let Err(error) = planning.finish(
        store,
        TraceOutcome::Ok,
        vec![TraceLink::Plan {
            plan_digest: plan.digest.clone(),
        }],
        planning_attributes,
    ) {
        return fail_query(
            store,
            None,
            root,
            "planning",
            "trace_finish",
            TraceOutcome::Error,
            error,
        );
    }

    let execution_identity = root_identity.child(&[b"rrflow.query.execute"])?;
    let storage_identity = execution_identity.child(&[b"rrflow.storage.runtime_read"])?;
    let mut execution_links = links;
    execution_links.push(TraceLink::Plan {
        plan_digest: plan.digest.clone(),
    });
    let execution_span = match DurableTraceSpan::start(
        store,
        scope.clone(),
        actor,
        execution_identity.clone(),
        Some(root_identity.span_id),
        TraceBoundary::Ql,
        "rrflow.query.execute",
        root.observed_at(),
        TraceDataClass::Control,
        execution_links,
        RuntimeProperties::new(),
    ) {
        Ok(span) => span,
        Err(error) => {
            return fail_query(
                store,
                None,
                root,
                "execution",
                "trace_start",
                TraceOutcome::Error,
                error,
            )
        }
    };
    let storage_span = match DurableTraceSpan::start(
        store,
        scope,
        actor,
        storage_identity,
        Some(execution_identity.span_id),
        TraceBoundary::Kv,
        "rrflow.storage.runtime_read",
        root.observed_at(),
        TraceDataClass::Control,
        execution_links_for_storage(&read, &plan),
        RuntimeProperties::from([(
            "evidence_policy".into(),
            RuntimeValue::String("complete_logical_bounded_physical".into()),
        )]),
    ) {
        Ok(span) => span,
        Err(error) => {
            return fail_query(
                store,
                Some(execution_span),
                root,
                "storage",
                "trace_start",
                TraceOutcome::Error,
                error,
            )
        }
    };
    let physical_before = store.physical_store_evidence();
    let execution_result = pipeline.execute(&plan, budget);
    let physical_after = store.physical_store_evidence();
    let execution = match execution_result {
        Ok(execution) => execution,
        Err(error) => {
            let outcome = if matches!(
                error,
                rrd_query::Error::Budget(_) | rrd_query::Error::StorageAdmission(_)
            ) {
                TraceOutcome::Denied
            } else {
                TraceOutcome::Error
            };
            let error_class = query_error_class(&error);
            let rendered = error.to_string();
            let mut storage_attributes =
                physical_storage_attributes(&physical_before, &physical_after, None);
            storage_attributes.insert(
                "error_class".into(),
                RuntimeValue::String(error_class.into()),
            );
            storage_attributes.insert(
                "error_digest".into(),
                RuntimeValue::Digest(digest::sha256_hex(rendered.as_bytes())),
            );
            let error: Box<dyn std::error::Error> = error.into();
            if let Err(trace_error) =
                storage_span.finish(store, outcome, Vec::new(), storage_attributes)
            {
                return fail_query(
                    store,
                    Some(execution_span),
                    root,
                    "storage",
                    "trace_finish",
                    TraceOutcome::Error,
                    format!("{rendered}; storage trace finish failed: {trace_error}").into(),
                );
            }
            return fail_query(
                store,
                Some(execution_span),
                root,
                "execution",
                error_class,
                outcome,
                error,
            );
        }
    };
    if let Err(error) = storage_span.finish(
        store,
        TraceOutcome::Ok,
        Vec::new(),
        physical_storage_attributes(&physical_before, &physical_after, Some(&execution)),
    ) {
        return fail_query(
            store,
            Some(execution_span),
            root,
            "storage",
            "trace_finish",
            TraceOutcome::Error,
            error,
        );
    }
    let execution_attributes = execution_attributes(&execution);
    if let Err(error) = execution_span.finish(
        store,
        TraceOutcome::Ok,
        Vec::new(),
        execution_attributes.clone(),
    ) {
        return fail_query(
            store,
            None,
            root,
            "execution",
            "trace_finish",
            TraceOutcome::Error,
            error,
        );
    }
    root.finish(
        store,
        TraceOutcome::Ok,
        vec![TraceLink::Plan {
            plan_digest: plan.digest.clone(),
        }],
        execution_attributes,
    )
    .map_err(|error| {
        format!("query completed but its authoritative root finish was not durable: {error}")
    })?;

    Ok(TracedQueryExecution {
        canonical,
        query,
        bound,
        plan,
        execution,
    })
}

pub(super) fn execution_links_for_storage(
    read: &rrd_core::ReadStamp,
    plan: &PhysicalPlan,
) -> Vec<TraceLink> {
    vec![
        TraceLink::Read {
            stamp: read.clone(),
        },
        TraceLink::Plan {
            plan_digest: plan.digest.clone(),
        },
    ]
}
