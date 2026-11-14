//! Diagnostic models, retention, indexes, collections, audit and sections.

use super::*;

pub(super) fn diagnostic_models(
    scope: &str,
    schema: Option<&DataSchemaRegistry>,
) -> Result<DiagnosticModelCatalogueSnapshot> {
    let mut models = Vec::new();
    if let Some(schema) = schema {
        for (id, model) in &schema.records {
            models.push(DiagnosticModelSnapshot {
                id: id.clone(),
                kind: DiagnosticModelKind::Record,
                property_count: diagnostic_count(model.properties.len())?,
                required_property_count: diagnostic_count(
                    model
                        .properties
                        .values()
                        .filter(|property| property.required)
                        .count(),
                )?,
                constraint_count: diagnostic_count(model.unique_properties.len())?,
                allow_additional_properties: model.allow_additional_properties,
            });
        }
        for (id, model) in &schema.relations {
            let constraint_count = model
                .from
                .len()
                .checked_add(model.to.len())
                .and_then(|count| count.checked_add(usize::from(model.unique_pair)))
                .and_then(|count| count.checked_add(usize::from(model.max_outgoing.is_some())))
                .and_then(|count| count.checked_add(usize::from(model.max_incoming.is_some())))
                .ok_or_else(|| ServiceError::Contract("model constraint count overflow".into()))?;
            models.push(DiagnosticModelSnapshot {
                id: id.clone(),
                kind: DiagnosticModelKind::Relation,
                property_count: diagnostic_count(model.properties.len())?,
                required_property_count: diagnostic_count(
                    model
                        .properties
                        .values()
                        .filter(|property| property.required)
                        .count(),
                )?,
                constraint_count: diagnostic_count(constraint_count)?,
                allow_additional_properties: model.allow_additional_properties,
            });
        }
        for (id, model) in &schema.events {
            let constraint_count = model
                .subject_types
                .len()
                .checked_add(usize::from(model.subject_required))
                .ok_or_else(|| ServiceError::Contract("model constraint count overflow".into()))?;
            models.push(DiagnosticModelSnapshot {
                id: id.clone(),
                kind: DiagnosticModelKind::Event,
                property_count: diagnostic_count(model.properties.len())?,
                required_property_count: diagnostic_count(
                    model
                        .properties
                        .values()
                        .filter(|property| property.required)
                        .count(),
                )?,
                constraint_count: diagnostic_count(constraint_count)?,
                allow_additional_properties: model.allow_additional_properties,
            });
        }
    }
    models.sort_by(|left, right| {
        left.kind
            .cmp(&right.kind)
            .then_with(|| left.id.cmp(&right.id))
    });
    Ok(DiagnosticModelCatalogueSnapshot {
        scope: scope.into(),
        schema_revision: schema.map(|schema| schema.revision),
        models,
    })
}

pub(super) fn capture_diagnostic_stamp(
    engine: &RrdEngine,
    scope: &ScopeId,
    now: u64,
) -> Result<EngineDiagnosticStamp> {
    let retention = diagnostic_retention(engine, now)?;
    Ok(EngineDiagnosticStamp {
        control_sequence: engine.storage.control().sequence()?,
        claim_sequence: engine.storage.claims().sequence()?,
        runtime: engine.storage.runtime().read_stamp(scope)?,
        retention_sha256: diagnostic_retention_sha256(&retention)?,
    })
}

