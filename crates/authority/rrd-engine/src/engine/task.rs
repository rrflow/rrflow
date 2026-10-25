//! Named tasks: governed, durable records of one unit of intended work.
//!
//! A task is one strict `rrflow-task` record in the canonical runtime log: one
//! version when it opens and a second, final version when it closes. A prompt
//! flight launched for an open task writes one `rrflow-task-includes` edge
//! from the task to the flight in the flight's own commit, so rrflowQL walks a
//! task to its flights and each flight to the records its context used.
//! Every task read names the row by identity, so it costs the task's own
//! versions and never the estate's. A task orchestrates nothing.

use super::flight::{public_read_stamp, read_budget};
use super::*;
use rrd_contract::{
    CloseTask, ListTasks, OpenTask, ReadTask, TaskPage, TaskReadResult, TaskRecord, TaskResult,
    TaskStatus, PROMPT_FLIGHT_KIND, TASK_FORMAT_VERSION, TASK_INCLUDES_KIND, TASK_KIND,
};
use rrd_store::RuntimeVersionedSource;
use std::collections::BTreeSet;

/// Write attempts before a persistent cursor race is reported as a conflict.
const MAX_TASK_COMMIT_ATTEMPTS: u8 = 3;
/// Direct-read key budget for a task write's own identity read, and for the
/// task check a flight launch makes. A task row has at most two versions.
pub(super) const TASK_WRITE_READ_KEYS: u64 = 4_096;

impl RrdEngine {
    /// Opens one named task. The same session and idempotency key name the
    /// same task, so a replayed open returns the recorded row.
    #[allow(clippy::too_many_arguments)]
    pub fn open_task(
        &self,
        session_id: &CorrelationId,
        token: &CorrelationId,
        idempotency_key: &CorrelationId,
        request: &OpenTask,
        now: u64,
        request_id: &str,
        operation_id: &str,
    ) -> Result<TaskResult> {
        request
            .validate()
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        self.authorize(
            session_id,
            token,
            SecurityAction::TaskWrite,
            now,
            request_id,
            operation_id,
        )?;
        let scope = self.query_scope(&request.scope)?;
        let operation_sha256 = operation_digest(request)?;
        let task_id = task_id(&self.instance, session_id, idempotency_key)?;
        let reference = task_reference(&task_id)?;

        for _ in 0..MAX_TASK_COMMIT_ATTEMPTS {
            let read = self.storage.runtime().read_stamp(&scope)?;
            let current_schema = self.storage.runtime().schema(&scope)?;
            if current_schema.as_ref().map(|schema| schema.revision) != read.schema_revision {
                continue;
            }
            let existing =
                self.read_task_versions(&read, &request.scope, &reference, TASK_WRITE_READ_KEYS)?;
            if let Some(task) = existing.task {
                if task.operation_sha256 != operation_sha256 {
                    return Err(ServiceError::IdempotencyConflict);
                }
                return Ok(TaskResult {
                    task,
                    idempotent_replay: true,
                });
            }

            let mut mutations = Vec::with_capacity(2);
            let mut registry = current_schema
                .clone()
                .unwrap_or_else(|| RuntimeSchemaRegistry::empty(1, "install task tables"));
            if register_task_schema(&mut registry)? {
                if let Some(current) = &current_schema {
                    registry.revision = current.revision.checked_add(1).ok_or_else(|| {
                        ServiceError::Contract("runtime schema revision overflowed".into())
                    })?;
                    registry.migration = "install task tables".into();
                }
                mutations.push(RuntimeMutation::Schema { registry });
            }
            let task_cursor = read.commit_cursor + 1 + mutations.len() as u64;
            let task = TaskRecord {
                format_version: TASK_FORMAT_VERSION,
                task_id: task_id.clone(),
                scope: request.scope.clone(),
                title: request.title.clone(),
                goal: request.goal.clone(),
                status: TaskStatus::Open,
                outcome: None,
                opened_at_unix_ms: now,
                closed_at_unix_ms: None,
                idempotency_key: idempotency_key.clone(),
                operation_sha256: operation_sha256.clone(),
                task_cursor,
                known_at_cursor: task_cursor,
            };
            task.validate()
                .map_err(|error| ServiceError::Contract(error.to_string()))?;
            mutations.push(RuntimeMutation::Record {
                record: RuntimeRecord {
                    reference: reference.clone(),
                    valid_from: now,
                    valid_to: None,
                    properties: task_properties(&task),
                },
            });
            if self.commit_task(
                &scope,
                session_id,
                now,
                &read,
                mutations,
                task_cursor,
                request_id,
                operation_id,
            )? {
                return Ok(TaskResult {
                    task,
                    idempotent_replay: false,
                });
            }
        }
        Err(ServiceError::StorageConflict(format!(
            "task open lost the runtime cursor race in all {MAX_TASK_COMMIT_ATTEMPTS} attempts"
        )))
    }

