//! Building and transitioning quantization artifacts and their catalogue records.

use super::*;

/// Publishes immutable quantized bytes in `ready` state without changing the
/// planner. Activation is a separate authenticated lifecycle operation.
/// The idempotency key of a lifecycle operation is already recorded at the
/// read its commit would have been conditional on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuantizationOperationRecorded {
    /// The same request: the caller replays the recorded event.
    Replay,
    /// Another request under the same key.
    Conflict,
}

impl std::fmt::Display for QuantizationOperationRecorded {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Replay => "quantization operation is already recorded",
            Self::Conflict => "quantization operation key is recorded for another request",
        })
    }
}

impl std::error::Error for QuantizationOperationRecorded {}

pub(super) fn check_operation(
    catalogue: &QuantizationArtifactCatalogue,
    operation: Option<&QuantizationOperationReceipt>,
) -> Result<(), QuantizationOperationRecorded> {
    match operation.and_then(|operation| {
        catalogue
            .operation(&operation.idempotency_key)
            .map(|outcome| outcome.operation_sha256 == operation.operation_sha256)
    }) {
        None => Ok(()),
        Some(true) => Err(QuantizationOperationRecorded::Replay),
        Some(false) => Err(QuantizationOperationRecorded::Conflict),
    }
}

pub fn build_traced_quantization_artifact<D>(
    data: &D,
    collection_id: ProjectionId,
    vector_name: ProjectionId,
    artifact: VectorArtifact,
    actor: &str,
    at: Millis,
    operation: Option<QuantizationOperationReceipt>,
) -> Result<QuantizationArtifactPublication, Box<dyn std::error::Error>>
where
    D: DataRuntimeAccess,
{
    if !matches!(
        artifact.kind(),
        rrd_vector::VectorArtifactKind::ScalarQuantized
            | rrd_vector::VectorArtifactKind::ProductQuantized
            | rrd_vector::VectorArtifactKind::BinaryQuantized
            | rrd_vector::VectorArtifactKind::TurboQuant
    ) {
        return Err("quantization lifecycle cannot build a non-quantized artifact".into());
    }
    let descriptor = artifact.descriptor();
    descriptor.validate()?;
    let scope = descriptor.scope().clone();
    let stamp = descriptor.stamp().clone();
    // Both identities exist before any bytes are staged, so one their types refuse stores nothing.
    let record_reference = QuantizationArtifactEntry::record_reference(&descriptor)?;
    let object_id = RuntimeId::new(format!("{}.{}.quantized-bytes", stamp.id, stamp.generation))?;
    let generation_bytes = stamp.generation.to_be_bytes();
    let identity = TraceIdentity::derive(&[
        scope.as_str().as_bytes(),
        stamp.id.as_str().as_bytes(),
        &generation_bytes,
        b"build",
    ])?;
    let trace_read = data.engine().runtime().read_stamp(&scope)?;
    let links = vec![TraceLink::Read { stamp: trace_read }];
    let span = DurableTraceSpan::start(
        data.engine(),
        scope.clone(),
        actor,
        identity,
        None,
        TraceBoundary::Vector,
        "vector.quantization.build",
        at,
        TraceDataClass::Control,
        links,
        quantization_trace_attributes(&stamp, artifact.kind(), "build"),
    )?;
    let result = (|| -> Result<QuantizationArtifactPublication, Box<dyn std::error::Error>> {
        let mut staged: Option<(rrd_core::ObjectReference, QuantizationArtifactEntry)> = None;
        // A commit that loses its cursor to another commit in the scope
        // retries from a fresh catalogue read, which re-checks the operation
        // and the generation; the bytes are staged once.
        for _ in 0..PUBLICATION_RETRIES {
            let (read, catalogue) = quantization_artifact_catalogue_at_read(data.engine(), &scope)?;
            check_operation(&catalogue, operation.as_ref())?;
            let key = (stamp.id.clone(), stamp.generation);
            if catalogue.artifacts.contains_key(&key) {
                return Err("quantization artifact generation is already built".into());
            }
            let event = catalogue.next_event(
                stamp.id.clone(),
                stamp.generation,
                QuantizationLifecycleAction::Build,
                at,
                operation.clone(),
            )?;
            let (object, entry) = match &staged {
                Some(staged) => staged.clone(),
                None => {
                    let object = data.stage_object(
                        object_id.as_str(),
                        Some(record_reference.clone()),
                        artifact.kind().media_type(),
                        artifact.as_bytes(),
                    )?;
                    let entry = QuantizationArtifactEntry::new(
                        collection_id.clone(),
                        vector_name.clone(),
                        artifact.kind(),
                        descriptor.clone(),
                        object.clone(),
                        at,
                    )?;
                    staged.insert((object, entry)).clone()
                }
            };
            let records = vec![
                quantization_artifact_record(&entry)?,
                quantization_lifecycle_record(&event)?,
            ];
            if let Some(commit) =
                commit_quantization_records(data, read, &scope, actor, at, records, Some(object))?
            {
                return Ok(QuantizationArtifactPublication {
                    lifecycle_revision: event.revision,
                    entry,
                    state: QuantizationArtifactState::Ready,
                    commit,
                });
            }
        }
        Err(publication_conflicts())
    })();
    match result {
        Ok(publication) => {
            span.finish(
                data.engine(),
                TraceOutcome::Ok,
                vec![TraceLink::Projection { stamp }],
                quantization_finish_attributes(
                    publication.lifecycle_revision,
                    publication.state,
                    &publication.entry,
                    &publication.commit,
                ),
            )?;
            Ok(publication)
        }
        Err(error) => finish_publication_error(data.engine(), span, "quantization_build", error),
    }
}

