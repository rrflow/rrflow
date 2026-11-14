//! Report views, case observations and combining runs into a report.

use super::*;

/// Summarize incomplete cases both on new runs and when combining retained
/// runs whose earlier implementation marked a partial run completed.
pub(super) fn partial_reason(cases: &[CaseResult]) -> Option<String> {
    let incomplete = cases
        .iter()
        .filter(|case| case.errors > 0 || case.truncated)
        .count();
    if incomplete == 0 {
        return None;
    }
    let errors: u64 = cases.iter().map(|case| case.errors).sum();
    let truncated = cases.iter().filter(|case| case.truncated).count();
    let first = cases.iter().find_map(|case| case.first_error.as_deref());
    let detail = first
        .map(|error| {
            format!(
                "; first error: {}",
                error.chars().take(512).collect::<String>()
            )
        })
        .unwrap_or_default();
    Some(format!("{incomplete} incomplete case(s), {errors} failed request(s), {truncated} truncated case(s){detail}"))
}

pub(super) fn metric(
    id: &str,
    label: &str,
    unit: &str,
    better: MetricDirection,
) -> Result<MetricDefinition> {
    Ok(MetricDefinition {
        id: Id::try_from(id.to_string())?,
        label: label.into(),
        unit: unit.into(),
        better,
    })
}

pub(super) fn report_metrics() -> Result<Vec<MetricDefinition>> {
    use MetricDirection::{Higher, Lower};
    Ok(vec![
        metric("recall_at_k", "Recall at k", "fraction", Higher)?,
        metric("queries_per_second", "Queries per second", "1/s", Higher)?,
        metric(
            "qps_at_recall_target",
            "Queries per second at target recall",
            "1/s",
            Higher,
        )?,
        metric("latency_p50_ms", "Latency p50", "ms", Lower)?,
        metric("latency_p95_ms", "Latency p95", "ms", Lower)?,
        metric("latency_p99_ms", "Latency p99", "ms", Lower)?,
        metric("case_attempted", "Searches attempted", "count", Higher)?,
        metric("case_succeeded", "Searches succeeded", "count", Higher)?,
        metric("search_errors", "Search errors", "count", Lower)?,
        metric("case_truncated", "Case truncated", "boolean", Lower)?,
        metric(
            "ingest_points_per_second",
            "Ingest rate",
            "points/s",
            Higher,
        )?,
        metric(
            "ready_seconds",
            "Time until search-ready after ingest",
            "s",
            Lower,
        )?,
        metric("memory_bytes", "Server memory", "bytes", Lower)?,
        metric("disk_bytes", "Server disk", "bytes", Lower)?,
    ])
}

pub(super) fn report_dimensions() -> Result<Vec<DimensionDefinition>> {
    [
        ("filter", "Filter"),
        ("setting", "Search setting"),
        ("concurrency", "Concurrent clients"),
    ]
    .into_iter()
    .map(|(id, label)| {
        Ok(DimensionDefinition {
            id: Id::try_from(id.to_string())?,
            label: label.into(),
        })
    })
    .collect()
}

pub(super) fn fixed(entries: &[(&str, String)]) -> BTreeMap<String, String> {
    entries
        .iter()
        .map(|(key, value)| ((*key).to_string(), value.clone()))
        .collect()
}

pub(super) fn view_id(text: &str) -> Result<Id> {
    Id::try_from(text.to_string())
}

pub(super) fn report_views(manifest: &VectorComparisonManifest) -> Result<Vec<ViewDefinition>> {
    let concurrency = manifest.workload.concurrency[0].to_string();
    let mut views = Vec::new();
    let mut filters = vec!["none"];
    if manifest.workload.filtered {
        filters.push("category");
    }
    for filter in filters {
        let selection = fixed(&[
            ("filter", filter.into()),
            ("concurrency", concurrency.clone()),
        ]);
        views.push(ViewDefinition::Curve {
            id: view_id(&format!("recall-speed-{filter}"))?,
            title: format!("Recall against speed, filter: {filter}, {concurrency} client(s)"),
            x: view_id("recall_at_k")?,
            y: view_id("queries_per_second")?,
            fixed: selection.clone(),
        });
        views.push(ViewDefinition::Bars {
            id: view_id(&format!("matched-quality-{filter}"))?,
            title: format!(
                "Speed at recall {} or better, filter: {filter}",
                manifest.quality.recall_target
            ),
            metric: view_id("qps_at_recall_target")?,
            fixed: selection,
        });
    }
    for (id, title, metric) in [
        ("ingest", "Ingest rate", "ingest_points_per_second"),
        ("ready", "Time until search-ready", "ready_seconds"),
        ("memory", "Server memory", "memory_bytes"),
        ("disk", "Server disk", "disk_bytes"),
    ] {
        views.push(ViewDefinition::Bars {
            id: view_id(id)?,
            title: title.into(),
            metric: view_id(metric)?,
            fixed: BTreeMap::new(),
        });
    }
    views.push(ViewDefinition::Table {
        id: view_id("all-cases")?,
        title: "Every measured case".into(),
        metrics: [
            "recall_at_k",
            "queries_per_second",
            "latency_p50_ms",
            "latency_p95_ms",
            "latency_p99_ms",
            "case_attempted",
            "case_succeeded",
            "search_errors",
            "case_truncated",
        ]
        .into_iter()
        .map(view_id)
        .collect::<Result<_>>()?,
        rows: ["setting", "filter", "concurrency"]
            .into_iter()
            .map(view_id)
            .collect::<Result<_>>()?,
        fixed: BTreeMap::new(),
    });
    Ok(views)
}

