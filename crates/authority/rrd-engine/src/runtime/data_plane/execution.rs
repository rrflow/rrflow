//! Traced embedding execution, the durable embedding backend and traced vector search.

use super::*;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TracedVectorSearch {
    pub prepared: PreparedVectorSearch,
    pub execution: SearchExecution,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TracedEmbeddingExecution {
    pub prepared: PreparedEmbedding,
    pub commit: RuntimeCommitOutcome,
}

pub fn execute_traced_embedding<E, S, B>(
    store: &E,
    job: &EmbeddingJob,
    source_reader: &mut S,
    backend: &mut B,
    actor: &str,
    at: Millis,
) -> Result<TracedEmbeddingExecution, Box<dyn std::error::Error>>
where
    E: StorageEngine,
    S: EmbeddingSourceReader,
    B: EmbeddingBackend,
{
    job.validate()?;
    store.runtime().validate_read_stamp(&job.read)?;
    let descriptor = backend.descriptor().clone();
    descriptor.validate()?;
    let job_digest = job.digest()?;
    let cursor = job.read.commit_cursor.to_be_bytes();
    let at_bytes = at.to_be_bytes();
    let identity = TraceIdentity::derive(&[
        job.scope.as_str().as_bytes(),
        job_digest.as_bytes(),
        descriptor.model.model_digest.as_bytes(),
        &cursor,
        &at_bytes,
    ])?;
    let root_identity = identity.clone();
    let infer_identity = root_identity.child(&[b"embedding.infer"])?;
    let commit_identity = root_identity.child(&[b"embedding.commit"])?;
    let links = vec![TraceLink::Read {
        stamp: job.read.clone(),
    }];
    let root = DurableTraceSpan::start(
        store,
        job.scope.clone(),
        actor,
        identity,
        None,
        TraceBoundary::Inference,
        "embedding.run",
        at,
        TraceDataClass::Control,
        links.clone(),
        embedding_job_attributes(job, &descriptor, &job_digest),
    )?;
    let mut traced_backend = DurableEmbeddingBackend {
        store,
        backend,
        scope: job.scope.clone(),
        actor,
        at: root.observed_at(),
        parent: root_identity.clone(),
        identity: infer_identity.clone(),
        read: job.read.clone(),
    };
    let prepared = match EmbeddingCoordinator::prepare(job, source_reader, &mut traced_backend) {
        Ok(prepared) => prepared,
        Err(error) => {
            let (class, outcome) = embedding_error_class(&error.to_string());
            return fail_data_plane(
                store,
                None,
                root,
                "preparation",
                class,
                outcome,
                error.into(),
            );
        }
    };

    let commit_span = match DurableTraceSpan::start(
        store,
        job.scope.clone(),
        actor,
        commit_identity.clone(),
        Some(root_identity.span_id.clone()),
        TraceBoundary::Engine,
        "embedding.commit",
        root.observed_at(),
        TraceDataClass::Control,
        links,
        RuntimeProperties::from([
            (
                "job_digest".into(),
                RuntimeValue::Digest(job_digest.clone()),
            ),
            (
                "mutation_kind".into(),
                RuntimeValue::String("vector".into()),
            ),
            (
                "durability".into(),
                RuntimeValue::String("authoritative".into()),
            ),
        ]),
    ) {
        Ok(span) => span,
        Err(error) => {
            return fail_data_plane(
                store,
                None,
                root,
                "commit",
                "trace_start",
                TraceOutcome::Error,
                error,
            )
        }
    };
    let rebased = match trace_only_rebase(store, &job.read) {
        Ok(read) => read,
        Err(error) => {
            return fail_data_plane(
                store,
                Some(commit_span),
                root,
                "commit_rebase",
                "concurrent_source_state",
                TraceOutcome::Denied,
                error,
            )
        }
    };
    let transaction = DataTransaction::new(
        rebased.clone(),
        RuntimeCommit {
            scope: job.scope.clone(),
            at,
            actor: actor.into(),
            expected_cursor: rebased.commit_cursor,
            mutations: vec![RuntimeMutation::Vector {
                vector: prepared.vector.clone(),
            }],
        },
    )?;
    let commit = match store.runtime().commit_data_transaction(&transaction) {
        Ok(commit) => commit,
        Err(error) => {
            let outcome = if matches!(error, rrd_store::Error::RuntimeConflict { .. }) {
                TraceOutcome::Denied
            } else {
                TraceOutcome::Error
            };
            return fail_data_plane(
                store,
                Some(commit_span),
                root,
                "commit",
                if outcome == TraceOutcome::Denied {
                    "cursor_conflict"
                } else {
                    "storage"
                },
                outcome,
                error.into(),
            );
        }
    };
    if let Err(error) = commit_span.finish(
        store,
        TraceOutcome::Ok,
        Vec::new(),
        RuntimeProperties::from([
            (
                "commit_id".into(),
                RuntimeValue::Digest(commit.commit_id.clone()),
            ),
            (
                "first_cursor".into(),
                RuntimeValue::Unsigned(commit.first_cursor),
            ),
            (
                "last_cursor".into(),
                RuntimeValue::Unsigned(commit.last_cursor),
            ),
            (
                "rebased_over_trace_events".into(),
                RuntimeValue::Unsigned(
                    rebased.commit_cursor.saturating_sub(job.read.commit_cursor),
                ),
            ),
        ]),
    ) {
        return fail_data_plane(
            store,
            None,
            root,
            "commit",
            "trace_finish",
            TraceOutcome::Error,
            error,
        );
    }
    let root_attributes = RuntimeProperties::from([
        (
            "commit_id".into(),
            RuntimeValue::Digest(commit.commit_id.clone()),
        ),
        (
            "outcome_cursor".into(),
            RuntimeValue::Unsigned(commit.last_cursor),
        ),
        (
            "output_dimensions".into(),
            RuntimeValue::Unsigned(prepared.vector.value.dimensions() as u64),
        ),
        (
            "output_kind".into(),
            RuntimeValue::String(vector_value_kind(&prepared.vector.value).into()),
        ),
    ]);
    root.finish(store, TraceOutcome::Ok, Vec::new(), root_attributes)
        .map_err(|error| {
            format!("embedding committed but its authoritative root finish failed: {error}")
        })?;
    Ok(TracedEmbeddingExecution { prepared, commit })
}

pub(super) struct DurableEmbeddingBackend<'a, E, B> {
    store: &'a E,
    backend: &'a mut B,
    scope: rrd_core::ScopeId,
    actor: &'a str,
    at: Millis,
    parent: TraceIdentity,
    identity: TraceIdentity,
    read: ReadStamp,
}

