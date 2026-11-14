//! Read validation, change pages and the data snapshot reducer.

use super::*;

/// One bounded replay page. Consumers advance to `through_cursor`, even if a
/// scope filter produced no matching changes, so sparse feeds cannot stall.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeReadValidation {
    pub method: String,
    pub change_reads: u64,
    pub proof_nodes: u16,
}

impl RuntimeReadValidation {
    pub fn new(method: impl Into<String>, change_reads: u64, proof_nodes: usize) -> Self {
        Self {
            method: method.into(),
            change_reads,
            proof_nodes: u16::try_from(proof_nodes).unwrap_or(u16::MAX),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeChangePage {
    pub requested_after: u64,
    pub through_cursor: u64,
    pub head_cursor: u64,
    pub validation: RuntimeReadValidation,
    pub changes: Vec<RuntimeChange>,
}

impl RuntimeChangePage {
    pub fn has_more(&self) -> bool {
        self.through_cursor < self.head_cursor
    }
}

/// A transaction-consistent structural graph reconstructed at a global cursor
/// and valid-time instant.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeGraphSnapshot {
    pub scope: ScopeId,
    pub valid_at: Millis,
    pub known_at_cursor: u64,
    pub records: Vec<RuntimeRecord>,
    pub relations: Vec<RuntimeRelation>,
}

/// One canonical value paired with the logical model selected by the same
/// catalogue snapshot as the read.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeModelValue<T> {
    pub model: crate::RuntimeLogicalModel,
    pub value: T,
}

/// Stable identity assigned to an append-only event by its authenticated-log
/// cursor. A correction retires this identity and appends a replacement event
/// in the same transaction; stored event bytes are never rewritten.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeEventValue {
    pub reference: RuntimeRef,
    pub occurred_at: Millis,
    pub event: RuntimeEvent,
}

/// Complete current logical data view at one valid time and one transaction
/// cursor. Every family is reduced from the same authenticated runtime log and
/// tagged by the same schema revision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeDataSnapshot {
    pub scope: ScopeId,
    pub valid_at: Millis,
    pub known_at_cursor: u64,
    pub schema_revision: u64,
    pub claims: Vec<Claim>,
    pub records: Vec<RuntimeModelValue<RuntimeRecord>>,
    pub relations: Vec<RuntimeModelValue<RuntimeRelation>>,
    pub events: Vec<RuntimeModelValue<RuntimeEventValue>>,
    pub vectors: Vec<RuntimeModelValue<RuntimeVector>>,
    pub series: Vec<RuntimeModelValue<RuntimeSeriesSample>>,
    pub geo: Vec<RuntimeModelValue<RuntimeGeo>>,
    pub objects: Vec<RuntimeModelValue<ObjectReference>>,
}

impl RuntimeDataSnapshot {
    pub fn contains(&self, model: crate::RuntimeLogicalModel, reference: &RuntimeRef) -> bool {
        if model.is_record_like() {
            self.records
                .iter()
                .any(|entry| entry.model == model && &entry.value.reference == reference)
        } else if model == crate::RuntimeLogicalModel::GraphRelation {
            self.relations
                .iter()
                .any(|entry| entry.model == model && &entry.value.reference == reference)
        } else if model.is_event_like() {
            self.events
                .iter()
                .any(|entry| entry.model == model && &entry.value.reference == reference)
        } else {
            match model {
                crate::RuntimeLogicalModel::Vector => self
                    .vectors
                    .iter()
                    .any(|entry| &entry.value.reference == reference),
                crate::RuntimeLogicalModel::TimeSeries => self
                    .series
                    .iter()
                    .any(|entry| &entry.value.reference == reference),
                crate::RuntimeLogicalModel::Geo => self
                    .geo
                    .iter()
                    .any(|entry| &entry.value.reference == reference),
                crate::RuntimeLogicalModel::Object => self
                    .objects
                    .iter()
                    .any(|entry| &entry.value.reference == reference),
                crate::RuntimeLogicalModel::ReasoningClaim
                | crate::RuntimeLogicalModel::Document
                | crate::RuntimeLogicalModel::Relational
                | crate::RuntimeLogicalModel::GraphNode
                | crate::RuntimeLogicalModel::GraphRelation
                | crate::RuntimeLogicalModel::KeyValue
                | crate::RuntimeLogicalModel::Event
                | crate::RuntimeLogicalModel::ReasoningRecord
                | crate::RuntimeLogicalModel::ReasoningEvent
                | crate::RuntimeLogicalModel::LifecycleRecord
                | crate::RuntimeLogicalModel::LifecycleEvent => false,
            }
        }
    }