/// Activates or retires one built generation through an append-only event.
/// Activation verifies the immutable object before it can become planner
/// visible, so corruption fails closed at the control-plane boundary.
#[allow(clippy::too_many_arguments)]
pub fn transition_traced_quantization_artifact<D>(
    data: &D,
    scope: &ScopeId,
    projection_id: &ProjectionId,
    generation: u64,
    action: QuantizationLifecycleAction,
    actor: &str,
    at: Millis,
    operation: Option<QuantizationOperationReceipt>,
) -> Result<QuantizationArtifactTransition, Box<dyn std::error::Error>>
where
    D: DataRuntimeAccess,
{
    if action == QuantizationLifecycleAction::Build {
        return Err("build requires immutable artifact bytes".into());
    }
    let action_name = match action {
        QuantizationLifecycleAction::Activate => "activate",
        QuantizationLifecycleAction::Retire => "retire",
        QuantizationLifecycleAction::Build => unreachable!("build was rejected above"),
    };
    let generation_bytes = generation.to_be_bytes();
    let identity = TraceIdentity::derive(&[
        scope.as_str().as_bytes(),
        projection_id.as_str().as_bytes(),
        &generation_bytes,
        action_name.as_bytes(),
    ])?;
    let trace_read = data.engine().runtime().read_stamp(scope)?;
    let links = vec![TraceLink::Read { stamp: trace_read }];
    let span = DurableTraceSpan::start(
        data.engine(),
        scope.clone(),
        actor,
        identity,
        None,
        TraceBoundary::Vector,
        format!("vector.quantization.{action_name}"),
        at,
        TraceDataClass::Control,
        links,
        RuntimeProperties::from([
            (
                "projection_id".into(),
                RuntimeValue::String(projection_id.to_string()),
            ),
            ("generation".into(), RuntimeValue::Unsigned(generation)),
            (
                "lifecycle_action".into(),
                RuntimeValue::String(action_name.into()),
            ),
        ]),
    )?;
    let result = (|| -> Result<QuantizationArtifactTransition, Box<dyn std::error::Error>> {
        let mut verified = false;
        // As for a build, a commit that loses its cursor retries from a fresh
        // catalogue read, which re-checks the operation and the transition.
        for _ in 0..PUBLICATION_RETRIES {
            let (read, catalogue) = quantization_artifact_catalogue_at_read(data.engine(), scope)?;
            check_operation(&catalogue, operation.as_ref())?;
            let key = (projection_id.clone(), generation);
            let entry = catalogue
                .artifacts
                .get(&key)
                .ok_or("quantization lifecycle target is absent")?
                .entry
                .clone();
            if action == QuantizationLifecycleAction::Activate && !verified {
                let bytes = data.objects().get(&entry.object)?;
                entry.decode_artifact(&bytes)?;
                verified = true;
            }
            let event = catalogue.next_event(
                projection_id.clone(),
                generation,
                action,
                at,
                operation.clone(),
            )?;
            let mut next = catalogue.clone();
            next.apply(&event)?;
            let state = next
                .artifacts
                .get(&key)
                .ok_or("quantization transition target disappeared")?
                .state;
            if let Some(commit) = commit_quantization_records(
                data,
                read,
                scope,
                actor,
                at,
                vec![quantization_lifecycle_record(&event)?],
                None,
            )? {
                return Ok(QuantizationArtifactTransition {
                    lifecycle_revision: event.revision,
                    entry,
                    state,
                    commit,
                });
            }
        }
        Err(publication_conflicts())
    })();
    match result {
        Ok(transition) => {
            span.finish(
                data.engine(),
                TraceOutcome::Ok,
                vec![TraceLink::Projection {
                    stamp: transition.entry.descriptor.stamp().clone(),
                }],
                quantization_finish_attributes(
                    transition.lifecycle_revision,
                    transition.state,
                    &transition.entry,
                    &transition.commit,
                ),
            )?;
            Ok(transition)
        }
        Err(error) => {
            finish_publication_error(data.engine(), span, "quantization_transition", error)
        }
    }
}