pub(super) fn system_kind(kind: SystemKind) -> &'static str {
    match kind {
        SystemKind::Rrflow => "rrflow",
        SystemKind::Qdrant => "qdrant",
        SystemKind::Surrealdb => "surrealdb",
        SystemKind::Milvus => "milvus",
        SystemKind::Custom => "custom",
    }
}

pub(super) fn observe(
    list: &mut Vec<Observation>,
    system: &Id,
    id: &str,
    case: &BTreeMap<String, String>,
    value: f64,
    samples: u64,
) -> Result<()> {
    list.push(Observation {
        system: system.clone(),
        metric: Id::try_from(id.to_string())?,
        case: case.clone(),
        value,
        samples,
        low: None,
        high: None,
    });
    Ok(())
}

pub(super) fn case_observations(run: &SystemRun, list: &mut Vec<Observation>) -> Result<()> {
    let system = &run.system;
    if let Some(ingest) = &run.ingest {
        let none = BTreeMap::new();
        observe(
            list,
            system,
            "ingest_points_per_second",
            &none,
            ingest.points_per_second,
            ingest.points,
        )?;
        observe(
            list,
            system,
            "ready_seconds",
            &none,
            ingest.ready_seconds,
            1,
        )?;
    }
    for (id, value) in [
        ("memory_bytes", run.memory_bytes),
        ("disk_bytes", run.disk_bytes),
    ] {
        if let Some(value) = value {
            observe(list, system, id, &BTreeMap::new(), value as f64, 1)?;
        }
    }
    for case in &run.cases {
        let labels = fixed(&[
            (
                "filter",
                if case.filtered { "category" } else { "none" }.to_string(),
            ),
            ("setting", case.setting.as_str().to_string()),
            ("concurrency", case.concurrency.to_string()),
        ]);
        observe(
            list,
            system,
            "case_attempted",
            &labels,
            case.attempted as f64,
            case.attempted,
        )?;
        observe(
            list,
            system,
            "case_succeeded",
            &labels,
            case.succeeded as f64,
            case.attempted,
        )?;
        observe(
            list,
            system,
            "search_errors",
            &labels,
            case.errors as f64,
            case.attempted,
        )?;
        observe(
            list,
            system,
            "case_truncated",
            &labels,
            if case.truncated { 1.0 } else { 0.0 },
            case.attempted,
        )?;
        // Incomplete cases keep counts and raw samples but must not enter
        // the normal recall/speed views as if they completed the workload.
        if case.succeeded == 0 || case.errors > 0 || case.truncated {
            continue;
        }
        list.push(Observation {
            system: system.clone(),
            metric: Id::try_from("recall_at_k".to_string())?,
            case: labels.clone(),
            value: case.recall_mean,
            samples: case.succeeded,
            low: Some(case.recall_low),
            high: Some(case.recall_high),
        });
        for (id, value) in [
            ("queries_per_second", case.queries_per_second),
            ("latency_p50_ms", case.latency_p50_ms),
            ("latency_p95_ms", case.latency_p95_ms),
            ("latency_p99_ms", case.latency_p99_ms),
        ] {
            observe(list, system, id, &labels, value, case.succeeded)?;
        }
    }
    Ok(())
}

