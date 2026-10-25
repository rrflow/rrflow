use super::*;
use rrd_contract::{
    DiagnosticMaintenanceSnapshot, DiagnosticMemtableSnapshot, DiagnosticPageCacheSnapshot,
    DiagnosticRrflowKvSnapshot, DiagnosticSegmentIoMode, DiagnosticSegmentIoSnapshot,
    DiagnosticSegmentSetSnapshot, DiagnosticStorageAdmission, DiagnosticStorageEvidenceLevel,
    DiagnosticStorageSnapshot, DiagnosticWriteAheadLogSnapshot, MAX_MESSAGE_BYTES,
};
use rrd_store::PhysicalStoreEvidence;

const MAX_DIAGNOSTIC_ASSEMBLY_ATTEMPTS: u8 = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
struct EngineDiagnosticStamp {
    claim_sequence: u64,
    control_sequence: u64,
    runtime: ReadStamp,
    retention_sha256: String,
}

impl RrdEngine {
    /// Observe physical counters without traversing runtime/graph history.
    /// Authorization and instance scope are identical to full diagnostics;
    /// these counters are observations, not a cross-model consistency claim.
    pub fn read_storage_diagnostics(
        &self,
        session_id: &CorrelationId,
        token: &CorrelationId,
        request: &rrd_contract::ReadStorageDiagnostics,
        now: u64,
        request_id: &str,
        operation_id: &str,
    ) -> Result<DiagnosticStorageSnapshot> {
        request
            .validate()
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        self.authorize(
            session_id,
            token,
            SecurityAction::DiagnosticsRead,
            now,
            request_id,
            operation_id,
        )?;
        self.query_scope(&request.scope)?;
        let snapshot = diagnostic_storage(self, now)?;
        snapshot
            .validate()
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        Ok(snapshot)
    }

    /// Assembles a bounded cross-model diagnostic view under one verified RRD
    /// read stamp. Concurrent mutations cause a retry; a response is never
    /// returned with coordinates from two different runtime states.
    #[allow(clippy::too_many_arguments)]
    pub fn read_diagnostic_snapshot(
        &self,
        session_id: &CorrelationId,
        token: &CorrelationId,
        request: &ReadDiagnosticSnapshot,
        now: u64,
        request_id: &str,
        operation_id: &str,
    ) -> Result<DiagnosticSnapshot> {
        self.read_diagnostic_snapshot_with_observer(
            session_id,
            token,
            request,
            now,
            request_id,
            operation_id,
            &mut |_| Ok(()),
        )
    }