    /// Closes an open task as `completed` or `abandoned`. Closing a task again
    /// with the same status and outcome replays the close; any other close of
    /// a closed task fails.
    #[allow(clippy::too_many_arguments)]
    pub fn close_task(
        &self,
        session_id: &CorrelationId,
        token: &CorrelationId,
        request: &CloseTask,
        now: u64,
        request_id: &str,
        operation_id: &str,
    ) -> Result<TaskResult> {
        request
            .validate()
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        self.authorize(
            session_id,
            token,
            SecurityAction::TaskWrite,
            now,
            request_id,
            operation_id,
        )?;
        let scope = self.query_scope(&request.scope)?;
        let reference = task_reference(&request.task_id)?;

        for _ in 0..MAX_TASK_COMMIT_ATTEMPTS {
            let read = self.storage.runtime().read_stamp(&scope)?;
            let current_schema = self.storage.runtime().schema(&scope)?;
            if current_schema.as_ref().map(|schema| schema.revision) != read.schema_revision {
                continue;
            }
            let current = self
                .read_task_versions(&read, &request.scope, &reference, TASK_WRITE_READ_KEYS)?
                .task
                .ok_or(ServiceError::TaskNotFound)?;
            if current.status.is_closed() {
                if current.status == request.status && current.outcome == request.outcome {
                    return Ok(TaskResult {
                        task: current,
                        idempotent_replay: true,
                    });
                }
                return Err(ServiceError::TaskClosed);
            }
            let closed_cursor = read.commit_cursor + 1;
            let task = TaskRecord {
                status: request.status,
                outcome: request.outcome.clone(),
                closed_at_unix_ms: Some(now.max(current.opened_at_unix_ms)),
                known_at_cursor: closed_cursor,
                ..current
            };
            task.validate()
                .map_err(|error| ServiceError::Contract(error.to_string()))?;
            // The closing version revises the same task over its whole valid
            // life; the runtime cursor, not valid time, records when the
            // close became known.
            let mutations = vec![RuntimeMutation::Record {
                record: RuntimeRecord {
                    reference: reference.clone(),
                    valid_from: task.opened_at_unix_ms,
                    valid_to: None,
                    properties: task_properties(&task),
                },
            }];
            if self.commit_task(
                &scope,
                session_id,
                now,
                &read,
                mutations,
                closed_cursor,
                request_id,
                operation_id,
            )? {
                return Ok(TaskResult {
                    task,
                    idempotent_replay: false,
                });
            }
        }
        Err(ServiceError::StorageConflict(format!(
            "task close lost the runtime cursor race in all {MAX_TASK_COMMIT_ATTEMPTS} attempts"
        )))
    }

    /// Bounded page of task records in creation order at one read stamp.
    pub fn list_tasks(
        &self,
        session_id: &CorrelationId,
        token: &CorrelationId,
        request: &ListTasks,
        now: u64,
        request_id: &str,
        operation_id: &str,
    ) -> Result<TaskPage> {
        request
            .validate()
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        self.authorize(
            session_id,
            token,
            SecurityAction::TaskRead,
            now,
            request_id,
            operation_id,
        )?;
        let scope = self.query_scope(&request.scope)?;
        let read = self.storage.runtime().read_stamp(&scope)?;
        let direct = self.storage.runtime().read_versioned(
            &read,
            &[RuntimeVersionedSource::Records {
                kind: Some(task_kind()?),
            }],
            read_budget(request.max_storage_keys)?,
        )?;
        let selected_versions = version_count(direct.changes.len())?;
        let mut rows = BTreeMap::<RuntimeRef, TaskVersions>::new();
        for change in &direct.changes {
            let RuntimeMutation::Record { record } = &change.mutation else {
                continue;
            };
            rows.entry(record.reference.clone())
                .or_default()
                .observe(change.cursor, record);
        }
        let mut tasks = rows
            .into_values()
            .filter(|versions| versions.first_cursor > request.after_cursor)
            .map(|versions| versions.task(&request.scope))
            .collect::<Result<Vec<_>>>()?;
        tasks.sort_by_key(|task| task.task_cursor);
        let limit = usize::from(request.limit);
        let truncated = tasks.len() > limit;
        tasks.truncate(limit);
        let page = TaskPage {
            scope: request.scope.clone(),
            read: public_read_stamp(&read),
            requested_after_cursor: request.after_cursor,
            tasks,
            truncated,
            selected_versions,
            read_evidence: crate::runtime::public_read_evidence(direct.evidence)?,
        };
        page.validate()
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        Ok(page)
    }