/// The fastest setting that met the declared recall, for each filter and load.
/// A system that never met it has no observation, which a reader can see.
pub(super) fn matched_quality(
    run: &SystemRun,
    target: f64,
    list: &mut Vec<Observation>,
) -> Result<()> {
    let mut best: BTreeMap<(bool, u32), &CaseResult> = BTreeMap::new();
    for case in &run.cases {
        if case.truncated || case.errors > 0 || case.succeeded == 0 || case.recall_mean < target {
            continue;
        }
        let key = (case.filtered, case.concurrency);
        if best
            .get(&key)
            .is_none_or(|current| case.queries_per_second > current.queries_per_second)
        {
            best.insert(key, case);
        }
    }
    for ((filtered, concurrency), case) in best {
        let labels = fixed(&[
            (
                "filter",
                if filtered { "category" } else { "none" }.to_string(),
            ),
            ("concurrency", concurrency.to_string()),
        ]);
        observe(
            list,
            &run.system,
            "qps_at_recall_target",
            &labels,
            case.queries_per_second,
            case.succeeded,
        )?;
    }
    Ok(())
}

pub(super) fn recall_verdict(run: &SystemRun, filtered: bool, target: f64) -> Result<Verdict> {
    let scope = if filtered { "filtered" } else { "unfiltered" };
    let best = run
        .cases
        .iter()
        .filter(|case| {
            case.filtered == filtered && case.succeeded > 0 && case.errors == 0 && !case.truncated
        })
        .map(|case| case.recall_mean)
        .fold(None, |best: Option<f64>, value| {
            Some(best.map_or(value, |current| current.max(value)))
        });
    let outcome = match best {
        Some(value) if value >= target => VerdictOutcome::Met,
        Some(_) => VerdictOutcome::NotMet,
        None => VerdictOutcome::NotRun,
    };
    Ok(Verdict {
        id: Id::try_from(format!("{scope}-recall-{}", run.system.as_str()))?,
        question: format!("Did it reach {scope} recall of {target} at any tested setting?"),
        system: Some(run.system.clone()),
        outcome,
        basis: best.map_or("no complete error-free case".to_string(), |value| {
            format!("best error-free mean recall {value:.4}")
        }),
    })
}

pub(super) fn report_limits(manifest: &VectorComparisonManifest) -> Vec<String> {
    let mut limits = vec![
        "Diagnostic evidence for optimization: not J-04 acceptance, and no parity or superiority claim.".to_string(),
        "Cases with errors or truncation cannot qualify for speed at the recall target. Server resource equivalence is not established by matching client metadata.".to_string(),
        "Closed-loop clients: each waits for its reply, so this understates latency under overload and is not an open-loop schedule.".to_string(),
        "Ingest is one sequential writer, which does not show any system's parallel write capacity.".to_string(),
        match &manifest.corpus.source {
            CorpusSource::GaussianMixture { .. } => {
                "The corpus is a synthetic Gaussian mixture, not embeddings of real content."
                    .to_string()
            }
            CorpusSource::Imported { origin, license, .. } => format!(
                "The corpus is {origin}, under {license}. Its filter categories are assigned at random, so a filter here follows no structure in the data."
            ),
        },
        "A configured index can go unused at small sizes: check each system's index evidence before reading its result as an index result.".to_string(),
        "Latency is measured in the harness and includes its own request encoding and reply parsing.".to_string(),
        "Memory compares an engine's resident set with a container's cgroup usage.".to_string(),
    ];
    for binding in &manifest.systems {
        limits.push(format!(
            "{}: {:?} index, storage {}, artifact {}, {:?} endpoint.",
            binding.id.as_str(),
            binding.index,
            binding.storage_profile,
            binding.artifact,
            binding.location
        ));
        for limit in &binding.limits {
            limits.push(format!("{}: {limit}", binding.id.as_str()));
        }
    }
    limits
}

