use super::*;

fn trace_id() -> TraceId {
    TraceId::new("0123456789abcdef0123456789abcdef").unwrap()
}

fn span_id() -> SpanId {
    SpanId::new("0123456789abcdef").unwrap()
}

fn valid_canonical_attribute_value(attribute: TraceAttribute) -> RuntimeValue {
    match attribute {
        TraceAttribute::Stage
        | TraceAttribute::Phase
        | TraceAttribute::Action
        | TraceAttribute::SkipReason
        | TraceAttribute::CacheStatus
        | TraceAttribute::FailedStage
        | TraceAttribute::ErrorClass
        | TraceAttribute::ReadStampValidation
        | TraceAttribute::VerificationStatus
        | TraceAttribute::PropagationStatus => RuntimeValue::String("verified_value".into()),
        TraceAttribute::Selected
        | TraceAttribute::Exact
        | TraceAttribute::Fallback
        | TraceAttribute::Truncated
        | TraceAttribute::Retryable => RuntimeValue::Bool(true),
        TraceAttribute::Attempt
        | TraceAttribute::InputItems
        | TraceAttribute::InputBytes
        | TraceAttribute::MaxStorageKeys
        | TraceAttribute::OutputItems
        | TraceAttribute::OutputBytes
        | TraceAttribute::SelectedVersions
        | TraceAttribute::KeysExamined
        | TraceAttribute::PagesExamined
        | TraceAttribute::RowsExamined
        | TraceAttribute::ReadPointReads
        | TraceAttribute::ReadRangeScans
        | TraceAttribute::ReadValuesDecoded
        | TraceAttribute::ReadStampValidationChangeReads
        | TraceAttribute::ReadStampValidationProofNodes
        | TraceAttribute::GraphSteps
        | TraceAttribute::CandidatesExamined
        | TraceAttribute::MappedBytes
        | TraceAttribute::ReadBytes
        | TraceAttribute::DecodedBytes
        | TraceAttribute::DecompressedBytes
        | TraceAttribute::BorrowedBytes
        | TraceAttribute::CopiedBytes
        | TraceAttribute::AllocatedBytes
        | TraceAttribute::ElapsedMicros
        | TraceAttribute::MemoryPeakBytes
        | TraceAttribute::SpillBytes
        | TraceAttribute::InputTokens
        | TraceAttribute::OutputTokens
        | TraceAttribute::Rank
        | TraceAttribute::SelectedSourceCount
        | TraceAttribute::SkippedSourceCount
        | TraceAttribute::CompactedBytes
        | TraceAttribute::CompactedTokens
        | TraceAttribute::StartCursor => RuntimeValue::Unsigned(1),
        TraceAttribute::Score | TraceAttribute::RrfContribution => {
            RuntimeValue::Decimal("0.5".into())
        }
        TraceAttribute::ErrorDigest | TraceAttribute::PropagationDigest => {
            RuntimeValue::Digest("a".repeat(64))
        }
    }
}

#[test]
fn w3c_id_widths_and_lifecycle_transitions_fail_closed() {
    assert!(TraceId::new("0".repeat(32)).is_err());
    assert!(TraceId::new("ABCDEF0123456789abcdef0123456789").is_err());
    assert!(SpanId::new("0123").is_err());

    let mut event = RuntimeTraceEvent::start(
        trace_id(),
        span_id(),
        None,
        TraceBoundary::Ql,
        TraceOperation::QlExecute,
        10,
        TraceDataClass::Control,
        Vec::new(),
        RuntimeProperties::new(),
    )
    .unwrap();
    event.duration_micros = Some(1);
    assert!(event.validate().is_err());
}

