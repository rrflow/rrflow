//! Launching, reading and listing prompt flights.

use super::*;

/// Launch attempts before a persistent cursor race is reported as a conflict.
pub(super) const MAX_FLIGHT_COMMIT_ATTEMPTS: u8 = 3;
/// Direct-read key budget for the launch's own idempotency and schema reads.
/// Flight rows are bounded by their key prefix, so this is the ceiling on how
/// many prior versions of one flight a launch may authenticate.
pub(super) const FLIGHT_LAUNCH_READ_KEYS: u64 = 4_096;

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
            let _transaction_guard = self.admit_transaction(&commit, request_id, operation_id)?;
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
                reference: RuntimeRef::event(event_kind()?, first_event_cursor + offset),
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
