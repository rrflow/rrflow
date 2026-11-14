//! The graph snapshot reducer and graph diffs.

use super::*;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeRecordChange {
    pub before: RuntimeRecord,
    pub after: RuntimeRecord,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeRelationChange {
    pub before: RuntimeRelation,
    pub after: RuntimeRelation,
}

/// Exact structural differential between two graph snapshots.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeGraphDiff {
    pub from_cursor: u64,
    pub to_cursor: u64,
    pub added_records: Vec<RuntimeRecord>,
    pub removed_records: Vec<RuntimeRecord>,
    pub changed_records: Vec<RuntimeRecordChange>,
    pub added_relations: Vec<RuntimeRelation>,
    pub removed_relations: Vec<RuntimeRelation>,
    pub changed_relations: Vec<RuntimeRelationChange>,
}

impl RuntimeGraphSnapshot {
    pub fn from_changes(
        changes: &[RuntimeChange],
        scope: ScopeId,
        valid_at: Millis,
        known_at_cursor: u64,
    ) -> Self {
        let mut records = BTreeMap::<RuntimeRef, RuntimeRecord>::new();
        let mut relations = BTreeMap::<RuntimeRef, RuntimeRelation>::new();
        for change in changes
            .iter()
            .filter(|change| change.cursor <= known_at_cursor && change.scope == scope)
        {
            match &change.mutation {
                RuntimeMutation::Record { record } => {
                    // Ignore versions whose modeled validity has not begun at
                    // this instant. For every eligible identity, the last
                    // transaction-visible version wins; its valid_to below can
                    // then explicitly retire the identity.
                    if record.valid_from <= valid_at {
                        records.insert(record.reference.clone(), record.clone());
                    }
                }
                RuntimeMutation::Relation { relation } => {
                    if relation.valid_from <= valid_at {
                        relations.insert(relation.reference.clone(), relation.clone());
                    }
                }
                RuntimeMutation::Event { event } => {
                    let event_ref = RuntimeRef::event(event.kind.clone(), change.cursor);
                    records.insert(
                        event_ref.clone(),
                        RuntimeRecord {
                            reference: event_ref.clone(),
                            valid_from: change.at,
                            valid_to: None,
                            properties: event.properties.clone(),
                        },
                    );
                    if let Some(subject) = &event.subject {
                        let relation_ref = RuntimeRef::event(
                            RuntimeType::new("emitted").expect("static runtime type is valid"),
                            change.cursor,
                        );
                        relations.insert(
                            relation_ref.clone(),
                            RuntimeRelation {
                                reference: relation_ref,
                                from: subject.clone(),
                                to: event_ref,
                                valid_from: change.at,
                                valid_to: None,
                                properties: RuntimeProperties::new(),
                            },
                        );
                    }
                }
                RuntimeMutation::Retire { retirement } if retirement.effective_at <= valid_at => {
                    retire_graph_identity(&mut records, &mut relations, retirement);
                }
                RuntimeMutation::Claim { .. }
                | RuntimeMutation::Schema { .. }
                | RuntimeMutation::Vector { .. }
                | RuntimeMutation::SeriesSample { .. }
                | RuntimeMutation::Geo { .. }
                | RuntimeMutation::Object { .. }
                | RuntimeMutation::Retire { .. } => {}
            }
        }
        let records = records
            .into_values()
            .filter(|record| valid_at_window(record.valid_from, record.valid_to, valid_at))
            .collect();
        let relations = relations
            .into_values()
            .filter(|relation| valid_at_window(relation.valid_from, relation.valid_to, valid_at))
            .collect();
        Self {
            scope,
            valid_at,
            known_at_cursor,
            records,
            relations,
        }
    }

