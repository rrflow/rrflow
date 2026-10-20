//! Prompt flights: governed, durable records of one prompt's passage through
//! the runtime boundary.
//!
//! A flight is one `rrflow-prompt-flight` record, one
//! `rrflow-prompt-flight-used` edge to each canonical record its context
//! used, append-only `rrflow-prompt-flight-event` events in the canonical
//! runtime log, and, when the flight joins a named task, one
//! `rrflow-task-includes` edge from that task, so every prompt is a graph
//! neighbourhood rrflowQL traverses.
//! The engine prepares the flight at one authenticated read stamp, assembles
//! any injected context through the same `context-assemble` path every client
//! uses, and commits record, edges, and events in one cursor-CAS runtime
//! commit. A flight read names its record, edges, and events by identity, so
//! it costs the flight's own size; no client-side flight lifecycle exists.
//!
//! A flight records only externally observable runtime events. It never
//! requests, infers, or persists hidden chain-of-thought.

use super::task::{register_task_includes_schema, task_includes_reference};
use super::*;
use rrd_contract::{
    context_request_sha256, prompt_cohort_sha256, ContextEvidenceKind, ContextPacket,
    ContextReadStamp, LaunchPromptFlight, LaunchPromptFlightResult, ListPromptFlights,
    PromptFlight, PromptFlightContextMode, PromptFlightContextRequest, PromptFlightContextSummary,
    PromptFlightEvent, PromptFlightMetrics, PromptFlightPage, PromptFlightProvider,
    PromptFlightReadResult, PromptFlightRecord, PromptFlightStage, PromptFlightStatus,
    PromptFlightUse, ReadAccessPath, ReadEvidence, ReadPathEvidence, ReadPromptFlight,
    ReadStampValidationEvidence, ReadStampValidationMethod, PROMPT_FLIGHT_EVENT_KIND,
    PROMPT_FLIGHT_FORMAT_VERSION, PROMPT_FLIGHT_KIND, PROMPT_FLIGHT_USE_KIND,
};
use rrd_store::{RuntimeReadBudget, RuntimeVersionedSource};
use std::collections::BTreeSet;

/// Launch attempts before a persistent cursor race is reported as a conflict.
const MAX_FLIGHT_COMMIT_ATTEMPTS: u8 = 3;
/// Direct-read key budget for the launch's own idempotency and schema reads.
/// Flight rows are bounded by their key prefix, so this is the ceiling on how
/// many prior versions of one flight a launch may authenticate.
const FLIGHT_LAUNCH_READ_KEYS: u64 = 4_096;

