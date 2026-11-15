//! Closed signal-catalogue contract for build identity and later projections.
//!
//! This test intentionally exercises only inert semantic descriptors. Runtime
//! collection, export, and authoritative engine state do not belong here.

use rrd_core::{
    signal_catalogue, signal_catalogue_sha256, DiagnosticLevel, MetricAttribute, MetricInstrument,
    MetricKind, MetricUnit, SignalCatalogue, TraceAttribute, TraceBoundary, TraceOperation,
    DEFAULT_METRIC_CARDINALITY_LIMIT, RUNTIME_TRACE_CONTRACT_VERSION, TELEMETRY_CONTRACT_VERSION,
};
use serde_json::{json, Value};

const FIXTURE: &str = include_str!("../fixtures/telemetry-catalogue-v1.json");

#[test]
fn catalogue_is_closed_ordered_and_derived_from_the_trace_owners() {
    let catalogue = signal_catalogue();
    catalogue.validate().unwrap();

    assert_eq!(catalogue.contract_version, TELEMETRY_CONTRACT_VERSION);
    assert_eq!(
        catalogue.runtime_trace_contract_version,
        RUNTIME_TRACE_CONTRACT_VERSION
    );
    assert_eq!(
        catalogue.default_metric_cardinality_limit,
        DEFAULT_METRIC_CARDINALITY_LIMIT
    );
    assert_eq!(catalogue.default_diagnostic_level, DiagnosticLevel::Normal);
    assert_eq!(catalogue.diagnostic_levels, DiagnosticLevel::ALL);
    assert_eq!(catalogue.metric_attributes, MetricAttribute::ALL);
    assert_eq!(
        catalogue
            .metric_attributes
            .iter()
            .map(|attribute| attribute.as_str())
            .collect::<Vec<_>>(),
        [
            "rrflow.boundary",
            "rrflow.operation",
            "rrflow.outcome",
            "rrflow.storage.profile",
            "rrflow.access.path",
            "rrflow.work.kind",
            "rrflow.queue.kind",
            "rrflow.cache.kind",
            "error.type",
        ]
    );

    assert_eq!(catalogue.trace_operations.len(), TraceOperation::ALL.len());
    for (descriptor, operation) in catalogue.trace_operations.iter().zip(TraceOperation::ALL) {
        assert_eq!(descriptor.name, operation.as_str());
        assert_eq!(descriptor.boundary, operation.boundary());
    }

    assert_eq!(catalogue.trace_attributes.len(), TraceAttribute::ALL.len());
    for (descriptor, attribute) in catalogue.trace_attributes.iter().zip(TraceAttribute::ALL) {
        assert_eq!(descriptor.name, attribute.as_str());
    }

    assert_eq!(catalogue.metric_instruments.len(), 22);
    assert_eq!(
        catalogue.metric_instruments.len(),
        MetricInstrument::ALL.len()
    );
    assert_eq!(
        catalogue
            .metric_instruments
            .iter()
            .map(|descriptor| descriptor.instrument.as_str())
            .collect::<Vec<_>>(),
        [
            "rrflow.operation.duration",
            "rrflow.operation.count",
            "rrflow.operation.inflight",
            "rrflow.queue.depth",
            "rrflow.queue.wait",
            "rrflow.kv.keys.examined",
            "rrflow.kv.pages.examined",
            "rrflow.kv.bytes",
            "rrflow.kv.cache.requests",
            "rrflow.query.rows",
            "rrflow.query.graph.steps",
            "rrflow.query.candidates",
            "rrflow.datafusion.memory.usage",
            "rrflow.datafusion.memory.peak",
            "rrflow.datafusion.spill",
            "rrflow.context.bytes",
            "rrflow.context.tokens",
            "rrflow.delivery.backlog",
            "rrflow.process.memory",
            "rrflow.process.cpu.time",
            "rrflow.telemetry.dropped",
            "rrflow.telemetry.export.errors",
        ]
    );

    let kinds_and_units = [
        (MetricKind::Histogram, MetricUnit::Seconds),
        (MetricKind::Counter, MetricUnit::Operations),
        (MetricKind::UpDownCounter, MetricUnit::Operations),
        (MetricKind::ObservableGauge, MetricUnit::Items),
        (MetricKind::Histogram, MetricUnit::Seconds),
        (MetricKind::Counter, MetricUnit::Items),
        (MetricKind::Counter, MetricUnit::Items),
        (MetricKind::Counter, MetricUnit::Bytes),
        (MetricKind::Counter, MetricUnit::Requests),
        (MetricKind::Counter, MetricUnit::Rows),
        (MetricKind::Counter, MetricUnit::Steps),
        (MetricKind::Counter, MetricUnit::Candidates),
        (MetricKind::ObservableGauge, MetricUnit::Bytes),
        (MetricKind::Histogram, MetricUnit::Bytes),
        (MetricKind::Counter, MetricUnit::Bytes),
        (MetricKind::Histogram, MetricUnit::Bytes),
        (MetricKind::Histogram, MetricUnit::Tokens),
        (MetricKind::ObservableGauge, MetricUnit::Items),
        (MetricKind::ObservableGauge, MetricUnit::Bytes),
        (MetricKind::Counter, MetricUnit::Seconds),
        (MetricKind::Counter, MetricUnit::Items),
        (MetricKind::Counter, MetricUnit::Items),
    ];
    for ((descriptor, instrument), (kind, unit)) in catalogue
        .metric_instruments
        .iter()
        .zip(MetricInstrument::ALL)
        .zip(kinds_and_units)
    {
        assert_eq!(descriptor.instrument, instrument);
        assert_eq!(descriptor.kind, kind, "{} kind", instrument.as_str());
        assert_eq!(descriptor.unit, unit, "{} unit", instrument.as_str());
        assert!(!descriptor.description.is_empty());
    }

    for descriptor in &catalogue.metric_instruments {
        assert!(!descriptor.attributes.is_empty());
        assert!(descriptor
            .attributes
            .iter()
            .all(|attribute| MetricAttribute::ALL.contains(attribute)));
        assert!(descriptor.attributes.windows(2).all(|pair| {
            let left = MetricAttribute::ALL
                .iter()
                .position(|attribute| attribute == &pair[0])
                .unwrap();
            let right = MetricAttribute::ALL
                .iter()
                .position(|attribute| attribute == &pair[1])
                .unwrap();
            left < right
        }));
    }
}

