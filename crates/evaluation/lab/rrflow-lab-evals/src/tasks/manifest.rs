//! Task comparison manifests, agent plans, preregistration and capture fields.

use super::*;

pub const TASKS_SCHEMA_VERSION: u16 = 1;

/// A pilot tests the test, so its report cannot answer the question. A confirmatory study can.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StudyStage {
    Pilot,
    #[default]
    Confirmatory,
}

/// Data a study requires of every finished run.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum CaptureField {
    Model,
    ProviderVersion,
    Tokens,
    CacheTokens,
    Cost,
    Turns,
    ToolCalls,
    Retrieval,
    Transcript,
}

impl CaptureField {
    pub fn label(self) -> &'static str {
        match self {
            Self::Model => "the model that answered",
            Self::ProviderVersion => "the provider's version",
            Self::Tokens => "tokens",
            Self::CacheTokens => "cache tokens",
            Self::Cost => "cost",
            Self::Turns => "turns",
            Self::ToolCalls => "tool calls",
            Self::Retrieval => "what retrieval returned",
            Self::Transcript => "the transcript",
        }
    }
}
pub const MAX_TASKS: usize = 1_024;
pub const MAX_REPETITIONS: u32 = 32;
pub(super) const Z95: f64 = 1.96;

/// Frozen before the first run. Changing any of it starts a different study.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Preregistration {
    pub question: String,
    /// The arm every other arm is compared with, pair by pair.
    pub baseline_arm: Id,
    /// The arm with no access to the repository. If it scores well, the tasks leak.
    pub floor_arm: Id,
    pub repetitions: u32,
    pub order_seed: u64,
    pub alpha: f64,
    /// A plan matches the historical file threshold only at or above both.
    pub min_file_recall: f64,
    pub min_file_precision: f64,
    pub max_floor_file_match_rate: f64,
    #[serde(default)]
    pub stage: StudyStage,
    /// A finished run lacking one of these stays visible and does not count.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_captures: Vec<CaptureField>,
}