    /// One task at one read stamp. Its flights are reached by traversing
    /// `rrflow-task-includes` from the task in rrflowQL.
    pub fn read_task(
        &self,
        session_id: &CorrelationId,
        token: &CorrelationId,
        request: &ReadTask,
        now: u64,
        request_id: &str,
        operation_id: &str,
    ) -> Result<TaskReadResult> {
        request
            .validate()
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        self.authorize(
            session_id,
            token,
            SecurityAction::TaskRead,
            now,
            request_id,
            operation_id,
        )?;
        let scope = self.query_scope(&request.scope)?;
        let read = self.storage.runtime().read_stamp(&scope)?;
        let versions = self.read_task_versions(
            &read,
            &request.scope,
            &task_reference(&request.task_id)?,
            request.max_storage_keys,
        )?;
        let result = TaskReadResult {
            task: versions.task.ok_or(ServiceError::TaskNotFound)?,
            read: public_read_stamp(&read),
            selected_versions: versions.selected_versions,
            read_evidence: versions.read_evidence,
        };
        result
            .validate()
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        Ok(result)
    }

    /// The open task a flight joins at this read stamp. A flight never joins
    /// a missing or closed task.
    pub(super) fn open_task_at(
        &self,
        read: &ReadStamp,
        scope: &str,
        task_id: &CanonicalId,
    ) -> Result<RuntimeRef> {
        let reference = task_reference(task_id)?;
        let task = self
            .read_task_versions(read, scope, &reference, TASK_WRITE_READ_KEYS)?
            .task
            .ok_or(ServiceError::TaskNotFound)?;
        if task.status.is_closed() {
            return Err(ServiceError::TaskClosed);
        }
        Ok(reference)
    }

    /// The row's own identity history at one stamp: at most two versions.
    fn read_task_versions(
        &self,
        read: &ReadStamp,
        scope: &str,
        reference: &RuntimeRef,
        max_storage_keys: u64,
    ) -> Result<TaskRead> {
        let direct = self.storage.runtime().read_versioned(
            read,
            &[RuntimeVersionedSource::Identity {
                model: RuntimeLogicalModel::LifecycleRecord,
                reference: reference.clone(),
            }],
            read_budget(max_storage_keys)?,
        )?;
        let mut versions = TaskVersions::default();
        for change in &direct.changes {
            if let RuntimeMutation::Record { record } = &change.mutation {
                if record.reference == *reference {
                    versions.observe(change.cursor, record);
                }
            }
        }
        Ok(TaskRead {
            task: if versions.first_cursor == 0 {
                None
            } else {
                Some(versions.task(scope)?)
            },
            selected_versions: version_count(direct.changes.len())?,
            read_evidence: crate::runtime::public_read_evidence(direct.evidence)?,
        })
    }

    /// Commits one task write at its read stamp. `Ok(false)` reports a lost
    /// cursor race the caller retries at a fresh stamp.
    #[allow(clippy::too_many_arguments)]
    fn commit_task(
        &self,
        scope: &ScopeId,
        session_id: &CorrelationId,
        now: u64,
        read: &ReadStamp,
        mutations: Vec<RuntimeMutation>,
        record_cursor: u64,
        request_id: &str,
        operation_id: &str,
    ) -> Result<bool> {
        let record_offset = mutations.len() as u64 - 1;
        let commit = RuntimeCommit {
            scope: scope.clone(),
            at: now,
            actor: format!("session:{}", session_id.as_str()),
            expected_cursor: read.commit_cursor,
            mutations,
        };
        commit.validate().map_err(core_contract)?;
        let _transaction_guard = self.admit_transaction(&commit, request_id, operation_id)?;
        match self.storage.runtime().commit(&commit) {
            Ok(outcome) => {
                if outcome.first_cursor + record_offset != record_cursor {
                    return Err(ServiceError::Storage(format!(
                        "task record cursor {record_cursor} disagrees with commit cursor {}",
                        outcome.first_cursor + record_offset
                    )));
                }
                Ok(true)
            }
            Err(rrd_store::Error::RuntimeConflict { .. }) => Ok(false),
            Err(error) => Err(error.into()),
        }
    }
}

