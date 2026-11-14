//! Vector and embedding trace attributes, error classes and trace-only rebase.

use super::*;

pub(super) fn vector_request_attributes(
    request: &SearchRequest,
    request_digest: &str,
    required_source_cursor: u64,
    ef_search: usize,
) -> RuntimeProperties {
    let mut attributes = RuntimeProperties::from([
        (
            "request_digest".into(),
            RuntimeValue::Digest(request_digest.into()),
        ),
        (
            "query_kind".into(),
            RuntimeValue::String(vector_query_kind(&request.query).into()),
        ),
        (
            "dimensions".into(),
            RuntimeValue::Unsigned(request.query.dimensions() as u64),
        ),
        (
            "metric".into(),
            RuntimeValue::String(metric_name(request.metric).into()),
        ),
        (
            "mode".into(),
            RuntimeValue::String(search_mode_name(request.mode).into()),
        ),
        ("top_k".into(), RuntimeValue::Unsigned(request.top_k as u64)),
        (
            "required_source_cursor".into(),
            RuntimeValue::Unsigned(required_source_cursor),
        ),
        ("ef_search".into(), RuntimeValue::Unsigned(ef_search as u64)),
        (
            "filter_present".into(),
            RuntimeValue::Bool(request.filter.is_some()),
        ),
        (
            "filter_property_count".into(),
            RuntimeValue::Unsigned(
                request
                    .filter
                    .as_ref()
                    .map_or(0, |filter| filter.referenced_properties().len())
                    as u64,
            ),
        ),
    ]);
    if let Some(model) = &request.embedding_model {
        attributes.insert(
            "model_space_digest".into(),
            RuntimeValue::Digest(model.digest.clone()),
        );
    }
    attributes
}

pub(super) fn required_projection_cursor<E: StorageEngine>(
    store: &E,
    read: &ReadStamp,
    family: ProjectionFamily,
) -> Result<u64, Box<dyn std::error::Error>> {
    let mut after = 0;
    let mut required = 0;
    loop {
        let page = store.runtime().outbox_since(after, 4_096)?;
        if page.is_empty() {
            return Ok(required);
        }
        let mut advanced = false;
        for work in page {
            if work.source_cursor > read.commit_cursor {
                return Ok(required);
            }
            after = after.max(work.source_cursor);
            advanced = true;
            if work.scope == read.scope && work.family == family {
                required = required.max(work.source_cursor);
            }
        }
        if !advanced {
            return Err("projection outbox pagination did not advance".into());
        }
    }
}

/// Rebases an exact write read stamp only across mutations proven to be
/// observability-only. Any record/relation/vector/claim/event other than the
/// canonical trace event makes the original data transaction stale.
pub(super) fn trace_only_rebase<E: StorageEngine>(
    store: &E,
    original: &ReadStamp,
) -> Result<ReadStamp, Box<dyn std::error::Error>> {
    let rebased = store.runtime().read_stamp(&original.scope)?;
    if rebased.commit_cursor < original.commit_cursor {
        return Err("embedding rebase moved behind its original read".into());
    }
    if rebased.commit_cursor == original.commit_cursor {
        return Ok(rebased);
    }
    let catalog = Catalog::capture_for_sources_at(
        store,
        original.clone(),
        &[],
        RuntimeReadBudget::authenticated_history_integrity(original.commit_cursor)?,
    )?;
    let prior_schema = catalog.schema_at(original.commit_cursor).cloned();
    let mut expected_schema = prior_schema
        .clone()
        .unwrap_or_else(|| RuntimeSchemaRegistry::empty(1, "install runtime trace contract"));
    let schema_install = RuntimeTraceEvent::register_schema(&mut expected_schema)?;
    if schema_install {
        if let Some(prior_schema) = &prior_schema {
            expected_schema.revision = prior_schema
                .revision
                .checked_add(1)
                .ok_or("runtime schema revision overflow during trace-only rebase")?;
            expected_schema.migration = "install canonical runtime trace contract".into();
        }
    }
    let gap = rebased
        .commit_cursor
        .checked_sub(original.commit_cursor)
        .ok_or("embedding rebase cursor underflow")?;
    let limit = usize::try_from(gap)
        .map_err(|_| "embedding trace-only rebase exceeds this platform's address space")?;
    let page = store
        .runtime()
        .read_changes(&rebased, original.commit_cursor, limit)?;
    if page.through_cursor != rebased.commit_cursor || page.changes.len() != limit {
        return Err("embedding trace-only rebase did not replay the complete cursor gap".into());
    }
    let mut schema_seen = false;
    for change in page.changes {
        match change.mutation {
            RuntimeMutation::Schema { registry }
                if schema_install && !schema_seen && registry == expected_schema =>
            {
                schema_seen = true;
            }
            RuntimeMutation::Event { event } if event.kind.as_str() == RUNTIME_TRACE_EVENT_TYPE => {
                let trace_id = event.properties.get("trace_id");
                let span_id = event.properties.get("span_id");
                if !matches!(trace_id, Some(RuntimeValue::String(value)) if value.len() == 32)
                    || !matches!(span_id, Some(RuntimeValue::String(value)) if value.len() == 16)
                {
                    return Err("embedding rebase encountered a malformed trace event".into());
                }
            }
            _ => {
                return Err(format!(
                    "embedding source state changed at runtime cursor {}",
                    change.cursor
                )
                .into())
            }
        }
    }
    if schema_install && !schema_seen {
        return Err("embedding rebase expected the canonical trace schema installation".into());
    }
    Ok(rebased)
}