pub(super) fn diagnostic_retention(
    engine: &RrdEngine,
    now: u64,
) -> Result<DiagnosticRetentionSnapshot> {
    let handles = engine.storage.runtime().snapshots(now)?;
    let mut leases = Vec::with_capacity(handles.len());
    let mut pins = Vec::with_capacity(handles.len());
    for handle in handles {
        let pin = RetentionPin::from_snapshot(&handle)
            .map_err(|error| ServiceError::Storage(error.to_string()))?;
        leases.push(DiagnosticSnapshotLease {
            id_sha256: handle.id.as_str().into(),
            scope: handle.read.scope.as_str().into(),
            owner: handle.owner,
            created_at_unix_ms: handle.created_at,
            expires_at_unix_ms: handle.expires_at,
            runtime_cursor: handle.read.commit_cursor,
            schema_revision: handle.read.schema_revision,
            catalogue_revision: handle.read.catalog_revision,
            runtime_manifest_sha256: handle.read.manifest_id,
        });
        pins.push(DiagnosticRetentionPin {
            id_sha256: pin.id.as_str().into(),
            snapshot_id_sha256: pin.snapshot_id.as_str().into(),
            scope: pin.scope.as_str().into(),
            runtime_manifest_sha256: pin.manifest_id,
            minimum_cursor: pin.minimum_cursor,
            expires_at_unix_ms: pin.expires_at,
        });
    }
    leases.sort_by(|left, right| left.id_sha256.cmp(&right.id_sha256));
    pins.sort_by(|left, right| left.id_sha256.cmp(&right.id_sha256));
    let oldest_retained_cursor = pins.iter().map(|pin| pin.minimum_cursor).min();
    Ok(DiagnosticRetentionSnapshot {
        observed_at_unix_ms: now,
        leases,
        pins,
        oldest_retained_cursor,
    })
}

pub(super) fn diagnostic_retention_sha256(
    retention: &DiagnosticRetentionSnapshot,
) -> Result<String> {
    let encoded =
        serde_json::to_vec(retention).map_err(|error| ServiceError::Contract(error.to_string()))?;
    Ok(digest::sha256_hex(&encoded))
}

pub(super) fn diagnostic_query_indexes(
    engine: &RrdEngine,
    scope: &ScopeId,
    public_scope: &str,
) -> Result<QueryIndexCatalogueSnapshot> {
    let catalogue = rrd_query::IndexCatalogueRepository::new(&engine.storage, scope.clone())
        .load()
        .map_err(|error| ServiceError::Query(error.to_string()))?;
    Ok(QueryIndexCatalogueSnapshot {
        scope: public_scope.into(),
        revision: catalogue.revision,
        indexes: catalogue
            .entries
            .values()
            .map(super::super::query::public_query_index)
            .collect::<Result<_>>()?,
    })
}

pub(super) fn diagnostic_vector_collections(
    engine: &RrdEngine,
    scope: &ScopeId,
    public_scope: &str,
) -> Result<VectorCollectionCatalogueSnapshot> {
    let catalogue = rrd_vector::VectorCollectionRepository::new(&engine.storage, scope.clone())
        .load()
        .map_err(super::super::vector::vector_collection_error)?;
    Ok(VectorCollectionCatalogueSnapshot {
        scope: public_scope.into(),
        revision: catalogue.revision,
        collections: catalogue
            .collections
            .values()
            .map(super::super::vector::public_vector_collection)
            .collect::<Result<_>>()?,
    })
}

pub(super) fn diagnostic_audit(engine: &RrdEngine, after: u64, limit: usize) -> Result<AuditPage> {
    let page = rrd_security::SecurityRepository::new(&engine.storage, engine.instance.clone())
        .audit_since(after, limit)
        .map_err(|error| ServiceError::Contract(error.to_string()))?;
    Ok(AuditPage {
        requested_after_sequence: after,
        through_sequence: page.through_sequence,
        chain_anchor_sha256: page.chain_anchor_sha256,
        chain_head_sha256: page.chain_head_sha256,
        records: page
            .records
            .into_iter()
            .map(|(sequence, record)| AuditRecordSnapshot {
                sequence,
                audit_id: record.audit_id,
                at_unix_ms: record.at_unix_ms,
                principal_id: record.principal_id,
                action: record.action,
                resource: record.resource,
                request_id: record.request_id,
                operation_id: record.operation_id,
                phase: record.phase,
                decision: record.decision,
                status_code: record.status_code,
                request_sha256: record.request_sha256,
                response_sha256: record.response_sha256,
                previous_audit_sha256: record.previous_audit_sha256,
                audit_sha256: record.audit_sha256,
            })
            .collect(),
    })
}

