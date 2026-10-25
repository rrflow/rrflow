use super::*;
use rrd_core::{RuntimeCommit, RuntimeProperties, RuntimeRecord, RuntimeRetirement};

fn scope() -> ScopeId {
    ScopeId::new("project:graph-reduction").unwrap()
}

fn node(index: u64) -> RuntimeRef {
    RuntimeRef::new("node", format!("n{index}")).unwrap()
}

fn change(cursor: u64, mutation: RuntimeMutation) -> RuntimeChange {
    RuntimeChange::committed(
        cursor,
        &RuntimeCommit {
            scope: scope(),
            at: 1,
            actor: "test:graph-reduction".into(),
            expected_cursor: cursor - 1,
            mutations: vec![mutation.clone()],
        },
        &format!("commit-{cursor}"),
        0,
        mutation,
        None,
    )
}

fn start() -> RuntimeChange {
    change(
        1,
        RuntimeMutation::Record {
            record: RuntimeRecord {
                reference: node(0),
                valid_from: 0,
                valid_to: None,
                properties: RuntimeProperties::new(),
            },
        },
    )
}

fn edge(index: u64) -> RuntimeChange {
    change(
        index + 2,
        RuntimeMutation::Relation {
            relation: RuntimeRelation {
                reference: RuntimeRef::new("link", format!("e{index}")).unwrap(),
                from: node(index),
                to: node(index + 1),
                valid_from: 0,
                valid_to: None,
                properties: RuntimeProperties::new(),
            },
        },
    )
}

#[test]
fn deep_expansion_reduces_each_supplied_change_once() {
    for depth in [8_u16, 32, 64] {
        let scope = scope();
        let mut expansion = AdjacencyExpansion::new(
            &scope,
            100,
            u64::MAX,
            &[],
            &[TraversalPlan {
                relation: RuntimeType::new("link").unwrap(),
                start: node(0),
                direction: TraversalDirection::Outgoing,
                max_depth: depth,
            }],
        );
        assert_eq!(expansion.next(&[start()]).len(), 1);
        for index in 0..u64::from(depth) {
            let edge = edge(index);
            // An adjacency range names the edge, then its identity range
            // repeats that version. This is the real reader's step contract.
            assert_eq!(expansion.next(std::slice::from_ref(&edge)).len(), 1);
            let next = expansion.next(&[edge]);
            assert_eq!(next.is_empty(), index + 1 == u64::from(depth));
        }
        assert_eq!(expansion.walks[0].visited.len(), usize::from(depth) + 1);
        let supplied = 1 + 2 * usize::from(depth);
        println!(
            "depth {depth}: {} temporal mutation visits; {supplied} supplied versions",
            expansion.reduction_visits
        );
        assert!(
            expansion.reduction_visits <= supplied,
            "depth {depth}: {} temporal mutation visits for {supplied} supplied versions",
            expansion.reduction_visits
        );
    }
}

/// Independent chronological replay of the pre-change semantics. Unlike the
/// production reducer, this sorts all history then overwrites/removes entries.
fn chronological_oracle(
    changes: &[RuntimeChange],
    valid_at: u64,
    known_at: u64,
) -> (
    BTreeMap<RuntimeRef, RuntimeRecord>,
    BTreeMap<RuntimeRef, RuntimeRelation>,
) {
    let mut ordered = changes.iter().collect::<Vec<_>>();
    ordered.sort_by_key(|change| change.cursor);
    let mut records = BTreeMap::new();
    let mut relations = BTreeMap::new();
    for change in ordered {
        if change.scope != scope() || change.cursor > known_at {
            continue;
        }
        match &change.mutation {
            RuntimeMutation::Record { record } if record.valid_from <= valid_at => {
                records.insert(record.reference.clone(), record.clone());
            }
            RuntimeMutation::Relation { relation } if relation.valid_from <= valid_at => {
                relations.insert(relation.reference.clone(), relation.clone());
            }
            RuntimeMutation::Retire { retirement } if retirement.effective_at <= valid_at => {
                if retirement.model.is_record_like() {
                    records.remove(&retirement.reference);
                } else if retirement.model == RuntimeLogicalModel::GraphRelation {
                    relations.remove(&retirement.reference);
                }
            }
            _ => {}
        }
    }
    records.retain(|_, record| record.valid_to.is_none_or(|end| valid_at < end));
    relations.retain(|_, relation| relation.valid_to.is_none_or(|end| valid_at < end));
    (records, relations)
}

#[test]
fn temporal_reduction_matches_chronological_oracle_for_reordered_and_repeated_history() {
    let mut expired_record = start().mutation;
    let RuntimeMutation::Record { record } = &mut expired_record else {
        unreachable!()
    };
    record.valid_from = 20;
    record.valid_to = Some(30);
    let mut expired_edge = edge(0).mutation;
    let RuntimeMutation::Relation { relation } = &mut expired_edge else {
        unreachable!()
    };
    relation.valid_from = 20;
    relation.valid_to = Some(30);
    relation.to = node(2);
    let mut moved_edge = edge(0).mutation;
    let RuntimeMutation::Relation { relation } = &mut moved_edge else {
        unreachable!()
    };
    relation.from = node(2);
    relation.to = node(3);
    relation.valid_from = 15; // Later commit can move valid time backwards.
    let mut future_edge = edge(0).mutation;
    let RuntimeMutation::Relation { relation } = &mut future_edge else {
        unreachable!()
    };
    relation.valid_from = 200;
    let mut future_record = start().mutation;
    let RuntimeMutation::Record { record } = &mut future_record else {
        unreachable!()
    };
    record.valid_from = 200;
    let mut other_scope = change(13, edge(0).mutation);
    other_scope.scope = ScopeId::new("project:other").unwrap();
    let history = vec![
        start(),
        edge(0),
        change(3, expired_record),
        change(4, expired_edge),
        change(
            5,
            RuntimeMutation::Retire {
                retirement: RuntimeRetirement {
                    model: RuntimeLogicalModel::Relational,
                    reference: node(0),
                    effective_at: 40,
                },
            },
        ),
        change(
            6,
            RuntimeMutation::Retire {
                retirement: RuntimeRetirement {
                    model: RuntimeLogicalModel::GraphRelation,
                    reference: RuntimeRef::new("link", "e0").unwrap(),
                    effective_at: 40,
                },
            },
        ),
        change(7, start().mutation),
        change(8, moved_edge),
        change(9, future_record),
        change(10, future_edge),
        other_scope,
    ];
    for valid_at in [0, 19, 20, 29, 30, 39, 40, 100, 200] {
        for known_at in [2, 4, 6, 8, 10, 13] {
            let expected = chronological_oracle(&history, valid_at, known_at);
            for offset in 0..history.len() {
                let mut reordered = history.clone();
                reordered.rotate_left(offset);
                if offset % 2 == 0 {
                    reordered.reverse();
                }
                // Duplicate an older history after every complete pass.
                reordered.extend(history.iter().rev().cloned());
                assert_eq!(
                    active_graph_values(&reordered, &scope(), valid_at, known_at),
                    expected,
                    "valid {valid_at}, known {known_at}, offset {offset}"
                );
            }
        }
    }
}