pub fn quantization_artifact_catalogue<E>(
    engine: &E,
    scope: &ScopeId,
) -> Result<QuantizationArtifactCatalogue, Box<dyn std::error::Error>>
where
    E: StorageEngine,
{
    quantization_artifact_catalogue_at_read(engine, scope).map(|(_, catalogue)| catalogue)
}

pub(super) fn quantization_artifact_catalogue_at_read<E>(
    engine: &E,
    scope: &ScopeId,
) -> Result<(ReadStamp, QuantizationArtifactCatalogue), Box<dyn std::error::Error>>
where
    E: StorageEngine,
{
    let read = engine.runtime().read_stamp(scope)?;
    let direct = read_vector_catalogue_versions(
        engine,
        &read,
        &[],
        RuntimeReadBudget::new(CATALOGUE_MAX_KEYS)?,
    )?;
    let catalogue = quantization_artifact_catalogue_from_changes(&direct.changes, scope)?;
    Ok((read, catalogue))
}

pub(crate) fn quantization_artifact_catalogue_from_changes(
    changes: &[RuntimeChange],
    scope: &ScopeId,
) -> Result<QuantizationArtifactCatalogue, Box<dyn std::error::Error>> {
    let mut artifact_records = BTreeMap::<RuntimeRef, (QuantizationArtifactEntry, String)>::new();
    let mut event_records = BTreeMap::<RuntimeRef, (QuantizationLifecycleEvent, String)>::new();
    let mut objects = BTreeMap::new();
    for change in changes.iter().filter(|change| &change.scope == scope) {
        if !change.verify_digest() {
            return Err("quantization catalogue encountered a corrupt change digest".into());
        }
        match &change.mutation {
            RuntimeMutation::Record { record }
                if record.reference.kind.as_str() == QUANTIZATION_ARTIFACT_RECORD_TYPE =>
            {
                if artifact_records
                    .insert(
                        record.reference.clone(),
                        (
                            quantization_entry_from_record(record)?,
                            change.commit_id.clone(),
                        ),
                    )
                    .is_some()
                {
                    return Err("duplicate quantization artifact record identity".into());
                }
            }
            RuntimeMutation::Record { record }
                if record.reference.kind.as_str() == QUANTIZATION_LIFECYCLE_RECORD_TYPE =>
            {
                if event_records
                    .insert(
                        record.reference.clone(),
                        (
                            quantization_event_from_record(record)?,
                            change.commit_id.clone(),
                        ),
                    )
                    .is_some()
                {
                    return Err("duplicate quantization lifecycle record identity".into());
                }
            }
            RuntimeMutation::Object { object }
                if object.subject.as_ref().is_some_and(|subject| {
                    subject.kind.as_str() == QUANTIZATION_ARTIFACT_RECORD_TYPE
                }) =>
            {
                objects
                    .insert(
                        object.reference.clone(),
                        (object.clone(), change.commit_id.clone()),
                    )
                    .is_none()
                    .then_some(())
                    .ok_or("duplicate quantization object mutation identity")?;
            }
            _ => {}
        }
    }
    let mut entries = Vec::with_capacity(artifact_records.len());
    let mut build_commits = BTreeMap::new();
    for (reference, (entry, record_commit)) in artifact_records {
        if entry.scope() != scope
            || QuantizationArtifactEntry::record_reference(&entry.descriptor)? != reference
        {
            return Err("quantization artifact record identity or scope differs".into());
        }
        let (object, object_commit) = objects
            .get(&entry.object.reference)
            .ok_or("quantization artifact references an absent object mutation")?;
        if object != &entry.object || object_commit != &record_commit {
            return Err("quantization artifact and object were not atomically built".into());
        }
        build_commits.insert(
            (
                entry.descriptor.stamp().id.clone(),
                entry.descriptor.stamp().generation,
            ),
            record_commit,
        );
        entries.push(entry);
    }
    let mut events = event_records.into_values().collect::<Vec<_>>();
    events.sort_by_key(|(event, _)| event.revision);
    let mut event_builds = BTreeSet::new();
    for (event, commit) in &events {
        if event.action == QuantizationLifecycleAction::Build {
            let key = (event.projection_id.clone(), event.generation);
            if !event_builds.insert(key.clone()) || build_commits.get(&key) != Some(commit) {
                return Err(
                    "quantization build event was not uniquely atomic with its artifact".into(),
                );
            }
        }
    }
    if event_builds != build_commits.keys().cloned().collect() {
        return Err("quantization artifact is missing its atomic build event".into());
    }
    QuantizationArtifactCatalogue::reconstruct(
        entries,
        events.into_iter().map(|(event, _)| event).collect(),
    )
    .map_err(Into::into)
}

