//! Request-local reduction of authenticated graph versions at one read time.
//!
//! Read steps can repeat versions or deliver an older cursor after a newer
//! one. Keep the greatest eligible cursor per identity, including a tombstone
//! for an expired version or retirement, so arrival order cannot revive it.
//! This is disposable query state; the store remains the version authority.

use rrd_core::{
    RuntimeChange, RuntimeLogicalModel, RuntimeMutation, RuntimeRecord, RuntimeRef,
    RuntimeRelation, ScopeId,
};
use std::collections::{btree_map::Entry, BTreeMap};

type Versions<T> = BTreeMap<RuntimeRef, (u64, Option<T>)>;

pub(crate) struct GraphState<'a> {
    scope: &'a ScopeId,
    valid_at: u64,
    known_at_cursor: u64,
    records: Versions<RuntimeRecord>,
    relations: Versions<RuntimeRelation>,
}

impl<'a> GraphState<'a> {
    pub(crate) fn new(scope: &'a ScopeId, valid_at: u64, known_at_cursor: u64) -> Self {
        Self {
            scope,
            valid_at,
            known_at_cursor,
            records: BTreeMap::new(),
            relations: BTreeMap::new(),
        }
    }

    pub(crate) fn apply(&mut self, change: &RuntimeChange) {
        if change.cursor > self.known_at_cursor || &change.scope != self.scope {
            return;
        }
        match &change.mutation {
            RuntimeMutation::Record { record } if record.valid_from <= self.valid_at => {
                replace(
                    &mut self.records,
                    &record.reference,
                    change.cursor,
                    record
                        .valid_to
                        .is_none_or(|end| self.valid_at < end)
                        .then_some(record),
                );
            }
            RuntimeMutation::Relation { relation } if relation.valid_from <= self.valid_at => {
                replace(
                    &mut self.relations,
                    &relation.reference,
                    change.cursor,
                    relation
                        .valid_to
                        .is_none_or(|end| self.valid_at < end)
                        .then_some(relation),
                );
            }
            RuntimeMutation::Retire { retirement } if retirement.effective_at <= self.valid_at => {
                if retirement.model.is_record_like() {
                    replace(
                        &mut self.records,
                        &retirement.reference,
                        change.cursor,
                        None,
                    );
                } else if retirement.model == RuntimeLogicalModel::GraphRelation {
                    replace(
                        &mut self.relations,
                        &retirement.reference,
                        change.cursor,
                        None,
                    );
                }
            }
            _ => {}
        }
    }

    pub(crate) fn has_record(&self, reference: &RuntimeRef) -> bool {
        self.records
            .get(reference)
            .is_some_and(|(_, value)| value.is_some())
    }

    pub(crate) fn relation(&self, reference: &RuntimeRef) -> Option<&RuntimeRelation> {
        self.relations.get(reference)?.1.as_ref()
    }

    pub(crate) fn into_values(
        self,
    ) -> (
        BTreeMap<RuntimeRef, RuntimeRecord>,
        BTreeMap<RuntimeRef, RuntimeRelation>,
    ) {
        let records = self
            .records
            .into_iter()
            .filter_map(|(key, (_, value))| value.map(|v| (key, v)))
            .collect();
        let relations = self
            .relations
            .into_iter()
            .filter_map(|(key, (_, value))| value.map(|v| (key, v)))
            .collect();
        (records, relations)
    }
}

fn replace<T: Clone>(
    versions: &mut Versions<T>,
    reference: &RuntimeRef,
    cursor: u64,
    value: Option<&T>,
) {
    match versions.entry(reference.clone()) {
        Entry::Vacant(entry) => {
            entry.insert((cursor, value.cloned()));
        }
        Entry::Occupied(mut entry) if cursor > entry.get().0 => {
            entry.insert((cursor, value.cloned()));
        }
        _ => {}
    }
}