#[test]
fn external_source_is_linked_without_claiming_shared_transactions() {
    let event = RuntimeTraceEvent::finish(
        trace_id(),
        span_id(),
        None,
        TraceBoundary::Adapter,
        TraceOperation::AdapterInvoke,
        20,
        800,
        TraceOutcome::Ok,
        TraceDataClass::Operator,
        vec![TraceLink::Source {
            source_kind: "operator_knowledge".into(),
            source_id: "pgvector:rrflow".into(),
            source_revision: "postgres-lsn:0/16B6C50".into(),
        }],
        RuntimeProperties::from([(
            TraceAttribute::OutputItems.into(),
            RuntimeValue::Unsigned(7),
        )]),
    )
    .unwrap();

    let runtime = event.into_runtime_event().unwrap();
    assert_eq!(runtime.kind.as_str(), RUNTIME_TRACE_EVENT_TYPE);
    assert_eq!(
        runtime.properties["outcome"],
        RuntimeValue::String("ok".into())
    );
    assert_eq!(
        RuntimeTraceEvent::event_schema().properties["links"].value_type,
        RuntimeValueType::List
    );
}

#[test]
fn schema_registration_is_strict_and_idempotent() {
    let mut registry = RuntimeSchemaRegistry::empty(1, "trace bootstrap");
    assert!(RuntimeTraceEvent::register_schema(&mut registry).unwrap());
    assert!(!RuntimeTraceEvent::register_schema(&mut registry).unwrap());
    assert_eq!(
        registry.events[&RuntimeTraceEvent::event_type().unwrap()],
        RuntimeTraceEvent::event_schema()
    );
}

#[test]
fn trace_commit_preparation_atomically_installs_schema_and_binds_the_read() {
    let scope = crate::ScopeId::new("instance:trace-commit").unwrap();
    let read = ReadStamp::new(scope.clone(), None, 0, 0, None).unwrap();
    let event = RuntimeTraceEvent::start(
        trace_id(),
        span_id(),
        None,
        TraceBoundary::Adapter,
        TraceOperation::AdapterTransfer,
        10,
        TraceDataClass::Control,
        Vec::new(),
        RuntimeProperties::new(),
    )
    .unwrap();
    let bootstrap = event
        .prepare_commit(&read, None, "cluster:transport")
        .unwrap();
    assert_eq!(bootstrap.scope, scope);
    assert_eq!(bootstrap.expected_cursor, 0);
    assert!(matches!(
        bootstrap.mutations[0],
        RuntimeMutation::Schema { .. }
    ));
    assert!(matches!(
        bootstrap.mutations[1],
        RuntimeMutation::Event { .. }
    ));

    let RuntimeMutation::Schema { registry } = &bootstrap.mutations[0] else {
        unreachable!()
    };
    let installed_read = ReadStamp::new(scope, Some(1), 0, 2, Some(bootstrap.digest())).unwrap();
    let event_only = event
        .prepare_commit(&installed_read, Some(registry), "cluster:transport")
        .unwrap();
    assert_eq!(event_only.mutations.len(), 1);
    assert!(matches!(
        event_only.mutations[0],
        RuntimeMutation::Event { .. }
    ));

    let stale = ReadStamp::new(
        installed_read.scope.clone(),
        Some(2),
        0,
        installed_read.commit_cursor,
        installed_read.head_digest.clone(),
    )
    .unwrap();
    assert!(event
        .prepare_commit(&stale, Some(registry), "cluster:transport")
        .is_err());
}

#[test]
fn attributes_are_bounded_before_they_reach_the_runtime_log() {
    let event = RuntimeTraceEvent::annotation(
        trace_id(),
        span_id(),
        None,
        TraceBoundary::Inference,
        TraceOperation::InferenceRoute,
        10,
        TraceOutcome::Running,
        TraceDataClass::Content,
        Vec::new(),
        RuntimeProperties::from([(
            TraceAttribute::Stage.into(),
            RuntimeValue::String("x".repeat(MAX_TRACE_STRING_BYTES + 1)),
        )]),
    );
    assert!(event.is_err());
}