pub(super) fn publication_conflicts() -> Box<dyn std::error::Error> {
    format!(
        "quantization lifecycle publication could not acquire cursor CAS after {PUBLICATION_RETRIES} conflicts"
    )
    .into()
}

/// Commits at `read`'s cursor. `None` means another commit in the scope, or
/// a schema change, moved the scope after `read`, and the caller retries from
/// a fresh read.
pub(super) fn commit_quantization_records<D>(
    data: &D,
    read: ReadStamp,
    scope: &ScopeId,
    actor: &str,
    at: Millis,
    records: Vec<RuntimeRecord>,
    object: Option<rrd_core::ObjectReference>,
) -> Result<Option<RuntimeCommitOutcome>, Box<dyn std::error::Error>>
where
    D: DataRuntimeAccess,
{
    let current = data.engine().runtime().schema(scope)?;
    if current.as_ref().map(|schema| schema.revision) != read.schema_revision {
        return Ok(None);
    }
    let expected_cursor = read.commit_cursor;
    let mut mutations = Vec::new();
    if let Some(registry) = quantization_schema_update(current)? {
        mutations.push(RuntimeMutation::Schema { registry });
    }
    mutations.extend(
        records
            .into_iter()
            .map(|record| RuntimeMutation::Record { record }),
    );
    if let Some(object) = object {
        mutations.push(RuntimeMutation::Object { object });
    }
    let transaction = DataTransaction::new(
        read,
        RuntimeCommit {
            scope: scope.clone(),
            at,
            actor: actor.to_owned(),
            expected_cursor,
            mutations,
        },
    )?;
    match data.commit(&transaction) {
        Ok(outcome) => Ok(Some(outcome)),
        Err(StoreError::RuntimeConflict { .. }) => Ok(None),
        Err(error) => Err(error.into()),
    }
}

