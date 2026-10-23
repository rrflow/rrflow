//! One encoded semantic write plan for rrflowMX and rrflowKV.
//!
//! The plan is prepared against one captured storage snapshot. Preparation
//! validates logical state and encodes the complete semantic effect set,
//! including scope-bound versions, graph adjacency, index source deltas,
//! governed-function receipts, projection work, audit, and outcome state. It
//! cannot publish. The owning repository applies the plan to the same open
//! transaction and only that transaction's commit receipt makes its effects
//! authoritative.

use super::runtime_state::{
    checked_key, current_accumulator, get, get_json, read_sequence, read_stamp_with,
    validate_read_stamp, values_for_scope, AccessRead,
};
use super::{read_versioned, schema_at_read, RuntimeReadBudget, RuntimeVersionedSource};
use crate::keyspaces::{self, Space};
use crate::transaction::TransactionWriteSet;
use crate::{Error, FunctionInvocationReceiptRecord, Result, StorageTransaction};
use rrd_core::{
    projection_family, AuditEnvelope, ProjectionWork, ReadStamp, RuntimeChange, RuntimeCommit,
    RuntimeCommitOutcome, RuntimeDataSnapshot, RuntimeMutation, RuntimeRecord, RuntimeRef,
    RuntimeRelation, RuntimeSchemaRegistry,
};
use std::collections::{BTreeMap, BTreeSet};

mod constraints;

#[cfg(test)]
#[path = "semantic_commit/constraint_tests.rs"]
mod constraint_tests;

/// A validated semantic commit plan that has not been published.
///
/// Encoded writes are private to ordinary repository callers, which can only
/// apply the complete plan. The temporary rrflowKV Raft bridge still receives
/// a raw operation vector so it can append coordinator metadata; that known
/// distributed-authority gap remains explicitly open and must be removed by
/// its owning convergence package.
#[derive(Debug)]
pub struct SemanticCommitPlan {
    outcome: RuntimeCommitOutcome,
    writes: TransactionWriteSet,
}

impl SemanticCommitPlan {
    pub fn outcome(&self) -> &RuntimeCommitOutcome {
        &self.outcome
    }

    pub(crate) fn apply(
        self,
        transaction: &mut dyn StorageTransaction,
    ) -> Result<RuntimeCommitOutcome> {
        let Self { outcome, writes } = self;
        for (key, value) in writes.into_mutations() {
            match value {
                Some(value) => transaction.put(key, value)?,
                None => transaction.delete(key)?,
            }
        }
        Ok(outcome)
    }

    pub(crate) fn into_encoded_writes(
        self,
    ) -> (RuntimeCommitOutcome, BTreeMap<Vec<u8>, Option<Vec<u8>>>) {
        (self.outcome, self.writes.into_mutations())
    }

    pub(crate) fn put_bytes(&mut self, space: Space, key: &[u8], value: Vec<u8>) -> Result<()> {
        self.writes.put(checked_key(space, key)?, value)
    }

    pub(crate) fn put_json<T: serde::Serialize>(
        &mut self,
        space: Space,
        key: &[u8],
        value: &T,
    ) -> Result<()> {
        self.put_bytes(space, key, serde_json::to_vec(value)?)
    }

    fn put_sequence(&mut self, key: &[u8], sequence: u64) -> Result<()> {
        self.put_bytes(keyspaces::META, key, sequence.to_string().into_bytes())
    }

    pub(crate) fn delete(&mut self, space: Space, key: &[u8]) -> Result<()> {
        self.writes.delete(checked_key(space, key)?)
    }
}

