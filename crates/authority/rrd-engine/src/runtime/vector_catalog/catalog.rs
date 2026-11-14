//! The vector artifact catalog: entries, records and schema.

use super::*;

/// Returns the typed, atomically bound catalog in revision order without
/// reading potentially large artifact bodies. This is the safe inspection
/// surface for Connectome and publication preflight.
pub fn vector_artifact_catalog_entries<E>(
    engine: &E,
    scope: &ScopeId,
) -> Result<Vec<VectorArtifactCatalogEntry>, Box<dyn std::error::Error>>
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
    vector_artifact_catalog_entries_from_changes(&direct.changes, scope)
}

/// Reconstructs the exact vector-artifact catalogue from a caller-owned,
/// already bounded and stamped runtime change set.
pub(crate) fn vector_artifact_catalog_entries_from_changes(
    changes: &[RuntimeChange],
    scope: &ScopeId,
) -> Result<Vec<VectorArtifactCatalogEntry>, Box<dyn std::error::Error>> {
    let mut records = BTreeMap::<RuntimeRef, (RuntimeRecord, String)>::new();
    let mut objects = BTreeMap::new();
    for change in changes.iter().filter(|change| &change.scope == scope) {
        if !change.verify_digest() {
            return Err("vector catalog replay encountered a corrupt change digest".into());
        }
        match &change.mutation {
            RuntimeMutation::Record { record }
                if record.reference.kind.as_str() == VECTOR_ARTIFACT_RECORD_TYPE =>
            {
                records.insert(
                    record.reference.clone(),
                    (record.clone(), change.commit_id.clone()),
                );
            }
            RuntimeMutation::Object { object }
                if object.subject.as_ref().is_some_and(|subject| {
                    subject.kind.as_str() == VECTOR_ARTIFACT_RECORD_TYPE
                }) =>
            {
                objects.insert(
                    object.reference.clone(),
                    (object.clone(), change.commit_id.clone()),
                );
            }
            _ => {}
        }
    }

    let mut entries = Vec::with_capacity(records.len());
    for (reference, (record, record_commit)) in records {
        let entry = entry_from_record(&record)?;
        if entry.scope() != scope
            || VectorArtifactCatalogEntry::record_reference(&entry.descriptor)? != reference
        {
            return Err("vector catalog record identity or scope differs from its entry".into());
        }
        let (object, object_commit) = objects
            .get(&entry.object.reference)
            .ok_or("vector catalog entry references an absent object mutation")?;
        if object != &entry.object || object_commit != &record_commit {
            return Err(
                "vector catalog record and object were not atomically published together".into(),
            );
        }
        entries.push(entry);
    }
    entries.sort_by_key(|entry| entry.catalog_revision);

    for (ordinal, entry) in entries.iter().enumerate() {
        let expected_revision = ordinal as u64;
        if entry.catalog_revision != expected_revision + 1 {
            return Err(format!(
                "vector catalog revision gap: expected {}, found {}",
                expected_revision + 1,
                entry.catalog_revision
            )
            .into());
        }
    }
    Ok(entries)
}

pub(super) fn commit_catalog_record<D>(
    data: &D,
    scope: &ScopeId,
    actor: &str,
    at: Millis,
    record: RuntimeRecord,
    object: rrd_core::ObjectReference,
) -> Result<RuntimeCommitOutcome, Box<dyn std::error::Error>>
where
    D: DataRuntimeAccess,
{
    for _ in 0..PUBLICATION_RETRIES {
        let read = data.engine().runtime().read_stamp(scope)?;
        let current = data.engine().runtime().schema(scope)?;
        if current.as_ref().map(|schema| schema.revision) != read.schema_revision {
            continue;
        }
        let mut mutations = Vec::new();
        if let Some(registry) = catalog_schema_update(current)? {
            mutations.push(RuntimeMutation::Schema { registry });
        }
        mutations.push(RuntimeMutation::Record {
            record: record.clone(),
        });
        mutations.push(RuntimeMutation::Object {
            object: object.clone(),
        });
        let expected_cursor = read.commit_cursor;
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
            Ok(outcome) => return Ok(outcome),
            Err(StoreError::RuntimeConflict { .. }) => continue,
            Err(error) => return Err(error.into()),
        }
    }
    Err(format!(
        "vector catalog publication could not acquire cursor CAS after {PUBLICATION_RETRIES} conflicts"
    )
    .into())
}

pub(super) fn catalog_schema_update(
    current: Option<RuntimeSchemaRegistry>,
) -> Result<Option<RuntimeSchemaRegistry>, Box<dyn std::error::Error>> {
    let record_type = RuntimeType::new(VECTOR_ARTIFACT_RECORD_TYPE)?;
    let schema = catalog_record_schema();
    let mut registry = current
        .clone()
        .unwrap_or_else(|| RuntimeSchemaRegistry::empty(1, "install vector artifact catalog"));
    registry.define_record_table(record_type, RuntimeLogicalModel::Relational, schema)?;
    ensure_object_table(&mut registry)?;
    if current.as_ref() == Some(&registry) {
        return Ok(None);
    }
    if let Some(current) = current {
        registry.revision = current
            .revision
            .checked_add(1)
            .ok_or("runtime schema revision overflow while installing vector catalog")?;
        registry.migration = "install authoritative vector artifact catalog".into();
    }
    Ok(Some(registry))
}