pub(super) fn quantization_schema_update(
    current: Option<RuntimeSchemaRegistry>,
) -> Result<Option<RuntimeSchemaRegistry>, Box<dyn std::error::Error>> {
    let artifact_type = RuntimeType::new(QUANTIZATION_ARTIFACT_RECORD_TYPE)?;
    let lifecycle_type = RuntimeType::new(QUANTIZATION_LIFECYCLE_RECORD_TYPE)?;
    let artifact_schema = quantization_artifact_record_schema();
    let lifecycle_schema = quantization_lifecycle_record_schema();
    let mut registry = current
        .clone()
        .unwrap_or_else(|| RuntimeSchemaRegistry::empty(1, "install quantization lifecycle"));
    registry.define_record_table(
        artifact_type,
        RuntimeLogicalModel::Relational,
        artifact_schema,
    )?;
    registry.define_record_table(
        lifecycle_type,
        RuntimeLogicalModel::Relational,
        lifecycle_schema,
    )?;
    ensure_object_table(&mut registry)?;
    if current.as_ref() == Some(&registry) {
        return Ok(None);
    }
    if let Some(current) = current {
        registry.revision = current
            .revision
            .checked_add(1)
            .ok_or("runtime schema revision overflow while installing quantization lifecycle")?;
        registry.migration = "install quantization artifact lifecycle".into();
    }
    Ok(Some(registry))
}

pub(super) fn quantization_artifact_record_schema() -> RuntimeRecordSchema {
    let required = |value_type| RuntimePropertySchema::required(value_type);
    RuntimeRecordSchema {
        properties: BTreeMap::from([
            (
                "contract_version".into(),
                required(RuntimeValueType::Unsigned),
            ),
            ("collection_id".into(), required(RuntimeValueType::String)),
            ("vector_name".into(), required(RuntimeValueType::String)),
            ("projection_id".into(), required(RuntimeValueType::String)),
            ("generation".into(), required(RuntimeValueType::Unsigned)),
            ("artifact_kind".into(), required(RuntimeValueType::String)),
            ("config_digest".into(), required(RuntimeValueType::Digest)),
            ("artifact_digest".into(), required(RuntimeValueType::Digest)),
            ("object_id".into(), required(RuntimeValueType::String)),
            ("object_sha256".into(), required(RuntimeValueType::Digest)),
            ("object_length".into(), required(RuntimeValueType::Unsigned)),
            ("built_at".into(), required(RuntimeValueType::Unsigned)),
            ("entry_digest".into(), required(RuntimeValueType::Digest)),
            ("entry_json".into(), required(RuntimeValueType::String)),
        ]),
        allow_additional_properties: false,
        unique_properties: BTreeSet::from(["entry_digest".into()]),
    }
}

pub(super) fn quantization_lifecycle_record_schema() -> RuntimeRecordSchema {
    let required = |value_type| RuntimePropertySchema::required(value_type);
    RuntimeRecordSchema {
        properties: BTreeMap::from([
            (
                "contract_version".into(),
                required(RuntimeValueType::Unsigned),
            ),
            ("revision".into(), required(RuntimeValueType::Unsigned)),
            ("projection_id".into(), required(RuntimeValueType::String)),
            ("generation".into(), required(RuntimeValueType::Unsigned)),
            ("action".into(), required(RuntimeValueType::String)),
            ("at".into(), required(RuntimeValueType::Unsigned)),
            ("event_digest".into(), required(RuntimeValueType::Digest)),
            ("event_json".into(), required(RuntimeValueType::String)),
        ]),
        allow_additional_properties: false,
        unique_properties: BTreeSet::from(["revision".into(), "event_digest".into()]),
    }
}

