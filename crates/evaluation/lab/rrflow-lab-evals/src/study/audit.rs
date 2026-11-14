//! Study checks and capture audits of recorded runs.

use super::*;

/// Every run this study would make.
pub fn planned_runs(manifest: &TaskComparisonManifest) -> u64 {
    let arms = manifest
        .arms
        .iter()
        .filter(|arm| arm.availability == Availability::Available)
        .count() as u64;
    manifest.tasks.len() as u64
        * arms
        * manifest.models.len() as u64
        * u64::from(manifest.preregistration.repetitions)
}

/// Reports every requirement. A digest mismatch or an unknown arm is an error, since then
/// there is no study to report on.
pub fn check_study(
    config: &StudyConfig,
    catalog: &TaskComparisonManifest,
    requirements: &StudyRequirements,
    proof: Option<&CaptureAudit>,
) -> Result<StudyCheck> {
    let manifest = resolve_study(config, catalog, requirements)?;
    if let (Some(expected), Some(proof)) = (&config.capture_proof_sha256, proof) {
        require(
            expected == &proof.digest()?,
            "the capture audit is not the one this study names",
        )?;
    }
    let rules = &manifest.preregistration;
    let mut outcomes = Vec::with_capacity(requirements.requirements.len());
    let mut ready = true;
    for requirement in &requirements.requirements {
        let level = match config.stage {
            StudyStage::Pilot => requirement.pilot,
            StudyStage::Confirmatory => requirement.confirmatory,
        };
        let (status, detail) = match evaluate(&requirement.check, config, &manifest, proof) {
            Ok(detail) => (RequirementStatus::Met, detail),
            Err(detail) if level == RequirementLevel::Advised => {
                (RequirementStatus::AdvisedNotMet, detail)
            }
            Err(detail) => {
                ready = false;
                (RequirementStatus::NotMet, detail)
            }
        };
        outcomes.push(RequirementOutcome {
            id: requirement.id.clone(),
            title: requirement.title.clone(),
            status,
            detail,
            sources: requirement.sources.clone(),
            records: requirement.records.clone(),
        });
    }
    let needed = discordant_pairs_needed(rules.alpha);
    Ok(StudyCheck {
        schema_version: STUDY_SCHEMA_VERSION,
        study: config.id.clone(),
        stage: config.stage,
        catalog_sha256: catalog.digest()?,
        requirements_sha256: requirements.digest()?,
        tasks: manifest.tasks.len() as u64,
        arms: manifest
            .arms
            .iter()
            .filter(|arm| arm.availability == Availability::Available)
            .count() as u64,
        models: manifest.models.len() as u64,
        runs: planned_runs(&manifest),
        worst_case_usd: manifest
            .spend
            .as_ref()
            .map(|spend| planned_runs(&manifest) as f64 * spend.max_usd_per_run),
        discordant_pairs_needed: needed,
        can_reach_significance: rules.repetitions == 1 && manifest.tasks.len() as u64 >= needed,
        ready,
        outcomes,
    })
}

