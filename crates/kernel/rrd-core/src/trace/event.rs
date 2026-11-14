//! Causal links and the persisted runtime trace event.

use super::*;

/// Relationship used when work is causally connected but is not a
/// synchronous child (for example a committed-event activation or retry).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraceCausalRelation {
    FollowsFrom,
    RetryOf,
    Resumes,
}

/// A typed causal coordinate. These links make a trace useful for correctness
/// and optimization rather than merely a wall-clock visualization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum TraceLink {
    Request {
        request_id: String,
        operation_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        parent_request_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        idempotency_digest: Option<String>,
    },
    ActorScope {
        actor: String,
        scope: ScopeId,
    },
    Authorization {
        decision: AuditDecision,
        policy_revision: u64,
        authorization_digest: String,
    },
    /// Correlates work with the exact generic reasoning cursor that selected
    /// it. The cursor owns no implicit lifecycle; its tree declares the route.
    ReasoningCursor {
        cursor: ReasoningActiveCursor,
    },
    RuntimeCursor {
        cursor: u64,
    },
    Read {
        stamp: ReadStamp,
    },
    Snapshot {
        snapshot_id: SnapshotId,
        cursor: u64,
    },
    Plan {
        plan_digest: String,
    },
    Projection {
        stamp: ProjectionStamp,
    },
    Source {
        source_kind: String,
        source_id: String,
        source_revision: String,
    },
    Commit {
        commit_id: String,
        first_cursor: u64,
        last_cursor: u64,
    },
    Resource {
        resource_kind: String,
        resource_id: String,
    },
    CausalSpan {
        trace_id: TraceId,
        span_id: SpanId,
        relation: TraceCausalRelation,
    },
    Output {
        output_digest: String,
        item_count: u64,
        byte_count: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeTraceEvent {
    pub contract_version: u16,
    pub trace_id: TraceId,
    pub span_id: SpanId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_span_id: Option<SpanId>,
    pub phase: TracePhase,
    pub boundary: TraceBoundary,
    pub name: String,
    pub at: Millis,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_micros: Option<u64>,
    pub outcome: TraceOutcome,
    pub data_class: TraceDataClass,
    #[serde(default)]
    pub links: Vec<TraceLink>,
    #[serde(default)]
    pub attributes: RuntimeProperties,
}

impl RuntimeTraceEvent {
    #[allow(clippy::too_many_arguments)]
    pub fn start(
        trace_id: TraceId,
        span_id: SpanId,
        parent_span_id: Option<SpanId>,
        boundary: TraceBoundary,
        name: impl Into<String>,
        at: Millis,
        data_class: TraceDataClass,
        links: Vec<TraceLink>,
        attributes: RuntimeProperties,
    ) -> Result<Self> {
        Self::new(
            trace_id,
            span_id,
            parent_span_id,
            TracePhase::Start,
            boundary,
            name,
            at,
            None,
            TraceOutcome::Running,
            data_class,
            links,
            attributes,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn annotation(
        trace_id: TraceId,
        span_id: SpanId,
        parent_span_id: Option<SpanId>,
        boundary: TraceBoundary,
        name: impl Into<String>,
        at: Millis,
        outcome: TraceOutcome,
        data_class: TraceDataClass,
        links: Vec<TraceLink>,
        attributes: RuntimeProperties,
    ) -> Result<Self> {
        Self::new(
            trace_id,
            span_id,
            parent_span_id,
            TracePhase::Annotation,
            boundary,
            name,
            at,
            None,
            outcome,
            data_class,
            links,
            attributes,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn finish(
        trace_id: TraceId,
        span_id: SpanId,
        parent_span_id: Option<SpanId>,
        boundary: TraceBoundary,
        name: impl Into<String>,
        at: Millis,
        duration_micros: u64,
        outcome: TraceOutcome,
        data_class: TraceDataClass,
        links: Vec<TraceLink>,
        attributes: RuntimeProperties,
    ) -> Result<Self> {
        Self::new(
            trace_id,
            span_id,
            parent_span_id,
            TracePhase::Finish,
            boundary,
            name,
            at,
            Some(duration_micros),
            outcome,
            data_class,
            links,
            attributes,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn new(
        trace_id: TraceId,
        span_id: SpanId,
        parent_span_id: Option<SpanId>,
        phase: TracePhase,
        boundary: TraceBoundary,
        name: impl Into<String>,
        at: Millis,
        duration_micros: Option<u64>,
        outcome: TraceOutcome,
        data_class: TraceDataClass,
        links: Vec<TraceLink>,
        attributes: RuntimeProperties,
    ) -> Result<Self> {
        let event = Self {
            contract_version: RUNTIME_TRACE_CONTRACT_VERSION,
            trace_id,
            span_id,
            parent_span_id,
            phase,
            boundary,
            name: name.into(),
            at,
            duration_micros,
            outcome,
            data_class,
            links,
            attributes,
        };
        event.validate()?;
        Ok(event)
    }

    pub fn validate(&self) -> Result<()> {
        if self.contract_version != RUNTIME_TRACE_CONTRACT_VERSION {
            return invalid(format!(
                "unsupported runtime trace contract version {}",
                self.contract_version
            ));
        }
        validate_operation_name(&self.name)?;
        let expected_boundary = TraceOperation::from_name(&self.name)
            .map(TraceOperation::boundary)
            .or_else(|| direct_convergence_operation_boundary(&self.name))
            .expect("validated trace operation must have one frozen boundary");
        if expected_boundary != self.boundary {
            return invalid("trace operation and boundary differ");
        }
        if self.parent_span_id.as_ref() == Some(&self.span_id) {
            return invalid("a trace span cannot parent itself");
        }
        match (self.phase, self.duration_micros, self.outcome) {
            (TracePhase::Start, None, TraceOutcome::Running) => {}
            (TracePhase::Annotation, None, _) => {}
            (TracePhase::Finish, Some(_), outcome) if outcome != TraceOutcome::Running => {}
            _ => {
                return invalid(
                    "trace phase, duration, and outcome do not form a valid lifecycle transition",
                );
            }
        }
        if self.links.len() > MAX_TRACE_LINKS {
            return invalid(format!(
                "runtime trace exceeds {MAX_TRACE_LINKS} causal links"
            ));
        }
        for link in &self.links {
            validate_link(link)?;
            if matches!(
                link,
                TraceLink::CausalSpan { trace_id, span_id, .. }
                    if trace_id == &self.trace_id && span_id == &self.span_id
            ) {
                return invalid("a trace span cannot causally link to itself");
            }
        }
        if self.attributes.len() > MAX_TRACE_ATTRIBUTES {
            return invalid(format!(
                "runtime trace exceeds {MAX_TRACE_ATTRIBUTES} attributes"
            ));
        }
        let mut nodes = 0;
        for (name, value) in &self.attributes {
            validate_attribute_name(name)?;
            if let Some(attribute) = TraceAttribute::from_name(name) {
                validate_canonical_attribute_value(attribute, value)?;
            }
            validate_value(value, 0, &mut nodes)?;
        }
        Ok(())
    }

    /// Converts the trace into the existing append-only runtime event model.
    /// Persistence therefore inherits the runtime cursor and hash-chain rather
    /// than creating an unrelated telemetry store.
    pub fn into_runtime_event(self) -> Result<RuntimeEvent> {
        self.validate()?;
        let mut properties = RuntimeProperties::from([
            (
                "contract_version".into(),
                RuntimeValue::Unsigned(u64::from(self.contract_version)),
            ),
            (
                "trace_id".into(),
                RuntimeValue::String(self.trace_id.to_string()),
            ),
            (
                "span_id".into(),
                RuntimeValue::String(self.span_id.to_string()),
            ),
            (
                "phase".into(),
                RuntimeValue::String(enum_name(self.phase).into()),
            ),
            (
                "boundary".into(),
                RuntimeValue::String(self.boundary.as_str().into()),
            ),
            ("name".into(), RuntimeValue::String(self.name)),
            ("at".into(), RuntimeValue::Unsigned(self.at)),
            (
                "outcome".into(),
                RuntimeValue::String(outcome_name(self.outcome).into()),
            ),
            (
                "data_class".into(),
                RuntimeValue::String(data_class_name(self.data_class).into()),
            ),
            (
                "links".into(),
                RuntimeValue::List(self.links.iter().map(link_value).collect()),
            ),
            ("attributes".into(), RuntimeValue::Map(self.attributes)),
        ]);
        if let Some(parent) = self.parent_span_id {
            properties.insert(
                "parent_span_id".into(),
                RuntimeValue::String(parent.to_string()),
            );
        }
        if let Some(duration) = self.duration_micros {
            properties.insert("duration_micros".into(), RuntimeValue::Unsigned(duration));
        }
        Ok(RuntimeEvent {
            kind: Self::event_type()?,
            subject: None,
            properties,
        })
    }

    pub fn event_type() -> Result<RuntimeType> {
        RuntimeType::new(RUNTIME_TRACE_EVENT_TYPE)
    }

    /// Strict schema entry an instance adds during bootstrap or migration.
    pub fn event_schema() -> RuntimeEventSchema {
        let mut properties = BTreeMap::new();
        for name in ["contract_version", "at"] {
            properties.insert(
                name.into(),
                RuntimePropertySchema::required(RuntimeValueType::Unsigned),
            );
        }
        for name in [
            "trace_id",
            "span_id",
            "phase",
            "boundary",
            "name",
            "outcome",
            "data_class",
        ] {
            properties.insert(
                name.into(),
                RuntimePropertySchema::required(RuntimeValueType::String),
            );
        }
        properties.insert(
            "parent_span_id".into(),
            RuntimePropertySchema::optional(RuntimeValueType::String),
        );
        properties.insert(
            "duration_micros".into(),
            RuntimePropertySchema::optional(RuntimeValueType::Unsigned),
        );
        properties.insert(
            "links".into(),
            RuntimePropertySchema::required(RuntimeValueType::List),
        );
        properties.insert(
            "attributes".into(),
            RuntimePropertySchema::required(RuntimeValueType::Map),
        );
        RuntimeEventSchema {
            properties,
            ..RuntimeEventSchema::default()
        }
    }

    /// Installs or repairs the canonical trace event declaration in a schema
    /// under construction. The caller owns schema revision allocation because
    /// that compare-and-swap must occur against its storage engine's exact
    /// read stamp.
    pub fn register_schema(registry: &mut RuntimeSchemaRegistry) -> Result<bool> {
        let kind = Self::event_type()?;
        let schema = Self::event_schema();
        registry.define_event_table(kind, RuntimeLogicalModel::Event, schema)
    }

    /// Prepares the one canonical runtime commit used by both local engines
    /// and consensus-backed trace writers. Schema installation/repair and the
    /// event share the caller's exact read cursor CAS.
    pub fn prepare_commit(
        &self,
        read: &ReadStamp,
        current_schema: Option<&RuntimeSchemaRegistry>,
        actor: &str,
    ) -> Result<RuntimeCommit> {
        self.validate()?;
        read.validate()?;
        if actor.trim().is_empty() {
            return invalid("runtime trace actor must not be empty");
        }
        if current_schema.map(|schema| schema.revision) != read.schema_revision {
            return invalid("runtime trace schema does not match its read stamp");
        }

        let mut mutations = Vec::new();
        let mut registry = current_schema
            .cloned()
            .unwrap_or_else(|| RuntimeSchemaRegistry::empty(1, "install runtime trace contract"));
        if Self::register_schema(&mut registry)? {
            if let Some(current) = current_schema {
                registry.revision =
                    current
                        .revision
                        .checked_add(1)
                        .ok_or_else(|| Error::InvalidRuntime {
                            reason:
                                "runtime schema revision overflow while installing trace contract"
                                    .into(),
                        })?;
                registry.migration = "install canonical runtime trace contract".into();
            }
            mutations.push(RuntimeMutation::Schema { registry });
        }
        mutations.push(RuntimeMutation::Event {
            event: self.clone().into_runtime_event()?,
        });
        let commit = RuntimeCommit {
            scope: read.scope.clone(),
            at: self.at,
            actor: actor.to_owned(),
            expected_cursor: read.commit_cursor,
            mutations,
        };
        commit.validate()?;
        Ok(commit)
    }
}