    /// Runs `during_assembly` inside every assembly attempt, after its first
    /// consistency stamp is captured, so a test can place concurrent work in
    /// the window that attempt compares.
    #[cfg(test)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn read_diagnostic_snapshot_during_assembly(
        &self,
        session_id: &CorrelationId,
        token: &CorrelationId,
        request: &ReadDiagnosticSnapshot,
        now: u64,
        request_id: &str,
        operation_id: &str,
        during_assembly: &mut dyn FnMut(u8) -> Result<()>,
    ) -> Result<DiagnosticSnapshot> {
        self.read_diagnostic_snapshot_with_observer(
            session_id,
            token,
            request,
            now,
            request_id,
            operation_id,
            during_assembly,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn read_diagnostic_snapshot_with_observer(
        &self,
        session_id: &CorrelationId,
        token: &CorrelationId,
        request: &ReadDiagnosticSnapshot,
        now: u64,
        request_id: &str,
        operation_id: &str,
        observer: &mut dyn FnMut(u8) -> Result<()>,
    ) -> Result<DiagnosticSnapshot> {
        request
            .validate()
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        self.authorize(
            session_id,
            token,
            SecurityAction::DiagnosticsRead,
            now,
            request_id,
            operation_id,
        )?;
        let scope = self.query_scope(&request.scope)?;

        for attempt in 1..=MAX_DIAGNOSTIC_ASSEMBLY_ATTEMPTS {
            let before = capture_diagnostic_stamp(self, &scope, now)?;
            observer(attempt)?;
            let runtime_changes = match diagnostic_runtime_changes(
                self,
                &before.runtime,
                request.runtime_max_scanned_changes,
            ) {
                Ok(changes) => changes,
                Err(error) if error.retryable() => continue,
                Err(error) => return Err(error),
            };
            let schema = self
                .storage
                .runtime()
                .schema(&scope)?
                .as_ref()
                .map(super::model::public_schema)
                .transpose()?;
            let models = diagnostic_models(&request.scope, schema.as_ref())?;
            let (graph, graph_difference) = match diagnostic_graph(
                &before.runtime,
                &runtime_changes,
                &request.scope,
                request.graph_valid_at_unix_ms,
                request.graph_known_at_cursor,
                request.graph_compare_cursor,
            ) {
                Ok(graph) => graph,
                Err(error) if error.retryable() => continue,
                Err(error) => return Err(error),
            };
            let vector_artifacts =
                diagnostic_vector_artifacts(&runtime_changes, &scope, &request.scope)?;
            let retention = diagnostic_retention(self, now)?;
            let retention_sha256 = diagnostic_retention_sha256(&retention)?;
            let query_indexes = diagnostic_query_indexes(self, &scope, &request.scope)?;
            let vector_collections = diagnostic_vector_collections(self, &scope, &request.scope)?;
            let estate = rrd_estate::EstateRepository::new(&self.storage, self.instance.clone())
                .load()?
                .as_ref()
                .map(rrd_estate::public_snapshot);
            let changes = self.read_changefeed_page(&ReadChangefeed {
                scope: request.scope.clone(),
                after_cursor: request.changes_after_cursor,
                limit: request.change_limit,
            })?;
            let audit = diagnostic_audit(
                self,
                request.audit_after_sequence,
                usize::from(request.audit_limit),
            )?;
            let product_capabilities = crate::product_capability_catalogue();
            let readiness = self.readiness(now)?;
            let after = capture_diagnostic_stamp(self, &scope, now)?;

            if before != after
                || readiness.claim_sequence != after.claim_sequence
                || readiness.runtime_cursor != after.runtime.commit_cursor
                || changes.head_cursor != after.runtime.commit_cursor
                || retention_sha256 != before.retention_sha256
                || retention_sha256 != after.retention_sha256
            {
                continue;
            }

            // Physical counters are an observation of this process, taken once
            // after the logical coordinates verified; they never gate a retry.
            let storage = diagnostic_storage(self, now)?;
            let sections = diagnostic_sections(
                after.runtime.commit_cursor,
                &product_capabilities,
                schema.as_ref(),
                &models,
                &graph,
                &graph_difference,
                &retention,
                &vector_artifacts,
                &query_indexes,
                &vector_collections,
                estate.as_ref(),
                &storage,
                &changes,
                &audit,
            )?;
            let snapshot = DiagnosticSnapshot {
                format_version: DIAGNOSTIC_SNAPSHOT_FORMAT_VERSION,
                observed_at_unix_ms: now,
                instance: ResourceId::new(ResourceKind::Instance, self.instance.as_str())
                    .expect("engine instance identity is canonical"),
                scope: request.scope.clone(),
                read: DiagnosticReadStamp {
                    claim_sequence: after.claim_sequence,
                    control_sequence: after.control_sequence,
                    runtime_cursor: after.runtime.commit_cursor,
                    schema_revision: after.runtime.schema_revision,
                    catalogue_revision: after.runtime.catalog_revision,
                    runtime_manifest_sha256: after.runtime.manifest_id,
                    retention_sha256: after.retention_sha256,
                    assembly_attempts: attempt,
                },
                readiness,
                product_capabilities,
                schema,
                models,
                graph,
                graph_difference,
                retention,
                vector_artifacts,
                query_indexes,
                vector_collections,
                estate,
                storage,
                changes,
                audit,
                sections,
            };
            snapshot
                .validate()
                .map_err(|error| ServiceError::Contract(error.to_string()))?;
            return Ok(snapshot);
        }

        Err(ServiceError::StorageConflict(format!(
            "diagnostic snapshot changed during all {MAX_DIAGNOSTIC_ASSEMBLY_ATTEMPTS} assembly attempts"
        )))
    }
}

#[allow(clippy::too_many_arguments)]
fn diagnostic_graph(
    read: &ReadStamp,
    changes: &[RuntimeChange],
    public_scope: &str,
    valid_at: u64,
    requested_known_at: Option<u64>,
    compare_cursor: u64,
) -> Result<(DiagnosticGraphSnapshot, DiagnosticGraphDifference)> {
    let known_at_cursor = requested_known_at.unwrap_or(read.commit_cursor);
    if known_at_cursor > read.commit_cursor {
        return Err(ServiceError::Contract(format!(
            "diagnostic graph cursor {known_at_cursor} exceeds captured head {}",
            read.commit_cursor
        )));
    }
    if compare_cursor > known_at_cursor {
        return Err(ServiceError::Contract(format!(
            "diagnostic graph comparison cursor {compare_cursor} exceeds target {known_at_cursor}"
        )));
    }
    let before =
        RuntimeGraphSnapshot::from_changes(changes, read.scope.clone(), valid_at, compare_cursor);
    let after =
        RuntimeGraphSnapshot::from_changes(changes, read.scope.clone(), valid_at, known_at_cursor);
    let difference = before.diff(&after);
    Ok((
        public_graph_snapshot(public_scope, &after)?,
        public_graph_difference(valid_at, &difference)?,
    ))
}

fn diagnostic_runtime_changes(
    engine: &RrdEngine,
    read: &ReadStamp,
    max_scanned_changes: u64,
) -> Result<Vec<RuntimeChange>> {
    if read.commit_cursor > max_scanned_changes {
        return Err(ServiceError::Contract(format!(
            "diagnostic runtime head {} exceeds the explicit replay budget {max_scanned_changes}; checkpointed projections are required for this range",
            read.commit_cursor
        )));
    }
    let mut after = 0;
    let mut changes = Vec::new();
    while after < read.commit_cursor {
        let remaining = read.commit_cursor - after;
        let limit = usize::try_from(remaining.min(1_024))
            .expect("diagnostic runtime page size is bounded by 1024");
        let page = engine.storage.runtime().read_changes(read, after, limit)?;
        changes.extend(page.changes);
        if page.through_cursor <= after {
            return Err(ServiceError::Storage(
                "diagnostic runtime replay made no cursor progress".into(),
            ));
        }
        after = page.through_cursor.min(read.commit_cursor);
    }
    Ok(changes)
}

fn diagnostic_vector_artifacts(
    changes: &[RuntimeChange],
    scope: &ScopeId,
    public_scope: &str,
) -> Result<DiagnosticVectorArtifactCatalogueSnapshot> {
    let entries = crate::runtime::vector_artifact_catalog_entries_from_changes(changes, scope)
        .map_err(|error| ServiceError::Vector(error.to_string()))?;
    let artifacts = entries
        .iter()
        .map(public_vector_artifact)
        .collect::<Result<Vec<_>>>()?;
    Ok(DiagnosticVectorArtifactCatalogueSnapshot {
        scope: public_scope.into(),
        revision: artifacts.last().map_or(0, |entry| entry.catalogue_revision),
        artifacts,
    })
}

fn public_vector_artifact(
    entry: &rrd_vector::VectorArtifactCatalogEntry,
) -> Result<DiagnosticVectorArtifactSnapshot> {
    let stamp = entry.descriptor.stamp();
    Ok(DiagnosticVectorArtifactSnapshot {
        catalogue_revision: entry.catalog_revision,
        kind: match entry.kind {
            rrd_vector::VectorArtifactKind::ExactSegment => {
                DiagnosticVectorArtifactKind::ExactSegment
            }
            rrd_vector::VectorArtifactKind::CompactDense => {
                DiagnosticVectorArtifactKind::CompactDense
            }
            rrd_vector::VectorArtifactKind::Hnsw => DiagnosticVectorArtifactKind::Hnsw,
            rrd_vector::VectorArtifactKind::ScalarQuantized
            | rrd_vector::VectorArtifactKind::ProductQuantized
            | rrd_vector::VectorArtifactKind::BinaryQuantized => {
                return Err(ServiceError::Vector(
                    "quantization lifecycle artifacts use their dedicated catalogue".into(),
                ))
            }
            rrd_vector::VectorArtifactKind::TurboQuant => DiagnosticVectorArtifactKind::TurboQuant,
        },
        scope: entry.scope().as_str().into(),
        projection_id: stamp.id.as_str().into(),
        generation: stamp.generation,
        source_cursor: stamp.source_cursor,
        config_sha256: stamp.config_digest.clone(),
        artifact_sha256: stamp.artifact_digest.clone(),
        object: public_diagnostic_reference(&entry.object.reference),
        object_sha256: entry.object.sha256.clone(),
        object_length: entry.object.length,
        media_type: entry.object.media_type.clone(),
        receipt: DataObjectReceipt {
            backend: entry.object.receipt.backend.clone(),
            key: entry.object.receipt.key.clone(),
            version: entry.object.receipt.version.clone(),
            etag: entry.object.receipt.etag.clone(),
        },
        published_at_unix_ms: entry.published_at,
        entry_sha256: entry.entry_digest.clone(),
    })
}

fn public_graph_snapshot(
    public_scope: &str,
    snapshot: &RuntimeGraphSnapshot,
) -> Result<DiagnosticGraphSnapshot> {
    Ok(DiagnosticGraphSnapshot {
        scope: public_scope.into(),
        valid_at_unix_ms: snapshot.valid_at,
        known_at_cursor: snapshot.known_at_cursor,
        records: snapshot
            .records
            .iter()
            .map(public_graph_record)
            .collect::<Result<_>>()?,
        relations: snapshot
            .relations
            .iter()
            .map(public_graph_relation)
            .collect::<Result<_>>()?,
    })
}

fn public_graph_difference(
    valid_at: u64,
    difference: &rrd_core::RuntimeGraphDiff,
) -> Result<DiagnosticGraphDifference> {
    Ok(DiagnosticGraphDifference {
        valid_at_unix_ms: valid_at,
        from_cursor: difference.from_cursor,
        to_cursor: difference.to_cursor,
        added_records: difference
            .added_records
            .iter()
            .map(public_graph_record)
            .collect::<Result<_>>()?,
        removed_records: difference
            .removed_records
            .iter()
            .map(public_graph_record)
            .collect::<Result<_>>()?,
        changed_records: difference
            .changed_records
            .iter()
            .map(|change| {
                Ok(DiagnosticGraphRecordChange {
                    before: public_graph_record(&change.before)?,
                    after: public_graph_record(&change.after)?,
                })
            })
            .collect::<Result<_>>()?,
        added_relations: difference
            .added_relations
            .iter()
            .map(public_graph_relation)
            .collect::<Result<_>>()?,
        removed_relations: difference
            .removed_relations
            .iter()
            .map(public_graph_relation)
            .collect::<Result<_>>()?,
        changed_relations: difference
            .changed_relations
            .iter()
            .map(|change| {
                Ok(DiagnosticGraphRelationChange {
                    before: public_graph_relation(&change.before)?,
                    after: public_graph_relation(&change.after)?,
                })
            })
            .collect::<Result<_>>()?,
    })
}

fn public_graph_record(record: &RuntimeRecord) -> Result<DiagnosticGraphRecordSnapshot> {
    Ok(DiagnosticGraphRecordSnapshot {
        reference: public_diagnostic_reference(&record.reference),
        valid_from_unix_ms: record.valid_from,
        valid_to_unix_ms: record.valid_to,
        properties: super::model::public_properties(&record.properties)?,
    })
}

fn public_graph_relation(relation: &RuntimeRelation) -> Result<DiagnosticGraphRelationSnapshot> {
    Ok(DiagnosticGraphRelationSnapshot {
        reference: public_diagnostic_reference(&relation.reference),
        from: public_diagnostic_reference(&relation.from),
        to: public_diagnostic_reference(&relation.to),
        valid_from_unix_ms: relation.valid_from,
        valid_to_unix_ms: relation.valid_to,
        properties: super::model::public_properties(&relation.properties)?,
    })
}

fn public_diagnostic_reference(reference: &RuntimeRef) -> DiagnosticRuntimeReference {
    DiagnosticRuntimeReference {
        kind: reference.kind.as_str().into(),
        id: reference.id.as_str().into(),
    }
}

fn diagnostic_models(
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

fn capture_diagnostic_stamp(
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

fn diagnostic_retention(engine: &RrdEngine, now: u64) -> Result<DiagnosticRetentionSnapshot> {
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

fn diagnostic_retention_sha256(retention: &DiagnosticRetentionSnapshot) -> Result<String> {
    let encoded =
        serde_json::to_vec(retention).map_err(|error| ServiceError::Contract(error.to_string()))?;
    Ok(digest::sha256_hex(&encoded))
}

fn diagnostic_query_indexes(
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
            .map(super::query::public_query_index)
            .collect::<Result<_>>()?,
    })
}

fn diagnostic_vector_collections(
    engine: &RrdEngine,
    scope: &ScopeId,
    public_scope: &str,
) -> Result<VectorCollectionCatalogueSnapshot> {
    let catalogue = rrd_vector::VectorCollectionRepository::new(&engine.storage, scope.clone())
        .load()
        .map_err(super::vector::vector_collection_error)?;
    Ok(VectorCollectionCatalogueSnapshot {
        scope: public_scope.into(),
        revision: catalogue.revision,
        collections: catalogue
            .collections
            .values()
            .map(super::vector::public_vector_collection)
            .collect::<Result<_>>()?,
    })
}

fn diagnostic_audit(engine: &RrdEngine, after: u64, limit: usize) -> Result<AuditPage> {
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
fn diagnostic_sections(
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

/// Projects the storage port's physical evidence into the public contract.
/// A profile that reports counters must report every one of them; a missing
/// counter or unknown level fails the read instead of rendering a guess.
fn diagnostic_storage(
    engine: &RrdEngine,
    observed_at_unix_ms: u64,
) -> Result<DiagnosticStorageSnapshot> {
    let evidence = engine.storage.physical_store_evidence()?;
    if evidence.backend != engine.storage.backend_name() {
        return Err(ServiceError::Storage(format!(
            "storage evidence names backend {} but the instance runs {}",
            evidence.backend,
            engine.storage.backend_name()
        )));
    }
    let (evidence_level, rrflow_kv) = match evidence.evidence_level.as_str() {
        rrd_store::LOGICAL_ONLY_EVIDENCE => (DiagnosticStorageEvidenceLevel::LogicalOnly, None),
        rrd_store::RRFLOW_KV_COUNTER_EVIDENCE => (
            DiagnosticStorageEvidenceLevel::RrflowKvCounters,
            Some(rrflow_kv_snapshot(&evidence)?),
        ),
        level => {
            return Err(ServiceError::Storage(format!(
                "storage reported unknown physical evidence level {level}"
            )))
        }
    };
    Ok(DiagnosticStorageSnapshot {
        observed_at_unix_ms,
        profile: engine.storage_profile_kind,
        evidence_level,
        rrflow_kv,
        admission: if evidence_level == DiagnosticStorageEvidenceLevel::RrflowKvCounters {
            let queue = engine.transaction_admission_snapshot()?;
            Some(DiagnosticStorageAdmission {
                checkpoint_entries: counter(evidence.checkpoint_entries, "checkpoint_entries")?,
                checkpoint_limit: counter(evidence.checkpoint_limit, "checkpoint_limit")?,
                native_transactions: counter(evidence.native_transactions, "native_transactions")?,
                native_transaction_bytes: counter(
                    evidence.native_transaction_bytes,
                    "native_transaction_bytes",
                )?,
                active_read_views: counter(evidence.active_read_views, "active_read_views")?,
                pinned_read_bytes: counter(evidence.pinned_read_bytes, "pinned_read_bytes")?,
                retained_mutable_bytes: counter(
                    evidence.retained_mutable_bytes,
                    "retained_mutable_bytes",
                )?,
                retained_source_bytes: counter(
                    evidence.retained_source_bytes,
                    "retained_source_bytes",
                )?,
                peak_source_bytes: counter(evidence.peak_source_bytes, "peak_source_bytes")?,
                denied_source_allocations: counter(
                    evidence.denied_source_allocations,
                    "denied_source_allocations",
                )?,
                retained_projected_bytes: counter(
                    evidence.retained_projected_bytes,
                    "retained_projected_bytes",
                )?,
                outstanding_requests: queue.outstanding_requests,
                outstanding_logical_bytes: queue.logical_bytes,
                queued_requests: queue.queued_requests as u64,
                active: queue.active,
                fenced: queue.fenced,
            })
        } else {
            None
        },
    })
}

fn counter(value: Option<u64>, name: &str) -> Result<u64> {
    value.ok_or_else(|| ServiceError::Storage(format!("rrflowKV physical evidence omitted {name}")))
}

fn segment_io_mode(mode: Option<&str>) -> Result<DiagnosticSegmentIoMode> {
    match mode {
        Some("auto") => Ok(DiagnosticSegmentIoMode::Auto),
        Some("mmap") => Ok(DiagnosticSegmentIoMode::Mmap),
        Some("io_uring") => Ok(DiagnosticSegmentIoMode::IoUring),
        Some("bounded") => Ok(DiagnosticSegmentIoMode::Bounded),
        mode => Err(ServiceError::Storage(format!(
            "rrflowKV physical evidence names unknown segment I/O mode {mode:?}"
        ))),
    }
}

/// A fallback reason is diagnostic text; it is bounded to one contract
/// message so a long error cannot fail the whole snapshot.
fn bounded_reason(reason: Option<&String>) -> Option<String> {
    let reason = reason?.replace('\0', "");
    if reason.is_empty() {
        return None;
    }
    let mut end = reason.len().min(MAX_MESSAGE_BYTES);
    while !reason.is_char_boundary(end) {
        end -= 1;
    }
    Some(reason[..end].to_owned())
}

fn rrflow_kv_snapshot(evidence: &PhysicalStoreEvidence) -> Result<DiagnosticRrflowKvSnapshot> {
    Ok(DiagnosticRrflowKvSnapshot {
        physical_sequence: counter(evidence.physical_sequence, "physical_sequence")?,
        manifest_generation: counter(evidence.manifest_generation, "manifest_generation")?,
        durable_sequence: counter(evidence.durable_sequence, "durable_sequence")?,
        memtable: DiagnosticMemtableSnapshot {
            versions: counter(evidence.memtable_versions, "memtable_versions")?,
            max_versions: counter(evidence.memtable_max_versions, "memtable_max_versions")?,
            approximate_bytes: counter(evidence.memtable_bytes, "memtable_bytes")?,
            keys: counter(evidence.memtable_keys, "memtable_keys")?,
            key_payload_bytes: counter(
                evidence.memtable_key_payload_bytes,
                "memtable_key_payload_bytes",
            )?,
            value_payload_bytes: counter(
                evidence.memtable_value_payload_bytes,
                "memtable_value_payload_bytes",
            )?,
            tombstones: counter(evidence.memtable_tombstones, "memtable_tombstones")?,
            spilled_chains: counter(evidence.memtable_spilled_chains, "memtable_spilled_chains")?,
            spilled_version_capacity: counter(
                evidence.memtable_spilled_version_capacity,
                "memtable_spilled_version_capacity",
            )?,
            version_record_bytes: counter(
                evidence.memtable_version_record_bytes,
                "memtable_version_record_bytes",
            )?,
            owned_bytes_lower_bound: counter(
                evidence.memtable_owned_bytes_lower_bound,
                "memtable_owned_bytes_lower_bound",
            )?,
        },
        write_ahead_log: DiagnosticWriteAheadLogSnapshot {
            payload_bytes: counter(evidence.wal_payload_bytes, "wal_payload_bytes")?,
            payload_max_bytes: counter(evidence.wal_payload_max_bytes, "wal_payload_max_bytes")?,
        },
        maintenance: DiagnosticMaintenanceSnapshot {
            automatic_flushes: counter(evidence.automatic_flushes, "automatic_flushes")?,
            failed_flushes: counter(
                evidence.failed_maintenance_flushes,
                "failed_maintenance_flushes",
            )?,
            write_stalls: counter(
                evidence.maintenance_write_stalls,
                "maintenance_write_stalls",
            )?,
            oversized_batches: counter(evidence.oversized_batches, "oversized_batches")?,
            automatic_compactions: counter(
                evidence.automatic_compactions,
                "automatic_compactions",
            )?,
            failed_compactions: counter(evidence.failed_compactions, "failed_compactions")?,
            compaction_input_bytes: counter(
                evidence.compaction_input_bytes,
                "compaction_input_bytes",
            )?,
            compaction_output_bytes: counter(
                evidence.compaction_output_bytes,
                "compaction_output_bytes",
            )?,
            peak_compaction_buffer_bytes: counter(
                evidence.peak_compaction_buffer_bytes,
                "peak_compaction_buffer_bytes",
            )?,
        },
        segment_set: DiagnosticSegmentSetSnapshot {
            segments: counter(evidence.segment_count, "segment_count")?,
            segment_bytes: counter(evidence.segment_bytes, "segment_bytes")?,
            l0_segments: counter(evidence.l0_segment_count, "l0_segment_count")?,
            l0_compaction_trigger: counter(
                evidence.l0_compaction_trigger,
                "l0_compaction_trigger",
            )?,
            compaction_debt_segments: counter(
                evidence.compaction_debt_segments,
                "compaction_debt_segments",
            )?,
            compaction_target_segment_bytes: counter(
                evidence.compaction_target_segment_bytes,
                "compaction_target_segment_bytes",
            )?,
        },
        page_cache: DiagnosticPageCacheSnapshot {
            capacity_bytes: counter(evidence.cache_capacity_bytes, "cache_capacity_bytes")?,
            resident_bytes: counter(evidence.cache_resident_bytes, "cache_resident_bytes")?,
            entries: counter(evidence.cache_entries, "cache_entries")?,
            hits: counter(evidence.cache_hits, "cache_hits")?,
            misses: counter(evidence.cache_misses, "cache_misses")?,
            evictions: counter(evidence.cache_evictions, "cache_evictions")?,
            page_loads: counter(evidence.page_loads, "page_loads")?,
            bytes_read: counter(evidence.page_bytes_read, "page_bytes_read")?,
            bytes_decoded: counter(evidence.page_bytes_decoded, "page_bytes_decoded")?,
            bytes_borrowed: counter(evidence.page_bytes_borrowed, "page_bytes_borrowed")?,
            bytes_allocated: counter(evidence.page_bytes_allocated, "page_bytes_allocated")?,
            bytes_copied: counter(evidence.page_bytes_copied, "page_bytes_copied")?,
            bytes_decompressed: counter(
                evidence.page_bytes_decompressed,
                "page_bytes_decompressed",
            )?,
            filter_checks: counter(evidence.filter_checks, "filter_checks")?,
            filter_negatives: counter(evidence.filter_negatives, "filter_negatives")?,
        },
        segment_io: DiagnosticSegmentIoSnapshot {
            max_open_files: counter(
                evidence.segment_io_max_open_files,
                "segment_io_max_open_files",
            )?,
            open_files: counter(evidence.segment_io_open_files, "segment_io_open_files")?,
            peak_open_files: counter(
                evidence.segment_io_peak_open_files,
                "segment_io_peak_open_files",
            )?,
            handle_evictions: counter(
                evidence.segment_io_handle_evictions,
                "segment_io_handle_evictions",
            )?,
            handle_reopens: counter(
                evidence.segment_io_handle_reopens,
                "segment_io_handle_reopens",
            )?,
            handle_denials: counter(
                evidence.segment_io_handle_denials,
                "segment_io_handle_denials",
            )?,

            requested_mode: segment_io_mode(evidence.segment_io_requested_mode.as_deref())?,
            mmap_segments: counter(
                evidence.segment_io_mmap_segments,
                "segment_io_mmap_segments",
            )?,
            io_uring_segments: counter(
                evidence.segment_io_uring_segments,
                "segment_io_uring_segments",
            )?,
            bounded_segments: counter(
                evidence.segment_io_bounded_segments,
                "segment_io_bounded_segments",
            )?,
            fallbacks: counter(evidence.segment_io_fallbacks, "segment_io_fallbacks")?,
            last_fallback_reason: bounded_reason(evidence.segment_io_last_fallback.as_ref()),
            read_operations: counter(
                evidence.segment_io_read_operations,
                "segment_io_read_operations",
            )?,
            mmap_reads: counter(evidence.segment_io_mmap_reads, "segment_io_mmap_reads")?,
            io_uring_reads: counter(evidence.segment_io_uring_reads, "segment_io_uring_reads")?,
            bounded_reads: counter(
                evidence.segment_io_bounded_reads,
                "segment_io_bounded_reads",
            )?,
            bytes_read: counter(evidence.segment_io_bytes_read, "segment_io_bytes_read")?,
            max_request_bytes: counter(
                evidence.segment_io_max_request_bytes,
                "segment_io_max_request_bytes",
            )?,
            peak_request_bytes: counter(
                evidence.segment_io_peak_request_bytes,
                "segment_io_peak_request_bytes",
            )?,
        },
    })
}

fn graph_difference_row_count(difference: &DiagnosticGraphDifference) -> usize {
    difference
        .added_records
        .len()
        .saturating_add(difference.removed_records.len())
        .saturating_add(difference.changed_records.len())
        .saturating_add(difference.added_relations.len())
        .saturating_add(difference.removed_relations.len())
        .saturating_add(difference.changed_relations.len())
}

fn diagnostic_count(value: usize) -> Result<u64> {
    u64::try_from(value).map_err(|_| ServiceError::Contract("diagnostic count exceeds u64".into()))
}

fn section(
    id: &str,
    authority: DiagnosticAuthority,
    coverage: DiagnosticCoverage,
    row_count: usize,
    known_at_cursor: u64,
    requested_after: Option<u64>,
) -> Result<DiagnosticSectionSnapshot> {
    Ok(DiagnosticSectionSnapshot {
        id: CanonicalId::new(id).map_err(|error| ServiceError::Contract(error.to_string()))?,
        authority,
        coverage,
        row_count: u64::try_from(row_count)
            .map_err(|_| ServiceError::Contract("diagnostic row count exceeds u64".into()))?,
        known_at_cursor,
        requested_after,
    })
}

#[cfg(test)]
mod storage_tests {
    use super::*;

    #[test]
    fn rrflow_kv_projection_fails_closed_on_a_missing_counter() {
        let evidence = PhysicalStoreEvidence::logical_only("rrflow_kv");
        let error = rrflow_kv_snapshot(&evidence).unwrap_err();
        assert!(error.to_string().contains("physical_sequence"));
    }

    #[test]
    fn segment_io_modes_map_exactly() {
        for (text, mode) in [
            ("auto", DiagnosticSegmentIoMode::Auto),
            ("mmap", DiagnosticSegmentIoMode::Mmap),
            ("io_uring", DiagnosticSegmentIoMode::IoUring),
            ("bounded", DiagnosticSegmentIoMode::Bounded),
        ] {
            assert_eq!(segment_io_mode(Some(text)).unwrap(), mode);
        }
        assert!(segment_io_mode(Some("uring")).is_err());
        assert!(segment_io_mode(None).is_err());
    }

    #[test]
    fn fallback_reasons_are_bounded_contract_messages() {
        assert_eq!(bounded_reason(None), None);
        assert_eq!(bounded_reason(Some(&String::new())), None);
        assert_eq!(
            bounded_reason(Some(&"a\0b".to_owned())),
            Some("ab".to_owned())
        );
        let long = "é".repeat(MAX_MESSAGE_BYTES);
        let bounded = bounded_reason(Some(&long)).unwrap();
        assert!(bounded.len() <= MAX_MESSAGE_BYTES);
        assert!(bounded.chars().all(|character| character == 'é'));
    }
}