pub(super) fn quantization_artifact_record(
    entry: &QuantizationArtifactEntry,
) -> Result<RuntimeRecord, Box<dyn std::error::Error>> {
    entry.validate()?;
    let stamp = entry.descriptor.stamp();
    Ok(RuntimeRecord {
        reference: QuantizationArtifactEntry::record_reference(&entry.descriptor)?,
        valid_from: entry.built_at,
        valid_to: None,
        properties: RuntimeProperties::from([
            (
                "contract_version".into(),
                RuntimeValue::Unsigned(entry.contract_version.into()),
            ),
            (
                "collection_id".into(),
                RuntimeValue::String(entry.collection_id.to_string()),
            ),
            (
                "vector_name".into(),
                RuntimeValue::String(entry.vector_name.to_string()),
            ),
            (
                "projection_id".into(),
                RuntimeValue::String(stamp.id.to_string()),
            ),
            (
                "generation".into(),
                RuntimeValue::Unsigned(stamp.generation),
            ),
            (
                "artifact_kind".into(),
                RuntimeValue::String(entry.kind.as_str().into()),
            ),
            (
                "config_digest".into(),
                RuntimeValue::Digest(stamp.config_digest.clone()),
            ),
            (
                "artifact_digest".into(),
                RuntimeValue::Digest(stamp.artifact_digest.clone()),
            ),
            (
                "object_id".into(),
                RuntimeValue::String(entry.object.reference.id.to_string()),
            ),
            (
                "object_sha256".into(),
                RuntimeValue::Digest(entry.object.sha256.clone()),
            ),
            (
                "object_length".into(),
                RuntimeValue::Unsigned(entry.object.length),
            ),
            ("built_at".into(), RuntimeValue::Unsigned(entry.built_at)),
            (
                "entry_digest".into(),
                RuntimeValue::Digest(entry.entry_digest.clone()),
            ),
            (
                "entry_json".into(),
                RuntimeValue::String(serde_json::to_string(entry)?),
            ),
        ]),
    })
}

pub(super) fn quantization_lifecycle_record(
    event: &QuantizationLifecycleEvent,
) -> Result<RuntimeRecord, Box<dyn std::error::Error>> {
    event.validate()?;
    let action = match event.action {
        QuantizationLifecycleAction::Build => "build",
        QuantizationLifecycleAction::Activate => "activate",
        QuantizationLifecycleAction::Retire => "retire",
    };
    Ok(RuntimeRecord {
        reference: QuantizationLifecycleEvent::record_reference(event.revision)?,
        valid_from: event.at,
        valid_to: None,
        properties: RuntimeProperties::from([
            (
                "contract_version".into(),
                RuntimeValue::Unsigned(event.contract_version.into()),
            ),
            ("revision".into(), RuntimeValue::Unsigned(event.revision)),
            (
                "projection_id".into(),
                RuntimeValue::String(event.projection_id.to_string()),
            ),
            (
                "generation".into(),
                RuntimeValue::Unsigned(event.generation),
            ),
            ("action".into(), RuntimeValue::String(action.into())),
            ("at".into(), RuntimeValue::Unsigned(event.at)),
            (
                "event_digest".into(),
                RuntimeValue::Digest(event.event_digest.clone()),
            ),
            (
                "event_json".into(),
                RuntimeValue::String(serde_json::to_string(event)?),
            ),
        ]),
    })
}

pub(super) fn quantization_entry_from_record(
    record: &RuntimeRecord,
) -> Result<QuantizationArtifactEntry, Box<dyn std::error::Error>> {
    if record.valid_to.is_some() {
        return Err("quantization artifact records cannot have a validity end".into());
    }
    let entry: QuantizationArtifactEntry =
        serde_json::from_str(string_property(&record.properties, "entry_json")?)?;
    entry.validate()?;
    if quantization_artifact_record(&entry)? != *record {
        return Err("quantization artifact record differs from canonical entry JSON".into());
    }
    Ok(entry)
}

pub(super) fn quantization_event_from_record(
    record: &RuntimeRecord,
) -> Result<QuantizationLifecycleEvent, Box<dyn std::error::Error>> {
    if record.valid_to.is_some() {
        return Err("quantization lifecycle records cannot have a validity end".into());
    }
    let event: QuantizationLifecycleEvent =
        serde_json::from_str(string_property(&record.properties, "event_json")?)?;
    event.validate()?;
    if quantization_lifecycle_record(&event)? != *record {
        return Err("quantization lifecycle record differs from canonical event JSON".into());
    }
    Ok(event)
}