impl<E: StorageEngine, B: EmbeddingBackend> EmbeddingBackend for DurableEmbeddingBackend<'_, E, B> {
    fn descriptor(&self) -> &EmbeddingBackendDescriptor {
        self.backend.descriptor()
    }

    fn embed(&mut self, request: &EmbeddingRequest) -> rrd_core::Result<rrd_core::VectorValue> {
        let descriptor = self.backend.descriptor().clone();
        let span = DurableTraceSpan::start(
            self.store,
            self.scope.clone(),
            self.actor,
            self.identity.clone(),
            Some(self.parent.span_id.clone()),
            TraceBoundary::Inference,
            "embedding.infer",
            self.at,
            TraceDataClass::Control,
            vec![TraceLink::Read {
                stamp: self.read.clone(),
            }],
            RuntimeProperties::from([
                (
                    "job_digest".into(),
                    RuntimeValue::Digest(request.job_digest.clone()),
                ),
                (
                    "source_digest".into(),
                    RuntimeValue::Digest(request.source_digest.clone()),
                ),
                (
                    "input_bytes".into(),
                    RuntimeValue::Unsigned(request.bytes.len() as u64),
                ),
                (
                    "media_type_digest".into(),
                    RuntimeValue::Digest(digest::sha256_hex(request.media_type.as_bytes())),
                ),
                (
                    "model_space_digest".into(),
                    RuntimeValue::Digest(descriptor.model.model_digest.clone()),
                ),
                (
                    "execution_target".into(),
                    RuntimeValue::String(execution_target_name(&descriptor.execution)),
                ),
            ]),
        )
        .map_err(core_trace_error)?;
        match self.backend.embed(request) {
            Ok(value) => {
                span.finish(
                    self.store,
                    TraceOutcome::Ok,
                    Vec::new(),
                    RuntimeProperties::from([
                        (
                            "output_dimensions".into(),
                            RuntimeValue::Unsigned(value.dimensions() as u64),
                        ),
                        (
                            "output_kind".into(),
                            RuntimeValue::String(vector_value_kind(&value).into()),
                        ),
                    ]),
                )
                .map_err(core_trace_error)?;
                Ok(value)
            }
            Err(error) => {
                let rendered = error.to_string();
                span.finish(
                    self.store,
                    TraceOutcome::Error,
                    Vec::new(),
                    error_attributes("inference", "backend", &rendered),
                )
                .map_err(core_trace_error)?;
                Err(error)
            }
        }
    }
}

