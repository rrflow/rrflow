//! Paired and floor verdicts and combining task runs.

use super::*;

/// Compares an arm with the baseline only on runs that exist for both.
pub(super) fn paired_verdict(
    manifest: &TaskComparisonManifest,
    runs: &[TaskRun],
    arm: &TaskArm,
    model: &TaskModel,
) -> Result<Verdict> {
    let rules = &manifest.preregistration;
    let lookup = |arm_id: &Id| -> BTreeMap<(&Id, u32), bool> {
        runs.iter()
            .filter(|run| {
                &run.arm == arm_id
                    && run.model == model.id
                    && matches!(
                        run.outcome,
                        TaskRunOutcome::Planned | TaskRunOutcome::NoPlan
                    )
            })
            .map(|run| {
                (
                    (&run.task, run.repetition),
                    run.score.matches_file_threshold,
                )
            })
            .collect()
    };
    let this_arm = lookup(&arm.id);
    let baseline = lookup(&rules.baseline_arm);
    let (mut pairs, mut only_arm, mut only_baseline) = (0_u64, 0_u64, 0_u64);
    for (key, solved) in &this_arm {
        if let Some(other) = baseline.get(key) {
            pairs += 1;
            match (solved, other) {
                (true, false) => only_arm += 1,
                (false, true) => only_baseline += 1,
                _ => {}
            }
        }
    }
    let p = exact_mcnemar(only_arm, only_baseline);
    let pilot = rules.stage == StudyStage::Pilot;
    let expected_pairs = (manifest.tasks.len() as u64) * u64::from(rules.repetitions);
    let outcome = if pairs == 0 || pilot {
        VerdictOutcome::NotRun
    } else if pairs != expected_pairs || rules.repetitions > 1 {
        VerdictOutcome::Inconclusive
    } else if only_arm > only_baseline && p < rules.alpha {
        VerdictOutcome::Met
    } else if only_baseline > only_arm && p < rules.alpha {
        VerdictOutcome::NotMet
    } else {
        VerdictOutcome::Inconclusive
    };
    let mut verdict = Verdict {
        id: id(&format!(
            "helps-{}",
            system_id(&arm.id, &model.id)?.as_str()
        ))?,
        question: format!(
            "Did {} match the historical file threshold more often than {} on the same tasks, at alpha {}?",
            arm.id.as_str(),
            rules.baseline_arm.as_str(),
            rules.alpha
        ),
        system: Some(system_id(&arm.id, &model.id)?),
        outcome,
        basis: if pairs == 0 {
            "no paired runs".to_string()
        } else if rules.repetitions > 1 {
            format!(
                "{pairs} of {expected_pairs} scheduled pairs: only this arm matched {only_arm}, only the baseline matched {only_baseline}. Repetitions of a task are not independent; this diagnostic has no clustered significance test."
            )
        } else {
            format!(
                "{pairs} of {expected_pairs} scheduled pairs: only this arm matched {only_arm}, only the baseline matched {only_baseline}, exact McNemar p = {p:.4}. A result that is not significant does not show the arm has no effect."
            )
        },
    };
    if pilot {
        verdict.basis = format!(
            "A pilot tests the test and cannot answer this. {}",
            verdict.basis
        );
    }
    Ok(verdict)
}

pub(super) fn floor_verdict(
    manifest: &TaskComparisonManifest,
    runs: &[TaskRun],
    model: &TaskModel,
) -> Result<Verdict> {
    let rules = &manifest.preregistration;
    let floor: Vec<&TaskRun> = runs
        .iter()
        .filter(|run| {
            run.arm == rules.floor_arm
                && run.model == model.id
                && matches!(
                    run.outcome,
                    TaskRunOutcome::Planned | TaskRunOutcome::NoPlan
                )
        })
        .collect();
    let solved = floor
        .iter()
        .filter(|run| run.score.matches_file_threshold)
        .count();
    let rate = solved as f64 / floor.len().max(1) as f64;
    Ok(Verdict {
        id: id(&format!("floor-match-{}", model.id.as_str()))?,
        question: format!(
            "Did the arm with no repository match the historical file threshold on at most {} of observed runs?",
            rules.max_floor_file_match_rate
        ),
        system: Some(system_id(&rules.floor_arm, &model.id)?),
        outcome: if floor.is_empty() {
            VerdictOutcome::NotRun
        } else if floor.len() != manifest.tasks.len() * rules.repetitions as usize {
            VerdictOutcome::Inconclusive
        } else if rate <= rules.max_floor_file_match_rate {
            VerdictOutcome::Met
        } else {
            VerdictOutcome::NotMet
        },
        basis: if floor.is_empty() {
            "no floor runs".to_string()
        } else {
            format!("{solved} of {} matched. This observation cannot prove the tasks have no leakage.", floor.len())
        },
    })
}