pub(super) fn ensure_object_table(
    registry: &mut RuntimeSchemaRegistry,
) -> Result<(), Box<dyn std::error::Error>> {
    let object_type = RuntimeType::new("object")?;
    match registry.tables.get(&object_type) {
        Some(table) if table.model == RuntimeLogicalModel::Object => Ok(()),
        Some(_) => Err("canonical object table belongs to another logical model".into()),
        None => {
            registry.tables.insert(
                object_type,
                RuntimeTableSchema::schemaless(RuntimeLogicalModel::Object),
            );
            Ok(())
        }
    }
}

pub(super) fn catalog_record_schema() -> RuntimeRecordSchema {
    let required = |value_type| RuntimePropertySchema::required(value_type);
    RuntimeRecordSchema {
        properties: BTreeMap::from([
            (
                "contract_version".into(),
                required(RuntimeValueType::Unsigned),
            ),
            (
                "catalog_revision".into(),
                required(RuntimeValueType::Unsigned),
            ),
            ("projection_id".into(), required(RuntimeValueType::String)),
            ("generation".into(), required(RuntimeValueType::Unsigned)),
            ("source_cursor".into(), required(RuntimeValueType::Unsigned)),
            ("config_digest".into(), required(RuntimeValueType::Digest)),
            ("artifact_digest".into(), required(RuntimeValueType::Digest)),
            ("state".into(), required(RuntimeValueType::String)),
            ("artifact_kind".into(), required(RuntimeValueType::String)),
            ("object_id".into(), required(RuntimeValueType::String)),
            ("object_sha256".into(), required(RuntimeValueType::Digest)),
            ("object_length".into(), required(RuntimeValueType::Unsigned)),
            ("entry_digest".into(), required(RuntimeValueType::Digest)),
            ("entry_json".into(), required(RuntimeValueType::String)),
            ("published_at".into(), required(RuntimeValueType::Unsigned)),
        ]),
        allow_additional_properties: false,
        unique_properties: BTreeSet::from(["catalog_revision".into(), "entry_digest".into()]),
    }
}

pub(super) fn catalog_record(
    entry: &VectorArtifactCatalogEntry,
) -> Result<RuntimeRecord, Box<dyn std::error::Error>> {
    entry.validate()?;
    let stamp = entry.descriptor.stamp();
    Ok(RuntimeRecord {
        reference: VectorArtifactCatalogEntry::record_reference(&entry.descriptor)?,
        valid_from: entry.published_at,
        valid_to: None,
        properties: RuntimeProperties::from([
            (
                "contract_version".into(),
                RuntimeValue::Unsigned(entry.contract_version.into()),
            ),
            (
                "catalog_revision".into(),
                RuntimeValue::Unsigned(entry.catalog_revision),
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
                "source_cursor".into(),
                RuntimeValue::Unsigned(stamp.source_cursor),
            ),
            (
                "config_digest".into(),
                RuntimeValue::Digest(stamp.config_digest.clone()),
            ),
            (
                "artifact_digest".into(),
                RuntimeValue::Digest(stamp.artifact_digest.clone()),
            ),
            ("state".into(), RuntimeValue::String("ready".into())),
            (
                "artifact_kind".into(),
                RuntimeValue::String(entry.kind.as_str().into()),
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
            (
                "entry_digest".into(),
                RuntimeValue::Digest(entry.entry_digest.clone()),
            ),
            (
                "entry_json".into(),
                RuntimeValue::String(serde_json::to_string(entry)?),
            ),
            (
                "published_at".into(),
                RuntimeValue::Unsigned(entry.published_at),
            ),
        ]),
    })
}

pub(super) fn entry_from_record(
    record: &RuntimeRecord,
) -> Result<VectorArtifactCatalogEntry, Box<dyn std::error::Error>> {
    if record.valid_to.is_some() {
        return Err("vector artifact catalog records cannot be retired by validity window".into());
    }
    let json = string_property(&record.properties, "entry_json")?;
    let entry: VectorArtifactCatalogEntry = serde_json::from_str(json)?;
    entry.validate()?;
    let expected = catalog_record(&entry)?;
    if expected.reference != record.reference
        || expected.valid_from != record.valid_from
        || expected.properties != record.properties
    {
        return Err(
            "vector artifact catalog record fields differ from canonical entry JSON".into(),
        );
    }
    Ok(entry)
}

pub(super) fn string_property<'a>(
    properties: &'a RuntimeProperties,
    name: &str,
) -> Result<&'a str, Box<dyn std::error::Error>> {
    match properties.get(name) {
        Some(RuntimeValue::String(value)) => Ok(value),
        _ => Err(
            format!("vector artifact record property {name:?} is missing or not a string").into(),
        ),
    }
}