pub(super) fn embedding_job_attributes(
    job: &EmbeddingJob,
    descriptor: &EmbeddingBackendDescriptor,
    job_digest: &str,
) -> RuntimeProperties {
    RuntimeProperties::from([
        ("job_digest".into(), RuntimeValue::Digest(job_digest.into())),
        (
            "source_digest".into(),
            RuntimeValue::Digest(job.expected_source_digest.clone()),
        ),
        (
            "model_space_digest".into(),
            RuntimeValue::Digest(descriptor.model.model_digest.clone()),
        ),
        (
            "requested_dimensions".into(),
            RuntimeValue::Unsigned(descriptor.model.dimensions as u64),
        ),
        (
            "backend_digest".into(),
            RuntimeValue::Digest(digest::sha256_hex(descriptor.id.as_bytes())),
        ),
        (
            "execution_target".into(),
            RuntimeValue::String(execution_target_name(&descriptor.execution)),
        ),
        (
            "network_required".into(),
            RuntimeValue::Bool(descriptor.network == rrd_inference::NetworkRequirement::Required),
        ),
        (
            "deterministic".into(),
            RuntimeValue::Bool(descriptor.deterministic),
        ),
        (
            "target_digest".into(),
            RuntimeValue::Digest(digest::sha256_hex(
                &serde_json::to_vec(&job.target).unwrap_or_default(),
            )),
        ),
    ])
}

pub(super) fn embedding_error_class(rendered: &str) -> (&'static str, TraceOutcome) {
    if rendered.contains("changed during inference")
        || rendered.contains("differs from the job")
        || rendered.contains("denies the network")
        || rendered.contains("requested model")
    {
        ("freshness_or_policy", TraceOutcome::Denied)
    } else if rendered.contains("backend") || rendered.contains("shape") {
        ("backend", TraceOutcome::Error)
    } else {
        ("contract", TraceOutcome::Error)
    }
}

pub(super) fn execution_target_name(target: &ExecutionTarget) -> String {
    match target {
        ExecutionTarget::Cpu => "cpu".into(),
        ExecutionTarget::Gpu { platform, device } => format!("gpu:{platform}:{device}"),
        ExecutionTarget::Remote { provider } => format!("remote:{provider}"),
    }
}

pub(super) fn vector_value_kind(value: &rrd_core::VectorValue) -> &'static str {
    match value {
        rrd_core::VectorValue::Dense { .. } => "dense",
        rrd_core::VectorValue::Sparse { .. } => "sparse",
        rrd_core::VectorValue::MultiDense { .. } => "multi_dense",
    }
}

pub(super) fn core_trace_error(error: Box<dyn std::error::Error>) -> rrd_core::Error {
    rrd_core::Error::InvalidRuntime {
        reason: format!("authoritative embedding trace failed: {error}"),
    }
}

pub(super) fn vector_plan_attributes(prepared: &PreparedVectorSearch) -> RuntimeProperties {
    let plan = prepared.plan();
    RuntimeProperties::from([
        (
            "plan_digest".into(),
            RuntimeValue::Digest(prepared.plan_digest().into()),
        ),
        (
            "catalog_revision".into(),
            RuntimeValue::Unsigned(prepared.catalog_revision()),
        ),
        (
            "selected_path".into(),
            RuntimeValue::String(access_path_name(plan.selected.kind).into()),
        ),
        (
            "selected_projection".into(),
            RuntimeValue::String(plan.selected.id.to_string()),
        ),
        (
            "source_cursor".into(),
            RuntimeValue::Unsigned(plan.selected.source_cursor),
        ),
        (
            "required_source_cursor".into(),
            RuntimeValue::Unsigned(plan.required_source_cursor),
        ),
        (
            "estimated_candidates".into(),
            RuntimeValue::Unsigned(plan.selected.estimated_candidates),
        ),
        (
            "estimated_cost".into(),
            RuntimeValue::Unsigned(plan.selected.estimated_cost),
        ),
        (
            "exact_rerank".into(),
            RuntimeValue::Unsigned(plan.selected.exact_rerank as u64),
        ),
        (
            "rejected_path_count".into(),
            RuntimeValue::Unsigned(plan.rejected.len() as u64),
        ),
        (
            "rejected_paths_digest".into(),
            RuntimeValue::Digest(digest::sha256_hex(
                &serde_json::to_vec(&plan.rejected).unwrap_or_default(),
            )),
        ),
        (
            "approximation_requested".into(),
            RuntimeValue::Bool(plan.approximation_requested),
        ),
    ])
}