pub fn combine(
    manifest: &VectorComparisonManifest,
    runs: &[SystemRun],
    artifacts: Vec<ArtifactReference>,
) -> Result<LabReport> {
    manifest.validate()?;
    let expected = digest(test_sha256(manifest)?)?;
    let mut identities: Option<&BTreeMap<String, Digest>> = None;
    for run in runs {
        require(
            run.schema_version == RUN_SCHEMA_VERSION,
            "unsupported run schema version",
        )?;
        require(
            run.identities.get("test") == Some(&expected),
            &format!("{} ran a different test", run.system.as_str()),
        )?;
        require(
            manifest
                .binding(run.system.as_str())
                .is_ok_and(|binding| binding_sha256(binding) == run.binding_sha256),
            &format!("{} ran under a different binding", run.system.as_str()),
        )?;
        if let Some(first) = identities {
            require(
                first == &run.identities,
                &format!("{} ran different inputs", run.system.as_str()),
            )?;
        }
        identities = Some(&run.identities);
    }
    let target = manifest.quality.recall_target;
    let mut systems = Vec::new();
    let mut observations = Vec::new();
    let mut verdicts = Vec::new();
    for binding in &manifest.systems {
        let run = runs.iter().find(|run| run.system == binding.id);
        let declared = match &binding.availability {
            Availability::Available => None,
            Availability::Unavailable { reason } => {
                Some((SystemStatus::Unavailable, reason.clone()))
            }
            Availability::Unimplemented { reason } => {
                Some((SystemStatus::Unimplemented, reason.clone()))
            }
            Availability::NotApplicable { reason } => {
                Some((SystemStatus::NotApplicable, reason.clone()))
            }
        };
        require(
            declared.is_none() || run.is_none(),
            &format!(
                "{} is declared as not running but has a run",
                binding.id.as_str()
            ),
        )?;
        let (status, reason) = match (&declared, run) {
            (Some((status, reason)), _) => (*status, Some(reason.clone())),
            (None, Some(run))
                if run.status == SystemStatus::Completed
                    && partial_reason(&run.cases).is_some() =>
            {
                (SystemStatus::Partial, partial_reason(&run.cases))
            }
            (None, Some(run)) => (run.status, run.reason.clone()),
            (None, None) => (
                SystemStatus::NotRun,
                Some("no run file was supplied".to_string()),
            ),
        };
        let mut configuration = BTreeMap::from([
            ("artifact".to_string(), binding.artifact.clone()),
            (
                "storage_profile".to_string(),
                binding.storage_profile.clone(),
            ),
            (
                "index".to_string(),
                serde_json::to_string(&binding.index).map_err(|error| error.to_string())?,
            ),
        ]);
        if let Some(evidence) = run.and_then(|run| run.index_evidence.clone()) {
            configuration.insert("index_evidence".to_string(), evidence);
        }
        if let Some(floor) = run.and_then(|run| run.request_floor.as_ref()) {
            configuration.insert(
                "request_floor".to_string(),
                format!(
                    "p50 {:.2} ms, p95 {:.2} ms at {} clients",
                    floor.p50_ms, floor.p95_ms, floor.clients
                ),
            );
        }
        systems.push(ReportSystem {
            id: binding.id.clone(),
            label: binding.label.clone(),
            kind: system_kind(binding.kind).to_string(),
            status,
            reason,
            observed_version: run.and_then(|run| run.observed_version.clone()),
            configuration,
        });
        if let Some(run) = run {
            case_observations(run, &mut observations)?;
            matched_quality(run, target, &mut observations)?;
            verdicts.push(recall_verdict(run, false, target)?);
            if manifest.workload.filtered {
                verdicts.push(recall_verdict(run, true, target)?);
            }
        }
    }
    let completed: Vec<_> = runs
        .iter()
        .filter(|run| run.status == SystemStatus::Completed)
        .collect();
    let same_environment = completed
        .windows(2)
        .all(|pair| pair[0].environment == pair[1].environment);
    verdicts.push(Verdict {
        id: Id::try_from("same-environment".to_string())?,
        question: "Did the benchmark client metadata match across runs?".into(),
        system: None,
        outcome: match (completed.len() < 2, same_environment) {
            (true, _) => VerdictOutcome::NotRun,
            (false, true) => VerdictOutcome::Met,
            (false, false) => VerdictOutcome::NotMet,
        },
        basis: format!(
            "{} completed run(s); client metadata only, not server resource equivalence",
            completed.len()
        ),
    });
    verdicts.push(Verdict {
        id: Id::try_from("server-resource-equivalence".to_string())?,
        question: "Were the server resource limits and deployment conditions equivalent?".into(),
        system: None,
        outcome: VerdictOutcome::NotRun,
        basis: "Server CPU, memory, storage and deployment equivalence has not been independently established by this report.".into(),
    });
    let environment = completed
        .first()
        .map(|run| run.environment.clone())
        .unwrap_or_default();
    let report = LabReport {
        schema_version: REPORT_SCHEMA_VERSION,
        test: manifest.test.clone(),
        evidence_class: LabEvidenceClass::Diagnostic,
        generated_by: format!("rrflow-lab {}", env!("CARGO_PKG_VERSION")),
        generated_at_unix_ms: now_unix_ms(),
        environment,
        identities: identities.cloned().unwrap_or_default(),
        systems,
        dimensions: report_dimensions()?,
        metrics: report_metrics()?,
        observations,
        verdicts,
        views: report_views(manifest)?,
        limits: report_limits(manifest),
        artifacts,
    };
    report.validate()?;
    Ok(report)
}