#[test]
fn golden_projection_and_fingerprint_are_stable() {
    let catalogue = signal_catalogue();
    assert_eq!(
        catalogue.sha256().unwrap(),
        signal_catalogue_sha256().unwrap()
    );
    let actual = json!({
        "catalogue": catalogue,
        "sha256": catalogue.sha256().unwrap(),
    });

    if std::env::var_os("TELEMETRY_GOLDEN_WRITE").is_some() {
        std::fs::write(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/fixtures/telemetry-catalogue-v1.json"
            ),
            format!("{}\n", serde_json::to_string_pretty(&actual).unwrap()),
        )
        .unwrap();
        return;
    }

    let expected: Value = serde_json::from_str(FIXTURE).unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn wire_and_semantic_drift_fail_closed() {
    let catalogue = signal_catalogue();

    let mut unknown = serde_json::to_value(&catalogue).unwrap();
    unknown["exporter"] = json!("hidden-authority");
    assert!(serde_json::from_value::<SignalCatalogue>(unknown).is_err());

    let mut nested_unknown = serde_json::to_value(&catalogue).unwrap();
    nested_unknown["metric_instruments"][0]["aggregation"] = json!("private");
    assert!(serde_json::from_value::<SignalCatalogue>(nested_unknown).is_err());

    let mut missing_operation = catalogue.clone();
    missing_operation.trace_operations.pop();
    assert!(missing_operation.validate().is_err());
    assert!(missing_operation.sha256().is_err());

    let mut wrong_boundary = catalogue.clone();
    wrong_boundary.trace_operations[0].boundary = TraceBoundary::Engine;
    assert!(wrong_boundary.validate().is_err());

    let mut unknown_trace_name = catalogue.clone();
    unknown_trace_name.trace_operations[0].name = "rrflow.dynamic.request".into();
    assert!(unknown_trace_name.validate().is_err());

    let mut reordered_attributes = catalogue.clone();
    reordered_attributes.trace_attributes.swap(0, 1);
    assert!(reordered_attributes.validate().is_err());
    assert!(reordered_attributes.sha256().is_err());

    let mut reordered_metric_attributes = catalogue.clone();
    reordered_metric_attributes.metric_attributes.swap(0, 1);
    assert!(reordered_metric_attributes.validate().is_err());

    let mut reordered_metrics = catalogue.clone();
    reordered_metrics.metric_instruments.swap(0, 1);
    assert!(reordered_metrics.validate().is_err());

    let mut wrong_metric = catalogue.clone();
    wrong_metric.metric_instruments[0].instrument = MetricInstrument::OperationCount;
    assert!(wrong_metric.validate().is_err());
    assert!(wrong_metric.sha256().is_err());

    let mut wrong_kind = catalogue.clone();
    wrong_kind.metric_instruments[0].kind = MetricKind::Counter;
    assert!(wrong_kind.validate().is_err());

    let mut wrong_unit = catalogue.clone();
    wrong_unit.metric_instruments[0].unit = MetricUnit::Bytes;
    assert!(wrong_unit.validate().is_err());

    let mut duplicate_dimension = catalogue.clone();
    duplicate_dimension.metric_instruments[0]
        .attributes
        .push(MetricAttribute::Boundary);
    assert!(duplicate_dimension.validate().is_err());

    let mut dynamic_dimension = serde_json::to_value(&catalogue).unwrap();
    dynamic_dimension["metric_instruments"][0]["attributes"]
        .as_array_mut()
        .unwrap()
        .push(json!("rrflow.request.id"));
    assert!(serde_json::from_value::<SignalCatalogue>(dynamic_dimension).is_err());
}

#[test]
fn catalogue_contains_no_effect_or_content_authority() {
    let encoded = serde_json::to_string(&signal_catalogue()).unwrap();
    for forbidden in [
        "actor_id",
        "authorization",
        "credential",
        "engram_id",
        "model_output",
        "project_id",
        "prompt",
        "query_text",
        "request_id",
        "source_body",
        "span_id",
        "trace_id",
    ] {
        assert!(
            !encoded.contains(forbidden),
            "signal catalogue leaked dynamic/content field {forbidden}"
        );
    }
}
