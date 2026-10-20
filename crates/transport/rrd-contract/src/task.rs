//! Named tasks: durable records of one unit of intended work that prompt
//! flights join.
//!
//! A task is an engine-owned `rrflow-task` record with a title, a goal, and a
//! status. A flight launched for an open task writes one
//! `rrflow-task-includes` edge from the task to the flight in the flight's own
//! commit, so rrflowQL walks a task to its flights and each flight to the
//! records its context used. A task groups work; it orchestrates nothing and
//! is not a routine.

use crate::{
    invalid, validate_sha256, CanonicalId, ContextReadStamp, CorrelationId, ReadEvidence, Result,
    MAX_CONTEXT_STORAGE_KEYS, MAX_ID_BYTES,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Wire version of the task record. Unknown versions are rejected.
pub const TASK_FORMAT_VERSION: u16 = 1;
/// Strict engine-owned record table holding one row per task.
pub const TASK_KIND: &str = "rrflow-task";
/// Strict engine-owned relation from a task to each prompt flight launched for
/// it. The edge identity is the flight identity, so a flight joins at most one
/// task by construction.
pub const TASK_INCLUDES_KIND: &str = "rrflow-task-includes";
pub const MAX_TASK_TITLE_BYTES: usize = 256;
pub const MAX_TASK_GOAL_BYTES: usize = 4 * 1024;
pub const MAX_TASK_OUTCOME_BYTES: usize = 4 * 1024;
pub const MAX_TASK_PAGE: u16 = 256;
pub const MAX_TASK_STORAGE_KEYS: u64 = MAX_CONTEXT_STORAGE_KEYS;

/// Lifecycle state of a task. A task opens as `open` and closes exactly once
/// as `completed` or `abandoned`; only an open task accepts new flights.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Open,
    Completed,
    Abandoned,
}

impl TaskStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Completed => "completed",
            Self::Abandoned => "abandoned",
        }
    }

    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "open" => Ok(Self::Open),
            "completed" => Ok(Self::Completed),
            "abandoned" => Ok(Self::Abandoned),
            other => invalid(format!("unknown task status {other:?}")),
        }
    }

    pub const fn is_closed(self) -> bool {
        !matches!(self, Self::Open)
    }
}

/// One governed request to open a named task.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OpenTask {
    pub scope: String,
    pub title: String,
    pub goal: String,
}

impl OpenTask {
    pub fn validate(&self) -> Result<()> {
        validate_scope(&self.scope)?;
        validate_title(&self.title)?;
        validate_goal(&self.goal)
    }
}

/// One governed request to close an open task.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CloseTask {
    pub scope: String,
    pub task_id: CanonicalId,
    /// `completed` or `abandoned`.
    pub status: TaskStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outcome: Option<String>,
}

impl CloseTask {
    pub fn validate(&self) -> Result<()> {
        validate_scope(&self.scope)?;
        if !self.status.is_closed() {
            return invalid("a task closes as completed or abandoned");
        }
        validate_outcome(self.outcome.as_deref())
    }
}

/// The durable task row. Every field is a typed property of the engine-owned
/// `rrflow-task` record; nothing is a serialized blob.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaskRecord {
    pub format_version: u16,
    pub task_id: CanonicalId,
    pub scope: String,
    pub title: String,
    pub goal: String,
    pub status: TaskStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outcome: Option<String>,
    pub opened_at_unix_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub closed_at_unix_ms: Option<u64>,
    /// The opening caller's replay coordinate. The same key with the same
    /// operation digest returns this task; a different digest is a conflict.
    pub idempotency_key: CorrelationId,
    pub operation_sha256: String,
    /// Cursor of the task's first committed version; stable creation order.
    pub task_cursor: u64,
    /// Cursor of the version this projection was read from.
    pub known_at_cursor: u64,
}