/// A run cannot start without these, and the person who pays decides them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SpendLimits {
    pub max_seconds_per_run: u64,
    pub max_usd_per_run: f64,
    pub max_usd_total: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TaskProviderKind {
    ClaudeCode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaskModel {
    pub id: Id,
    pub provider: TaskProviderKind,
    pub model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaskArm {
    pub id: Id,
    pub label: String,
    pub availability: Availability,
    /// False runs the agent in an empty directory.
    pub repository_access: bool,
    /// Tool names the agent may use. Empty means none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<String>,
    /// Text placed ahead of the task, the same for every task in the arm.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preface: Option<String>,
    /// What a reader must know about this arm's results, repeated in the report.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub limits: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AgentTask {
    pub id: Id,
    pub title: String,
    pub repository: String,
    /// The revision the change started from, which holds nothing of its answer.
    pub revision: String,
    pub prompt: String,
    /// The answer key. It stays in the harness and never reaches the agent.
    pub expected_files: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub expected_records: Vec<String>,
    /// Where the task and its answer came from.
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaskComparisonManifest {
    pub schema_version: u16,
    pub test: TestIdentity,
    pub preregistration: Preregistration,
    /// Empty until the owner chooses, and nothing can run while it is.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub models: Vec<TaskModel>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spend: Option<SpendLimits>,
    pub arms: Vec<TaskArm>,
    pub tasks: Vec<AgentTask>,
}

/// The file name a path ends in, so a prompt cannot give the answer away that way either.
pub(super) fn file_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

impl TaskComparisonManifest {
    pub fn validate(&self) -> Result<()> {
        require(
            self.schema_version == TASKS_SCHEMA_VERSION,
            "unsupported task manifest schema version",
        )?;
        let plan = &self.preregistration;
        require(
            !plan.question.trim().is_empty(),
            "the preregistration must state its question",
        )?;
        require(
            (1..=MAX_REPETITIONS).contains(&plan.repetitions),
            "repetitions are outside bounds",
        )?;
        for (value, label) in [
            (plan.alpha, "alpha"),
            (plan.min_file_recall, "file recall threshold"),
            (plan.min_file_precision, "file precision threshold"),
            (plan.max_floor_file_match_rate, "floor threshold"),
        ] {
            require(
                value > 0.0 && value < 1.0,
                &format!("{label} must be between 0 and 1"),
            )?;
        }
        let mut arms = BTreeSet::new();
        for arm in &self.arms {
            let id = arm.id.as_str();
            require(arms.insert(&arm.id), &format!("{id} is listed twice"))?;
            require(!arm.label.trim().is_empty(), &format!("{id} needs a label"))?;
            if let Availability::Unavailable { reason }
            | Availability::Unimplemented { reason }
            | Availability::NotApplicable { reason } = &arm.availability
            {
                require(
                    !reason.trim().is_empty(),
                    &format!("{id} must say why it does not run"),
                )?;
            }
            require(
                arm.repository_access || arm.tools.is_empty(),
                &format!("{id} has no repository, so it cannot have tools"),
            )?;
        }
        for (role, id) in [("baseline", &plan.baseline_arm), ("floor", &plan.floor_arm)] {
            let arm = self
                .arms
                .iter()
                .find(|arm| &arm.id == id)
                .ok_or_else(|| format!("the {role} arm is not listed"))?;
            require(
                arm.availability == Availability::Available,
                &format!("the {role} arm must be able to run"),
            )?;
        }
        let floor = self.arm(plan.floor_arm.as_str())?;
        require(
            !floor.repository_access,
            "the floor arm must not see the repository",
        )?;
        require(
            self.arm(plan.baseline_arm.as_str())?.repository_access,
            "the baseline arm must see the repository",
        )?;
        let mut models = BTreeSet::new();
        for model in &self.models {
            require(models.insert(&model.id), "a model is listed twice")?;
            require(!model.model.trim().is_empty(), "a model must be named")?;
        }
        if let Some(spend) = &self.spend {
            require(
                spend.max_seconds_per_run > 0
                    && spend.max_usd_per_run > 0.0
                    && spend.max_usd_total >= spend.max_usd_per_run,
                "spend limits must be positive, and the total must cover one run",
            )?;
        }
        require(
            !self.tasks.is_empty() && self.tasks.len() <= MAX_TASKS,
            "task count is outside bounds",
        )?;
        let mut tasks = BTreeSet::new();
        for task in &self.tasks {
            let id = task.id.as_str();
            require(tasks.insert(&task.id), &format!("{id} is listed twice"))?;
            require(
                !task.prompt.trim().is_empty() && !task.expected_files.is_empty(),
                &format!("{id} needs a prompt and an answer"),
            )?;
            require(
                task.revision.len() == 40
                    && task.revision.bytes().all(|byte| byte.is_ascii_hexdigit()),
                &format!("{id} must pin a full revision"),
            )?;
            for path in &task.expected_files {
                require(
                    !task.prompt.contains(path.as_str()) && !task.prompt.contains(file_name(path)),
                    &format!("{id}'s prompt gives away {path}"),
                )?;
            }
        }
        Ok(())
    }

    pub fn arm(&self, id: &str) -> Result<&TaskArm> {
        self.arms
            .iter()
            .find(|arm| arm.id.as_str() == id)
            .ok_or_else(|| format!("no arm named {id}"))
    }

    /// Refuses unless the people paying have chosen models and a limit.
    pub fn require_runnable(&self) -> Result<&SpendLimits> {
        require(
            !self.models.is_empty(),
            "no model is chosen, so nothing can run",
        )?;
        require(
            !self.preregistration.required_captures.is_empty(),
            "the study requires no data of its runs, so nothing can run",
        )?;
        self.spend
            .as_ref()
            .ok_or_else(|| "no spend limit is set, so nothing can run".to_string())
    }

    pub fn digest(&self) -> Result<Digest> {
        Digest::try_from(sha256_hex(&canonical_json(self)?))
    }
}

/// One run in the frozen plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RunKey {
    pub task: Id,
    pub arm: Id,
    pub model: Id,
    pub repetition: u32,
}

/// Every run in a seeded shuffle, so arms spread evenly over time of day and any service change.
pub fn run_order(manifest: &TaskComparisonManifest) -> Vec<RunKey> {
    let mut keys = Vec::new();
    for task in &manifest.tasks {
        for arm in manifest
            .arms
            .iter()
            .filter(|arm| arm.availability == Availability::Available)
        {
            for model in &manifest.models {
                for repetition in 1..=manifest.preregistration.repetitions {
                    keys.push(RunKey {
                        task: task.id.clone(),
                        arm: arm.id.clone(),
                        model: model.id.clone(),
                        repetition,
                    });
                }
            }
        }
    }
    shuffle(&mut keys, manifest.preregistration.order_seed);
    keys
}

/// A shuffle that a seed fully decides.
pub fn shuffle<T>(items: &mut [T], seed: u64) {
    let mut state = seed;
    for index in (1..items.len()).rev() {
        state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        items.swap(index, (z % (index as u64 + 1)) as usize);
    }
}

/// What the agent was asked to reply with. Unknown fields are ignored, so a longer reply
/// is still graded on what it names.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct AgentPlan {
    pub files: Vec<String>,
    pub records: Vec<String>,
    pub stop_conditions: Vec<String>,
    pub open_questions: Vec<String>,
}

pub(super) const REPLY_FORMAT: &str = "Plan this change. Do not change any file. Reply with one JSON object and nothing else:\n{\"files\": [\"each repository path the change would edit or add\"], \"records\": [\"identifiers of the records that own this work, such as OBJ-11, POAM-046 or E-04\"], \"stop_conditions\": [\"what should stop the work\"], \"open_questions\": [\"what you would need answered first\"]}";

pub fn task_prompt(task: &AgentTask, arm: &TaskArm) -> String {
    let mut prompt = String::new();
    if let Some(preface) = &arm.preface {
        prompt.push_str(preface);
        prompt.push_str("\n\n");
    }
    prompt.push_str(&task.prompt);
    prompt.push_str("\n\n");
    prompt.push_str(REPLY_FORMAT);
    prompt
}

/// The last JSON object in a reply with a `files` list, whether or not it is fenced.
pub fn parse_plan(reply: &str) -> Option<AgentPlan> {
    let trimmed = reply.trim();
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) {
        if value.get("files").is_some() {
            return serde_json::from_value(value).ok();
        }
    }
    let mut found = None;
    for (start, _) in trimmed.match_indices('{') {
        let mut stream =
            serde_json::Deserializer::from_str(&trimmed[start..]).into_iter::<serde_json::Value>();
        if let Some(Ok(value)) = stream.next() {
            if value.get("files").is_some() {
                if let Ok(plan) = serde_json::from_value::<AgentPlan>(value) {
                    found = Some(plan);
                }
            }
        }
    }
    found
}