impl RrdEngine {
    /// Records one prompt flight. The `observe` provider prepares the prompt
    /// packet inside the engine, so the flight completes synchronously and
    /// durably as `prepared`; nothing outside the engine runs.
    #[allow(clippy::too_many_arguments)]
    pub fn launch_prompt_flight(
        &self,
        session_id: &CorrelationId,
        token: &CorrelationId,
        idempotency_key: &CorrelationId,
        request: &LaunchPromptFlight,
        now: u64,
        request_id: &str,
        operation_id: &str,
    ) -> Result<LaunchPromptFlightResult> {
        request
            .validate()
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        let assemble = request
            .context
            .assemble_context(&request.scope, &request.prompt);
        self.enforce_recall_configuration("flight.launch", &assemble)?;
        let (_, _, authorization) = self.authorize_resource_without_data_policy(
            session_id,
            token,
            SecurityAction::FlightLaunch,
            &self.instance_resource(),
            now,
            request_id,
            operation_id,
        )?;
        let scope = self.query_scope(&request.scope)?;
        let operation_sha256 = operation_digest(request)?;
        let flight_id = flight_id(&self.instance, session_id, idempotency_key)?;
        let reference = flight_reference(&flight_id)?;
        let started = Instant::now();

        for _ in 0..MAX_FLIGHT_COMMIT_ATTEMPTS {
            let read = self.storage.runtime().read_stamp(&scope)?;
            let current_schema = self.storage.runtime().schema(&scope)?;
            if current_schema.as_ref().map(|schema| schema.revision) != read.schema_revision {
                continue;
            }

            // Exact replay: the same session and idempotency key already
            // recorded this flight. The stored operation digest decides
            // between a replay and a conflicting reuse of the key.
            let existing = self.read_flight_versions(
                &read,
                &request.scope,
                &reference,
                FLIGHT_LAUNCH_READ_KEYS,
                false,
            )?;
            if let Some(record) = existing.record {
                if record.operation_sha256 != operation_sha256 {
                    return Err(ServiceError::IdempotencyConflict);
                }
                let flight = self.read_flight_at(
                    &read,
                    &request.scope,
                    &reference,
                    FLIGHT_LAUNCH_READ_KEYS,
                )?;
                return Ok(LaunchPromptFlightResult {
                    flight,
                    idempotent_replay: true,
                });
            }
            // A flight joins only a task that is open at its own stamp.
            let task = request
                .task_id
                .as_ref()
                .map(|task_id| self.open_task_at(&read, &request.scope, task_id))
                .transpose()?;

            let mut events = Vec::with_capacity(3);
            let elapsed = |started: &Instant| {
                u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
            };
            events.push(flight_event(
                now,
                0,
                PromptFlightStage::Prompt,
                "prompt_received",
                "Prompt entered the runtime boundary",
                String::new(),
                BTreeMap::from([(
                    "prompt_bytes".into(),
                    QueryValue::Unsigned(request.prompt.len() as u64),
                )]),
            )?);

            let (context, uses) = match request.context_mode {
                PromptFlightContextMode::Fresh => {
                    events.push(flight_event(
                        now,
                        elapsed(&started),
                        PromptFlightStage::Context,
                        "fresh_baseline",
                        "Baseline context withheld",
                        "No canonical context is injected ahead of the prompt. Authoritative history is unchanged and remains readable outside the packet.".into(),
                        BTreeMap::from([("context_items".into(), QueryValue::Unsigned(0))]),
                    )?);
                    (None, Vec::new())
                }
                PromptFlightContextMode::Pruned => {
                    let packet = self.assemble_context_at(&assemble, authorization.as_ref())?;
                    if packet.read.runtime_cursor != read.commit_cursor
                        || packet.read.runtime_manifest_sha256 != read.manifest_id
                    {
                        // The runtime moved between the flight stamp and the
                        // context read; one flight never mixes two stamps.
                        continue;
                    }
                    events.push(context_assembled_event(now, elapsed(&started), &packet)?);
                    (
                        Some(context_summary(&assemble, &packet)?),
                        context_uses(&packet)?,
                    )
                }
            };
            let use_count = u64::try_from(uses.len())
                .map_err(|_| ServiceError::Contract("flight use count exceeds u64".into()))?;

            events.push(flight_event(
                now,
                elapsed(&started),
                PromptFlightStage::Outcome,
                "handoff_ready",
                "Prompt packet prepared without launching a provider",
                "The packet, its context coordinates, and every stage event are durable. A provider arm is an explicit opt-in adapter; none ran.".into(),
                BTreeMap::from([(
                    "provider".into(),
                    QueryValue::String(PromptFlightProvider::Observe.as_str().into()),
                )]),
            )?);
            for (ordinal, event) in events.iter_mut().enumerate() {
                event.ordinal = u64::try_from(ordinal).map_err(|_| {
                    ServiceError::Contract("flight event ordinal exceeds u64".into())
                })?;
            }
            let elapsed_ms = events.last().map_or(0, |event| event.elapsed_ms);
            let event_count = u64::try_from(events.len())
                .map_err(|_| ServiceError::Contract("flight event count exceeds u64".into()))?;

            let mut mutations = Vec::with_capacity(events.len() + uses.len() + 3);
            let mut registry = current_schema
                .clone()
                .unwrap_or_else(|| RuntimeSchemaRegistry::empty(1, "install prompt flight tables"));
            let used_kinds = uses
                .iter()
                .map(|(target, _)| target.kind.clone())
                .collect::<BTreeSet<_>>();
            let flight_tables_changed = register_flight_schema(&mut registry, &used_kinds)?;
            let task_tables_changed =
                task.is_some() && register_task_includes_schema(&mut registry)?;
            if flight_tables_changed || task_tables_changed {
                if let Some(current) = &current_schema {
                    registry.revision = current.revision.checked_add(1).ok_or_else(|| {
                        ServiceError::Contract("runtime schema revision overflowed".into())
                    })?;
                    registry.migration = "install prompt flight tables".into();
                }
                mutations.push(RuntimeMutation::Schema { registry });
            }
            let schema_offset = mutations.len() as u64;
            let record_cursor = read.commit_cursor + 1 + schema_offset;
            let record = PromptFlightRecord {
                format_version: PROMPT_FLIGHT_FORMAT_VERSION,
                flight_id: flight_id.clone(),
                scope: request.scope.clone(),
                cohort_sha256: prompt_cohort_sha256(&request.prompt),
                prompt: request.prompt.clone(),
                provider: request.provider,
                context_mode: request.context_mode,
                context_request: request.context.clone(),
                acceptance_marker: request.acceptance_marker.clone(),
                task_id: request.task_id.clone(),
                status: PromptFlightStatus::Prepared,
                created_at_unix_ms: now,
                idempotency_key: idempotency_key.clone(),
                operation_sha256: operation_sha256.clone(),
                read: public_read_stamp(&read),
                metrics: PromptFlightMetrics {
                    context_items: context.as_ref().map_or(0, |context| context.items),
                    context_output_bytes: context
                        .as_ref()
                        .map_or(0, |context| context.output_bytes),
                    elapsed_ms,
                },
                context,
                flight_cursor: record_cursor,
                known_at_cursor: record_cursor,
                use_count,
                event_count,
            };
            mutations.push(RuntimeMutation::Record {
                record: RuntimeRecord {
                    reference: reference.clone(),
                    valid_from: now,
                    valid_to: None,
                    properties: record_properties(&record)?,
                },
            });
            for (target, used) in &uses {
                mutations.push(RuntimeMutation::Relation {
                    relation: RuntimeRelation {
                        reference: use_reference(&flight_id, used.rank)?,
                        from: reference.clone(),
                        to: target.clone(),
                        valid_from: now,
                        valid_to: None,
                        properties: use_properties(used),
                    },
                });
            }
            for event in &events {
                mutations.push(RuntimeMutation::Event {
                    event: RuntimeEvent {
                        kind: event_kind()?,
                        subject: Some(reference.clone()),
                        properties: event_properties(event)?,
                    },
                });
            }
            // The task edge follows the events, so the event cursors stay
            // computable from the record alone.
            if let Some(task) = &task {
                mutations.push(RuntimeMutation::Relation {
                    relation: RuntimeRelation {
                        reference: task_includes_reference(&reference)?,
                        from: task.clone(),
                        to: reference.clone(),
                        valid_from: now,
                        valid_to: None,
                        properties: RuntimeProperties::new(),
                    },
                });
            }
            let commit = RuntimeCommit {
                scope: scope.clone(),
                at: now,
                actor: format!("session:{}", session_id.as_str()),
                expected_cursor: read.commit_cursor,
                mutations,
            };
            commit.validate().map_err(core_contract)?;

            // The gate closes the in-process race with a unique-index
            // installation that may be proving rows in this same table.
            let _transaction_guard = self
                .transaction_gate
                .lock()
                .map_err(|_| ServiceError::Storage("engine transaction gate is poisoned".into()))?;
            match self.storage.runtime().commit(&commit) {
                Ok(outcome) => {
                    if outcome.first_cursor + schema_offset != record_cursor {
                        return Err(ServiceError::Storage(format!(
                            "prompt flight record cursor {record_cursor} disagrees with commit cursor {}",
                            outcome.first_cursor + schema_offset
                        )));
                    }
                    let flight = PromptFlight {
                        record,
                        uses: uses.into_iter().map(|(_, used)| used).collect(),
                        events,
                    };
                    flight
                        .validate()
                        .map_err(|error| ServiceError::Contract(error.to_string()))?;
                    return Ok(LaunchPromptFlightResult {
                        flight,
                        idempotent_replay: false,
                    });
                }
                Err(rrd_store::Error::RuntimeConflict { .. }) => continue,
                Err(error) => return Err(error.into()),
            }
        }
        Err(ServiceError::StorageConflict(format!(
            "prompt flight launch lost the runtime cursor race in all {MAX_FLIGHT_COMMIT_ATTEMPTS} attempts"
        )))
    }