impl TaskRecord {
    pub fn validate(&self) -> Result<()> {
        if self.format_version != TASK_FORMAT_VERSION {
            return invalid(format!("task format version must be {TASK_FORMAT_VERSION}"));
        }
        validate_scope(&self.scope)?;
        validate_title(&self.title)?;
        validate_goal(&self.goal)?;
        validate_outcome(self.outcome.as_deref())?;
        if self.opened_at_unix_ms == 0 {
            return invalid("task opened_at_unix_ms must be greater than zero");
        }
        validate_sha256(&self.operation_sha256, "task operation_sha256")?;
        if self.task_cursor == 0 {
            return invalid("task cursor coordinates are invalid");
        }
        // An open task has exactly its opening version; closing writes the
        // second and last one.
        match (self.status.is_closed(), self.closed_at_unix_ms) {
            (false, None) => {
                if self.outcome.is_some() || self.known_at_cursor != self.task_cursor {
                    return invalid("an open task has no outcome and one version");
                }
            }
            (true, Some(closed_at)) => {
                if closed_at < self.opened_at_unix_ms || self.known_at_cursor <= self.task_cursor {
                    return invalid("a closed task closes after it opened");
                }
            }
            _ => return invalid("a task has a close time exactly when it is closed"),
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaskResult {
    pub task: TaskRecord,
    pub idempotent_replay: bool,
}

impl TaskResult {
    pub fn validate(&self) -> Result<()> {
        self.task.validate()
    }
}

/// Bounded, cursor-ordered page of task records at one read stamp.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ListTasks {
    pub scope: String,
    /// Return tasks whose first committed version is after this cursor.
    pub after_cursor: u64,
    pub limit: u16,
    pub max_storage_keys: u64,
}

impl ListTasks {
    pub fn validate(&self) -> Result<()> {
        validate_scope(&self.scope)?;
        if self.limit == 0 || self.limit > MAX_TASK_PAGE {
            return invalid(format!("task page limit must be in 1..={MAX_TASK_PAGE}"));
        }
        validate_storage_keys(self.max_storage_keys)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaskPage {
    pub scope: String,
    pub read: ContextReadStamp,
    pub requested_after_cursor: u64,
    pub tasks: Vec<TaskRecord>,
    /// True when more tasks follow the last returned `task_cursor`.
    pub truncated: bool,
    pub selected_versions: u64,
    pub read_evidence: ReadEvidence,
}

impl TaskPage {
    pub fn validate(&self) -> Result<()> {
        validate_scope(&self.scope)?;
        self.read.validate()?;
        self.read_evidence.validate()?;
        if self.tasks.len() > usize::from(MAX_TASK_PAGE) {
            return invalid("task page exceeds its limit");
        }
        let mut previous = self.requested_after_cursor;
        for task in &self.tasks {
            task.validate()?;
            if task.scope != self.scope {
                return invalid("task page mixes scopes");
            }
            if task.task_cursor <= previous || task.known_at_cursor > self.read.runtime_cursor {
                return invalid("task page is not in ascending task-cursor order");
            }
            previous = task.task_cursor;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReadTask {
    pub scope: String,
    pub task_id: CanonicalId,
    pub max_storage_keys: u64,
}

impl ReadTask {
    pub fn validate(&self) -> Result<()> {
        validate_scope(&self.scope)?;
        validate_storage_keys(self.max_storage_keys)
    }
}

/// One task at one read stamp. Its flights are graph neighbours: rrflowQL
/// traverses `rrflow-task-includes` from the task, so this result carries no
/// second copy of the edge list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaskReadResult {
    pub task: TaskRecord,
    pub read: ContextReadStamp,
    pub selected_versions: u64,
    pub read_evidence: ReadEvidence,
}

impl TaskReadResult {
    pub fn validate(&self) -> Result<()> {
        self.task.validate()?;
        self.read.validate()?;
        self.read_evidence.validate()?;
        if self.task.known_at_cursor > self.read.runtime_cursor {
            return invalid("task was read beyond its read stamp");
        }
        Ok(())
    }
}

fn validate_scope(scope: &str) -> Result<()> {
    if scope.is_empty() || scope.len() > MAX_ID_BYTES || scope.as_bytes().contains(&0) {
        return invalid("task scope is invalid");
    }
    Ok(())
}

fn validate_title(title: &str) -> Result<()> {
    if title.trim().is_empty()
        || title.len() > MAX_TASK_TITLE_BYTES
        || title.contains(['\0', '\n', '\r'])
    {
        return invalid(
            "task title must be one non-empty line within its byte bound and without NUL",
        );
    }
    Ok(())
}

fn validate_goal(goal: &str) -> Result<()> {
    if goal.trim().is_empty() || goal.len() > MAX_TASK_GOAL_BYTES || goal.contains('\0') {
        return invalid("task goal must be non-empty, within its byte bound, and without NUL");
    }
    Ok(())
}

fn validate_outcome(outcome: Option<&str>) -> Result<()> {
    if let Some(outcome) = outcome {
        if outcome.trim().is_empty()
            || outcome.len() > MAX_TASK_OUTCOME_BYTES
            || outcome.contains('\0')
        {
            return invalid("task outcome is empty, oversized, or contains NUL");
        }
    }
    Ok(())
}

fn validate_storage_keys(max_storage_keys: u64) -> Result<()> {
    if max_storage_keys == 0 || max_storage_keys > MAX_TASK_STORAGE_KEYS {
        return invalid(format!(
            "task max_storage_keys must be in 1..={MAX_TASK_STORAGE_KEYS}"
        ));
    }
    Ok(())
}
