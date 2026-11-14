//! Study requirements, resolution and evaluation.

use super::*;

pub const STUDY_SCHEMA_VERSION: u16 = 1;
pub const AUDIT_SCHEMA_VERSION: u16 = 1;
pub(super) const MAX_REQUIREMENTS: usize = 256;
pub(super) const MAX_AUDIT_PROBLEMS: usize = 256;
/// exact_mcnemar needs at most this many one-sided pairs to pass any alpha a study can state.
pub(super) const MAX_DISCORDANT_PAIRS: u64 = 1_024;

/// Names that move to a newer model without anyone choosing it.
pub(super) const FLOATING_MODEL_NAMES: [&str; 5] = ["sonnet", "opus", "haiku", "fable", "latest"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CatalogReference {
    pub path: String,
    /// The catalog this study was configured against. A changed catalog means choosing again.
    pub sha256: Digest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "select", rename_all = "snake_case", deny_unknown_fields)]
pub enum TaskSelection {
    All,
    Seeded { count: u32, seed: u64 },
    Listed { ids: Vec<Id> },
}

/// Thresholds a study may set for itself. Anything left out keeps the catalog's value.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AnalysisChoices {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alpha: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_file_recall: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_file_precision: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_floor_file_match_rate: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order_seed: Option<u64>,
}

/// What a person or a screen edits. It names a credential and never holds one.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StudyConfig {
    pub schema_version: u16,
    pub id: Id,
    pub title: String,
    pub stage: StudyStage,
    pub catalog: CatalogReference,
    pub tasks: TaskSelection,
    pub arms: Vec<Id>,
    pub baseline_arm: Id,
    pub floor_arm: Id,
    pub models: Vec<TaskModel>,
    pub repetitions: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spend: Option<SpendLimits>,
    /// The environment variable holding the provider's key.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credential_env: Option<String>,
    #[serde(default)]
    pub analysis: AnalysisChoices,
    /// The audit reporting capture coverage; it does not authenticate run provenance.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capture_proof_sha256: Option<Digest>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RequirementLevel {
    Required,
    Advised,
}

