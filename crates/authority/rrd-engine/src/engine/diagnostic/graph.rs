//! Diagnostic graph, records, runtime changes, vector artifacts and public graph projections.

use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn diagnostic_graph(
    read: &ReadStamp,
    changes: &[RuntimeChange],
    public_scope: &str,
    valid_at: u64,
    requested_known_at: Option<u64>,
    compare_cursor: u64,
    max_records: Option<u64>,
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
    let keep = bounded_records(&after, max_records);
    Ok((
        public_graph_snapshot(public_scope, &after, keep.as_ref())?,
        public_graph_difference(valid_at, &difference, keep.as_ref())?,
    ))
}

/// The records a bounded read returns: at most `max`, the most linked first,
/// ties broken by reference so the same graph always yields the same set.
/// `None` when nothing is left out.
pub(super) fn bounded_records(
    snapshot: &RuntimeGraphSnapshot,
    max: Option<u64>,
) -> Option<std::collections::BTreeSet<RuntimeRef>> {
    let max = usize::try_from(max?).unwrap_or(usize::MAX);
    if snapshot.records.len() <= max {
        return None;
    }
    let mut degree = std::collections::BTreeMap::<&RuntimeRef, u64>::new();
    for relation in &snapshot.relations {
        *degree.entry(&relation.from).or_default() += 1;
        *degree.entry(&relation.to).or_default() += 1;
    }
    let links = |reference: &RuntimeRef| degree.get(reference).copied().unwrap_or(0);
    let mut ranked: Vec<&RuntimeRef> = snapshot
        .records
        .iter()
        .map(|record| &record.reference)
        .collect();
    ranked.sort_by(|a, b| links(b).cmp(&links(a)).then_with(|| a.cmp(b)));
    Some(ranked.into_iter().take(max).cloned().collect())
}

pub(super) fn diagnostic_runtime_changes(
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

pub(super) fn diagnostic_vector_artifacts(
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

pub(super) fn public_vector_artifact(
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

pub(super) fn public_graph_snapshot(
    public_scope: &str,
    snapshot: &RuntimeGraphSnapshot,
    keep: Option<&std::collections::BTreeSet<RuntimeRef>>,
) -> Result<DiagnosticGraphSnapshot> {
    let kept = |reference: &RuntimeRef| keep.is_none_or(|keep| keep.contains(reference));
    Ok(DiagnosticGraphSnapshot {
        scope: public_scope.into(),
        valid_at_unix_ms: snapshot.valid_at,
        known_at_cursor: snapshot.known_at_cursor,
        records: snapshot
            .records
            .iter()
            .filter(|record| kept(&record.reference))
            .map(public_graph_record)
            .collect::<Result<_>>()?,
        relations: snapshot
            .relations
            .iter()
            .filter(|relation| kept(&relation.from) && kept(&relation.to))
            .map(public_graph_relation)
            .collect::<Result<_>>()?,
        records_total: snapshot.records.len() as u64,
        relations_total: snapshot.relations.len() as u64,
    })
}

pub(super) fn public_graph_difference(
    valid_at: u64,
    difference: &rrd_core::RuntimeGraphDiff,
    keep: Option<&std::collections::BTreeSet<RuntimeRef>>,
) -> Result<DiagnosticGraphDifference> {
    let kept = |reference: &RuntimeRef| keep.is_none_or(|keep| keep.contains(reference));
    let record = |record: &&RuntimeRecord| kept(&record.reference);
    let relation = |relation: &&RuntimeRelation| kept(&relation.from) && kept(&relation.to);
    let total = difference.added_records.len()
        + difference.removed_records.len()
        + difference.changed_records.len()
        + difference.added_relations.len()
        + difference.removed_relations.len()
        + difference.changed_relations.len();
    let mut public = DiagnosticGraphDifference {
        valid_at_unix_ms: valid_at,
        from_cursor: difference.from_cursor,
        to_cursor: difference.to_cursor,
        added_records: difference
            .added_records
            .iter()
            .filter(record)
            .map(public_graph_record)
            .collect::<Result<_>>()?,
        removed_records: difference
            .removed_records
            .iter()
            .filter(record)
            .map(public_graph_record)
            .collect::<Result<_>>()?,
        changed_records: difference
            .changed_records
            .iter()
            .filter(|change| kept(&change.after.reference))
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
            .filter(relation)
            .map(public_graph_relation)
            .collect::<Result<_>>()?,
        removed_relations: difference
            .removed_relations
            .iter()
            .filter(relation)
            .map(public_graph_relation)
            .collect::<Result<_>>()?,
        changed_relations: difference
            .changed_relations
            .iter()
            .filter(|change| kept(&change.after.from) && kept(&change.after.to))
            .map(|change| {
                Ok(DiagnosticGraphRelationChange {
                    before: public_graph_relation(&change.before)?,
                    after: public_graph_relation(&change.after)?,
                })
            })
            .collect::<Result<_>>()?,
        omitted_entries: 0,
    };
    let returned = public.added_records.len()
        + public.removed_records.len()
        + public.changed_records.len()
        + public.added_relations.len()
        + public.removed_relations.len()
        + public.changed_relations.len();
    public.omitted_entries = (total - returned) as u64;
    Ok(public)
}

pub(super) fn public_graph_record(record: &RuntimeRecord) -> Result<DiagnosticGraphRecordSnapshot> {
    Ok(DiagnosticGraphRecordSnapshot {
        reference: public_diagnostic_reference(&record.reference),
        valid_from_unix_ms: record.valid_from,
        valid_to_unix_ms: record.valid_to,
        properties: super::super::model::public_properties(&record.properties)?,
    })
}

pub(super) fn public_graph_relation(
    relation: &RuntimeRelation,
) -> Result<DiagnosticGraphRelationSnapshot> {
    Ok(DiagnosticGraphRelationSnapshot {
        reference: public_diagnostic_reference(&relation.reference),
        from: public_diagnostic_reference(&relation.from),
        to: public_diagnostic_reference(&relation.to),
        valid_from_unix_ms: relation.valid_from,
        valid_to_unix_ms: relation.valid_to,
        properties: super::super::model::public_properties(&relation.properties)?,
    })
}

pub(super) fn public_diagnostic_reference(reference: &RuntimeRef) -> DiagnosticRuntimeReference {
    DiagnosticRuntimeReference {
        kind: reference.kind.as_str().into(),
        id: reference.id.as_str().into(),
    }
}