#[allow(clippy::too_many_arguments)]
pub(super) fn diagnostic_sections(
    known_at_cursor: u64,
    product_capabilities: &rrd_contract::ProductCapabilityCatalogue,
    schema: Option<&DataSchemaRegistry>,
    models: &DiagnosticModelCatalogueSnapshot,
    graph: &DiagnosticGraphSnapshot,
    graph_difference: &DiagnosticGraphDifference,
    retention: &DiagnosticRetentionSnapshot,
    vector_artifacts: &DiagnosticVectorArtifactCatalogueSnapshot,
    query_indexes: &QueryIndexCatalogueSnapshot,
    vector_collections: &VectorCollectionCatalogueSnapshot,
    estate: Option<&rrd_contract::EstateSnapshot>,
    storage: &DiagnosticStorageSnapshot,
    changes: &ChangefeedPage,
    audit: &AuditPage,
) -> Result<Vec<DiagnosticSectionSnapshot>> {
    let mut sections = vec![
        section(
            "audit",
            DiagnosticAuthority::Authoritative,
            DiagnosticCoverage::Bounded,
            audit.records.len(),
            known_at_cursor,
            Some(audit.requested_after_sequence),
        )?,
        section(
            "changes",
            DiagnosticAuthority::Authoritative,
            DiagnosticCoverage::Bounded,
            changes.changes.len(),
            known_at_cursor,
            Some(changes.requested_after_cursor),
        )?,
        section(
            "estate",
            DiagnosticAuthority::Authoritative,
            DiagnosticCoverage::Complete,
            usize::from(estate.is_some()),
            known_at_cursor,
            None,
        )?,
        section(
            "graph",
            DiagnosticAuthority::Projection,
            DiagnosticCoverage::Complete,
            graph.records.len().saturating_add(graph.relations.len()),
            graph.known_at_cursor,
            None,
        )?,
        section(
            "graph-difference",
            DiagnosticAuthority::Projection,
            DiagnosticCoverage::Complete,
            graph_difference_row_count(graph_difference),
            graph_difference.to_cursor,
            Some(graph_difference.from_cursor),
        )?,
        section(
            "models",
            DiagnosticAuthority::Projection,
            DiagnosticCoverage::Complete,
            models.models.len(),
            known_at_cursor,
            None,
        )?,
        section(
            "product-capabilities",
            DiagnosticAuthority::Authoritative,
            DiagnosticCoverage::Complete,
            product_capabilities.capabilities.len(),
            known_at_cursor,
            None,
        )?,
        section(
            "query-indexes",
            DiagnosticAuthority::Projection,
            DiagnosticCoverage::Complete,
            query_indexes.indexes.len(),
            known_at_cursor,
            None,
        )?,
        section(
            "retention",
            DiagnosticAuthority::Authoritative,
            DiagnosticCoverage::Complete,
            retention.leases.len().saturating_add(retention.pins.len()),
            known_at_cursor,
            None,
        )?,
        section(
            "schema",
            DiagnosticAuthority::Authoritative,
            DiagnosticCoverage::Complete,
            usize::from(schema.is_some()),
            known_at_cursor,
            None,
        )?,
        section(
            "storage",
            DiagnosticAuthority::Observation,
            DiagnosticCoverage::Complete,
            usize::from(storage.rrflow_kv.is_some()),
            known_at_cursor,
            None,
        )?,
        section(
            "vector-artifacts",
            DiagnosticAuthority::Authoritative,
            DiagnosticCoverage::Complete,
            vector_artifacts.artifacts.len(),
            known_at_cursor,
            None,
        )?,
        section(
            "vector-collections",
            DiagnosticAuthority::Authoritative,
            DiagnosticCoverage::Complete,
            vector_collections.collections.len(),
            known_at_cursor,
            None,
        )?,
    ];
    sections.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(sections)
}