/// The frozen manifest `tasks-run` reads, written only once the binding requirements are met.
pub fn plan_study(
    config: &StudyConfig,
    catalog: &TaskComparisonManifest,
    requirements: &StudyRequirements,
    proof: Option<&CaptureAudit>,
) -> Result<TaskComparisonManifest> {
    let check = check_study(config, catalog, requirements, proof)?;
    if !check.ready {
        let unmet: Vec<String> = check
            .outcomes
            .iter()
            .filter(|outcome| outcome.status == RequirementStatus::NotMet)
            .map(|outcome| format!("{}: {}", outcome.id.as_str(), outcome.detail))
            .collect();
        return Err(format!("this study is not ready. {}", unmet.join(" ")));
    }
    let manifest = resolve_study(config, catalog, requirements)?;
    manifest.validate()?;
    Ok(manifest)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FieldCapture {
    pub field: CaptureField,
    pub captured: u64,
    pub missing: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ArmCapture {
    pub arm: Id,
    pub model: Id,
    pub model_name: String,
    pub tools: Vec<String>,
    pub runs_expected: u64,
    pub runs_present: u64,
    /// Runs that finished and hold every required field.
    pub runs_counted: u64,
    /// Runs that timed out or failed, which are results and need no further data.
    pub runs_failed: u64,
    /// Runs whose tools never came up, which cannot count as evidence for or against the arm.
    pub runs_without_tools: u64,
    /// At least one run counted, and no finished run lacks anything.
    pub proven: bool,
    pub fields: Vec<FieldCapture>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CaptureAudit {
    pub schema_version: u16,
    pub manifest_sha256: Digest,
    pub stage: StudyStage,
    pub runs_expected: u64,
    pub runs_present: u64,
    pub runs_counted: u64,
    /// Every arm proven, and no problem found. The study itself may still have runs to make.
    pub complete: bool,
    pub arms: Vec<ArmCapture>,
    pub problems: Vec<String>,
}

impl CaptureAudit {
    pub fn digest(&self) -> Result<Digest> {
        Digest::try_from(sha256_hex(&canonical_json(self)?))
    }
}

/// Reads the runs for what actually arrived. This is the proof that a study's data is there,
/// so it trusts nothing about how the runs were made.
pub fn audit_runs(manifest: &TaskComparisonManifest, runs: &[TaskRun]) -> Result<CaptureAudit> {
    manifest.validate()?;
    let digest = manifest.digest()?;
    let rules = &manifest.preregistration;
    let per_arm_runs = manifest.tasks.len() as u64 * u64::from(rules.repetitions);
    let mut problems = Vec::new();
    let mut problem = |text: String| {
        if problems.len() < MAX_AUDIT_PROBLEMS {
            problems.push(text);
        }
    };
    for run in runs {
        if run.manifest_sha256 != digest {
            problem(format!(
                "{} ran under a different design",
                run.task.as_str()
            ));
        }
    }
    let mut arms = Vec::new();
    for arm in manifest
        .arms
        .iter()
        .filter(|arm| arm.availability == Availability::Available)
    {
        for model in &manifest.models {
            let own: Vec<&TaskRun> = runs
                .iter()
                .filter(|run| run.arm == arm.id && run.model == model.id)
                .collect();
            let mut fields: BTreeMap<CaptureField, FieldCapture> = rules
                .required_captures
                .iter()
                .map(|field| {
                    (
                        *field,
                        FieldCapture {
                            field: *field,
                            captured: 0,
                            missing: 0,
                        },
                    )
                })
                .collect();
            let (mut counted, mut failed, mut without_tools, mut lacking) = (0, 0, 0, false);
            for run in &own {
                match run.outcome {
                    TaskRunOutcome::TimedOut | TaskRunOutcome::ProviderFailed => failed += 1,
                    TaskRunOutcome::ToolingFailed => {
                        without_tools += 1;
                        problem(format!(
                            "{} in {}: {}",
                            run.task.as_str(),
                            arm.id.as_str(),
                            run.reason.as_deref().unwrap_or("its tools did not come up")
                        ));
                    }
                    TaskRunOutcome::Planned | TaskRunOutcome::NoPlan => {
                        let missing = run.missing_captures(arm, &rules.required_captures);
                        for (field, entry) in fields.iter_mut() {
                            if missing.contains(field) {
                                entry.missing += 1;
                            } else {
                                entry.captured += 1;
                            }
                        }
                        if missing.is_empty() {
                            counted += 1;
                        } else {
                            lacking = true;
                            problem(format!(
                                "{} in {} is missing {}",
                                run.task.as_str(),
                                arm.id.as_str(),
                                missing
                                    .iter()
                                    .map(|field| field.label())
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            ));
                        }
                        if let Some(reported) = &run.model_reported {
                            if !reported.starts_with(&model.model) {
                                lacking = true;
                                problem(format!(
                                    "{} in {} was answered by {reported}, and {} was asked for",
                                    run.task.as_str(),
                                    arm.id.as_str(),
                                    model.model
                                ));
                            }
                        }
                    }
                }
            }
            arms.push(ArmCapture {
                arm: arm.id.clone(),
                model: model.id.clone(),
                model_name: model.model.clone(),
                tools: arm.tools.clone(),
                runs_expected: per_arm_runs,
                runs_present: own.len() as u64,
                runs_counted: counted,
                runs_failed: failed,
                runs_without_tools: without_tools,
                proven: counted > 0 && !lacking && without_tools == 0,
                fields: fields.into_values().collect(),
            });
        }
    }
    let complete = !arms.is_empty() && arms.iter().all(|arm| arm.proven) && problems.is_empty();
    Ok(CaptureAudit {
        schema_version: AUDIT_SCHEMA_VERSION,
        manifest_sha256: digest,
        stage: rules.stage,
        runs_expected: arms.iter().map(|arm| arm.runs_expected).sum(),
        runs_present: runs.len() as u64,
        runs_counted: arms.iter().map(|arm| arm.runs_counted).sum(),
        complete,
        arms,
        problems,
    })
}