    /// Resolves a schema-less claim-only log into the same snapshot shape used
    /// by typed runtime data. This preserves the original claim append path
    /// without creating a parallel recall authority. Any schema-bound mutation
    /// still fails closed until a schema is installed.
    pub fn from_claim_changes(
        changes: &[RuntimeChange],
        scope: ScopeId,
        valid_at: Millis,
        known_at_cursor: u64,
    ) -> Result<Self> {
        let mut claims = BTreeMap::<(Subject, Predicate), Vec<Claim>>::new();
        for change in changes
            .iter()
            .filter(|change| change.cursor <= known_at_cursor && change.scope == scope)
        {
            match &change.mutation {
                RuntimeMutation::Claim { claim } => {
                    claims
                        .entry((claim.subject.clone(), claim.predicate.clone()))
                        .or_default()
                        .push(claim.clone());
                }
                _ => {
                    return Err(Error::InvalidRuntime {
                        reason: format!(
                            "runtime mutation at cursor {} requires a schema before snapshot assembly",
                            change.cursor
                        ),
                    });
                }
            }
        }
        Ok(Self {
            scope,
            valid_at,
            known_at_cursor,
            schema_revision: 0,
            claims: resolve_snapshot_claims(claims, valid_at),
            records: Vec::new(),
            relations: Vec::new(),
            events: Vec::new(),
            vectors: Vec::new(),
            series: Vec::new(),
            geo: Vec::new(),
            objects: Vec::new(),
        })
    }