    pub fn outgoing<'a>(
        &'a self,
        from: &'a RuntimeRef,
    ) -> impl Iterator<Item = &'a RuntimeRelation> {
        self.relations
            .iter()
            .filter(move |relation| &relation.from == from)
    }

    pub fn incoming<'a>(&'a self, to: &'a RuntimeRef) -> impl Iterator<Item = &'a RuntimeRelation> {
        self.relations
            .iter()
            .filter(move |relation| &relation.to == to)
    }

    /// Breadth-first structural traversal. `max_depth == 0` returns only the
    /// start node. An empty relation-kind set permits every edge type.
    pub fn traverse(
        &self,
        start: &RuntimeRef,
        max_depth: usize,
        relation_kinds: &BTreeSet<RuntimeType>,
    ) -> Vec<RuntimeRef> {
        let mut visited = BTreeSet::from([start.clone()]);
        let mut queue = VecDeque::from([(start.clone(), 0usize)]);
        while let Some((current, depth)) = queue.pop_front() {
            if depth >= max_depth {
                continue;
            }
            for relation in self.outgoing(&current) {
                if !relation_kinds.is_empty() && !relation_kinds.contains(&relation.reference.kind)
                {
                    continue;
                }
                if visited.insert(relation.to.clone()) {
                    queue.push_back((relation.to.clone(), depth + 1));
                }
            }
        }
        visited.into_iter().collect()
    }

    pub fn diff(&self, newer: &Self) -> RuntimeGraphDiff {
        let before_records = self
            .records
            .iter()
            .map(|record| (record.reference.clone(), record))
            .collect::<BTreeMap<_, _>>();
        let after_records = newer
            .records
            .iter()
            .map(|record| (record.reference.clone(), record))
            .collect::<BTreeMap<_, _>>();
        let before_relations = self
            .relations
            .iter()
            .map(|relation| (relation.reference.clone(), relation))
            .collect::<BTreeMap<_, _>>();
        let after_relations = newer
            .relations
            .iter()
            .map(|relation| (relation.reference.clone(), relation))
            .collect::<BTreeMap<_, _>>();

        RuntimeGraphDiff {
            from_cursor: self.known_at_cursor,
            to_cursor: newer.known_at_cursor,
            added_records: after_records
                .iter()
                .filter(|(id, _)| !before_records.contains_key(*id))
                .map(|(_, record)| (*record).clone())
                .collect(),
            removed_records: before_records
                .iter()
                .filter(|(id, _)| !after_records.contains_key(*id))
                .map(|(_, record)| (*record).clone())
                .collect(),
            changed_records: after_records
                .iter()
                .filter_map(|(id, after)| {
                    let before = before_records.get(id)?;
                    (*before != *after).then(|| RuntimeRecordChange {
                        before: (*before).clone(),
                        after: (*after).clone(),
                    })
                })
                .collect(),
            added_relations: after_relations
                .iter()
                .filter(|(id, _)| !before_relations.contains_key(*id))
                .map(|(_, relation)| (*relation).clone())
                .collect(),
            removed_relations: before_relations
                .iter()
                .filter(|(id, _)| !after_relations.contains_key(*id))
                .map(|(_, relation)| (*relation).clone())
                .collect(),
            changed_relations: after_relations
                .iter()
                .filter_map(|(id, after)| {
                    let before = before_relations.get(id)?;
                    (*before != *after).then(|| RuntimeRelationChange {
                        before: (*before).clone(),
                        after: (*after).clone(),
                    })
                })
                .collect(),
        }
    }
}

pub(super) fn retire_graph_identity(
    records: &mut BTreeMap<RuntimeRef, RuntimeRecord>,
    relations: &mut BTreeMap<RuntimeRef, RuntimeRelation>,
    retirement: &RuntimeRetirement,
) {
    match retirement.model {
        crate::RuntimeLogicalModel::Document
        | crate::RuntimeLogicalModel::Relational
        | crate::RuntimeLogicalModel::GraphNode
        | crate::RuntimeLogicalModel::KeyValue
        | crate::RuntimeLogicalModel::ReasoningRecord
        | crate::RuntimeLogicalModel::LifecycleRecord => {
            records.remove(&retirement.reference);
        }
        crate::RuntimeLogicalModel::GraphRelation => {
            relations.remove(&retirement.reference);
        }
        crate::RuntimeLogicalModel::Event
        | crate::RuntimeLogicalModel::ReasoningEvent
        | crate::RuntimeLogicalModel::LifecycleEvent => {
            records.remove(&retirement.reference);
            let emitted = RuntimeRef {
                kind: RuntimeType::new("emitted").expect("static runtime type is valid"),
                id: retirement.reference.id.clone(),
            };
            relations.remove(&emitted);
        }
        crate::RuntimeLogicalModel::Vector
        | crate::RuntimeLogicalModel::TimeSeries
        | crate::RuntimeLogicalModel::Geo
        | crate::RuntimeLogicalModel::Object
        | crate::RuntimeLogicalModel::ReasoningClaim => {}
    }
}

pub(super) fn duplicate_identity<T>(kind: &str, reference: &RuntimeRef) -> Result<T> {
    Err(Error::InvalidRuntime {
        reason: format!(
            "runtime commit contains duplicate {kind} identity {}/{}",
            reference.kind, reference.id
        ),
    })
}