pub(super) fn vector_execution_attributes(
    request: &SearchRequest,
    prepared: &PreparedVectorSearch,
    execution: &SearchExecution,
) -> RuntimeProperties {
    let selected = prepared.plan().selected.kind;
    RuntimeProperties::from([
        (
            "selected_path".into(),
            RuntimeValue::String(access_path_name(selected).into()),
        ),
        (
            "hit_count".into(),
            RuntimeValue::Unsigned(execution.hits.len() as u64),
        ),
        (
            "exact_rerank".into(),
            RuntimeValue::Unsigned(prepared.plan().selected.exact_rerank as u64),
        ),
        (
            "fallback_to_exact".into(),
            RuntimeValue::Bool(
                prepared.plan().approximation_requested && !selected.is_approximate(),
            ),
        ),
        (
            "filter_selectivity".into(),
            RuntimeValue::String(
                if request.filter.is_some() {
                    "not_measured"
                } else {
                    "not_applicable"
                }
                .into(),
            ),
        ),
    ])
}

pub(super) fn vector_error_class(rendered: &str) -> (&'static str, TraceOutcome) {
    if rendered.contains("stale")
        || rendered.contains("no vector access path")
        || rendered.contains("catalog conflict")
        || rendered.contains("quarantined")
    {
        ("freshness_or_policy", TraceOutcome::Denied)
    } else if rendered.contains("absent") || rendered.contains("differs") {
        ("integrity", TraceOutcome::Error)
    } else {
        ("contract", TraceOutcome::Error)
    }
}

pub(super) fn access_path_name(kind: AccessPathKind) -> &'static str {
    match kind {
        AccessPathKind::ExactScan => "exact_scan",
        AccessPathKind::ExactSegment => "exact_segment",
        AccessPathKind::Hnsw => "hnsw",
        AccessPathKind::ScalarQuantized => "scalar_quantized",
        AccessPathKind::ProductQuantized => "product_quantized",
        AccessPathKind::BinaryQuantized => "binary_quantized",
        AccessPathKind::TurboQuant => "turboquant",
    }
}

pub(super) fn metric_name(metric: ScoreMetric) -> &'static str {
    match metric {
        ScoreMetric::Cosine => "cosine",
        ScoreMetric::Dot => "dot",
        ScoreMetric::Euclidean => "euclidean",
        ScoreMetric::Manhattan => "manhattan",
    }
}

pub(super) fn search_mode_name(mode: SearchMode) -> &'static str {
    match mode {
        SearchMode::Exact => "exact",
        SearchMode::AllowApproximate { .. } => "allow_approximate",
        SearchMode::RequireApproximate { .. } => "require_approximate",
    }
}

pub(super) fn vector_query_kind(query: &VectorQuery) -> &'static str {
    match query {
        VectorQuery::Dense { .. } => "dense",
        VectorQuery::Sparse { .. } => "sparse",
        VectorQuery::MultiDense { .. } => "multi_dense",
    }
}

pub(super) fn error_attributes(stage: &str, class: &str, rendered: &str) -> RuntimeProperties {
    RuntimeProperties::from([
        ("failed_stage".into(), RuntimeValue::String(stage.into())),
        ("error_class".into(), RuntimeValue::String(class.into())),
        (
            "error_digest".into(),
            RuntimeValue::Digest(digest::sha256_hex(rendered.as_bytes())),
        ),
    ])
}

#[allow(clippy::too_many_arguments)]
pub(super) fn fail_data_plane<E: StorageEngine, T>(
    store: &E,
    child: Option<DurableTraceSpan>,
    root: DurableTraceSpan,
    stage: &str,
    class: &str,
    outcome: TraceOutcome,
    error: Box<dyn std::error::Error>,
) -> Result<T, Box<dyn std::error::Error>> {
    let rendered = error.to_string();
    let mut trace_errors = Vec::new();
    if let Some(child) = child {
        if let Err(trace_error) = child.finish(
            store,
            outcome,
            Vec::new(),
            error_attributes(stage, class, &rendered),
        ) {
            trace_errors.push(format!("child finish: {trace_error}"));
        }
    }
    if let Err(trace_error) = root.finish(
        store,
        outcome,
        Vec::new(),
        error_attributes(stage, class, &rendered),
    ) {
        trace_errors.push(format!("root finish: {trace_error}"));
    }
    if trace_errors.is_empty() {
        Err(error)
    } else {
        Err(format!(
            "{rendered}; authoritative data-plane trace also failed ({})",
            trace_errors.join("; ")
        )
        .into())
    }
}