    pub fn from_changes(
        changes: &[RuntimeChange],
        schema: &RuntimeSchemaRegistry,
        scope: ScopeId,
        valid_at: Millis,
        known_at_cursor: u64,
    ) -> Result<Self> {
        schema.validate()?;
        let tables = schema.catalogue_tables()?;
        let model_for = |reference: &RuntimeRef| -> Result<crate::RuntimeLogicalModel> {
            tables
                .get(&reference.kind)
                .map(|table| table.model)
                .ok_or_else(|| Error::InvalidRuntime {
                    reason: format!(
                        "runtime value type {} is absent from schema revision {}",
                        reference.kind, schema.revision
                    ),
                })
        };

        let mut records = BTreeMap::<RuntimeRef, RuntimeModelValue<RuntimeRecord>>::new();
        let mut claims = BTreeMap::<(Subject, Predicate), Vec<Claim>>::new();
        let mut relations = BTreeMap::<RuntimeRef, RuntimeModelValue<RuntimeRelation>>::new();
        let mut events = BTreeMap::<RuntimeRef, RuntimeModelValue<RuntimeEventValue>>::new();
        let mut vectors = BTreeMap::<RuntimeRef, RuntimeModelValue<RuntimeVector>>::new();
        let mut series = BTreeMap::<RuntimeRef, RuntimeModelValue<RuntimeSeriesSample>>::new();
        let mut geo = BTreeMap::<RuntimeRef, RuntimeModelValue<RuntimeGeo>>::new();
        let mut objects = BTreeMap::<RuntimeRef, RuntimeModelValue<ObjectReference>>::new();

        for change in changes
            .iter()
            .filter(|change| change.cursor <= known_at_cursor && change.scope == scope)
        {
            match &change.mutation {
                RuntimeMutation::Claim { claim } => {
                    claims
                        .entry((claim.subject.clone(), claim.predicate.clone()))
                        .or_default()
                        .push(claim.clone());
                }
                RuntimeMutation::Record { record } if record.valid_from <= valid_at => {
                    let model = model_for(&record.reference)?;
                    if !model.is_record_like() {
                        return data_snapshot_model_error(&record.reference, model, "record");
                    }
                    records.insert(
                        record.reference.clone(),
                        RuntimeModelValue {
                            model,
                            value: record.clone(),
                        },
                    );
                }
                RuntimeMutation::Relation { relation } if relation.valid_from <= valid_at => {
                    let model = model_for(&relation.reference)?;
                    if model != crate::RuntimeLogicalModel::GraphRelation {
                        return data_snapshot_model_error(
                            &relation.reference,
                            model,
                            "graph relation",
                        );
                    }
                    relations.insert(
                        relation.reference.clone(),
                        RuntimeModelValue {
                            model,
                            value: relation.clone(),
                        },
                    );
                }
                RuntimeMutation::Event { event } if change.at <= valid_at => {
                    let reference = runtime_event_reference(event, change.cursor);
                    let model = model_for(&reference)?;
                    if !model.is_event_like() {
                        return data_snapshot_model_error(&reference, model, "event");
                    }
                    events.insert(
                        reference.clone(),
                        RuntimeModelValue {
                            model,
                            value: RuntimeEventValue {
                                reference,
                                occurred_at: change.at,
                                event: event.clone(),
                            },
                        },
                    );
                }
                RuntimeMutation::Vector { vector } if vector.valid_from <= valid_at => {
                    let model = model_for(&vector.reference)?;
                    if model != crate::RuntimeLogicalModel::Vector {
                        return data_snapshot_model_error(&vector.reference, model, "vector");
                    }
                    vectors.insert(
                        vector.reference.clone(),
                        RuntimeModelValue {
                            model,
                            value: vector.clone(),
                        },
                    );
                }
                RuntimeMutation::SeriesSample { sample } if sample.observed_at <= valid_at => {
                    let model = model_for(&sample.reference)?;
                    if model != crate::RuntimeLogicalModel::TimeSeries {
                        return data_snapshot_model_error(
                            &sample.reference,
                            model,
                            "time-series sample",
                        );
                    }
                    series.insert(
                        sample.reference.clone(),
                        RuntimeModelValue {
                            model,
                            value: sample.clone(),
                        },
                    );
                }
                RuntimeMutation::Geo { geo: value } if value.valid_from <= valid_at => {
                    let model = model_for(&value.reference)?;
                    if model != crate::RuntimeLogicalModel::Geo {
                        return data_snapshot_model_error(&value.reference, model, "geo value");
                    }
                    geo.insert(
                        value.reference.clone(),
                        RuntimeModelValue {
                            model,
                            value: value.clone(),
                        },
                    );
                }
                RuntimeMutation::Object { object } if change.at <= valid_at => {
                    let model = model_for(&object.reference)?;
                    if model != crate::RuntimeLogicalModel::Object {
                        return data_snapshot_model_error(
                            &object.reference,
                            model,
                            "object reference",
                        );
                    }
                    objects.insert(
                        object.reference.clone(),
                        RuntimeModelValue {
                            model,
                            value: object.clone(),
                        },
                    );
                }
                RuntimeMutation::Retire { retirement } if retirement.effective_at <= valid_at => {
                    let actual = model_for(&retirement.reference)?;
                    if actual != retirement.model {
                        return data_snapshot_model_error(
                            &retirement.reference,
                            actual,
                            "retirement",
                        );
                    }
                    retire_data_identity(
                        retirement,
                        &mut records,
                        &mut relations,
                        &mut events,
                        &mut vectors,
                        &mut series,
                        &mut geo,
                        &mut objects,
                    );
                }
                RuntimeMutation::Schema { .. }
                | RuntimeMutation::Record { .. }
                | RuntimeMutation::Relation { .. }
                | RuntimeMutation::Event { .. }
                | RuntimeMutation::Vector { .. }
                | RuntimeMutation::SeriesSample { .. }
                | RuntimeMutation::Geo { .. }
                | RuntimeMutation::Object { .. }
                | RuntimeMutation::Retire { .. } => {}
            }
        }

        let claims = resolve_snapshot_claims(claims, valid_at);

        Ok(Self {
            scope,
            valid_at,
            known_at_cursor,
            schema_revision: schema.revision,
            claims,
            records: records
                .into_values()
                .filter(|record| {
                    valid_at_window(record.value.valid_from, record.value.valid_to, valid_at)
                })
                .collect(),
            relations: relations
                .into_values()
                .filter(|relation| {
                    valid_at_window(relation.value.valid_from, relation.value.valid_to, valid_at)
                })
                .collect(),
            events: events.into_values().collect(),
            vectors: vectors
                .into_values()
                .filter(|vector| {
                    valid_at_window(vector.value.valid_from, vector.value.valid_to, valid_at)
                })
                .collect(),
            series: series.into_values().collect(),
            geo: geo
                .into_values()
                .filter(|value| {
                    valid_at_window(value.value.valid_from, value.value.valid_to, valid_at)
                })
                .collect(),
            objects: objects.into_values().collect(),
        })
    }
}