pub(crate) fn prepare_semantic_commit(
    reader: &(impl AccessRead + ?Sized),
    commit: &RuntimeCommit,
    read: Option<&ReadStamp>,
    archived_audit: Option<&AuditEnvelope>,
    function_receipts: Option<(&str, &[FunctionInvocationReceiptRecord])>,
) -> Result<SemanticCommitPlan> {
    commit.validate()?;
    if read.is_some() && archived_audit.is_some() {
        return Err(Error::Archive(
            "archive replay cannot also supply a live read stamp".into(),
        ));
    }
    if let Some(audit) = archived_audit {
        audit.validate()?;
        if let Some(read) = &audit.read {
            read.validate()?;
        }
    } else if let Some(read) = read {
        validate_read_stamp(reader, read)?;
    }

    let commit_id = commit.digest();
    let start = read_sequence(reader, &keyspaces::runtime_cursor_key())?;
    if start != commit.expected_cursor {
        return Err(Error::RuntimeConflict {
            expected: commit.expected_cursor,
            actual: start,
        });
    }
    let mut accumulator = current_accumulator(reader, start)?;
    validate_retirement_targets(reader, commit, start)?;
    let effective_schema = validate_schema_and_objects(reader, commit)?;
    validate_references(reader, commit)?;
    let catalogue_revision =
        read_sequence(reader, &keyspaces::catalog_revision_key(&commit.scope))?;

    let claim_count = commit
        .mutations
        .iter()
        .filter(|mutation| matches!(mutation, RuntimeMutation::Claim { .. }))
        .count();
    let claim_start = read_sequence(reader, &keyspaces::sequence_watermark_key())?;
    let mut claim_sequence = claim_start;
    let mut cursor = start;
    let mut previous_digest = get(
        reader,
        keyspaces::META,
        &keyspaces::runtime_last_digest_key(),
    )?
    .map(String::from_utf8)
    .transpose()
    .map_err(|error| Error::CorruptWatermark(error.to_string()))?
    .filter(|digest| !digest.is_empty());
    let previous_audit_digest = get(
        reader,
        keyspaces::META,
        &keyspaces::runtime_last_audit_digest_key(),
    )?
    .map(String::from_utf8)
    .transpose()
    .map_err(|error| Error::CorruptWatermark(error.to_string()))?
    .filter(|digest| !digest.is_empty());

    let mut plan = SemanticCommitPlan {
        outcome: RuntimeCommitOutcome {
            commit_id: commit_id.clone(),
            first_cursor: 0,
            last_cursor: 0,
            count: commit.mutations.len(),
            first_claim_sequence: None,
            last_claim_sequence: None,
            outbox_count: 0,
        },
        writes: TransactionWriteSet::default(),
    };
    let mut outbox_count = 0;
    let mut staged_relations = BTreeMap::<RuntimeRef, Option<RuntimeRelation>>::new();
    for (ordinal, mutation) in commit.mutations.iter().enumerate() {
        if let RuntimeMutation::Claim { claim } = mutation {
            claim.validate()?;
            claim_sequence = claim_sequence
                .checked_add(1)
                .ok_or(Error::SequenceOverflow)?;
            let claim_key = keyspaces::claim_key(
                &claim.subject,
                &claim.predicate,
                claim.valid_from,
                claim.tx_time,
            );
            plan.put_json(keyspaces::CLAIMS, &claim_key, claim)?;
            plan.put_bytes(
                keyspaces::SEQUENCE_INDEX,
                &keyspaces::sequence_key(claim_sequence),
                claim_key,
            )?;
        }

        cursor = cursor.checked_add(1).ok_or(Error::SequenceOverflow)?;
        let change = RuntimeChange::committed(
            cursor,
            commit,
            &commit_id,
            ordinal as u64,
            mutation.clone(),
            previous_digest.clone(),
        );
        plan.put_json(
            keyspaces::RUNTIME_CHANGES,
            &keyspaces::runtime_change_key(cursor),
            &change,
        )?;
        for node in accumulator.append_change(&change)? {
            plan.put_bytes(
                keyspaces::META,
                &keyspaces::runtime_accumulator_node_key(node.level, node.index),
                node.digest.into_bytes(),
            )?;
        }
        if let Some(family) = projection_family(mutation) {
            let work = ProjectionWork::for_change(
                commit.scope.clone(),
                cursor,
                commit_id.clone(),
                ordinal as u64,
                family,
            )?;
            plan.put_json(
                keyspaces::RUNTIME_PROJECTION_DELTAS,
                &keyspaces::runtime_projection_delta_key(&work),
                &work,
            )?;
            plan.put_json(
                keyspaces::RUNTIME_OUTBOX,
                &keyspaces::runtime_outbox_key(cursor),
                &work,
            )?;
            outbox_count += 1;
        }
        encode_domain_effect(
            reader,
            &mut plan,
            &mut staged_relations,
            commit,
            mutation,
            &change,
        )?;
        previous_digest = Some(change.digest);
    }

    if let Some(schema) = effective_schema.as_ref() {
        super::encode_record_index_effects(
            reader,
            &mut plan,
            commit,
            schema,
            catalogue_revision,
            start,
        )?;
    }
    super::encode_vector_source_effects(reader, &mut plan, commit, start)?;
    if let Some((instance, receipts)) = function_receipts {
        super::encode_function_invocation_receipts(
            reader, &mut plan, instance, &commit_id, receipts,
        )?;
    }

    if claim_count > 0 {
        plan.put_sequence(&keyspaces::sequence_watermark_key(), claim_sequence)?;
    }
    plan.put_sequence(&keyspaces::runtime_cursor_key(), cursor)?;
    plan.put_bytes(
        keyspaces::META,
        &keyspaces::runtime_last_digest_key(),
        previous_digest.as_deref().unwrap_or("").as_bytes().to_vec(),
    )?;
    plan.put_json(
        keyspaces::META,
        &keyspaces::runtime_accumulator_state_key(),
        &accumulator,
    )?;
    // Every semantic transaction writes the catalogue watermark it observed.
    // A concurrent catalogue transition therefore conflicts at physical
    // commit even for internal callers that did not supply a ReadStamp.
    plan.put_sequence(
        &keyspaces::catalog_revision_key(&commit.scope),
        catalogue_revision,
    )?;

    let audit_read = archived_audit
        .and_then(|audit| audit.read.as_ref())
        .or(read);
    let audit = AuditEnvelope::accepted_commit_at_read(
        commit,
        audit_read,
        &commit_id,
        cursor,
        previous_audit_digest,
    )?;
    if archived_audit.is_some_and(|expected| expected != &audit) {
        return Err(Error::Archive(format!(
            "runtime commit {commit_id} audit envelope differs from its archive"
        )));
    }
    plan.put_json(
        keyspaces::RUNTIME_AUDIT,
        &keyspaces::runtime_audit_key(&commit_id),
        &audit,
    )?;
    plan.put_bytes(
        keyspaces::META,
        &keyspaces::runtime_last_audit_digest_key(),
        audit.digest.as_bytes().to_vec(),
    )?;

    let outcome = RuntimeCommitOutcome {
        commit_id,
        first_cursor: start.checked_add(1).ok_or(Error::SequenceOverflow)?,
        last_cursor: cursor,
        count: commit.mutations.len(),
        first_claim_sequence: (claim_count > 0)
            .then_some(claim_start.checked_add(1).ok_or(Error::SequenceOverflow)?),
        last_claim_sequence: (claim_count > 0).then_some(claim_sequence),
        outbox_count,
    };
    plan.put_json(
        keyspaces::RUNTIME_COMMITS,
        &keyspaces::runtime_commit_key(&outcome.commit_id),
        &outcome,
    )?;
    plan.outcome = outcome;
    Ok(plan)
}