/// Agents write paths several ways, and none of them should change a score.
pub fn normalize_path(path: &str) -> String {
    let forward = path.trim().trim_matches('`').replace('\\', "/");
    let relative = forward.trim_start_matches("./").trim_start_matches('/');
    let bare = relative.split(['#', ' ']).next().unwrap_or(relative);
    // A line reference such as `reader.rs:42` names the same file.
    match bare.rsplit_once(':') {
        Some((file, line)) if !line.is_empty() && line.bytes().all(|b| b.is_ascii_digit()) => {
            file.to_string()
        }
        _ => bare.to_string(),
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PlanScore {
    pub files_named: u64,
    pub files_matched: u64,
    pub files_expected: u64,
    pub file_recall: f64,
    pub file_precision: f64,
    pub records_cited: u64,
    pub records_expected: u64,
    pub matches_file_threshold: bool,
}

pub fn score_plan(task: &AgentTask, plan: &AgentPlan, rules: &Preregistration) -> PlanScore {
    let expected: BTreeSet<String> = task
        .expected_files
        .iter()
        .map(|path| normalize_path(path))
        .collect();
    let named: BTreeSet<String> = plan.files.iter().map(|path| normalize_path(path)).collect();
    let matched = named.intersection(&expected).count() as u64;
    let file_recall = matched as f64 / expected.len().max(1) as f64;
    let file_precision = if named.is_empty() {
        0.0
    } else {
        matched as f64 / named.len() as f64
    };
    let records_cited = task
        .expected_records
        .iter()
        .filter(|record| {
            plan.records
                .iter()
                .any(|cited| cited.eq_ignore_ascii_case(record))
        })
        .count() as u64;
    PlanScore {
        files_named: named.len() as u64,
        files_matched: matched,
        files_expected: expected.len() as u64,
        file_recall,
        file_precision,
        records_cited,
        records_expected: task.expected_records.len() as u64,
        matches_file_threshold: file_recall >= rules.min_file_recall
            && file_precision >= rules.min_file_precision,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TaskRunOutcome {
    /// It replied with a plan, which was graded.
    Planned,
    /// It replied without a readable plan, which does not match the file threshold.
    NoPlan,
    /// It hit its time limit, so the comparison is incomplete.
    TimedOut,
    /// The provider failed, so the comparison is incomplete and stays visible.
    ProviderFailed,
    /// The arm's own tools did not come up, so this measured nothing about the arm.
    ToolingFailed,
}

/// What an MCP tool gave the agent and what the agent did with it, separately, because a
/// correct result that goes unused is a different failure from a search that missed.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RetrievalCapture {
    pub calls: u64,
    pub bytes_returned: u64,
    pub files_returned: u64,
    /// Files the change needed that the tool returned.
    pub needed_files_returned: u64,
    /// Of those, the ones the plan named.
    pub needed_files_used: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaskRun {
    pub schema_version: u16,
    pub manifest_sha256: Digest,
    pub task: Id,
    pub arm: Id,
    pub model: Id,
    pub repetition: u32,
    pub outcome: TaskRunOutcome,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    pub score: PlanScore,
    pub seconds: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_usd: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turns: Option<u64>,
    /// The provider's whole output, kept beside the run under this digest.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply_sha256: Option<Digest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_reported: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_read_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_write_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<BTreeMap<String, u64>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retrieval: Option<RetrievalCapture>,
}