pub fn execute_traced_vector_search<E: StorageEngine>(
    store: &E,
    runtime: &VectorRuntime,
    request: &SearchRequest,
    ef_search: usize,
    actor: &str,
    at: Millis,
) -> Result<TracedVectorSearch, Box<dyn std::error::Error>> {
    request.validate()?;
    store.runtime().validate_read_stamp(&request.read)?;
    let required_source_cursor =
        required_projection_cursor(store, &request.read, ProjectionFamily::Vector)?;
    let request_digest = request.digest()?;
    let cursor = request.read.commit_cursor.to_be_bytes();
    let at_bytes = at.to_be_bytes();
    let identity = TraceIdentity::derive(&[
        request.scope.as_str().as_bytes(),
        request_digest.as_bytes(),
        &cursor,
        &at_bytes,
    ])?;
    let root_identity = identity.clone();
    let links = vec![TraceLink::Read {
        stamp: request.read.clone(),
    }];
    let root = DurableTraceSpan::start(
        store,
        request.scope.clone(),
        actor,
        identity,
        None,
        TraceBoundary::Vector,
        "vector.search",
        at,
        TraceDataClass::Control,
        links.clone(),
        vector_request_attributes(request, &request_digest, required_source_cursor, ef_search),
    )?;

    let planning_identity = root_identity.child(&[b"vector.plan"])?;
    let planning = match DurableTraceSpan::start(
        store,
        request.scope.clone(),
        actor,
        planning_identity,
        Some(root_identity.span_id.clone()),
        TraceBoundary::Vector,
        "vector.plan",
        root.observed_at(),
        TraceDataClass::Control,
        links.clone(),
        RuntimeProperties::new(),
    ) {
        Ok(span) => span,
        Err(error) => {
            return fail_data_plane(
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
    let prepared = match runtime.prepare_search_at(request, required_source_cursor, ef_search) {
        Ok(prepared) => prepared,
        Err(error) => {
            let (class, outcome) = vector_error_class(&error.to_string());
            return fail_data_plane(
                store,
                Some(planning),
                root,
                "planning",
                class,
                outcome,
                error.into(),
            );
        }
    };
    let plan_links = vec![
        TraceLink::Plan {
            plan_digest: prepared.plan_digest().into(),
        },
        TraceLink::Projection {
            stamp: prepared.selected_stamp().clone(),
        },
    ];
    if let Err(error) = planning.finish(
        store,
        TraceOutcome::Ok,
        plan_links.clone(),
        vector_plan_attributes(&prepared),
    ) {
        return fail_data_plane(
            store,
            None,
            root,
            "planning",
            "trace_finish",
            TraceOutcome::Error,
            error,
        );
    }

    let execution_identity = root_identity.child(&[b"vector.execute"])?;
    let mut execution_links = links;
    execution_links.extend(plan_links.clone());
    let execution_span = match DurableTraceSpan::start(
        store,
        request.scope.clone(),
        actor,
        execution_identity,
        Some(root_identity.span_id),
        TraceBoundary::Vector,
        "vector.execute",
        root.observed_at(),
        TraceDataClass::Control,
        execution_links,
        RuntimeProperties::new(),
    ) {
        Ok(span) => span,
        Err(error) => {
            return fail_data_plane(
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
    let execution = match runtime.execute_search(request, &prepared) {
        Ok(execution) => execution,
        Err(error) => {
            let (class, outcome) = vector_error_class(&error.to_string());
            return fail_data_plane(
                store,
                Some(execution_span),
                root,
                "execution",
                class,
                outcome,
                error.into(),
            );
        }
    };
    let execution_attributes = vector_execution_attributes(request, &prepared, &execution);
    if let Err(error) = execution_span.finish(
        store,
        TraceOutcome::Ok,
        Vec::new(),
        execution_attributes.clone(),
    ) {
        return fail_data_plane(
            store,
            None,
            root,
            "execution",
            "trace_finish",
            TraceOutcome::Error,
            error,
        );
    }
    root.finish(store, TraceOutcome::Ok, plan_links, execution_attributes)
        .map_err(|error| {
            format!("vector search completed but its authoritative root finish failed: {error}")
        })?;
    Ok(TracedVectorSearch {
        prepared,
        execution,
    })
}
