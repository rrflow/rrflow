//! Prompt flight and task contracts.

use super::*;

pub(super) fn flight_context_request() -> rrd_contract::PromptFlightContextRequest {
    rrd_contract::PromptFlightContextRequest {
        valid_at: 1_000,
        seeds: Vec::new(),
        max_graph_depth: 2,
        max_items: 16,
        max_output_bytes: 65_536,
        max_storage_keys: 4_096,
    }
}

pub(super) fn flight_read_stamp(runtime_cursor: u64) -> rrd_contract::ContextReadStamp {
    rrd_contract::ContextReadStamp {
        runtime_manifest_sha256: "a".repeat(64),
        runtime_cursor,
        schema_revision: Some(3),
        catalogue_revision: 1,
    }
}

pub(super) fn flight_event(
    ordinal: u64,
    elapsed_ms: u64,
    stage: rrd_contract::PromptFlightStage,
) -> rrd_contract::PromptFlightEvent {
    rrd_contract::PromptFlightEvent {
        ordinal,
        at_unix_ms: 2_000 + elapsed_ms,
        elapsed_ms,
        stage,
        kind: CanonicalId::new("prompt_received").unwrap(),
        label: "Prompt entered the runtime boundary".into(),
        detail: String::new(),
        data: BTreeMap::from([("bytes".into(), rrd_contract::QueryValue::Unsigned(16))]),
    }
}

pub(super) fn flight_record() -> rrd_contract::PromptFlightRecord {
    let prompt = "  inspect runtime ";
    rrd_contract::PromptFlightRecord {
        format_version: rrd_contract::PROMPT_FLIGHT_FORMAT_VERSION,
        flight_id: CanonicalId::new(format!("flight-{}", "b".repeat(64))).unwrap(),
        scope: "instance:alpha".into(),
        cohort_sha256: rrd_contract::prompt_cohort_sha256(prompt),
        prompt: prompt.into(),
        provider: rrd_contract::PromptFlightProvider::Observe,
        context_mode: rrd_contract::PromptFlightContextMode::Fresh,
        context_request: flight_context_request(),
        acceptance_marker: Some("all checks passed".into()),
        task_id: None,
        status: rrd_contract::PromptFlightStatus::Prepared,
        created_at_unix_ms: 2_000,
        idempotency_key: CorrelationId::new("idem-flight-1").unwrap(),
        operation_sha256: "c".repeat(64),
        read: flight_read_stamp(40),
        context: None,
        metrics: rrd_contract::PromptFlightMetrics {
            context_items: 0,
            context_output_bytes: 0,
            elapsed_ms: 7,
        },
        flight_cursor: 41,
        known_at_cursor: 41,
        use_count: 0,
        event_count: 3,
    }
}

pub(super) fn flight_use(rank: u64, id: &str) -> rrd_contract::PromptFlightUse {
    rrd_contract::PromptFlightUse {
        reference: rrd_contract::DataReference {
            kind: CanonicalId::new("note").unwrap(),
            id: CanonicalId::new(id).unwrap(),
        },
        rank,
        score: 0.03,
        // Deliberately unsorted (text < graph); callers sort it.
        evidence: vec![
            rrd_contract::ContextEvidenceKind::Graph,
            rrd_contract::ContextEvidenceKind::Text,
        ],
    }
}