    /// Bounded page of flight records in creation order at one read stamp.
    pub fn list_prompt_flights(
        &self,
        session_id: &CorrelationId,
        token: &CorrelationId,
        request: &ListPromptFlights,
        now: u64,
        request_id: &str,
        operation_id: &str,
    ) -> Result<PromptFlightPage> {
        request
            .validate()
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        self.authorize(
            session_id,
            token,
            SecurityAction::FlightRead,
            now,
            request_id,
            operation_id,
        )?;
        let scope = self.query_scope(&request.scope)?;
        let read = self.storage.runtime().read_stamp(&scope)?;
        let direct = self.storage.runtime().read_versioned(
            &read,
            &[RuntimeVersionedSource::Records {
                kind: Some(record_kind()?),
            }],
            read_budget(request.max_storage_keys)?,
        )?;
        let selected_versions = version_count(direct.changes.len())?;
        let mut rows = BTreeMap::<RuntimeRef, FlightVersions>::new();
        for change in &direct.changes {
            let RuntimeMutation::Record { record } = &change.mutation else {
                continue;
            };
            rows.entry(record.reference.clone())
                .or_default()
                .observe(change.cursor, record);
        }
        let mut flights = rows
            .into_values()
            .filter(|versions| versions.first_cursor > request.after_cursor)
            .map(|versions| versions.record(&request.scope))
            .collect::<Result<Vec<_>>>()?;
        flights.sort_by_key(|record| record.flight_cursor);
        let limit = usize::from(request.limit);
        let truncated = flights.len() > limit;
        flights.truncate(limit);
        let page = PromptFlightPage {
            scope: request.scope.clone(),
            read: public_read_stamp(&read),
            requested_after_cursor: request.after_cursor,
            flights,
            truncated,
            selected_versions,
            read_evidence: crate::runtime::public_read_evidence(direct.evidence)?,
        };
        page.validate()
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        Ok(page)
    }

    /// One complete flight, record and events, at one read stamp.
    pub fn read_prompt_flight(
        &self,
        session_id: &CorrelationId,
        token: &CorrelationId,
        request: &ReadPromptFlight,
        now: u64,
        request_id: &str,
        operation_id: &str,
    ) -> Result<PromptFlightReadResult> {
        request
            .validate()
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        self.authorize(
            session_id,
            token,
            SecurityAction::FlightRead,
            now,
            request_id,
            operation_id,
        )?;
        let scope = self.query_scope(&request.scope)?;
        let read = self.storage.runtime().read_stamp(&scope)?;
        let reference = flight_reference(&request.flight_id)?;
        let versions = self.read_flight_versions(
            &read,
            &request.scope,
            &reference,
            request.max_storage_keys,
            true,
        )?;
        let record = versions.record.ok_or(ServiceError::FlightNotFound)?;
        let flight = assemble_flight(record, versions.uses, versions.events)?;
        let result = PromptFlightReadResult {
            flight,
            read: public_read_stamp(&read),
            selected_versions: versions.selected_versions,
            read_evidence: versions.read_evidence,
        };
        result
            .validate()
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        Ok(result)
    }

    fn read_flight_at(
        &self,
        read: &ReadStamp,
        scope: &str,
        reference: &RuntimeRef,
        max_storage_keys: u64,
    ) -> Result<PromptFlight> {
        let versions = self.read_flight_versions(read, scope, reference, max_storage_keys, true)?;
        let record = versions.record.ok_or(ServiceError::FlightNotFound)?;
        assemble_flight(record, versions.uses, versions.events)
    }

    /// Stamped direct reads of one flight. The first read is the row's own
    /// identity history; it fixes the commit cursor and how many edges and
    /// events that commit wrote. The second read, at the same stamp, names each
    /// edge by its deterministic identity and each event by its log cursor, so
    /// the cost is the flight's own size and never the estate's.
    fn read_flight_versions(
        &self,
        read: &ReadStamp,
        scope: &str,
        reference: &RuntimeRef,
        max_storage_keys: u64,
        with_body: bool,
    ) -> Result<FlightRead> {
        let head = self.storage.runtime().read_versioned(
            read,
            &[RuntimeVersionedSource::Identity {
                model: RuntimeLogicalModel::LifecycleRecord,
                reference: reference.clone(),
            }],
            read_budget(max_storage_keys)?,
        )?;
        let mut versions = FlightVersions::default();
        for change in &head.changes {
            if let RuntimeMutation::Record { record } = &change.mutation {
                if record.reference == *reference {
                    versions.observe(change.cursor, record);
                }
            }
        }
        let mut selected_versions = version_count(head.changes.len())?;
        let mut read_evidence = crate::runtime::public_read_evidence(head.evidence)?;
        if versions.first_cursor == 0 {
            return Ok(FlightRead {
                record: None,
                uses: Vec::new(),
                events: Vec::new(),
                selected_versions,
                read_evidence,
            });
        }
        let record = versions.record(scope)?;
        if !with_body {
            return Ok(FlightRead {
                record: Some(record),
                uses: Vec::new(),
                events: Vec::new(),
                selected_versions,
                read_evidence,
            });
        }

        let first_event_cursor = record
            .flight_cursor
            .checked_add(record.use_count)
            .and_then(|cursor| cursor.checked_add(1))
            .ok_or_else(|| flight_storage("event cursor overflows u64"))?;
        let mut sources = Vec::new();
        for rank in 1..=record.use_count {
            sources.push(RuntimeVersionedSource::Identity {
                model: RuntimeLogicalModel::GraphRelation,
                reference: use_reference(&record.flight_id, rank)?,
            });
        }
        for offset in 0..record.event_count {
            sources.push(RuntimeVersionedSource::Identity {
                model: RuntimeLogicalModel::LifecycleEvent,
                reference: RuntimeRef::new(
                    PROMPT_FLIGHT_EVENT_KIND,
                    format!("cursor:{}", first_event_cursor + offset),
                )
                .map_err(core_contract)?,
            });
        }
        let remaining = max_storage_keys.saturating_sub(read_evidence.keys_examined);
        if remaining == 0 {
            return Err(rrd_store::Error::RuntimeReadBudgetExceeded {
                limit: max_storage_keys,
                observed: read_evidence.keys_examined,
            }
            .into());
        }
        let body =
            self.storage
                .runtime()
                .read_versioned(read, &sources, read_budget(remaining)?)?;
        let mut uses = Vec::new();
        let mut events = Vec::new();
        for change in &body.changes {
            match &change.mutation {
                RuntimeMutation::Relation { relation } if relation.from == *reference => {
                    uses.push(public_use(relation)?);
                }
                RuntimeMutation::Event { event } if event.subject.as_ref() == Some(reference) => {
                    events.push(public_event(&event.properties)?);
                }
                _ => {
                    return Err(flight_storage(
                        "a flight identity read returned a version the flight did not write",
                    ))
                }
            }
        }
        uses.sort_by_key(|used| used.rank);
        events.sort_by_key(|event| event.ordinal);
        selected_versions = selected_versions
            .checked_add(version_count(body.changes.len())?)
            .ok_or_else(|| flight_storage("selected versions overflow u64"))?;
        read_evidence = merge_read_evidence(
            max_storage_keys,
            read_evidence,
            crate::runtime::public_read_evidence(body.evidence)?,
        )?;
        Ok(FlightRead {
            record: Some(record),
            uses,
            events,
            selected_versions,
            read_evidence,
        })
    }
}

struct FlightRead {
    record: Option<PromptFlightRecord>,
    uses: Vec<PromptFlightUse>,
    events: Vec<PromptFlightEvent>,
    selected_versions: u64,
    read_evidence: rrd_contract::ReadEvidence,
}

