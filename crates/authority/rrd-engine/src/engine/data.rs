use super::*;
use rrd_contract::{DataSchemaSnapshot, ReadDataSchema};
use rrd_store::{RuntimeReadBudget, RuntimeVersionedSource};

impl RrdEngine {
    /// Read the native catalogue at one authenticated stamp without scanning
    /// application data or the causal log. This administrative operation does
    /// not support filtered grants: a restricted principal must fail closed.
    #[allow(clippy::too_many_arguments)]
    pub fn read_data_schema(
        &self,
        session_id: &CorrelationId,
        token: &CorrelationId,
        request: &ReadDataSchema,
        now: u64,
        request_id: &str,
        operation_id: &str,
    ) -> Result<DataSchemaSnapshot> {
        request
            .validate()
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        self.authorize_resource_without_data_policy(
            session_id,
            token,
            SecurityAction::DiagnosticsRead,
            &self.instance_resource(),
            now,
            request_id,
            operation_id,
        )?;
        let scope = self.query_scope(&request.scope)?;
        let read = self.storage.runtime().read_stamp(&scope)?;
        let direct = self.storage.runtime().read_versioned(
            &read,
            &[RuntimeVersionedSource::Schema],
            RuntimeReadBudget::new(u64::from(request.max_storage_keys))?,
        )?;
        let schema = direct
            .changes
            .iter()
            .rev()
            .find_map(|change| match &change.mutation {
                RuntimeMutation::Schema { registry }
                    if Some(registry.revision) == read.schema_revision =>
                {
                    Some(registry)
                }
                _ => None,
            })
            .map(public_schema)
            .transpose()?;
        if schema.as_ref().map(|registry| registry.revision) != read.schema_revision {
            return Err(ServiceError::Query(
                "stamped schema revision is unavailable".into(),
            ));
        }
        Ok(DataSchemaSnapshot {
            scope: request.scope.clone(),
            known_at_cursor: read.commit_cursor,
            read_manifest_sha256: read.manifest_id,
            schema,
            read_evidence: crate::runtime::public_read_evidence(direct.evidence)?,
        })
    }

    /// Read every logical model through one catalogue revision, valid-time
    /// instant, authenticated log root, and transaction cursor.
    #[allow(clippy::too_many_arguments)]
    pub fn read_data_snapshot(
        &self,
        session_id: &CorrelationId,
        token: &CorrelationId,
        request: &ReadDataSnapshot,
        now: u64,
        request_id: &str,
        operation_id: &str,
    ) -> Result<DataSnapshot> {
        request
            .validate()
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        self.authorize(
            session_id,
            token,
            SecurityAction::QueryExecute,
            now,
            request_id,
            operation_id,
        )?;
        let scope = ScopeId::new(format!("instance:{}", self.instance)).map_err(core_contract)?;
        let key_budget = usize::try_from(request.max_storage_keys)
            .map_err(|_| ServiceError::Query("data snapshot key budget exceeds usize".into()))?;
        let snapshot =
            self.storage
                .runtime()
                .data_snapshot(&scope, request.valid_at, key_budget)?;

        public_data_snapshot(snapshot)
    }
}

pub(in crate::engine) fn public_data_snapshot(
    read: RuntimeDataSnapshotRead,
) -> Result<DataSnapshot> {
    let RuntimeDataSnapshotRead {
        read,
        selected_versions,
        read_evidence,
        snapshot,
    } = read;
    let mut entries = Vec::with_capacity(
        snapshot.records.len()
            + snapshot.relations.len()
            + snapshot.events.len()
            + snapshot.vectors.len()
            + snapshot.series.len()
            + snapshot.geo.len()
            + snapshot.objects.len(),
    );
    for entry in snapshot.records {
        let reference = entry.value.reference.clone();
        entries.push(snapshot_entry(
            entry.model,
            reference,
            RuntimeMutation::Record {
                record: entry.value,
            },
        )?);
    }
    for entry in snapshot.relations {
        let reference = entry.value.reference.clone();
        entries.push(snapshot_entry(
            entry.model,
            reference,
            RuntimeMutation::Relation {
                relation: entry.value,
            },
        )?);
    }
    for entry in snapshot.events {
        let reference = entry.value.reference.clone();
        entries.push(snapshot_entry(
            entry.model,
            reference,
            RuntimeMutation::Event {
                event: entry.value.event,
            },
        )?);
    }
    for entry in snapshot.vectors {
        let reference = entry.value.reference.clone();
        entries.push(snapshot_entry(
            entry.model,
            reference,
            RuntimeMutation::Vector {
                vector: entry.value,
            },
        )?);
    }
    for entry in snapshot.series {
        let reference = entry.value.reference.clone();
        entries.push(snapshot_entry(
            entry.model,
            reference,
            RuntimeMutation::SeriesSample {
                sample: entry.value,
            },
        )?);
    }
    for entry in snapshot.geo {
        let reference = entry.value.reference.clone();
        entries.push(snapshot_entry(
            entry.model,
            reference,
            RuntimeMutation::Geo { geo: entry.value },
        )?);
    }
    for entry in snapshot.objects {
        let reference = entry.value.reference.clone();
        entries.push(snapshot_entry(
            entry.model,
            reference,
            RuntimeMutation::Object {
                object: entry.value,
            },
        )?);
    }

    Ok(DataSnapshot {
        scope: snapshot.scope.to_string(),
        valid_at: snapshot.valid_at,
        known_at_cursor: snapshot.known_at_cursor,
        schema_revision: snapshot.schema_revision,
        read_manifest_sha256: read.manifest_id,
        selected_versions,
        read_evidence: crate::runtime::public_read_evidence(read_evidence)?,
        entries,
    })
}

fn snapshot_entry(
    model: RuntimeLogicalModel,
    reference: RuntimeRef,
    mutation: RuntimeMutation,
) -> Result<DataSnapshotEntry> {
    Ok(DataSnapshotEntry {
        model: public_logical_model(model),
        target: public_data_target(model, &reference)?,
        value: public_data_mutation(&mutation)?,
    })
}