#[test]
pub(super) fn prompt_flight_contract_is_bounded_strict_and_separately_authorized() {
    let launch = rrd_contract::LaunchPromptFlight {
        scope: "instance:alpha".into(),
        prompt: "inspect runtime".into(),
        provider: rrd_contract::PromptFlightProvider::Observe,
        context_mode: rrd_contract::PromptFlightContextMode::Pruned,
        context: flight_context_request(),
        acceptance_marker: None,
        task_id: None,
    };
    launch.validate().unwrap();
    assert_eq!(
        launch.cohort_sha256(),
        rrd_contract::prompt_cohort_sha256("  inspect runtime\n")
    );
    let mut empty_prompt = launch.clone();
    empty_prompt.prompt = "   ".into();
    assert!(empty_prompt.validate().is_err());
    let mut oversized_prompt = launch.clone();
    oversized_prompt.prompt = "x".repeat(rrd_contract::MAX_PROMPT_FLIGHT_PROMPT_BYTES + 1);
    assert!(oversized_prompt.validate().is_err());
    let mut bad_budget = launch.clone();
    bad_budget.context.max_items = rrd_contract::MAX_CONTEXT_ITEMS + 1;
    assert!(bad_budget.validate().is_err());
    let mut empty_marker = launch.clone();
    empty_marker.acceptance_marker = Some(" ".into());
    assert!(empty_marker.validate().is_err());
    assert!(serde_json::from_value::<rrd_contract::LaunchPromptFlight>(serde_json::json!({
        "scope": "instance:alpha",
        "prompt": "inspect runtime",
        "provider": "codex",
        "context_mode": "pruned",
        "context": {"valid_at": 1000, "max_graph_depth": 2, "max_items": 16, "max_output_bytes": 65536, "max_storage_keys": 4096}
    }))
    .is_err());
    assert!(serde_json::from_value::<rrd_contract::LaunchPromptFlight>(serde_json::json!({
        "scope": "instance:alpha",
        "prompt": "inspect runtime",
        "provider": "observe",
        "context_mode": "pruned",
        "context": {"valid_at": 1000, "max_graph_depth": 2, "max_items": 16, "max_output_bytes": 65536, "max_storage_keys": 4096},
        "reasoning_profile": "ultra"
    }))
    .is_err());
    assert_eq!(
        serde_json::to_value(rrd_contract::PromptFlightStage::Outcome).unwrap(),
        "outcome"
    );

    let record = flight_record();
    record.validate().unwrap();
    let mut wrong_cohort = record.clone();
    wrong_cohort.cohort_sha256 = "d".repeat(64);
    assert!(wrong_cohort.validate().is_err());
    let mut fresh_with_context = record.clone();
    fresh_with_context.metrics.context_items = 1;
    assert!(fresh_with_context.validate().is_err());
    let mut committed_before_read = record.clone();
    committed_before_read.flight_cursor = 40;
    committed_before_read.known_at_cursor = 40;
    assert!(committed_before_read.validate().is_err());
    let mut pruned_without_summary = record.clone();
    pruned_without_summary.context_mode = rrd_contract::PromptFlightContextMode::Pruned;
    assert!(pruned_without_summary.validate().is_err());
    pruned_without_summary.context = Some(rrd_contract::PromptFlightContextSummary {
        request_sha256: "e".repeat(64),
        plan_sha256: "e".repeat(64),
        packet_sha256: "e".repeat(64),
        items: 2,
        output_bytes: 300,
        truncated: false,
        selected_versions: 9,
    });
    assert!(pruned_without_summary.validate().is_err());
    pruned_without_summary.metrics.context_items = 2;
    pruned_without_summary.metrics.context_output_bytes = 300;
    pruned_without_summary.validate().unwrap();

    let mut fresh_with_uses = record.clone();
    fresh_with_uses.use_count = 1;
    assert!(fresh_with_uses.validate().is_err());

    let flight = rrd_contract::PromptFlight {
        record: record.clone(),
        uses: Vec::new(),
        events: vec![
            flight_event(0, 0, rrd_contract::PromptFlightStage::Prompt),
            flight_event(1, 3, rrd_contract::PromptFlightStage::Context),
            flight_event(2, 7, rrd_contract::PromptFlightStage::Outcome),
        ],
    };
    flight.validate().unwrap();
    let mut gapped = flight.clone();
    gapped.events[1].ordinal = 5;
    assert!(gapped.validate().is_err());
    let mut backwards = flight.clone();
    backwards.events[2].elapsed_ms = 1;
    assert!(backwards.validate().is_err());
    let mut too_long = flight.clone();
    too_long.record.metrics.elapsed_ms = 6;
    assert!(too_long.validate().is_err());
    let mut miscounted = flight.clone();
    miscounted.events.pop();
    assert!(miscounted.validate().is_err());
    let encoded = serde_json::to_value(&flight).unwrap();
    assert_eq!(
        serde_json::from_value::<rrd_contract::PromptFlight>(encoded).unwrap(),
        flight
    );

    // A pruned flight's edges: rank order, one per record, sorted evidence.
    let mut graph = flight.clone();
    graph.record.context_mode = rrd_contract::PromptFlightContextMode::Pruned;
    graph.record.context = Some(rrd_contract::PromptFlightContextSummary {
        request_sha256: "e".repeat(64),
        plan_sha256: "e".repeat(64),
        packet_sha256: "e".repeat(64),
        items: 3,
        output_bytes: 300,
        truncated: false,
        selected_versions: 9,
    });
    graph.record.metrics.context_items = 3;
    graph.record.metrics.context_output_bytes = 300;
    graph.record.use_count = 2;
    let mut sorted_evidence = flight_use(1, "alpha");
    sorted_evidence.evidence.sort();
    let mut second = flight_use(3, "beta");
    second.evidence.sort();
    graph.uses = vec![sorted_evidence.clone(), second.clone()];
    graph.validate().unwrap();
    let mut too_many = graph.clone();
    too_many.record.use_count = 4;
    assert!(too_many.record.validate().is_err(), "more edges than items");
    let mut miscounted_uses = graph.clone();
    miscounted_uses.uses.pop();
    assert!(miscounted_uses.validate().is_err());
    let mut unordered_uses = graph.clone();
    unordered_uses.uses.swap(0, 1);
    assert!(unordered_uses.validate().is_err());
    let mut duplicate_record = graph.clone();
    duplicate_record.uses[1].reference = sorted_evidence.reference.clone();
    assert!(duplicate_record.validate().is_err());
    let mut unsorted_evidence = graph.clone();
    unsorted_evidence.uses[0] = flight_use(1, "alpha");
    assert!(
        unsorted_evidence.validate().is_err(),
        "evidence must be sorted"
    );
    let mut zero_score = graph.clone();
    zero_score.uses[0].score = 0.0;
    assert!(zero_score.validate().is_err());
    let encoded = serde_json::to_value(&graph).unwrap();
    assert_eq!(
        serde_json::from_value::<rrd_contract::PromptFlight>(encoded).unwrap(),
        graph
    );

    rrd_contract::ListPromptFlights {
        scope: "instance:alpha".into(),
        after_cursor: 0,
        limit: rrd_contract::MAX_PROMPT_FLIGHT_PAGE,
        max_storage_keys: 1,
    }
    .validate()
    .unwrap();
    assert!(rrd_contract::ListPromptFlights {
        scope: "instance:alpha".into(),
        after_cursor: 0,
        limit: 0,
        max_storage_keys: 1,
    }
    .validate()
    .is_err());
    assert!(rrd_contract::ReadPromptFlight {
        scope: "instance:alpha".into(),
        flight_id: record.flight_id.clone(),
        max_storage_keys: rrd_contract::MAX_PROMPT_FLIGHT_STORAGE_KEYS + 1,
    }
    .validate()
    .is_err());
    let mut later = record.clone();
    later.flight_id = CanonicalId::new("flight-later").unwrap();
    later.flight_cursor = 45;
    later.known_at_cursor = 45;
    let page = rrd_contract::PromptFlightPage {
        scope: "instance:alpha".into(),
        read: flight_read_stamp(50),
        requested_after_cursor: 40,
        flights: vec![record.clone(), later.clone()],
        truncated: false,
        selected_versions: 2,
        read_evidence: read_evidence(),
    };
    page.validate().unwrap();
    let mut unordered = page.clone();
    unordered.flights.swap(0, 1);
    assert!(unordered.validate().is_err());
    let mut beyond_stamp = page.clone();
    beyond_stamp.read = flight_read_stamp(44);
    assert!(beyond_stamp.validate().is_err());

    let catalogue = rrd_contract::endpoint_catalogue();
    let launch_descriptor = catalogue
        .endpoints
        .iter()
        .find(|endpoint| endpoint.operation.as_str() == "flight-launch")
        .unwrap();
    assert_eq!(launch_descriptor.path, "/v1/flights");
    assert!(launch_descriptor.mutation);
    assert_eq!(
        launch_descriptor.action.fixed_action(),
        Some(rrd_contract::SecurityAction::FlightLaunch)
    );
    for (operation, path) in [
        ("flight-list", "/v1/flights/list"),
        ("flight-read", "/v1/flights/read"),
    ] {
        let descriptor = catalogue
            .endpoints
            .iter()
            .find(|endpoint| endpoint.operation.as_str() == operation)
            .unwrap();
        assert_eq!(descriptor.path, path);
        assert!(!descriptor.mutation);
        assert_eq!(
            descriptor.action.fixed_action(),
            Some(rrd_contract::SecurityAction::FlightRead)
        );
    }
    assert_eq!(rrd_contract::SecurityAction::ALL.len(), 59);
}