/// The versions of one flight row seen at a read stamp. The first version
/// fixes creation order; the latest version is the current projection.
#[derive(Default)]
struct FlightVersions {
    first_cursor: u64,
    latest_cursor: u64,
    latest: Option<RuntimeRecord>,
}

impl FlightVersions {
    fn observe(&mut self, cursor: u64, record: &RuntimeRecord) {
        if self.first_cursor == 0 || cursor < self.first_cursor {
            self.first_cursor = cursor;
        }
        if cursor >= self.latest_cursor {
            self.latest_cursor = cursor;
            self.latest = Some(record.clone());
        }
    }

    fn record(self, scope: &str) -> Result<PromptFlightRecord> {
        let latest = self
            .latest
            .ok_or_else(|| ServiceError::Storage("prompt flight row has no version".into()))?;
        public_record(&latest, scope, self.first_cursor, self.latest_cursor)
    }
}

fn assemble_flight(
    record: PromptFlightRecord,
    uses: Vec<PromptFlightUse>,
    events: Vec<PromptFlightEvent>,
) -> Result<PromptFlight> {
    if uses.len() as u64 != record.use_count {
        return Err(ServiceError::Storage(format!(
            "prompt flight {} has {} edges at this stamp but records {}",
            record.flight_id,
            uses.len(),
            record.use_count
        )));
    }
    if events.len() as u64 != record.event_count {
        return Err(ServiceError::Storage(format!(
            "prompt flight {} has {} events at this stamp but records {}",
            record.flight_id,
            events.len(),
            record.event_count
        )));
    }
    let flight = PromptFlight {
        record,
        uses,
        events,
    };
    flight
        .validate()
        .map_err(|error| ServiceError::Storage(error.to_string()))?;
    Ok(flight)
}

/// Deterministic flight identity: one session and one idempotency key name
/// exactly one flight, so a replayed launch finds its row without a second
/// receipt store.
fn flight_id(
    instance: &CanonicalId,
    session_id: &CorrelationId,
    idempotency_key: &CorrelationId,
) -> Result<CanonicalId> {
    let identity = digest::sha256_hex(
        format!(
            "rrd-prompt-flight-v1\0{}\0{}\0{}",
            instance.as_str(),
            session_id.as_str(),
            idempotency_key.as_str()
        )
        .as_bytes(),
    );
    CanonicalId::new(format!("flight-{identity}"))
        .map_err(|error| ServiceError::Contract(error.to_string()))
}

fn flight_reference(flight_id: &CanonicalId) -> Result<RuntimeRef> {
    RuntimeRef::new(PROMPT_FLIGHT_KIND, flight_id.as_str()).map_err(core_contract)
}

fn record_kind() -> Result<RuntimeType> {
    RuntimeType::new(PROMPT_FLIGHT_KIND).map_err(core_contract)
}

fn event_kind() -> Result<RuntimeType> {
    RuntimeType::new(PROMPT_FLIGHT_EVENT_KIND).map_err(core_contract)
}

fn use_kind() -> Result<RuntimeType> {
    RuntimeType::new(PROMPT_FLIGHT_USE_KIND).map_err(core_contract)
}

/// Edge identity: the flight identity and the item's packet rank. It is
/// canonical, unique within the flight, and computable from the record alone.
fn use_reference(flight_id: &CanonicalId, rank: u64) -> Result<RuntimeRef> {
    RuntimeRef::new(PROMPT_FLIGHT_USE_KIND, format!("{flight_id}.{rank}")).map_err(core_contract)
}

/// One edge per context item that is a canonical record, in packet order.
/// Claim items are not graph nodes and get no edge.
fn context_uses(packet: &ContextPacket) -> Result<Vec<(RuntimeRef, PromptFlightUse)>> {
    let mut uses = Vec::new();
    for (position, item) in packet.items.iter().enumerate() {
        let Some(rest) = item.identity.strip_prefix("record:") else {
            continue;
        };
        let (kind, id) = rest.split_once(':').ok_or_else(|| {
            ServiceError::Storage(format!(
                "context item identity {} is not record:<kind>:<id>",
                item.identity
            ))
        })?;
        let canonical = |value: &str| {
            CanonicalId::new(value).map_err(|error| ServiceError::Storage(error.to_string()))
        };
        let used = PromptFlightUse {
            reference: DataReference {
                kind: canonical(kind)?,
                id: canonical(id)?,
            },
            rank: u64::try_from(position + 1)
                .map_err(|_| ServiceError::Contract("context rank exceeds u64".into()))?,
            score: item.score,
            evidence: item
                .evidence
                .iter()
                .map(|evidence| evidence.kind)
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect(),
        };
        used.validate()
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        uses.push((runtime_ref(&used.reference)?, used));
    }
    Ok(uses)
}

fn use_properties(used: &PromptFlightUse) -> RuntimeProperties {
    RuntimeProperties::from([
        ("rank".to_owned(), RuntimeValue::Unsigned(used.rank)),
        (
            "score".to_owned(),
            RuntimeValue::Decimal(used.score.to_string()),
        ),
        (
            "evidence".to_owned(),
            RuntimeValue::List(
                used.evidence
                    .iter()
                    .map(|kind| RuntimeValue::String(evidence_text(*kind).into()))
                    .collect(),
            ),
        ),
    ])
}

fn public_use(relation: &RuntimeRelation) -> Result<PromptFlightUse> {
    let properties = &relation.properties;
    let score = match properties.get("score") {
        Some(RuntimeValue::Decimal(value)) => value
            .parse::<f64>()
            .map_err(|_| flight_storage("use score is not a decimal"))?,
        _ => return Err(flight_storage("use score is missing or not a decimal")),
    };
    let evidence = match properties.get("evidence") {
        Some(RuntimeValue::List(values)) => values
            .iter()
            .map(|value| match value {
                RuntimeValue::String(text) => parse_evidence(text),
                _ => Err(flight_storage("use evidence entry is not text")),
            })
            .collect::<Result<Vec<_>>>()?,
        _ => return Err(flight_storage("use evidence is missing or not a list")),
    };
    let used = PromptFlightUse {
        reference: public_change_ref(&relation.to)?,
        rank: unsigned(properties, "rank")?,
        score,
        evidence,
    };
    used.validate()
        .map_err(|error| flight_storage(&error.to_string()))?;
    Ok(used)
}

fn evidence_text(kind: ContextEvidenceKind) -> &'static str {
    match kind {
        ContextEvidenceKind::Seed => "seed",
        ContextEvidenceKind::Text => "text",
        ContextEvidenceKind::Graph => "graph",
        ContextEvidenceKind::Vector => "vector",
    }
}