#[test]
fn persisted_read_and_projection_links_retain_complete_reconstructable_stamps() {
    let read = ReadStamp::new(
        crate::ScopeId::new("instance:trace").unwrap(),
        Some(7),
        11,
        29,
        Some("a".repeat(64)),
    )
    .unwrap();
    let projection = ProjectionStamp {
        contract_version: crate::DATA_RUNTIME_CONTRACT_VERSION,
        id: crate::ProjectionId::new("vector-operator").unwrap(),
        generation: 3,
        source_cursor: 29,
        config_digest: "b".repeat(64),
        artifact_digest: "c".repeat(64),
        state: crate::ProjectionState::Ready,
    };
    projection.validate().unwrap();
    let runtime = RuntimeTraceEvent::annotation(
        trace_id(),
        span_id(),
        None,
        TraceBoundary::Vector,
        TraceOperation::VectorProjection,
        10,
        TraceOutcome::Ok,
        TraceDataClass::Control,
        vec![
            TraceLink::Read {
                stamp: read.clone(),
            },
            TraceLink::Projection {
                stamp: projection.clone(),
            },
        ],
        RuntimeProperties::new(),
    )
    .unwrap()
    .into_runtime_event()
    .unwrap();
    let RuntimeValue::List(links) = &runtime.properties["links"] else {
        panic!("links must remain a typed list")
    };
    let RuntimeValue::Map(read_link) = &links[0] else {
        panic!("read link must remain a typed map")
    };
    assert_eq!(
        read_link["contract_version"],
        RuntimeValue::Unsigned(u64::from(read.contract_version))
    );
    assert_eq!(
        read_link["schema_revision"],
        RuntimeValue::Unsigned(read.schema_revision.unwrap())
    );
    assert_eq!(
        read_link["catalog_revision"],
        RuntimeValue::Unsigned(read.catalog_revision)
    );
    assert_eq!(
        read_link["commit_cursor"],
        RuntimeValue::Unsigned(read.commit_cursor)
    );
    assert!(
        !read_link.contains_key("cursor"),
        "read links must not retain a compatibility alias for commit_cursor"
    );
    assert_eq!(
        read_link["head_digest"],
        RuntimeValue::Digest(read.head_digest.unwrap())
    );
    let RuntimeValue::Map(projection_link) = &links[1] else {
        panic!("projection link must remain a typed map")
    };
    assert_eq!(
        projection_link["config_digest"],
        RuntimeValue::Digest(projection.config_digest)
    );
    assert_eq!(
        projection_link["state"],
        RuntimeValue::String("ready".into())
    );
}

#[test]
fn canonical_operations_are_closed_unique_and_cover_every_boundary() {
    let names = TraceOperation::ALL
        .iter()
        .map(|operation| operation.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(names.len(), TraceOperation::ALL.len());
    for operation in TraceOperation::ALL {
        let expected_prefix = format!("rrflow.{}.", operation.boundary().as_str());
        assert!(operation.as_str().starts_with(&expected_prefix));
        assert_eq!(
            TraceOperation::from_name(operation.as_str()),
            Some(operation)
        );
        assert!(operation.as_str().bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_')
        }));
    }
    for boundary in TraceBoundary::ALL {
        assert!(TraceOperation::ALL
            .iter()
            .any(|operation| operation.boundary() == boundary));
    }

    let unknown = RuntimeTraceEvent::start(
        trace_id(),
        span_id(),
        None,
        TraceBoundary::Vector,
        "rrflow.vector.project_42",
        10,
        TraceDataClass::Control,
        Vec::new(),
        RuntimeProperties::new(),
    );
    assert!(unknown.is_err());

    let mismatched = RuntimeTraceEvent::start(
        trace_id(),
        span_id(),
        None,
        TraceBoundary::Kv,
        TraceOperation::VectorSearch,
        10,
        TraceDataClass::Control,
        Vec::new(),
        RuntimeProperties::new(),
    );
    assert!(mismatched.is_err());
}