pub(super) fn task_record() -> rrd_contract::TaskRecord {
    rrd_contract::TaskRecord {
        format_version: rrd_contract::TASK_FORMAT_VERSION,
        task_id: CanonicalId::new(format!("task-{}", "d".repeat(64))).unwrap(),
        scope: "instance:alpha".into(),
        title: "Session recovery".into(),
        goal: "Recover a durable session.".into(),
        status: rrd_contract::TaskStatus::Open,
        outcome: None,
        opened_at_unix_ms: 2_000,
        closed_at_unix_ms: None,
        idempotency_key: CorrelationId::new("idem-task-1").unwrap(),
        operation_sha256: "e".repeat(64),
        task_cursor: 41,
        known_at_cursor: 41,
    }
}

#[test]
pub(super) fn task_contract_is_bounded_strict_and_separately_authorized() {
    let open = rrd_contract::OpenTask {
        scope: "instance:alpha".into(),
        title: "Session recovery".into(),
        goal: "Recover a durable session.".into(),
    };
    open.validate().unwrap();
    for title in [
        "   ".to_owned(),
        "two\nlines".to_owned(),
        "x".repeat(rrd_contract::MAX_TASK_TITLE_BYTES + 1),
    ] {
        let mut invalid = open.clone();
        invalid.title = title;
        assert!(invalid.validate().is_err());
    }
    let mut no_goal = open.clone();
    no_goal.goal = " ".into();
    assert!(no_goal.validate().is_err());
    let mut long_goal = open.clone();
    long_goal.goal = "g".repeat(rrd_contract::MAX_TASK_GOAL_BYTES + 1);
    assert!(long_goal.validate().is_err());
    assert!(
        serde_json::from_value::<rrd_contract::OpenTask>(serde_json::json!({
            "scope": "instance:alpha", "title": "t", "goal": "g", "owner": "someone"
        }))
        .is_err()
    );

    let close = rrd_contract::CloseTask {
        scope: "instance:alpha".into(),
        task_id: task_record().task_id,
        status: rrd_contract::TaskStatus::Completed,
        outcome: Some("Recovered.".into()),
    };
    close.validate().unwrap();
    let mut reopen = close.clone();
    reopen.status = rrd_contract::TaskStatus::Open;
    assert!(reopen.validate().is_err());
    let mut blank_outcome = close.clone();
    blank_outcome.outcome = Some("  ".into());
    assert!(blank_outcome.validate().is_err());
    assert_eq!(
        serde_json::to_value(rrd_contract::TaskStatus::Abandoned).unwrap(),
        serde_json::json!("abandoned")
    );

    let open_task = task_record();
    open_task.validate().unwrap();
    let mut open_with_outcome = open_task.clone();
    open_with_outcome.outcome = Some("early".into());
    assert!(open_with_outcome.validate().is_err());
    let mut open_twice_known = open_task.clone();
    open_twice_known.known_at_cursor = 42;
    assert!(open_twice_known.validate().is_err());
    let mut closed = open_task.clone();
    closed.status = rrd_contract::TaskStatus::Completed;
    closed.outcome = Some("Recovered.".into());
    closed.closed_at_unix_ms = Some(2_500);
    closed.known_at_cursor = 47;
    closed.validate().unwrap();
    let mut closed_without_time = closed.clone();
    closed_without_time.closed_at_unix_ms = None;
    assert!(closed_without_time.validate().is_err());
    let mut closed_before_open = closed.clone();
    closed_before_open.closed_at_unix_ms = Some(1_999);
    assert!(closed_before_open.validate().is_err());
    let mut closed_in_one_version = closed.clone();
    closed_in_one_version.known_at_cursor = closed.task_cursor;
    assert!(closed_in_one_version.validate().is_err());
    let encoded = serde_json::to_value(&closed).unwrap();
    assert_eq!(
        serde_json::from_value::<rrd_contract::TaskRecord>(encoded).unwrap(),
        closed
    );

    let page = rrd_contract::TaskPage {
        scope: "instance:alpha".into(),
        read: flight_read_stamp(50),
        requested_after_cursor: 0,
        tasks: vec![open_task.clone(), {
            let mut later = open_task.clone();
            later.task_cursor = 45;
            later.known_at_cursor = 45;
            later
        }],
        truncated: false,
        selected_versions: 2,
        read_evidence: read_evidence(),
    };
    page.validate().unwrap();
    let mut unordered = page.clone();
    unordered.tasks.swap(0, 1);
    assert!(unordered.validate().is_err());
    let mut mixed = page.clone();
    mixed.tasks[1].scope = "instance:beta".into();
    assert!(mixed.validate().is_err());
    let read = rrd_contract::TaskReadResult {
        task: closed.clone(),
        read: flight_read_stamp(46),
        selected_versions: 2,
        read_evidence: read_evidence(),
    };
    assert!(read.validate().is_err(), "read beyond its stamp");

    // A flight names the task it joins; without one the field is omitted.
    let mut joined = rrd_contract::LaunchPromptFlight {
        scope: "instance:alpha".into(),
        prompt: "inspect runtime".into(),
        provider: rrd_contract::PromptFlightProvider::Observe,
        context_mode: rrd_contract::PromptFlightContextMode::Fresh,
        context: flight_context_request(),
        acceptance_marker: None,
        task_id: None,
    };
    assert!(serde_json::to_value(&joined)
        .unwrap()
        .get("task_id")
        .is_none());
    joined.task_id = Some(open_task.task_id.clone());
    let encoded = serde_json::to_value(&joined).unwrap();
    assert_eq!(
        encoded["task_id"],
        serde_json::json!(open_task.task_id.as_str())
    );
    assert_eq!(
        serde_json::from_value::<rrd_contract::LaunchPromptFlight>(encoded).unwrap(),
        joined
    );

    let catalogue = rrd_contract::endpoint_catalogue();
    for (operation, path, mutation, action) in [
        (
            "task-open",
            "/v1/tasks",
            true,
            rrd_contract::SecurityAction::TaskWrite,
        ),
        (
            "task-close",
            "/v1/tasks/close",
            true,
            rrd_contract::SecurityAction::TaskWrite,
        ),
        (
            "task-list",
            "/v1/tasks/list",
            false,
            rrd_contract::SecurityAction::TaskRead,
        ),
        (
            "task-read",
            "/v1/tasks/read",
            false,
            rrd_contract::SecurityAction::TaskRead,
        ),
    ] {
        let descriptor = catalogue
            .endpoints
            .iter()
            .find(|endpoint| endpoint.operation.as_str() == operation)
            .unwrap();
        assert_eq!(descriptor.path, path);
        assert_eq!(descriptor.mutation, mutation);
        assert_eq!(descriptor.action.fixed_action(), Some(action));
    }
}