fn parse_evidence(text: &str) -> Result<ContextEvidenceKind> {
    match text {
        "seed" => Ok(ContextEvidenceKind::Seed),
        "text" => Ok(ContextEvidenceKind::Text),
        "graph" => Ok(ContextEvidenceKind::Graph),
        "vector" => Ok(ContextEvidenceKind::Vector),
        other => Err(flight_storage(&format!(
            "unknown use evidence kind {other:?}"
        ))),
    }
}

/// Combines the logical I/O of the flight's two stamped reads into the one
/// evidence the response reports, under the caller's single key budget.
fn merge_read_evidence(
    key_budget: u64,
    first: ReadEvidence,
    second: ReadEvidence,
) -> Result<ReadEvidence> {
    fn sum(left: u64, right: u64) -> Result<u64> {
        left.checked_add(right)
            .ok_or_else(|| flight_storage("read evidence counter overflowed"))
    }
    let mut paths = BTreeMap::<ReadAccessPath, ReadPathEvidence>::new();
    for path in first.paths.into_iter().chain(second.paths) {
        match paths.get_mut(&path.path) {
            Some(total) => {
                total.point_reads = sum(total.point_reads, path.point_reads)?;
                total.range_scans = sum(total.range_scans, path.range_scans)?;
                total.keys_examined = sum(total.keys_examined, path.keys_examined)?;
                total.values_decoded = sum(total.values_decoded, path.values_decoded)?;
                total.decoded_bytes = sum(total.decoded_bytes, path.decoded_bytes)?;
            }
            None => {
                paths.insert(path.path, path);
            }
        }
    }
    let current = ReadStampValidationMethod::AuthenticatedCurrentHead;
    let stamp_validation =
        if first.stamp_validation.method == current && second.stamp_validation.method == current {
            first.stamp_validation
        } else {
            ReadStampValidationEvidence {
                method: ReadStampValidationMethod::Rfc9162DirectVersions,
                change_reads: sum(
                    first.stamp_validation.change_reads,
                    second.stamp_validation.change_reads,
                )?,
                proof_nodes: first
                    .stamp_validation
                    .proof_nodes
                    .saturating_add(second.stamp_validation.proof_nodes),
            }
        };
    let evidence = ReadEvidence {
        contract_version: first.contract_version,
        key_budget,
        point_reads: sum(first.point_reads, second.point_reads)?,
        range_scans: sum(first.range_scans, second.range_scans)?,
        keys_examined: sum(first.keys_examined, second.keys_examined)?,
        values_decoded: sum(first.values_decoded, second.values_decoded)?,
        decoded_bytes: sum(first.decoded_bytes, second.decoded_bytes)?,
        stamp_validation,
        paths: paths.into_values().collect(),
    };
    evidence
        .validate()
        .map_err(|error| flight_storage(&error.to_string()))?;
    Ok(evidence)
}

pub(super) fn read_budget(max_storage_keys: u64) -> Result<RuntimeReadBudget> {
    RuntimeReadBudget::new(max_storage_keys)
        .map_err(|error| ServiceError::Contract(error.to_string()))
}

fn version_count(count: usize) -> Result<u64> {
    u64::try_from(count)
        .map_err(|_| ServiceError::Storage("selected prompt flight versions exceed u64".into()))
}

pub(super) fn public_read_stamp(read: &ReadStamp) -> ContextReadStamp {
    ContextReadStamp {
        runtime_manifest_sha256: read.manifest_id.clone(),
        runtime_cursor: read.commit_cursor,
        schema_revision: read.schema_revision,
        catalogue_revision: read.catalog_revision,
    }
}

fn flight_event(
    now: u64,
    elapsed_ms: u64,
    stage: PromptFlightStage,
    kind: &str,
    label: &str,
    detail: String,
    data: BTreeMap<String, QueryValue>,
) -> Result<PromptFlightEvent> {
    Ok(PromptFlightEvent {
        ordinal: 0,
        at_unix_ms: now.saturating_add(elapsed_ms),
        elapsed_ms,
        stage,
        kind: CanonicalId::new(kind).map_err(|error| ServiceError::Contract(error.to_string()))?,
        label: label.into(),
        detail,
        data,
    })
}

/// The context stage event carries the engine-owned plan decisions, not the
/// packet items: a client renders which avenues were selected or skipped and
/// why, and fetches items through `context-assemble` under its own grant.
fn context_assembled_event(
    now: u64,
    elapsed_ms: u64,
    packet: &ContextPacket,
) -> Result<PromptFlightEvent> {
    let stages = packet
        .plan
        .stages
        .iter()
        .map(|stage| {
            let mut entry = BTreeMap::from([
                (
                    "kind".to_owned(),
                    QueryValue::String(enum_text(&stage.kind)?),
                ),
                (
                    "status".to_owned(),
                    QueryValue::String(enum_text(&stage.status)?),
                ),
                ("exact".to_owned(), QueryValue::Bool(stage.exact)),
                (
                    "reason".to_owned(),
                    QueryValue::String(stage.reason.clone()),
                ),
                (
                    "decision_sha256".to_owned(),
                    QueryValue::Digest(stage.decision_sha256.clone()),
                ),
            ]);
            if let Some(path) = &stage.access_path {
                entry.insert(
                    "access_path".to_owned(),
                    QueryValue::String(enum_text(path)?),
                );
            }
            Ok(QueryValue::Map(entry))
        })
        .collect::<Result<Vec<_>>>()?;
    let (kind, label) = if packet.items.is_empty() {
        ("context_empty", "Prompt matched no canonical context")
    } else {
        ("context_assembled", "Prompt-matched context assembled")
    };
    flight_event(
        now,
        elapsed_ms,
        PromptFlightStage::Context,
        kind,
        label,
        format!(
            "{} item(s), {} byte(s), truncated={}, selected_versions={} at runtime cursor {}.",
            packet.items.len(),
            packet.output_bytes,
            packet.truncated,
            packet.selected_versions,
            packet.read.runtime_cursor
        ),
        BTreeMap::from([
            (
                "context_items".into(),
                QueryValue::Unsigned(packet.items.len() as u64),
            ),
            (
                "output_bytes".into(),
                QueryValue::Unsigned(packet.output_bytes),
            ),
            ("truncated".into(), QueryValue::Bool(packet.truncated)),
            (
                "plan_sha256".into(),
                QueryValue::Digest(packet.plan.plan_sha256.clone()),
            ),
            (
                "packet_sha256".into(),
                QueryValue::Digest(packet.packet_sha256.clone()),
            ),
            ("stages".into(), QueryValue::List(stages)),
        ]),
    )
}