/// The closed list of checks. A requirement cannot name a check that does not exist here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "check", rename_all = "snake_case", deny_unknown_fields)]
pub enum RequirementCheck {
    FrozenDesign,
    BaselineArm,
    FloorArm,
    ComparedArm,
    MechanicalGrading,
    PinnedModels,
    DeclaredTools,
    SpendCoversPlan,
    CredentialByName,
    CanReachSignificance,
    CaptureFields { fields: Vec<CaptureField> },
    CaptureProven,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Requirement {
    pub id: Id,
    pub title: String,
    pub statement: String,
    pub why: String,
    /// Entries in the lab's source list.
    pub sources: Vec<Id>,
    /// Owning records in this repository, such as an objective or a gate.
    pub records: Vec<String>,
    pub pilot: RequirementLevel,
    pub confirmatory: RequirementLevel,
    pub check: RequirementCheck,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StudyRequirements {
    pub schema_version: u16,
    pub family: Id,
    pub requirements: Vec<Requirement>,
}

impl StudyRequirements {
    /// Every requirement has to say where it comes from, and only from something that exists.
    pub fn validate(&self, sources: &SourceList) -> Result<()> {
        require(
            self.schema_version == STUDY_SCHEMA_VERSION,
            "unsupported study requirements schema version",
        )?;
        require(
            !self.requirements.is_empty() && self.requirements.len() <= MAX_REQUIREMENTS,
            "requirement count is outside bounds",
        )?;
        let known: BTreeSet<&Id> = sources.sources.iter().map(|source| &source.id).collect();
        let mut seen = BTreeSet::new();
        for requirement in &self.requirements {
            let name = requirement.id.as_str();
            require(
                seen.insert(&requirement.id),
                &format!("{name} is listed twice"),
            )?;
            require(
                !requirement.statement.trim().is_empty() && !requirement.why.trim().is_empty(),
                &format!("{name} must state what it asks and why"),
            )?;
            require(
                !requirement.sources.is_empty() || !requirement.records.is_empty(),
                &format!("{name} must cite a source or an owning record"),
            )?;
            for source in &requirement.sources {
                require(
                    known.contains(source),
                    &format!(
                        "{name} cites {}, which is not in the source list",
                        source.as_str()
                    ),
                )?;
            }
        }
        Ok(())
    }

    fn captures(&self) -> Vec<CaptureField> {
        let fields: BTreeSet<CaptureField> = self
            .requirements
            .iter()
            .filter_map(|requirement| match &requirement.check {
                RequirementCheck::CaptureFields { fields } => Some(fields.iter().copied()),
                _ => None,
            })
            .flatten()
            .collect();
        fields.into_iter().collect()
    }

    pub fn digest(&self) -> Result<Digest> {
        Digest::try_from(sha256_hex(&canonical_json(self)?))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RequirementStatus {
    Met,
    NotMet,
    /// Not met, and this stage only advises it, so the study can go ahead and says so.
    AdvisedNotMet,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RequirementOutcome {
    pub id: Id,
    pub title: String,
    pub status: RequirementStatus,
    pub detail: String,
    pub sources: Vec<Id>,
    pub records: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StudyCheck {
    pub schema_version: u16,
    pub study: Id,
    pub stage: StudyStage,
    pub catalog_sha256: Digest,
    pub requirements_sha256: Digest,
    pub tasks: u64,
    pub arms: u64,
    pub models: u64,
    pub runs: u64,
    /// Every run at its limit. Absent until a limit is set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worst_case_usd: Option<f64>,
    /// One-sided pairs exact McNemar needs at this study's alpha.
    pub discordant_pairs_needed: u64,
    pub can_reach_significance: bool,
    pub ready: bool,
    pub outcomes: Vec<RequirementOutcome>,
}

/// One-sided pairs needed before exact McNemar can pass alpha, whatever the arms do.
pub fn discordant_pairs_needed(alpha: f64) -> u64 {
    (1..=MAX_DISCORDANT_PAIRS)
        .find(|pairs| exact_mcnemar(*pairs, 0) < alpha)
        .unwrap_or(MAX_DISCORDANT_PAIRS)
}

pub(super) fn select_tasks(
    config: &StudyConfig,
    catalog: &TaskComparisonManifest,
) -> Result<Vec<usize>> {
    let total = catalog.tasks.len();
    Ok(match &config.tasks {
        TaskSelection::All => (0..total).collect(),
        TaskSelection::Seeded { count, seed } => {
            require(
                *count > 0 && *count as usize <= total,
                &format!("the catalog has {total} tasks, and {count} were asked for"),
            )?;
            let mut order: Vec<usize> = (0..total).collect();
            shuffle(&mut order, *seed);
            order.truncate(*count as usize);
            order.sort_unstable();
            order
        }
        TaskSelection::Listed { ids } => {
            require(!ids.is_empty(), "the study lists no tasks")?;
            let mut chosen = Vec::with_capacity(ids.len());
            for id in ids {
                let position = catalog
                    .tasks
                    .iter()
                    .position(|task| &task.id == id)
                    .ok_or_else(|| format!("the catalog has no task named {}", id.as_str()))?;
                chosen.push(position);
            }
            chosen.sort_unstable();
            chosen.dedup();
            chosen
        }
    })
}

/// Applies the choices to the catalog. An arm left out stays in the report, marked as left
/// out, so a reader can see what was not measured. This does not judge the result.
pub fn resolve_study(
    config: &StudyConfig,
    catalog: &TaskComparisonManifest,
    requirements: &StudyRequirements,
) -> Result<TaskComparisonManifest> {
    require(
        config.schema_version == STUDY_SCHEMA_VERSION,
        "unsupported study schema version",
    )?;
    let digest = catalog.digest()?;
    require(
        config.catalog.sha256 == digest,
        &format!(
            "the catalog changed since this study was configured: it is now {}",
            digest.as_str()
        ),
    )?;
    let chosen: BTreeSet<&Id> = config.arms.iter().collect();
    for arm in &config.arms {
        let found = catalog
            .arms
            .iter()
            .find(|candidate| &candidate.id == arm)
            .ok_or_else(|| format!("the catalog has no arm named {}", arm.as_str()))?;
        if let Availability::Unavailable { reason }
        | Availability::Unimplemented { reason }
        | Availability::NotApplicable { reason } = &found.availability
        {
            return Err(format!("{} cannot run: {reason}", arm.as_str()));
        }
    }
    let arms: Vec<TaskArm> = catalog
        .arms
        .iter()
        .cloned()
        .map(|mut arm| {
            if arm.availability == Availability::Available && !chosen.contains(&arm.id) {
                arm.availability = Availability::NotApplicable {
                    reason: "Left out of this study.".to_string(),
                };
            }
            arm
        })
        .collect();
    let tasks = select_tasks(config, catalog)?
        .into_iter()
        .map(|position| catalog.tasks[position].clone())
        .collect();
    let mut preregistration = catalog.preregistration.clone();
    preregistration.stage = config.stage;
    preregistration.baseline_arm = config.baseline_arm.clone();
    preregistration.floor_arm = config.floor_arm.clone();
    preregistration.repetitions = config.repetitions;
    preregistration.required_captures = requirements.captures();
    let choices = &config.analysis;
    preregistration.alpha = choices.alpha.unwrap_or(preregistration.alpha);
    preregistration.min_file_recall = choices
        .min_file_recall
        .unwrap_or(preregistration.min_file_recall);
    preregistration.min_file_precision = choices
        .min_file_precision
        .unwrap_or(preregistration.min_file_precision);
    preregistration.max_floor_file_match_rate = choices
        .max_floor_file_match_rate
        .unwrap_or(preregistration.max_floor_file_match_rate);
    preregistration.order_seed = choices.order_seed.unwrap_or(preregistration.order_seed);
    let mut test = catalog.test.clone();
    test.id = config.id.clone();
    test.title = config.title.clone();
    Ok(TaskComparisonManifest {
        schema_version: catalog.schema_version,
        test,
        preregistration,
        models: config.models.clone(),
        spend: config.spend.clone(),
        arms,
        tasks,
    })
}

pub(super) fn is_floating_name(model: &str) -> bool {
    let name = model.trim().to_ascii_lowercase();
    FLOATING_MODEL_NAMES.contains(&name.as_str())
        || name.ends_with("-latest")
        || !name.bytes().any(|byte| byte.is_ascii_digit())
}

pub(super) fn is_variable_name(name: &str) -> bool {
    let mut bytes = name.bytes();
    bytes.next().is_some_and(|first| first.is_ascii_uppercase())
        && bytes.all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
        && name.len() <= 64
}

pub(super) fn arm_uses_mcp(arm: &TaskArm) -> bool {
    arm.tools.iter().any(|tool| tool.starts_with("mcp__"))
}

/// Why a proof does not cover this study, or nothing if it does.
pub(super) fn proof_gap(manifest: &TaskComparisonManifest, proof: &CaptureAudit) -> Option<String> {
    for arm in manifest
        .arms
        .iter()
        .filter(|arm| arm.availability == Availability::Available)
    {
        for model in &manifest.models {
            let proven = proof.arms.iter().any(|entry| {
                entry.arm == arm.id
                    && entry.model_name == model.model
                    && entry.tools == arm.tools
                    && entry.proven
            });
            if !proven {
                return Some(format!(
                    "no audited run of {} with {} and these tools",
                    arm.id.as_str(),
                    model.model
                ));
            }
        }
    }
    None
}

pub(super) fn evaluate(
    check: &RequirementCheck,
    config: &StudyConfig,
    manifest: &TaskComparisonManifest,
    proof: Option<&CaptureAudit>,
) -> std::result::Result<String, String> {
    let rules = &manifest.preregistration;
    let usable = |id: &Id| {
        manifest
            .arms
            .iter()
            .find(|arm| &arm.id == id && arm.availability == Availability::Available)
    };
    match check {
        RequirementCheck::FrozenDesign => manifest
            .validate()
            .map(|_| format!("Order seed {}, alpha {}.", rules.order_seed, rules.alpha)),
        RequirementCheck::BaselineArm => {
            let arm = usable(&rules.baseline_arm)
                .ok_or_else(|| format!("{} is not in this study.", rules.baseline_arm.as_str()))?;
            if !arm.repository_access {
                return Err(format!("{} cannot see the repository.", arm.id.as_str()));
            }
            if arm_uses_mcp(arm) {
                return Err(format!(
                    "{} uses an MCP tool, so it holds what is being tested.",
                    arm.id.as_str()
                ));
            }
            Ok(format!("{} with {}.", arm.id.as_str(), arm.tools.join(", ")))
        }
        RequirementCheck::FloorArm => {
            let arm = usable(&rules.floor_arm)
                .ok_or_else(|| format!("{} is not in this study.", rules.floor_arm.as_str()))?;
            if arm.repository_access || !arm.tools.is_empty() {
                return Err(format!("{} can reach the repository.", arm.id.as_str()));
            }
            Ok(format!(
                "{}, which may solve at most {} of tasks.",
                arm.id.as_str(),
                rules.max_floor_file_match_rate
            ))
        }
        RequirementCheck::ComparedArm => {
            let compared: Vec<&str> = manifest
                .arms
                .iter()
                .filter(|arm| {
                    arm.availability == Availability::Available
                        && arm.id != rules.baseline_arm
                        && arm.id != rules.floor_arm
                })
                .map(|arm| arm.id.as_str())
                .collect();
            if compared.is_empty() {
                return Err("Nothing is compared with the baseline.".to_string());
            }
            Ok(format!("Compares {} with the baseline.", compared.join(", ")))
        }
        RequirementCheck::MechanicalGrading => {
            if manifest
                .tasks
                .iter()
                .any(|task| task.expected_files.is_empty())
            {
                return Err("A task has no answer to grade against.".to_string());
            }
            Ok(format!(
                "Solved at file recall {} and precision {}.",
                rules.min_file_recall, rules.min_file_precision
            ))
        }
        RequirementCheck::PinnedModels => {
            if manifest.models.is_empty() {
                return Err("No model is chosen.".to_string());
            }
            if let Some(model) = manifest
                .models
                .iter()
                .find(|model| is_floating_name(&model.model))
            {
                return Err(format!(
                    "{} can change between runs, so name an exact version.",
                    model.model
                ));
            }
            let names: Vec<&str> = manifest.models.iter().map(|model| model.model.as_str()).collect();
            Ok(names.join(", "))
        }
        RequirementCheck::DeclaredTools => {
            if let Some(arm) = manifest.arms.iter().find(|arm| {
                arm.availability == Availability::Available
                    && arm.repository_access
                    && arm.tools.is_empty()
            }) {
                return Err(format!("{} lists no tools.", arm.id.as_str()));
            }
            Ok("Every arm lists its tools.".to_string())
        }
        RequirementCheck::SpendCoversPlan => {
            let spend = manifest
                .spend
                .as_ref()
                .ok_or_else(|| "No spend limit is set.".to_string())?;
            let worst = planned_runs(manifest) as f64 * spend.max_usd_per_run;
            if worst > spend.max_usd_total {
                return Err(format!(
                    "{} runs at ${} each could cost ${worst}, above the ${} total.",
                    planned_runs(manifest),
                    spend.max_usd_per_run,
                    spend.max_usd_total
                ));
            }
            Ok(format!(
                "At most ${worst} across {} runs, within the ${} total.",
                planned_runs(manifest),
                spend.max_usd_total
            ))
        }
        RequirementCheck::CredentialByName => match config.credential_env.as_deref() {
            None => Err("No credential variable is named.".to_string()),
            Some(name) if is_variable_name(name) => Ok(format!("Read from {name}.")),
            Some(_) => Err(
                "That is not a variable name. Name the variable, and keep the key out of this file."
                    .to_string(),
            ),
        },
        RequirementCheck::CanReachSignificance => {
            if rules.repetitions > 1 {
                return Err("Repeated runs of a task are correlated; a clustered significance test is not implemented.".to_string());
            }
            let needed = discordant_pairs_needed(rules.alpha);
            let tasks = manifest.tasks.len() as u64;
            if tasks < needed {
                return Err(format!(
                    "{tasks} tasks can give at most {tasks} one-sided pairs, and exact McNemar \
                     needs {needed} at alpha {}. This design cannot show a difference.",
                    rules.alpha
                ));
            }
            Ok(format!(
                "{tasks} tasks, and {needed} one-sided pairs are the least that could reach alpha {}.",
                rules.alpha
            ))
        }
        RequirementCheck::CaptureFields { fields } => {
            if fields.is_empty() {
                return Err("No data is required of a run.".to_string());
            }
            Ok(format!("{} fields are required of every finished run.", fields.len()))
        }
        RequirementCheck::CaptureProven => {
            let proof = proof.ok_or_else(|| "No capture audit is supplied.".to_string())?;
            if let Some(gap) = proof_gap(manifest, proof) {
                return Err(format!("The audit does not cover this study: {gap}."));
            }
            Ok("The supplied audit reports capture coverage for every arm; run provenance is not authenticated.".to_string())
        }
    }
}