#[test]
fn canonical_attributes_and_direct_convergence_inventory_are_closed() {
    let names = TraceAttribute::ALL
        .iter()
        .map(|attribute| attribute.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(names.len(), TraceAttribute::ALL.len());
    for attribute in TraceAttribute::ALL {
        assert_eq!(
            TraceAttribute::from_name(attribute.as_str()),
            Some(*attribute)
        );
        assert!(attribute
            .as_str()
            .bytes()
            .all(|byte| { byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_' }));
        RuntimeTraceEvent::annotation(
            trace_id(),
            span_id(),
            None,
            TraceBoundary::Engine,
            TraceOperation::EngineOperation,
            10,
            TraceOutcome::Running,
            TraceDataClass::Control,
            Vec::new(),
            RuntimeProperties::from([(
                attribute.to_string(),
                valid_canonical_attribute_value(*attribute),
            )]),
        )
        .unwrap_or_else(|error| {
            panic!("canonical attribute {} failed: {error}", attribute.as_str())
        });
    }

    assert!(DIRECT_CONVERGENCE_OPERATION_NAMES
        .windows(2)
        .all(|pair| pair[0] < pair[1]));
    assert!(DIRECT_CONVERGENCE_OPERATION_NAMES
        .iter()
        .all(|name| TraceOperation::from_name(name).is_none()
            && direct_convergence_operation_boundary(name).is_some()));
    assert!(DIRECT_CONVERGENCE_ATTRIBUTE_NAMES
        .windows(2)
        .all(|pair| pair[0] < pair[1]));
    assert!(DIRECT_CONVERGENCE_ATTRIBUTE_NAMES
        .iter()
        .all(|name| TraceAttribute::from_name(name).is_none()));

    let unknown = RuntimeTraceEvent::annotation(
        trace_id(),
        span_id(),
        None,
        TraceBoundary::Engine,
        TraceOperation::EngineOperation,
        10,
        TraceOutcome::Running,
        TraceDataClass::Control,
        Vec::new(),
        RuntimeProperties::from([("project_specific_counter".into(), RuntimeValue::Unsigned(1))]),
    );
    assert!(unknown.is_err());

    for (attribute, wrong_value) in [
        (TraceAttribute::InputBytes, RuntimeValue::String("1".into())),
        (TraceAttribute::Selected, RuntimeValue::Unsigned(1)),
        (TraceAttribute::Score, RuntimeValue::Decimal("NaN".into())),
        (
            TraceAttribute::ErrorDigest,
            RuntimeValue::Digest("not-a-digest".into()),
        ),
        (
            TraceAttribute::ErrorClass,
            RuntimeValue::String("raw error text is forbidden".into()),
        ),
    ] {
        let invalid = RuntimeTraceEvent::annotation(
            trace_id(),
            span_id(),
            None,
            TraceBoundary::Engine,
            TraceOperation::EngineOperation,
            10,
            TraceOutcome::Running,
            TraceDataClass::Control,
            Vec::new(),
            RuntimeProperties::from([(attribute.to_string(), wrong_value)]),
        );
        assert!(
            invalid.is_err(),
            "{} accepted wrong value",
            attribute.as_str()
        );
    }
}

#[test]
fn trace_wire_contract_rejects_old_and_extended_shapes() {
    let event = RuntimeTraceEvent::annotation(
        trace_id(),
        span_id(),
        None,
        TraceBoundary::Adapter,
        TraceOperation::AdapterInvoke,
        10,
        TraceOutcome::Ok,
        TraceDataClass::Control,
        vec![TraceLink::Source {
            source_kind: "project_database".into(),
            source_id: "adapter-source-1".into(),
            source_revision: "revision-1".into(),
        }],
        RuntimeProperties::new(),
    )
    .unwrap();

    let mut extended_event = serde_json::to_value(&event).unwrap();
    extended_event
        .as_object_mut()
        .unwrap()
        .insert("provider".into(), serde_json::json!("hidden-authority"));
    assert!(serde_json::from_value::<RuntimeTraceEvent>(extended_event).is_err());

    let mut old_boundary = serde_json::to_value(&event).unwrap();
    let old_boundary = old_boundary.as_object_mut().unwrap();
    let boundary = old_boundary.remove("boundary").unwrap();
    old_boundary.insert("domain".into(), boundary);
    assert!(
        serde_json::from_value::<RuntimeTraceEvent>(serde_json::Value::Object(
            old_boundary.clone()
        ))
        .is_err()
    );

    let mut extended_link = serde_json::to_value(&event).unwrap();
    extended_link["links"][0]["adapter"] = serde_json::json!("hidden-authority");
    assert!(serde_json::from_value::<RuntimeTraceEvent>(extended_link).is_err());

    let mut retired_link = serde_json::to_value(event).unwrap();
    retired_link["links"][0] = serde_json::json!({
        "kind": "provider",
        "provider": "claude",
        "invocation_id": "invocation-1"
    });
    assert!(serde_json::from_value::<RuntimeTraceEvent>(retired_link).is_err());
}

#[test]
fn canonical_links_retain_correlation_authority_and_output_coordinates() {
    let scope = ScopeId::new("instance:trace-links").unwrap();
    let causal_span = SpanId::new("fedcba9876543210").unwrap();
    let links = vec![
        TraceLink::Request {
            request_id: "request-7".into(),
            operation_id: "query-execute".into(),
            parent_request_id: Some("request-6".into()),
            idempotency_digest: Some("1".repeat(64)),
        },
        TraceLink::ActorScope {
            actor: "seat:clyffy".into(),
            scope: scope.clone(),
        },
        TraceLink::Authorization {
            decision: AuditDecision::Allow,
            policy_revision: 9,
            authorization_digest: "2".repeat(64),
        },
        TraceLink::Source {
            source_kind: "project_tree".into(),
            source_id: "snapshot-7".into(),
            source_revision: "revision-3".into(),
        },
        TraceLink::Commit {
            commit_id: "3".repeat(64),
            first_cursor: 40,
            last_cursor: 43,
        },
        TraceLink::Resource {
            resource_kind: "instance".into(),
            resource_id: "rrflow-test".into(),
        },
        TraceLink::CausalSpan {
            trace_id: trace_id(),
            span_id: causal_span,
            relation: TraceCausalRelation::FollowsFrom,
        },
        TraceLink::Output {
            output_digest: "4".repeat(64),
            item_count: 7,
            byte_count: 512,
        },
    ];
    let runtime = RuntimeTraceEvent::annotation(
        trace_id(),
        span_id(),
        None,
        TraceBoundary::Engine,
        TraceOperation::EngineOperation,
        10,
        TraceOutcome::Ok,
        TraceDataClass::Control,
        links,
        RuntimeProperties::new(),
    )
    .unwrap()
    .into_runtime_event()
    .unwrap();
    assert_eq!(
        runtime.properties["boundary"],
        RuntimeValue::String("engine".into())
    );
    let RuntimeValue::List(links) = &runtime.properties["links"] else {
        panic!("links must remain a typed list")
    };
    let kinds = links
        .iter()
        .map(|link| {
            let RuntimeValue::Map(link) = link else {
                panic!("link must remain a typed map")
            };
            let RuntimeValue::String(kind) = &link["kind"] else {
                panic!("link kind must remain a string")
            };
            kind.as_str()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        kinds,
        [
            "request",
            "actor_scope",
            "authorization",
            "source",
            "commit",
            "resource",
            "causal_span",
            "output",
        ]
    );

    let self_link = RuntimeTraceEvent::annotation(
        trace_id(),
        span_id(),
        None,
        TraceBoundary::Routine,
        TraceOperation::RoutineStep,
        11,
        TraceOutcome::Running,
        TraceDataClass::Control,
        vec![TraceLink::CausalSpan {
            trace_id: trace_id(),
            span_id: span_id(),
            relation: TraceCausalRelation::RetryOf,
        }],
        RuntimeProperties::new(),
    );
    assert!(self_link.is_err());

    let unstamped_authorization = RuntimeTraceEvent::annotation(
        trace_id(),
        span_id(),
        None,
        TraceBoundary::Engine,
        TraceOperation::EngineAuthorize,
        12,
        TraceOutcome::Denied,
        TraceDataClass::Control,
        vec![TraceLink::Authorization {
            decision: AuditDecision::Deny,
            policy_revision: 0,
            authorization_digest: "5".repeat(64),
        }],
        RuntimeProperties::new(),
    );
    assert!(unstamped_authorization.is_err());
}