fn enum_text<T: Serialize>(value: &T) -> Result<String> {
    match serde_json::to_value(value).map_err(contract_json)? {
        serde_json::Value::String(text) => Ok(text),
        other => Err(ServiceError::Contract(format!(
            "context plan field {other} is not a text enumeration"
        ))),
    }
}

fn context_summary(
    assemble: &rrd_contract::AssembleContext,
    packet: &ContextPacket,
) -> Result<PromptFlightContextSummary> {
    Ok(PromptFlightContextSummary {
        request_sha256: context_request_sha256(assemble)
            .map_err(|error| ServiceError::Contract(error.to_string()))?,
        plan_sha256: packet.plan.plan_sha256.clone(),
        packet_sha256: packet.packet_sha256.clone(),
        items: packet.items.len() as u64,
        output_bytes: packet.output_bytes,
        truncated: packet.truncated,
        selected_versions: packet.selected_versions,
    })
}

/// Installs or repairs the canonical flight tables in a schema under
/// construction. The caller owns the revision bump because that
/// compare-and-swap belongs to its exact read stamp.
fn register_flight_schema(
    registry: &mut RuntimeSchemaRegistry,
    used_kinds: &BTreeSet<RuntimeType>,
) -> Result<bool> {
    let record_kind = record_kind()?;
    let event_kind = event_kind()?;
    let required = |value_type: RuntimeValueType| RuntimePropertySchema::required(value_type);
    let optional = |value_type: RuntimeValueType| RuntimePropertySchema::optional(value_type);
    let record_schema = RuntimeRecordSchema {
        properties: BTreeMap::from([
            (
                "format_version".into(),
                required(RuntimeValueType::Unsigned),
            ),
            ("cohort_sha256".into(), required(RuntimeValueType::Digest)),
            ("prompt".into(), required(RuntimeValueType::String)),
            ("provider".into(), required(RuntimeValueType::String)),
            ("context_mode".into(), required(RuntimeValueType::String)),
            (
                "context_valid_at_unix_ms".into(),
                required(RuntimeValueType::Unsigned),
            ),
            ("context_seeds".into(), required(RuntimeValueType::List)),
            (
                "context_max_graph_depth".into(),
                required(RuntimeValueType::Unsigned),
            ),
            (
                "context_max_items".into(),
                required(RuntimeValueType::Unsigned),
            ),
            (
                "context_max_output_bytes".into(),
                required(RuntimeValueType::Unsigned),
            ),
            (
                "context_max_storage_keys".into(),
                required(RuntimeValueType::Unsigned),
            ),
            (
                "acceptance_marker".into(),
                optional(RuntimeValueType::String),
            ),
            ("task_id".into(), optional(RuntimeValueType::String)),
            ("status".into(), required(RuntimeValueType::String)),
            (
                "created_at_unix_ms".into(),
                required(RuntimeValueType::Unsigned),
            ),
            ("idempotency_key".into(), required(RuntimeValueType::String)),
            (
                "operation_sha256".into(),
                required(RuntimeValueType::Digest),
            ),
            (
                "read_runtime_manifest_sha256".into(),
                required(RuntimeValueType::Digest),
            ),
            (
                "read_runtime_cursor".into(),
                required(RuntimeValueType::Unsigned),
            ),
            (
                "read_schema_revision".into(),
                optional(RuntimeValueType::Unsigned),
            ),
            (
                "read_catalogue_revision".into(),
                required(RuntimeValueType::Unsigned),
            ),
            (
                "context_request_sha256".into(),
                optional(RuntimeValueType::Digest),
            ),
            (
                "context_plan_sha256".into(),
                optional(RuntimeValueType::Digest),
            ),
            (
                "context_packet_sha256".into(),
                optional(RuntimeValueType::Digest),
            ),
            ("context_items".into(), required(RuntimeValueType::Unsigned)),
            (
                "context_output_bytes".into(),
                required(RuntimeValueType::Unsigned),
            ),
            ("context_truncated".into(), required(RuntimeValueType::Bool)),
            (
                "context_selected_versions".into(),
                required(RuntimeValueType::Unsigned),
            ),
            ("elapsed_ms".into(), required(RuntimeValueType::Unsigned)),
            ("use_count".into(), required(RuntimeValueType::Unsigned)),
            ("event_count".into(), required(RuntimeValueType::Unsigned)),
        ]),
        allow_additional_properties: false,
        unique_properties: BTreeSet::new(),
    };
    let event_schema = RuntimeEventSchema {
        subject_required: true,
        subject_types: BTreeSet::from([record_kind.clone()]),
        properties: BTreeMap::from([
            ("ordinal".into(), required(RuntimeValueType::Unsigned)),
            ("at_unix_ms".into(), required(RuntimeValueType::Unsigned)),
            ("elapsed_ms".into(), required(RuntimeValueType::Unsigned)),
            ("stage".into(), required(RuntimeValueType::String)),
            ("kind".into(), required(RuntimeValueType::String)),
            ("label".into(), required(RuntimeValueType::String)),
            ("detail".into(), required(RuntimeValueType::String)),
            ("data".into(), required(RuntimeValueType::Map)),
        ]),
        allow_additional_properties: false,
    };
    let record_changed = registry
        .define_record_table(
            record_kind.clone(),
            RuntimeLogicalModel::LifecycleRecord,
            record_schema,
        )
        .map_err(core_contract)?;
    let event_changed = registry
        .define_event_table(
            event_kind,
            RuntimeLogicalModel::LifecycleEvent,
            event_schema,
        )
        .map_err(core_contract)?;
    // The edge table's target set grows with every record kind a flight has
    // used, so an edge is never written to an undeclared endpoint kind.
    let use_kind = use_kind()?;
    let mut targets = registry
        .relations
        .get(&use_kind)
        .map(|schema| schema.to.clone())
        .unwrap_or_default();
    targets.extend(used_kinds.iter().cloned());
    let use_changed = if targets.is_empty() {
        false
    } else {
        registry
            .define_relation_table(
                use_kind,
                RuntimeRelationSchema {
                    from: BTreeSet::from([record_kind]),
                    to: targets,
                    properties: BTreeMap::from([
                        ("rank".into(), required(RuntimeValueType::Unsigned)),
                        ("score".into(), required(RuntimeValueType::Decimal)),
                        ("evidence".into(), required(RuntimeValueType::List)),
                    ]),
                    allow_additional_properties: false,
                    // One edge per distinct packet record is constructed here
                    // and validated by the contract. A schema pair constraint
                    // would make every later commit in the scope load every
                    // relation in it (POAM-034).
                    unique_pair: false,
                    max_outgoing: None,
                    max_incoming: None,
                },
            )
            .map_err(core_contract)?
    };
    Ok(record_changed || event_changed || use_changed)
}