/// One report over every run, refusing any run made under a different frozen design.
pub fn combine_task_runs(
    manifest: &TaskComparisonManifest,
    runs: &[TaskRun],
    artifacts: Vec<ArtifactReference>,
    generated_at_unix_ms: u64,
) -> Result<LabReport> {
    manifest.validate()?;
    let digest = manifest.digest()?;
    let mut seen = BTreeSet::new();
    for run in runs {
        require(
            run.manifest_sha256 == digest,
            &format!("{} ran under a different frozen design", run.task.as_str()),
        )?;
        require(
            seen.insert((&run.task, &run.arm, &run.model, run.repetition)),
            "a run appears twice",
        )?;
        let arm = manifest.arm(run.arm.as_str())?;
        require(
            manifest.tasks.iter().any(|task| task.id == run.task)
                && manifest.models.iter().any(|model| model.id == run.model)
                && (1..=manifest.preregistration.repetitions).contains(&run.repetition),
            "run coordinates are outside the frozen design",
        )?;
        require(
            arm.availability == Availability::Available,
            &format!(
                "{} is declared as not running but has a run",
                arm.id.as_str()
            ),
        )?;
    }
    let required = &manifest.preregistration.required_captures;
    let counted: Vec<TaskRun> = runs
        .iter()
        .filter(|run| {
            manifest
                .arm(run.arm.as_str())
                .is_ok_and(|arm| run.counts(arm, required))
        })
        .cloned()
        .collect();
    let mut systems = Vec::new();
    let mut observations = Vec::new();
    let mut verdicts = Vec::new();
    for arm in &manifest.arms {
        let declared = match &arm.availability {
            Availability::Available => None,
            Availability::Unavailable { reason } => Some((SystemStatus::Unavailable, reason)),
            Availability::Unimplemented { reason } => Some((SystemStatus::Unimplemented, reason)),
            Availability::NotApplicable { reason } => Some((SystemStatus::NotApplicable, reason)),
        };
        if let Some((status, reason)) = declared {
            systems.push(ReportSystem {
                id: arm.id.clone(),
                label: arm.label.clone(),
                kind: "agent-arm".into(),
                status,
                reason: Some(reason.clone()),
                observed_version: None,
                configuration: BTreeMap::new(),
            });
            continue;
        }
        if manifest.models.is_empty() {
            systems.push(ReportSystem {
                id: arm.id.clone(),
                label: arm.label.clone(),
                kind: "agent-arm".into(),
                status: SystemStatus::NotRun,
                reason: Some("no model is chosen yet".into()),
                observed_version: None,
                configuration: BTreeMap::new(),
            });
            continue;
        }
        for model in &manifest.models {
            let system = system_id(&arm.id, &model.id)?;
            let own: Vec<&TaskRun> = counted
                .iter()
                .filter(|run| run.arm == arm.id && run.model == model.id)
                .collect();
            let made = runs
                .iter()
                .filter(|run| run.arm == arm.id && run.model == model.id)
                .count();
            let usable = own
                .iter()
                .filter(|run| {
                    matches!(
                        run.outcome,
                        TaskRunOutcome::Planned | TaskRunOutcome::NoPlan
                    )
                })
                .count();
            let expected = manifest.tasks.len() * manifest.preregistration.repetitions as usize;
            let status = if made == 0 {
                SystemStatus::NotRun
            } else if usable == 0 {
                SystemStatus::Failed
            } else if made != expected || usable != expected {
                SystemStatus::Partial
            } else {
                SystemStatus::Completed
            };
            systems.push(ReportSystem {
                id: system.clone(),
                label: format!("{}, {}", arm.label, model.model),
                kind: "agent-arm".into(),
                status,
                reason: match status {
                    SystemStatus::NotRun => Some("no run file was supplied".to_string()),
                    SystemStatus::Failed | SystemStatus::Partial => Some(format!(
                        "{made} of {expected} scheduled runs supplied; {usable} have usable replies and required capture; see the capture audit"
                    )),
                    _ => None,
                },
                observed_version: Some(model.model.clone()),
                configuration: BTreeMap::new(),
            });
            if !own.is_empty() {
                system_observations(&system, &own, &mut observations)?;
            }
            if made > own.len() {
                observations.push(observation(
                    &system,
                    "runs_not_counted",
                    BTreeMap::new(),
                    (made - own.len()) as f64,
                    made as u64,
                    None,
                )?);
            }
            if arm.id != manifest.preregistration.baseline_arm
                && arm.id != manifest.preregistration.floor_arm
            {
                verdicts.push(paired_verdict(manifest, &counted, arm, model)?);
            }
        }
    }
    for model in &manifest.models {
        verdicts.push(floor_verdict(manifest, &counted, model)?);
    }
    use MetricDirection::{Higher, Lower};
    let mut limits = vec![
        "Diagnostic evidence for optimization: not J-04 acceptance, and no parity or superiority claim.".to_string(),
        "The reference is the files one historical change touched. Other valid plans can name different files; file matches do not prove correctness or task completion.".to_string(),
        "Historical-file-match thresholds were frozen before any run and are a judgement.".to_string(),
        "Grading reads the plan. It does not check that the plan would work if carried out.".to_string(),
    ];
    if manifest.preregistration.stage == StudyStage::Pilot {
        limits.insert(
            0,
            "This is a pilot. It tests the test, and it cannot say whether an arm helps."
                .to_string(),
        );
    }
    for arm in &manifest.arms {
        for limit in &arm.limits {
            limits.push(format!("{}: {limit}", arm.id.as_str()));
        }
    }
    let report = LabReport {
        schema_version: REPORT_SCHEMA_VERSION,
        test: manifest.test.clone(),
        evidence_class: LabEvidenceClass::Diagnostic,
        generated_by: format!("rrflow-lab {}", env!("CARGO_PKG_VERSION")),
        generated_at_unix_ms,
        environment: BTreeMap::new(),
        identities: BTreeMap::from([("manifest".to_string(), digest)]),
        systems,
        dimensions: vec![DimensionDefinition {
            id: id("task")?,
            label: "Task".into(),
        }],
        metrics: vec![
            metric(
                "file_match_rate",
                "Plans matching historical file thresholds",
                "fraction",
                Higher,
            )?,
            metric(
                "file_recall",
                "Needed files the plan named",
                "fraction",
                Higher,
            )?,
            metric(
                "file_precision",
                "Named files that were needed",
                "fraction",
                Higher,
            )?,
            metric("records_cited", "Owning records cited", "fraction", Higher)?,
            metric(
                "runs_without_a_plan",
                "Runs that gave no plan",
                "count",
                Lower,
            )?,
            metric(
                "runs_not_counted",
                "Runs that cannot count, for missing data or tools that never came up",
                "count",
                Lower,
            )?,
            metric(
                "needed_returned",
                "Needed files the tool returned",
                "fraction",
                Higher,
            )?,
            metric(
                "returned_used",
                "Returned needed files the plan named",
                "fraction",
                Higher,
            )?,
            metric(
                "cache_read_tokens",
                "Median cache-read tokens per run",
                "tokens",
                Lower,
            )?,
            metric("tool_calls", "Median tool calls per run", "calls", Lower)?,
            metric("seconds", "Median time per run", "s", Lower)?,
            metric(
                "input_tokens",
                "Median input tokens per run",
                "tokens",
                Lower,
            )?,
            metric(
                "output_tokens",
                "Median output tokens per run",
                "tokens",
                Lower,
            )?,
            metric("cost_usd", "Median cost per run", "USD", Lower)?,
        ],
        observations,
        verdicts,
        views: vec![
            ViewDefinition::Bars {
                id: id("file-match")?,
                title: "Historical file-threshold matches".into(),
                metric: id("file_match_rate")?,
                fixed: BTreeMap::new(),
            },
            ViewDefinition::Bars {
                id: id("cost")?,
                title: "Median cost per run".into(),
                metric: id("cost_usd")?,
                fixed: BTreeMap::new(),
            },
            ViewDefinition::Bars {
                id: id("tokens")?,
                title: "Median input tokens per run".into(),
                metric: id("input_tokens")?,
                fixed: BTreeMap::new(),
            },
            ViewDefinition::Table {
                id: id("by-task")?,
                title: "Historical file-threshold matches, by task".into(),
                metrics: vec![id("file_match_rate")?],
                rows: vec![id("task")?],
                fixed: BTreeMap::new(),
            },
        ],
        limits,
        artifacts,
    };
    report.validate()?;
    Ok(report)
}