struct TaskRead {
    task: Option<TaskRecord>,
    selected_versions: u64,
    read_evidence: rrd_contract::ReadEvidence,
}

/// The versions of one task row seen at a read stamp. The first version fixes
/// creation order; the latest version is the current projection.
#[derive(Default)]
struct TaskVersions {
    first_cursor: u64,
    latest_cursor: u64,
    latest: Option<RuntimeRecord>,
}

impl TaskVersions {
    fn observe(&mut self, cursor: u64, record: &RuntimeRecord) {
        if self.first_cursor == 0 || cursor < self.first_cursor {
            self.first_cursor = cursor;
        }
        if cursor >= self.latest_cursor {
            self.latest_cursor = cursor;
            self.latest = Some(record.clone());
        }
    }

    fn task(self, scope: &str) -> Result<TaskRecord> {
        let latest = self
            .latest
            .ok_or_else(|| ServiceError::Storage("task row has no version".into()))?;
        public_task(&latest, scope, self.first_cursor, self.latest_cursor)
    }
}

/// Deterministic task identity: one session and one idempotency key name
/// exactly one task, so a replayed open finds its row without a receipt store.
fn task_id(
    instance: &CanonicalId,
    session_id: &CorrelationId,
    idempotency_key: &CorrelationId,
) -> Result<CanonicalId> {
    let identity = digest::sha256_hex(
        format!(
            "rrd-task-v1\0{}\0{}\0{}",
            instance.as_str(),
            session_id.as_str(),
            idempotency_key.as_str()
        )
        .as_bytes(),
    );
    CanonicalId::new(format!("task-{identity}"))
        .map_err(|error| ServiceError::Contract(error.to_string()))
}

fn task_reference(task_id: &CanonicalId) -> Result<RuntimeRef> {
    RuntimeRef::new(TASK_KIND, task_id.as_str()).map_err(core_contract)
}

fn task_kind() -> Result<RuntimeType> {
    RuntimeType::new(TASK_KIND).map_err(core_contract)
}

/// Edge identity: the flight identity. A flight therefore joins at most one
/// task, and its edge is addressable from the flight alone.
pub(super) fn task_includes_reference(flight: &RuntimeRef) -> Result<RuntimeRef> {
    RuntimeRef::new(TASK_INCLUDES_KIND, flight.id.as_str()).map_err(core_contract)
}

/// Installs or repairs the canonical task record table. The caller owns the
/// revision bump because that compare-and-swap belongs to its read stamp.
fn register_task_schema(registry: &mut RuntimeSchemaRegistry) -> Result<bool> {
    let required = |value_type: RuntimeValueType| RuntimePropertySchema::required(value_type);
    let optional = |value_type: RuntimeValueType| RuntimePropertySchema::optional(value_type);
    registry
        .define_record_table(
            task_kind()?,
            RuntimeLogicalModel::LifecycleRecord,
            RuntimeRecordSchema {
                properties: BTreeMap::from([
                    (
                        "format_version".into(),
                        required(RuntimeValueType::Unsigned),
                    ),
                    ("title".into(), required(RuntimeValueType::String)),
                    ("goal".into(), required(RuntimeValueType::String)),
                    ("status".into(), required(RuntimeValueType::String)),
                    ("outcome".into(), optional(RuntimeValueType::String)),
                    (
                        "opened_at_unix_ms".into(),
                        required(RuntimeValueType::Unsigned),
                    ),
                    (
                        "closed_at_unix_ms".into(),
                        optional(RuntimeValueType::Unsigned),
                    ),
                    ("idempotency_key".into(), required(RuntimeValueType::String)),
                    (
                        "operation_sha256".into(),
                        required(RuntimeValueType::Digest),
                    ),
                ]),
                allow_additional_properties: false,
                unique_properties: BTreeSet::new(),
            },
        )
        .map_err(core_contract)
}

/// Installs the strict task-to-flight edge table in a flight's schema under
/// construction. It declares no pair or degree constraint: the edge identity
/// is the flight identity, and a constraint would make every later commit in
/// the scope load every relation in it (POAM-034).
pub(super) fn register_task_includes_schema(registry: &mut RuntimeSchemaRegistry) -> Result<bool> {
    registry
        .define_relation_table(
            RuntimeType::new(TASK_INCLUDES_KIND).map_err(core_contract)?,
            RuntimeRelationSchema {
                from: BTreeSet::from([task_kind()?]),
                to: BTreeSet::from([RuntimeType::new(PROMPT_FLIGHT_KIND).map_err(core_contract)?]),
                properties: BTreeMap::new(),
                allow_additional_properties: false,
                unique_pair: false,
                max_outgoing: None,
                max_incoming: None,
            },
        )
        .map_err(core_contract)
}