fn record_properties(record: &PromptFlightRecord) -> Result<RuntimeProperties> {
    let mut properties = RuntimeProperties::from([
        (
            "format_version".to_owned(),
            RuntimeValue::Unsigned(u64::from(record.format_version)),
        ),
        (
            "cohort_sha256".to_owned(),
            RuntimeValue::Digest(record.cohort_sha256.clone()),
        ),
        (
            "prompt".to_owned(),
            RuntimeValue::String(record.prompt.clone()),
        ),
        (
            "provider".to_owned(),
            RuntimeValue::String(record.provider.as_str().into()),
        ),
        (
            "context_mode".to_owned(),
            RuntimeValue::String(record.context_mode.as_str().into()),
        ),
        (
            "context_valid_at_unix_ms".to_owned(),
            RuntimeValue::Unsigned(record.context_request.valid_at),
        ),
        (
            "context_seeds".to_owned(),
            RuntimeValue::List(
                record
                    .context_request
                    .seeds
                    .iter()
                    .map(|seed| {
                        RuntimeValue::Map(BTreeMap::from([
                            (
                                "kind".to_owned(),
                                RuntimeValue::String(seed.kind.as_str().into()),
                            ),
                            (
                                "id".to_owned(),
                                RuntimeValue::String(seed.id.as_str().into()),
                            ),
                        ]))
                    })
                    .collect(),
            ),
        ),
        (
            "context_max_graph_depth".to_owned(),
            RuntimeValue::Unsigned(u64::from(record.context_request.max_graph_depth)),
        ),
        (
            "context_max_items".to_owned(),
            RuntimeValue::Unsigned(record.context_request.max_items),
        ),
        (
            "context_max_output_bytes".to_owned(),
            RuntimeValue::Unsigned(record.context_request.max_output_bytes),
        ),
        (
            "context_max_storage_keys".to_owned(),
            RuntimeValue::Unsigned(record.context_request.max_storage_keys),
        ),
        (
            "status".to_owned(),
            RuntimeValue::String(record.status.as_str().into()),
        ),
        (
            "created_at_unix_ms".to_owned(),
            RuntimeValue::Unsigned(record.created_at_unix_ms),
        ),
        (
            "idempotency_key".to_owned(),
            RuntimeValue::String(record.idempotency_key.as_str().into()),
        ),
        (
            "operation_sha256".to_owned(),
            RuntimeValue::Digest(record.operation_sha256.clone()),
        ),
        (
            "read_runtime_manifest_sha256".to_owned(),
            RuntimeValue::Digest(record.read.runtime_manifest_sha256.clone()),
        ),
        (
            "read_runtime_cursor".to_owned(),
            RuntimeValue::Unsigned(record.read.runtime_cursor),
        ),
        (
            "read_catalogue_revision".to_owned(),
            RuntimeValue::Unsigned(record.read.catalogue_revision),
        ),
        (
            "context_items".to_owned(),
            RuntimeValue::Unsigned(record.metrics.context_items),
        ),
        (
            "context_output_bytes".to_owned(),
            RuntimeValue::Unsigned(record.metrics.context_output_bytes),
        ),
        (
            "context_truncated".to_owned(),
            RuntimeValue::Bool(
                record
                    .context
                    .as_ref()
                    .is_some_and(|context| context.truncated),
            ),
        ),
        (
            "context_selected_versions".to_owned(),
            RuntimeValue::Unsigned(
                record
                    .context
                    .as_ref()
                    .map_or(0, |context| context.selected_versions),
            ),
        ),
        (
            "elapsed_ms".to_owned(),
            RuntimeValue::Unsigned(record.metrics.elapsed_ms),
        ),
        (
            "use_count".to_owned(),
            RuntimeValue::Unsigned(record.use_count),
        ),
        (
            "event_count".to_owned(),
            RuntimeValue::Unsigned(record.event_count),
        ),
    ]);
    if let Some(marker) = &record.acceptance_marker {
        properties.insert(
            "acceptance_marker".into(),
            RuntimeValue::String(marker.clone()),
        );
    }
    if let Some(task_id) = &record.task_id {
        properties.insert(
            "task_id".into(),
            RuntimeValue::String(task_id.as_str().into()),
        );
    }
    if let Some(revision) = record.read.schema_revision {
        properties.insert(
            "read_schema_revision".into(),
            RuntimeValue::Unsigned(revision),
        );
    }
    if let Some(context) = &record.context {
        properties.insert(
            "context_request_sha256".into(),
            RuntimeValue::Digest(context.request_sha256.clone()),
        );
        properties.insert(
            "context_plan_sha256".into(),
            RuntimeValue::Digest(context.plan_sha256.clone()),
        );
        properties.insert(
            "context_packet_sha256".into(),
            RuntimeValue::Digest(context.packet_sha256.clone()),
        );
    }
    Ok(properties)
}