fn validate_schema_and_objects(
    reader: &(impl AccessRead + ?Sized),
    commit: &RuntimeCommit,
) -> Result<Option<RuntimeSchemaRegistry>> {
    let previous_schema: Option<RuntimeSchemaRegistry> = get_json(
        reader,
        keyspaces::RUNTIME_SCHEMAS,
        &keyspaces::runtime_schema_key(&commit.scope),
    )?;
    let proposed_schema = commit.mutations.iter().find_map(|mutation| match mutation {
        RuntimeMutation::Schema { registry } => Some(registry),
        _ => None,
    });
    let schema_free_claims = previous_schema.is_none()
        && proposed_schema.is_none()
        && commit
            .mutations
            .iter()
            .all(|mutation| matches!(mutation, RuntimeMutation::Claim { .. }));
    if schema_free_claims {
        return Ok(None);
    }
    let effective_schema = match (previous_schema.as_ref(), proposed_schema) {
        (None, Some(registry)) if registry.revision == 1 => registry,
        (None, Some(registry)) => {
            return Err(Error::RuntimeSchemaConflict {
                expected: 1,
                actual: registry.revision,
            });
        }
        (Some(previous), Some(registry))
            if registry.revision == previous.revision.saturating_add(1) =>
        {
            registry
        }
        (Some(previous), Some(registry)) => {
            return Err(Error::RuntimeSchemaConflict {
                expected: previous.revision.saturating_add(1),
                actual: registry.revision,
            });
        }
        (Some(previous), None) => previous,
        (None, None) => return Err(Error::RuntimeSchemaMissing(commit.scope.to_string())),
    };
    let (existing_records, existing_relations) =
        constraints::load(reader, commit, effective_schema, proposed_schema.is_some())?;
    effective_schema.validate_objects(&commit.mutations, &existing_records, &existing_relations)?;
    Ok(Some(effective_schema.clone()))
}