fn task_properties(task: &TaskRecord) -> RuntimeProperties {
    let mut properties = RuntimeProperties::from([
        (
            "format_version".to_owned(),
            RuntimeValue::Unsigned(u64::from(task.format_version)),
        ),
        ("title".to_owned(), RuntimeValue::String(task.title.clone())),
        ("goal".to_owned(), RuntimeValue::String(task.goal.clone())),
        (
            "status".to_owned(),
            RuntimeValue::String(task.status.as_str().into()),
        ),
        (
            "opened_at_unix_ms".to_owned(),
            RuntimeValue::Unsigned(task.opened_at_unix_ms),
        ),
        (
            "idempotency_key".to_owned(),
            RuntimeValue::String(task.idempotency_key.as_str().into()),
        ),
        (
            "operation_sha256".to_owned(),
            RuntimeValue::Digest(task.operation_sha256.clone()),
        ),
    ]);
    if let Some(outcome) = &task.outcome {
        properties.insert("outcome".into(), RuntimeValue::String(outcome.clone()));
    }
    if let Some(closed_at) = task.closed_at_unix_ms {
        properties.insert(
            "closed_at_unix_ms".into(),
            RuntimeValue::Unsigned(closed_at),
        );
    }
    properties
}

/// Projects one stored row. A task row carries no scope property: the scope
/// is the runtime scope it was committed in, which every reader names.
fn public_task(
    record: &RuntimeRecord,
    scope: &str,
    task_cursor: u64,
    known_at_cursor: u64,
) -> Result<TaskRecord> {
    let properties = &record.properties;
    let format_version = u16::try_from(unsigned(properties, "format_version")?)
        .map_err(|_| task_storage("format_version exceeds u16"))?;
    if format_version != TASK_FORMAT_VERSION {
        return Err(task_storage(&format!(
            "unsupported task format version {format_version}"
        )));
    }
    let task = TaskRecord {
        format_version,
        task_id: CanonicalId::new(record.reference.id.as_str())
            .map_err(|error| task_storage(&error.to_string()))?,
        scope: scope.to_owned(),
        title: text(properties, "title")?,
        goal: text(properties, "goal")?,
        status: TaskStatus::parse(&text(properties, "status")?)
            .map_err(|error| task_storage(&error.to_string()))?,
        outcome: match properties.get("outcome") {
            None => None,
            Some(RuntimeValue::String(value)) => Some(value.clone()),
            Some(_) => return Err(task_storage("outcome is not text")),
        },
        opened_at_unix_ms: unsigned(properties, "opened_at_unix_ms")?,
        closed_at_unix_ms: match properties.get("closed_at_unix_ms") {
            None => None,
            Some(RuntimeValue::Unsigned(value)) => Some(*value),
            Some(_) => return Err(task_storage("closed_at_unix_ms is not unsigned")),
        },
        idempotency_key: CorrelationId::new(text(properties, "idempotency_key")?)
            .map_err(|error| task_storage(&error.to_string()))?,
        operation_sha256: match properties.get("operation_sha256") {
            Some(RuntimeValue::Digest(value)) => value.clone(),
            _ => return Err(task_storage("operation_sha256 is missing or not a digest")),
        },
        task_cursor,
        known_at_cursor,
    };
    task.validate()
        .map_err(|error| task_storage(&error.to_string()))?;
    Ok(task)
}

fn version_count(count: usize) -> Result<u64> {
    u64::try_from(count)
        .map_err(|_| ServiceError::Storage("selected task versions exceed u64".into()))
}

fn task_storage(reason: &str) -> ServiceError {
    ServiceError::Storage(format!("task row is malformed: {reason}"))
}

fn unsigned(properties: &RuntimeProperties, name: &str) -> Result<u64> {
    match properties.get(name) {
        Some(RuntimeValue::Unsigned(value)) => Ok(*value),
        _ => Err(task_storage(&format!("{name} is missing or not unsigned"))),
    }
}

fn text(properties: &RuntimeProperties, name: &str) -> Result<String> {
    match properties.get(name) {
        Some(RuntimeValue::String(value)) => Ok(value.clone()),
        _ => Err(task_storage(&format!("{name} is missing or not text"))),
    }
}