pub(super) fn resolve_snapshot_claims(
    claims: BTreeMap<(Subject, Predicate), Vec<Claim>>,
    valid_at: Millis,
) -> Vec<Claim> {
    claims
        .into_values()
        .filter_map(|mut versions| {
            versions.sort_by(|left, right| {
                right
                    .valid_from
                    .cmp(&left.valid_from)
                    .then_with(|| right.tx_time.cmp(&left.tx_time))
                    .then_with(|| left.canonical_bytes().cmp(&right.canonical_bytes()))
            });
            crate::resolve_as_of(&versions, valid_at).cloned()
        })
        .collect()
}

pub(super) fn runtime_event_reference(event: &RuntimeEvent, cursor: u64) -> RuntimeRef {
    RuntimeRef::event(event.kind.clone(), cursor)
}

pub(super) fn data_snapshot_model_error<T>(
    reference: &RuntimeRef,
    model: crate::RuntimeLogicalModel,
    expected: &str,
) -> Result<T> {
    Err(Error::InvalidRuntime {
        reason: format!(
            "catalogue model {model:?} for {}/{} cannot be read as {expected}",
            reference.kind, reference.id
        ),
    })
}

#[allow(clippy::too_many_arguments)]
pub(super) fn retire_data_identity(
    retirement: &RuntimeRetirement,
    records: &mut BTreeMap<RuntimeRef, RuntimeModelValue<RuntimeRecord>>,
    relations: &mut BTreeMap<RuntimeRef, RuntimeModelValue<RuntimeRelation>>,
    events: &mut BTreeMap<RuntimeRef, RuntimeModelValue<RuntimeEventValue>>,
    vectors: &mut BTreeMap<RuntimeRef, RuntimeModelValue<RuntimeVector>>,
    series: &mut BTreeMap<RuntimeRef, RuntimeModelValue<RuntimeSeriesSample>>,
    geo: &mut BTreeMap<RuntimeRef, RuntimeModelValue<RuntimeGeo>>,
    objects: &mut BTreeMap<RuntimeRef, RuntimeModelValue<ObjectReference>>,
) {
    let reference = &retirement.reference;
    if retirement.model.is_record_like() {
        records.remove(reference);
    } else if retirement.model == crate::RuntimeLogicalModel::GraphRelation {
        relations.remove(reference);
    } else if retirement.model.is_event_like() {
        events.remove(reference);
    } else {
        match retirement.model {
            crate::RuntimeLogicalModel::Vector => {
                vectors.remove(reference);
            }
            crate::RuntimeLogicalModel::TimeSeries => {
                series.remove(reference);
            }
            crate::RuntimeLogicalModel::Geo => {
                geo.remove(reference);
            }
            crate::RuntimeLogicalModel::Object => {
                objects.remove(reference);
            }
            crate::RuntimeLogicalModel::ReasoningClaim
            | crate::RuntimeLogicalModel::Document
            | crate::RuntimeLogicalModel::Relational
            | crate::RuntimeLogicalModel::GraphNode
            | crate::RuntimeLogicalModel::GraphRelation
            | crate::RuntimeLogicalModel::KeyValue
            | crate::RuntimeLogicalModel::Event
            | crate::RuntimeLogicalModel::ReasoningRecord
            | crate::RuntimeLogicalModel::ReasoningEvent
            | crate::RuntimeLogicalModel::LifecycleRecord
            | crate::RuntimeLogicalModel::LifecycleEvent => {}
        }
    }
}