/// Projects one stored row. A flight row carries no scope property: the scope
/// is the runtime scope it was committed in, which every reader names.
fn public_record(
    record: &RuntimeRecord,
    scope: &str,
    flight_cursor: u64,
    known_at_cursor: u64,
) -> Result<PromptFlightRecord> {
    let properties = &record.properties;
    let format_version = u16::try_from(unsigned(properties, "format_version")?)
        .map_err(|_| flight_storage("format_version exceeds u16"))?;
    if format_version != PROMPT_FLIGHT_FORMAT_VERSION {
        return Err(flight_storage(&format!(
            "unsupported prompt flight format version {format_version}"
        )));
    }
    let flight_id = CanonicalId::new(record.reference.id.as_str())
        .map_err(|error| flight_storage(&error.to_string()))?;
    let context_mode = PromptFlightContextMode::parse(&text(properties, "context_mode")?)
        .map_err(|error| flight_storage(&error.to_string()))?;
    let context = match context_mode {
        PromptFlightContextMode::Fresh => None,
        PromptFlightContextMode::Pruned => Some(PromptFlightContextSummary {
            request_sha256: digest_text(properties, "context_request_sha256")?,
            plan_sha256: digest_text(properties, "context_plan_sha256")?,
            packet_sha256: digest_text(properties, "context_packet_sha256")?,
            items: unsigned(properties, "context_items")?,
            output_bytes: unsigned(properties, "context_output_bytes")?,
            truncated: boolean(properties, "context_truncated")?,
            selected_versions: unsigned(properties, "context_selected_versions")?,
        }),
    };
    let seeds = match properties.get("context_seeds") {
        Some(RuntimeValue::List(values)) => values
            .iter()
            .map(|value| match value {
                RuntimeValue::Map(entry) => Ok(DataReference {
                    kind: canonical_entry(entry, "kind")?,
                    id: canonical_entry(entry, "id")?,
                }),
                _ => Err(flight_storage("context seed is not a reference map")),
            })
            .collect::<Result<Vec<_>>>()?,
        _ => return Err(flight_storage("context_seeds is missing or not a list")),
    };
    Ok(PromptFlightRecord {
        format_version,
        flight_id,
        scope: scope.to_owned(),
        cohort_sha256: digest_text(properties, "cohort_sha256")?,
        prompt: text(properties, "prompt")?,
        provider: PromptFlightProvider::parse(&text(properties, "provider")?)
            .map_err(|error| flight_storage(&error.to_string()))?,
        context_mode,
        context_request: PromptFlightContextRequest {
            valid_at: unsigned(properties, "context_valid_at_unix_ms")?,
            seeds,
            max_graph_depth: u8::try_from(unsigned(properties, "context_max_graph_depth")?)
                .map_err(|_| flight_storage("context_max_graph_depth exceeds u8"))?,
            max_items: unsigned(properties, "context_max_items")?,
            max_output_bytes: unsigned(properties, "context_max_output_bytes")?,
            max_storage_keys: unsigned(properties, "context_max_storage_keys")?,
        },
        acceptance_marker: optional_text(properties, "acceptance_marker")?,
        task_id: optional_text(properties, "task_id")?
            .map(CanonicalId::new)
            .transpose()
            .map_err(|error| flight_storage(&error.to_string()))?,
        status: PromptFlightStatus::parse(&text(properties, "status")?)
            .map_err(|error| flight_storage(&error.to_string()))?,
        created_at_unix_ms: unsigned(properties, "created_at_unix_ms")?,
        idempotency_key: CorrelationId::new(text(properties, "idempotency_key")?)
            .map_err(|error| flight_storage(&error.to_string()))?,
        operation_sha256: digest_text(properties, "operation_sha256")?,
        read: ContextReadStamp {
            runtime_manifest_sha256: digest_text(properties, "read_runtime_manifest_sha256")?,
            runtime_cursor: unsigned(properties, "read_runtime_cursor")?,
            schema_revision: optional_unsigned(properties, "read_schema_revision")?,
            catalogue_revision: unsigned(properties, "read_catalogue_revision")?,
        },
        context,
        metrics: PromptFlightMetrics {
            context_items: unsigned(properties, "context_items")?,
            context_output_bytes: unsigned(properties, "context_output_bytes")?,
            elapsed_ms: unsigned(properties, "elapsed_ms")?,
        },
        flight_cursor,
        known_at_cursor,
        use_count: unsigned(properties, "use_count")?,
        event_count: unsigned(properties, "event_count")?,
    })
}

fn event_properties(event: &PromptFlightEvent) -> Result<RuntimeProperties> {
    Ok(RuntimeProperties::from([
        ("ordinal".to_owned(), RuntimeValue::Unsigned(event.ordinal)),
        (
            "at_unix_ms".to_owned(),
            RuntimeValue::Unsigned(event.at_unix_ms),
        ),
        (
            "elapsed_ms".to_owned(),
            RuntimeValue::Unsigned(event.elapsed_ms),
        ),
        (
            "stage".to_owned(),
            RuntimeValue::String(event.stage.as_str().into()),
        ),
        (
            "kind".to_owned(),
            RuntimeValue::String(event.kind.as_str().into()),
        ),
        (
            "label".to_owned(),
            RuntimeValue::String(event.label.clone()),
        ),
        (
            "detail".to_owned(),
            RuntimeValue::String(event.detail.clone()),
        ),
        (
            "data".to_owned(),
            RuntimeValue::Map(runtime_properties(&event.data)?),
        ),
    ]))
}

fn public_event(properties: &RuntimeProperties) -> Result<PromptFlightEvent> {
    let data = match properties.get("data") {
        Some(RuntimeValue::Map(values)) => public_properties(values)?,
        _ => return Err(flight_storage("event data is missing or not a map")),
    };
    Ok(PromptFlightEvent {
        ordinal: unsigned(properties, "ordinal")?,
        at_unix_ms: unsigned(properties, "at_unix_ms")?,
        elapsed_ms: unsigned(properties, "elapsed_ms")?,
        stage: PromptFlightStage::parse(&text(properties, "stage")?)
            .map_err(|error| flight_storage(&error.to_string()))?,
        kind: CanonicalId::new(text(properties, "kind")?)
            .map_err(|error| flight_storage(&error.to_string()))?,
        label: text(properties, "label")?,
        detail: text(properties, "detail")?,
        data,
    })
}

fn flight_storage(reason: &str) -> ServiceError {
    ServiceError::Storage(format!("prompt flight row is malformed: {reason}"))
}

fn unsigned(properties: &RuntimeProperties, name: &str) -> Result<u64> {
    match properties.get(name) {
        Some(RuntimeValue::Unsigned(value)) => Ok(*value),
        _ => Err(flight_storage(&format!(
            "{name} is missing or not unsigned"
        ))),
    }
}

fn optional_unsigned(properties: &RuntimeProperties, name: &str) -> Result<Option<u64>> {
    match properties.get(name) {
        None => Ok(None),
        Some(RuntimeValue::Unsigned(value)) => Ok(Some(*value)),
        _ => Err(flight_storage(&format!("{name} is not unsigned"))),
    }
}

fn boolean(properties: &RuntimeProperties, name: &str) -> Result<bool> {
    match properties.get(name) {
        Some(RuntimeValue::Bool(value)) => Ok(*value),
        _ => Err(flight_storage(&format!("{name} is missing or not boolean"))),
    }
}

fn text(properties: &RuntimeProperties, name: &str) -> Result<String> {
    match properties.get(name) {
        Some(RuntimeValue::String(value)) => Ok(value.clone()),
        _ => Err(flight_storage(&format!("{name} is missing or not text"))),
    }
}

fn optional_text(properties: &RuntimeProperties, name: &str) -> Result<Option<String>> {
    match properties.get(name) {
        None => Ok(None),
        Some(RuntimeValue::String(value)) => Ok(Some(value.clone())),
        _ => Err(flight_storage(&format!("{name} is not text"))),
    }
}

fn digest_text(properties: &RuntimeProperties, name: &str) -> Result<String> {
    match properties.get(name) {
        Some(RuntimeValue::Digest(value)) => Ok(value.clone()),
        _ => Err(flight_storage(&format!(
            "{name} is missing or not a digest"
        ))),
    }
}

fn canonical_entry(entry: &BTreeMap<String, RuntimeValue>, name: &str) -> Result<CanonicalId> {
    match entry.get(name) {
        Some(RuntimeValue::String(value)) => {
            CanonicalId::new(value.as_str()).map_err(|error| flight_storage(&error.to_string()))
        }
        _ => Err(flight_storage(&format!(
            "context seed {name} is missing or not text"
        ))),
    }
}