fn validate_references(reader: &(impl AccessRead + ?Sized), commit: &RuntimeCommit) -> Result<()> {
    let new_records = commit
        .mutations
        .iter()
        .filter_map(|mutation| match mutation {
            RuntimeMutation::Record { record } => Some(record.reference.clone()),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    for mutation in &commit.mutations {
        let references: Vec<&RuntimeRef> = match mutation {
            RuntimeMutation::Relation { relation } => vec![&relation.from, &relation.to],
            RuntimeMutation::Event { event } => event.subject.iter().collect(),
            RuntimeMutation::Vector { vector } => vec![&vector.subject],
            RuntimeMutation::SeriesSample { sample } => vec![&sample.series],
            RuntimeMutation::Geo { geo } => vec![&geo.subject],
            RuntimeMutation::Object { object } => object.subject.iter().collect(),
            RuntimeMutation::Claim { .. }
            | RuntimeMutation::Schema { .. }
            | RuntimeMutation::Record { .. }
            | RuntimeMutation::Retire { .. } => Vec::new(),
        };
        for reference in references {
            if !new_records.contains(reference)
                && get(
                    reader,
                    keyspaces::RUNTIME_RECORDS,
                    &keyspaces::runtime_identity_key(
                        keyspaces::RUNTIME_RECORDS,
                        &commit.scope,
                        reference,
                    ),
                )?
                .is_none()
            {
                return Err(Error::DanglingRuntimeReference(format!(
                    "{}/{} in scope {}",
                    reference.kind, reference.id, commit.scope
                )));
            }
        }
    }
    Ok(())
}

fn validate_retirement_targets(
    reader: &(impl AccessRead + ?Sized),
    commit: &RuntimeCommit,
    head: u64,
) -> Result<()> {
    let retirements = commit
        .mutations
        .iter()
        .filter_map(|mutation| match mutation {
            RuntimeMutation::Retire { retirement } => Some(retirement),
            _ => None,
        })
        .collect::<Vec<_>>();
    if retirements.is_empty() {
        return Ok(());
    }
    let read = read_stamp_with(reader, &commit.scope)?;
    if read.commit_cursor != head {
        return Err(Error::ReadStampUnavailable(format!(
            "retirement validation for {} moved from cursor {head} to {}",
            commit.scope, read.commit_cursor
        )));
    }
    let mut sources = vec![RuntimeVersionedSource::Schema];
    sources.extend(
        retirements
            .iter()
            .map(|retirement| RuntimeVersionedSource::Identity {
                model: retirement.model,
                reference: retirement.reference.clone(),
            }),
    );
    let direct = read_versioned(
        reader,
        &read,
        &sources,
        RuntimeReadBudget::authenticated_history_integrity(head)?,
    )?;
    let schema = schema_at_read(&read, &direct.changes)?;
    let mut snapshots = BTreeMap::new();
    for retirement in retirements {
        let snapshot = match snapshots.entry(retirement.effective_at) {
            std::collections::btree_map::Entry::Occupied(entry) => entry.into_mut(),
            std::collections::btree_map::Entry::Vacant(entry) => entry.insert(
                RuntimeDataSnapshot::from_changes(
                    &direct.changes,
                    &schema,
                    commit.scope.clone(),
                    retirement.effective_at,
                    head,
                )
                .map_err(Error::from)?,
            ),
        };
        if !snapshot.contains(retirement.model, &retirement.reference) {
            return Err(Error::RuntimeTargetNotFound(format!(
                "{:?} {}/{} at {} in scope {}",
                retirement.model,
                retirement.reference.kind,
                retirement.reference.id,
                retirement.effective_at,
                commit.scope
            )));
        }
    }
    Ok(())
}

fn encode_domain_effect(
    reader: &(impl AccessRead + ?Sized),
    plan: &mut SemanticCommitPlan,
    staged_relations: &mut BTreeMap<RuntimeRef, Option<RuntimeRelation>>,
    commit: &RuntimeCommit,
    mutation: &RuntimeMutation,
    change: &RuntimeChange,
) -> Result<()> {
    match mutation {
        RuntimeMutation::Schema { registry } => {
            plan.put_json(
                keyspaces::RUNTIME_SCHEMAS,
                &keyspaces::runtime_schema_key(&commit.scope),
                registry,
            )?;
            plan.put_json(
                keyspaces::RUNTIME_SCHEMA_VERSIONS,
                &keyspaces::runtime_schema_version_key(&commit.scope, change.cursor),
                change,
            )
        }
        RuntimeMutation::Claim { claim } => plan.put_json(
            keyspaces::RUNTIME_CLAIM_VERSIONS,
            &keyspaces::runtime_claim_version_key(&commit.scope, claim, change.cursor),
            change,
        ),
        RuntimeMutation::Record { record } => {
            plan.put_json(
                keyspaces::RUNTIME_RECORDS,
                &keyspaces::runtime_identity_key(
                    keyspaces::RUNTIME_RECORDS,
                    &commit.scope,
                    &record.reference,
                ),
                record,
            )?;
            plan.put_json(
                keyspaces::RUNTIME_RECORD_VERSIONS,
                &keyspaces::runtime_version_key(
                    keyspaces::RUNTIME_RECORD_VERSIONS,
                    &commit.scope,
                    &record.reference,
                    record.valid_from,
                    change.cursor,
                ),
                change,
            )
        }
        RuntimeMutation::Relation { relation } => {
            if let Some(previous) =
                current_relation(reader, staged_relations, &commit.scope, &relation.reference)?
            {
                delete_current_adjacency(plan, &commit.scope, &previous)?;
            }
            plan.put_json(
                keyspaces::RUNTIME_RELATIONS,
                &keyspaces::runtime_identity_key(
                    keyspaces::RUNTIME_RELATIONS,
                    &commit.scope,
                    &relation.reference,
                ),
                relation,
            )?;
            plan.put_json(
                keyspaces::RUNTIME_RELATION_VERSIONS,
                &keyspaces::runtime_version_key(
                    keyspaces::RUNTIME_RELATION_VERSIONS,
                    &commit.scope,
                    &relation.reference,
                    relation.valid_from,
                    change.cursor,
                ),
                change,
            )?;
            put_adjacency(plan, &commit.scope, relation, relation.valid_from, change)?;
            staged_relations.insert(relation.reference.clone(), Some(relation.clone()));
            Ok(())
        }
        RuntimeMutation::Vector { vector } => {
            super::encode_vector_version(plan, &commit.scope, vector, change)
        }
        RuntimeMutation::Event { event } => {
            let reference = keyspaces::runtime_event_reference(event, change.cursor)?;
            plan.put_json(
                keyspaces::RUNTIME_EVENT_VERSIONS,
                &keyspaces::runtime_version_key(
                    keyspaces::RUNTIME_EVENT_VERSIONS,
                    &commit.scope,
                    &reference,
                    change.at,
                    change.cursor,
                ),
                change,
            )
        }
        RuntimeMutation::SeriesSample { sample } => {
            plan.put_json(
                keyspaces::RUNTIME_SERIES,
                &keyspaces::runtime_identity_key(
                    keyspaces::RUNTIME_SERIES,
                    &commit.scope,
                    &sample.reference,
                ),
                sample,
            )?;
            plan.put_json(
                keyspaces::RUNTIME_SERIES_VERSIONS,
                &keyspaces::runtime_version_key(
                    keyspaces::RUNTIME_SERIES_VERSIONS,
                    &commit.scope,
                    &sample.reference,
                    sample.observed_at,
                    change.cursor,
                ),
                change,
            )
        }
        RuntimeMutation::Geo { geo } => {
            plan.put_json(
                keyspaces::RUNTIME_GEO,
                &keyspaces::runtime_identity_key(
                    keyspaces::RUNTIME_GEO,
                    &commit.scope,
                    &geo.reference,
                ),
                geo,
            )?;
            plan.put_json(
                keyspaces::RUNTIME_GEO_VERSIONS,
                &keyspaces::runtime_version_key(
                    keyspaces::RUNTIME_GEO_VERSIONS,
                    &commit.scope,
                    &geo.reference,
                    geo.valid_from,
                    change.cursor,
                ),
                change,
            )
        }
        RuntimeMutation::Object { object } => {
            plan.put_json(
                keyspaces::RUNTIME_OBJECTS,
                &keyspaces::runtime_identity_key(
                    keyspaces::RUNTIME_OBJECTS,
                    &commit.scope,
                    &object.reference,
                ),
                object,
            )?;
            plan.put_json(
                keyspaces::RUNTIME_OBJECT_VERSIONS,
                &keyspaces::runtime_version_key(
                    keyspaces::RUNTIME_OBJECT_VERSIONS,
                    &commit.scope,
                    &object.reference,
                    change.at,
                    change.cursor,
                ),
                change,
            )
        }
        RuntimeMutation::Retire { retirement } => {
            if retirement.model.is_record_like() {
                plan.put_json(
                    keyspaces::RUNTIME_RECORD_VERSIONS,
                    &keyspaces::runtime_version_key(
                        keyspaces::RUNTIME_RECORD_VERSIONS,
                        &commit.scope,
                        &retirement.reference,
                        retirement.effective_at,
                        change.cursor,
                    ),
                    change,
                )?;
                return plan.delete(
                    keyspaces::RUNTIME_RECORDS,
                    &keyspaces::runtime_identity_key(
                        keyspaces::RUNTIME_RECORDS,
                        &commit.scope,
                        &retirement.reference,
                    ),
                );
            }
            let current_space = match retirement.model {
                rrd_core::RuntimeLogicalModel::GraphRelation => {
                    if let Some(relation) = current_relation(
                        reader,
                        staged_relations,
                        &commit.scope,
                        &retirement.reference,
                    )? {
                        plan.put_json(
                            keyspaces::RUNTIME_RELATION_VERSIONS,
                            &keyspaces::runtime_version_key(
                                keyspaces::RUNTIME_RELATION_VERSIONS,
                                &commit.scope,
                                &retirement.reference,
                                retirement.effective_at,
                                change.cursor,
                            ),
                            change,
                        )?;
                        put_temporal_adjacency(
                            plan,
                            &commit.scope,
                            &relation,
                            retirement.effective_at,
                            change,
                        )?;
                        delete_current_adjacency(plan, &commit.scope, &relation)?;
                        staged_relations.insert(retirement.reference.clone(), None);
                    }
                    Some(keyspaces::RUNTIME_RELATIONS)
                }
                rrd_core::RuntimeLogicalModel::Vector => {
                    super::encode_vector_retirement(
                        plan,
                        &commit.scope,
                        &retirement.reference,
                        retirement.effective_at,
                        change,
                    )?;
                    None
                }
                rrd_core::RuntimeLogicalModel::Event
                | rrd_core::RuntimeLogicalModel::ReasoningEvent
                | rrd_core::RuntimeLogicalModel::LifecycleEvent => {
                    plan.put_json(
                        keyspaces::RUNTIME_EVENT_VERSIONS,
                        &keyspaces::runtime_version_key(
                            keyspaces::RUNTIME_EVENT_VERSIONS,
                            &commit.scope,
                            &retirement.reference,
                            retirement.effective_at,
                            change.cursor,
                        ),
                        change,
                    )?;
                    None
                }
                rrd_core::RuntimeLogicalModel::TimeSeries => {
                    plan.put_json(
                        keyspaces::RUNTIME_SERIES_VERSIONS,
                        &keyspaces::runtime_version_key(
                            keyspaces::RUNTIME_SERIES_VERSIONS,
                            &commit.scope,
                            &retirement.reference,
                            retirement.effective_at,
                            change.cursor,
                        ),
                        change,
                    )?;
                    Some(keyspaces::RUNTIME_SERIES)
                }
                rrd_core::RuntimeLogicalModel::Geo => {
                    plan.put_json(
                        keyspaces::RUNTIME_GEO_VERSIONS,
                        &keyspaces::runtime_version_key(
                            keyspaces::RUNTIME_GEO_VERSIONS,
                            &commit.scope,
                            &retirement.reference,
                            retirement.effective_at,
                            change.cursor,
                        ),
                        change,
                    )?;
                    Some(keyspaces::RUNTIME_GEO)
                }
                rrd_core::RuntimeLogicalModel::Object => {
                    plan.put_json(
                        keyspaces::RUNTIME_OBJECT_VERSIONS,
                        &keyspaces::runtime_version_key(
                            keyspaces::RUNTIME_OBJECT_VERSIONS,
                            &commit.scope,
                            &retirement.reference,
                            retirement.effective_at,
                            change.cursor,
                        ),
                        change,
                    )?;
                    Some(keyspaces::RUNTIME_OBJECTS)
                }
                rrd_core::RuntimeLogicalModel::ReasoningClaim
                | rrd_core::RuntimeLogicalModel::Document
                | rrd_core::RuntimeLogicalModel::Relational
                | rrd_core::RuntimeLogicalModel::GraphNode
                | rrd_core::RuntimeLogicalModel::KeyValue
                | rrd_core::RuntimeLogicalModel::ReasoningRecord
                | rrd_core::RuntimeLogicalModel::LifecycleRecord => None,
            };
            if let Some(space) = current_space {
                plan.delete(
                    space,
                    &keyspaces::runtime_identity_key(space, &commit.scope, &retirement.reference),
                )?;
            }
            Ok(())
        }
    }
}

fn current_relation(
    reader: &(impl AccessRead + ?Sized),
    staged_relations: &mut BTreeMap<RuntimeRef, Option<RuntimeRelation>>,
    scope: &rrd_core::ScopeId,
    reference: &RuntimeRef,
) -> Result<Option<RuntimeRelation>> {
    if let Some(staged) = staged_relations.get(reference) {
        return Ok(staged.clone());
    }
    let current = get_json(
        reader,
        keyspaces::RUNTIME_RELATIONS,
        &keyspaces::runtime_identity_key(keyspaces::RUNTIME_RELATIONS, scope, reference),
    )?;
    staged_relations.insert(reference.clone(), current.clone());
    Ok(current)
}

fn put_adjacency(
    plan: &mut SemanticCommitPlan,
    scope: &rrd_core::ScopeId,
    relation: &RuntimeRelation,
    effective_at: u64,
    change: &RuntimeChange,
) -> Result<()> {
    for (current_space, version_space) in [
        (
            keyspaces::RUNTIME_OUTGOING_EDGES,
            keyspaces::RUNTIME_OUTGOING_EDGE_VERSIONS,
        ),
        (
            keyspaces::RUNTIME_INCOMING_EDGES,
            keyspaces::RUNTIME_INCOMING_EDGE_VERSIONS,
        ),
    ] {
        plan.put_json(
            current_space,
            &keyspaces::runtime_adjacency_key(current_space, scope, relation),
            relation,
        )?;
        plan.put_json(
            version_space,
            &keyspaces::runtime_adjacency_version_key(
                version_space,
                scope,
                relation,
                effective_at,
                change.cursor,
            ),
            change,
        )?;
    }
    Ok(())
}

fn put_temporal_adjacency(
    plan: &mut SemanticCommitPlan,
    scope: &rrd_core::ScopeId,
    relation: &RuntimeRelation,
    effective_at: u64,
    change: &RuntimeChange,
) -> Result<()> {
    for version_space in [
        keyspaces::RUNTIME_OUTGOING_EDGE_VERSIONS,
        keyspaces::RUNTIME_INCOMING_EDGE_VERSIONS,
    ] {
        plan.put_json(
            version_space,
            &keyspaces::runtime_adjacency_version_key(
                version_space,
                scope,
                relation,
                effective_at,
                change.cursor,
            ),
            change,
        )?;
    }
    Ok(())
}

fn delete_current_adjacency(
    plan: &mut SemanticCommitPlan,
    scope: &rrd_core::ScopeId,
    relation: &RuntimeRelation,
) -> Result<()> {
    for space in [
        keyspaces::RUNTIME_OUTGOING_EDGES,
        keyspaces::RUNTIME_INCOMING_EDGES,
    ] {
        plan.delete(
            space,
            &keyspaces::runtime_adjacency_key(space, scope, relation),
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::runtime_state::{get, get_json};
    use super::*;
    use crate::{RrflowKvStore, RrflowMxStore, StorageEngine};
    use rrd_core::{
        RuntimeLogicalModel, RuntimeProperties, RuntimeRecord, RuntimeRecordSchema, RuntimeRef,
        RuntimeRelationSchema, RuntimeRetirement, RuntimeSchemaRegistry, RuntimeType,
    };
    use std::collections::BTreeSet;

    struct CommittedFixture {
        scope: rrd_core::ScopeId,
        original: RuntimeRelation,
        replacement: RuntimeRelation,
        commit_ids: Vec<String>,
    }

    fn record(id: &str) -> RuntimeRecord {
        RuntimeRecord {
            reference: RuntimeRef::new("file", id).unwrap(),
            valid_from: 100,
            valid_to: None,
            properties: RuntimeProperties::new(),
        }
    }

    fn relation(id: &str, to: &str, valid_from: u64) -> RuntimeRelation {
        RuntimeRelation {
            reference: RuntimeRef::new("imports", id).unwrap(),
            from: RuntimeRef::new("file", "a").unwrap(),
            to: RuntimeRef::new("file", to).unwrap(),
            valid_from,
            valid_to: None,
            properties: RuntimeProperties::new(),
        }
    }

    fn schema() -> RuntimeSchemaRegistry {
        let file = RuntimeType::new("file").unwrap();
        let mut registry = RuntimeSchemaRegistry::empty(1, "semantic commit fan-out");
        registry
            .define_record_table(
                file.clone(),
                RuntimeLogicalModel::Relational,
                RuntimeRecordSchema::default(),
            )
            .unwrap();
        registry
            .define_relation_table(
                RuntimeType::new("imports").unwrap(),
                RuntimeRelationSchema {
                    from: BTreeSet::from([file.clone()]),
                    to: BTreeSet::from([file]),
                    ..RuntimeRelationSchema::default()
                },
            )
            .unwrap();
        registry
    }

    fn commit_fixture(storage: &dyn StorageEngine) -> CommittedFixture {
        let scope = rrd_core::ScopeId::new("project:semantic-atomicity").unwrap();
        let original = relation("a-target", "b", 100);
        let replacement = relation("a-target", "c", 200);
        let first = storage
            .runtime()
            .commit(&RuntimeCommit {
                scope: scope.clone(),
                at: 100,
                actor: "agent:semantic-atomicity".into(),
                expected_cursor: 0,
                mutations: vec![
                    RuntimeMutation::Schema { registry: schema() },
                    RuntimeMutation::Record {
                        record: record("a"),
                    },
                    RuntimeMutation::Record {
                        record: record("b"),
                    },
                    RuntimeMutation::Record {
                        record: record("c"),
                    },
                    RuntimeMutation::Relation {
                        relation: original.clone(),
                    },
                ],
            })
            .unwrap();
        let second = storage
            .runtime()
            .commit(&RuntimeCommit {
                scope: scope.clone(),
                at: 200,
                actor: "agent:semantic-atomicity".into(),
                expected_cursor: first.last_cursor,
                mutations: vec![
                    RuntimeMutation::Relation {
                        relation: replacement.clone(),
                    },
                    RuntimeMutation::Retire {
                        retirement: RuntimeRetirement {
                            model: RuntimeLogicalModel::GraphRelation,
                            reference: replacement.reference.clone(),
                            effective_at: 300,
                        },
                    },
                ],
            })
            .unwrap();
        assert_eq!((first.last_cursor, second.last_cursor), (5, 7));
        CommittedFixture {
            scope,
            original,
            replacement,
            commit_ids: vec![first.commit_id, second.commit_id],
        }
    }

    fn assert_physical_fan_out(storage: &dyn StorageEngine, fixture: &CommittedFixture) {
        let transaction = storage.begin_transaction().unwrap();
        for id in ["a", "b", "c"] {
            let reference = RuntimeRef::new("file", id).unwrap();
            let current: RuntimeRecord = get_json(
                &*transaction,
                keyspaces::RUNTIME_RECORDS,
                &keyspaces::runtime_identity_key(
                    keyspaces::RUNTIME_RECORDS,
                    &fixture.scope,
                    &reference,
                ),
            )
            .unwrap()
            .unwrap();
            assert_eq!(current.reference, reference);
        }
        let record_history: RuntimeChange = get_json(
            &*transaction,
            keyspaces::RUNTIME_RECORD_VERSIONS,
            &keyspaces::runtime_version_key(
                keyspaces::RUNTIME_RECORD_VERSIONS,
                &fixture.scope,
                &RuntimeRef::new("file", "a").unwrap(),
                100,
                2,
            ),
        )
        .unwrap()
        .unwrap();
        assert!(matches!(
            record_history.mutation,
            RuntimeMutation::Record { .. }
        ));

        let current_relation_key = keyspaces::runtime_identity_key(
            keyspaces::RUNTIME_RELATIONS,
            &fixture.scope,
            &fixture.replacement.reference,
        );
        assert!(get(
            &*transaction,
            keyspaces::RUNTIME_RELATIONS,
            &current_relation_key,
        )
        .unwrap()
        .is_none());

        for (relation, effective_at, cursor, retired) in [
            (&fixture.original, 100, 5, false),
            (&fixture.replacement, 200, 6, false),
            (&fixture.replacement, 300, 7, true),
        ] {
            let version: RuntimeChange = get_json(
                &*transaction,
                keyspaces::RUNTIME_RELATION_VERSIONS,
                &keyspaces::runtime_version_key(
                    keyspaces::RUNTIME_RELATION_VERSIONS,
                    &fixture.scope,
                    &relation.reference,
                    effective_at,
                    cursor,
                ),
            )
            .unwrap()
            .unwrap();
            assert_eq!(
                matches!(version.mutation, RuntimeMutation::Retire { .. }),
                retired
            );
            for space in [
                keyspaces::RUNTIME_OUTGOING_EDGE_VERSIONS,
                keyspaces::RUNTIME_INCOMING_EDGE_VERSIONS,
            ] {
                let adjacency_version: RuntimeChange = get_json(
                    &*transaction,
                    space,
                    &keyspaces::runtime_adjacency_version_key(
                        space,
                        &fixture.scope,
                        relation,
                        effective_at,
                        cursor,
                    ),
                )
                .unwrap()
                .unwrap();
                assert_eq!(adjacency_version.digest, version.digest);
            }
        }
        for relation in [&fixture.original, &fixture.replacement] {
            for space in [
                keyspaces::RUNTIME_OUTGOING_EDGES,
                keyspaces::RUNTIME_INCOMING_EDGES,
            ] {
                assert!(get(
                    &*transaction,
                    space,
                    &keyspaces::runtime_adjacency_key(space, &fixture.scope, relation),
                )
                .unwrap()
                .is_none());
            }
        }

        let deltas = storage.runtime().projection_deltas_since(0, 16).unwrap();
        let outbox = storage.runtime().outbox_since(0, 16).unwrap();
        assert_eq!(deltas, outbox);
        assert_eq!(deltas.len(), 6);
        assert_eq!(deltas.last().unwrap().source_cursor, 7);
        for commit_id in &fixture.commit_ids {
            assert!(storage.runtime().audit(commit_id).unwrap().is_some());
            assert!(storage
                .runtime()
                .commit_outcome(commit_id)
                .unwrap()
                .is_some());
        }
    }

    fn remove_runtime_accumulator(storage: &dyn StorageEngine) {
        let mut transaction = storage.begin_transaction().unwrap();
        transaction
            .delete(
                checked_key(keyspaces::META, &keyspaces::runtime_accumulator_state_key()).unwrap(),
            )
            .unwrap();
        transaction
            .commit(crate::keyspaces::Durability::Authoritative)
            .unwrap();
    }

    fn assert_missing_accumulator_is_rejected(storage: &dyn StorageEngine, cursor: u64) {
        let error = storage
            .runtime()
            .commit(&RuntimeCommit {
                scope: rrd_core::ScopeId::new("project:semantic-atomicity").unwrap(),
                at: 400,
                actor: "agent:semantic-atomicity".into(),
                expected_cursor: cursor,
                mutations: vec![RuntimeMutation::Record {
                    record: RuntimeRecord {
                        valid_from: 400,
                        ..record("d")
                    },
                }],
            })
            .unwrap_err();
        assert!(matches!(
            error,
            Error::Substrate(reason)
                if reason == format!("runtime accumulator is absent at non-empty cursor {cursor}")
        ));
    }

    #[test]
    fn graph_temporal_and_delivery_effects_share_one_plan_on_mx_kv_and_reopen() {
        let mx = RrflowMxStore::new();
        let mx_fixture = commit_fixture(&mx);
        assert_physical_fan_out(&mx, &mx_fixture);

        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("rrflow-kv");
        let kv_fixture = {
            let kv = RrflowKvStore::open(&path).unwrap();
            let fixture = commit_fixture(&kv);
            assert_physical_fan_out(&kv, &fixture);
            fixture
        };
        let reopened = RrflowKvStore::open(&path).unwrap();
        assert_physical_fan_out(&reopened, &kv_fixture);
        assert_eq!(mx_fixture.commit_ids, kv_fixture.commit_ids);
    }

    #[test]
    fn nonempty_runtime_never_rebuilds_a_missing_accumulator_from_the_change_log() {
        let mx = RrflowMxStore::new();
        let mx_fixture = commit_fixture(&mx);
        remove_runtime_accumulator(&mx);
        assert_missing_accumulator_is_rejected(&mx, 7);

        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("rrflow-kv");
        {
            let kv = RrflowKvStore::open(&path).unwrap();
            let kv_fixture = commit_fixture(&kv);
            assert_eq!(kv_fixture.commit_ids, mx_fixture.commit_ids);
            remove_runtime_accumulator(&kv);
        }
        let reopened = RrflowKvStore::open(&path).unwrap();
        assert_missing_accumulator_is_rejected(&reopened, 7);
    }
}
