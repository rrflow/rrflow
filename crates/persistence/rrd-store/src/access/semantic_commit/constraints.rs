//! Select current constraint inputs; the kernel remains the verdict authority.
use super::super::runtime_state::scan_space;
use super::*;

pub(super) fn load(
    reader: &(impl AccessRead + ?Sized),
    commit: &RuntimeCommit,
    schema: &RuntimeSchemaRegistry,
    schema_changed: bool,
) -> Result<(Vec<RuntimeRecord>, Vec<RuntimeRelation>)> {
    if schema_changed {
        // A schema change can constrain existing untouched data. Preserve the
        // original whole-scope validation, including changed validity rules.
        let records = if schema
            .records
            .values()
            .any(|s| !s.unique_properties.is_empty())
        {
            values_for_scope(reader, keyspaces::RUNTIME_RECORDS, &commit.scope)?
        } else {
            Vec::new()
        };
        let relations = if schema
            .relations
            .values()
            .any(|s| s.unique_pair || s.max_outgoing.is_some() || s.max_incoming.is_some())
        {
            values_for_scope(reader, keyspaces::RUNTIME_RELATIONS, &commit.scope)?
        } else {
            Vec::new()
        };
        return Ok((records, relations));
    }

    let mut kinds = BTreeSet::new();
    let mut ranges = BTreeMap::new();
    for mutation in &commit.mutations {
        match mutation {
            RuntimeMutation::Record { record } => {
                if schema
                    .records
                    .get(&record.reference.kind)
                    .is_some_and(|s| !s.unique_properties.is_empty())
                {
                    kinds.insert(&record.reference.kind);
                }
            }
            RuntimeMutation::Relation { relation } => {
                let Some(rule) = schema.relations.get(&relation.reference.kind) else {
                    continue;
                };
                if rule.max_outgoing.is_some() {
                    ranges.insert(
                        keyspaces::runtime_adjacency_prefix(
                            keyspaces::RUNTIME_OUTGOING_EDGES,
                            &commit.scope,
                            &relation.from,
                            &relation.reference.kind,
                        ),
                        keyspaces::RUNTIME_OUTGOING_EDGES,
                    );
                } else if rule.unique_pair {
                    ranges.insert(
                        keyspaces::runtime_adjacency_pair_prefix(&commit.scope, relation),
                        keyspaces::RUNTIME_OUTGOING_EDGES,
                    );
                }
                if rule.max_incoming.is_some() {
                    ranges.insert(
                        keyspaces::runtime_adjacency_prefix(
                            keyspaces::RUNTIME_INCOMING_EDGES,
                            &commit.scope,
                            &relation.to,
                            &relation.reference.kind,
                        ),
                        keyspaces::RUNTIME_INCOMING_EDGES,
                    );
                }
            }
            _ => {}
        }
    }
    // Value-specific unique indexes are a separate E-02 requirement. Until
    // then scan only the affected kind, never unrelated record tables.
    let mut records = Vec::new();
    for kind in kinds {
        let prefix =
            keyspaces::runtime_kind_prefix(keyspaces::RUNTIME_RECORDS, &commit.scope, kind);
        for (key, bytes) in scan_space(reader, keyspaces::RUNTIME_RECORDS, &prefix)? {
            let record: RuntimeRecord = serde_json::from_slice(&bytes)?;
            if &record.reference.kind != kind
                || key
                    != keyspaces::runtime_identity_key(
                        keyspaces::RUNTIME_RECORDS,
                        &commit.scope,
                        &record.reference,
                    )
            {
                return Err(Error::Substrate(
                    "constraint record key differs from its identity".into(),
                ));
            }
            records.push(record);
        }
    }
    let mut relations = BTreeMap::new();
    for (prefix, space) in ranges {
        for (key, bytes) in scan_space(reader, space, &prefix)? {
            let relation: RuntimeRelation = serde_json::from_slice(&bytes)?;
            if key != keyspaces::runtime_adjacency_key(space, &commit.scope, &relation) {
                return Err(Error::Substrate(
                    "constraint adjacency key differs from its relation".into(),
                ));
            }
            if let Some(previous) = relations.insert(relation.reference.clone(), relation.clone()) {
                if previous != relation {
                    return Err(Error::Substrate(
                        "constraint adjacency copies disagree".into(),
                    ));
                }
            }
        }
    }
    // Moves only remove occupancy from an old group; all proposed new groups
    // are present above. Kernel validation overlays the mutations by identity,
    // preserving its existing batch ordering and retirement semantics.
    Ok((records, relations.into_values().collect()))
}
