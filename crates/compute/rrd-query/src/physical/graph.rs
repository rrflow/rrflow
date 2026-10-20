//! Native temporal adjacency traversal for rrflowQL graph sources.
//!
//! A bounded traversal reads its start identity, then one breadth-first level
//! at a time the temporal adjacency range of every frontier node and the
//! complete identity history of every relation those ranges name. Physical
//! work therefore follows the visited neighbourhood and its histories, never
//! the estate's relation or record families. Rows come from one replay over
//! the authenticated subset with the same temporal semantics as a replay of
//! the complete families.

use crate::catalog::version_source_covers;
use crate::execute::{active_graph_values, string, QueryRow};
use crate::{Source, TraversalDirection};
use rrd_core::{
    RuntimeChange, RuntimeLogicalModel, RuntimeMutation, RuntimeRef, RuntimeRelation, RuntimeType,
    RuntimeValue, ScopeId,
};
use rrd_store::{RuntimeAdjacencyDirection, RuntimeVersionedSource};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// One bounded traversal source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TraversalPlan {
    pub(crate) relation: RuntimeType,
    pub(crate) start: RuntimeRef,
    pub(crate) direction: TraversalDirection,
    pub(crate) max_depth: u16,
}

impl TraversalPlan {
    pub(crate) fn from_source(source: &Source) -> Option<Self> {
        match source {
            Source::Traversal {
                relation,
                start,
                direction,
                max_depth,
            } => Some(Self {
                relation: relation.clone(),
                start: start.clone(),
                direction: *direction,
                max_depth: *max_depth,
            }),
            _ => None,
        }
    }

    fn adjacency_directions(&self) -> &'static [RuntimeAdjacencyDirection] {
        match self.direction {
            TraversalDirection::Outgoing => &[RuntimeAdjacencyDirection::Outgoing],
            TraversalDirection::Incoming => &[RuntimeAdjacencyDirection::Incoming],
            TraversalDirection::Both => &[
                RuntimeAdjacencyDirection::Outgoing,
                RuntimeAdjacencyDirection::Incoming,
            ],
        }
    }
}

/// The node a traversal reaches from `current` across `relation`. A relation
/// whose endpoints are both `current` yields `current`, which is already
/// visited.
fn successor<'a>(
    relation: &'a RuntimeRelation,
    current: &RuntimeRef,
    direction: TraversalDirection,
) -> Option<&'a RuntimeRef> {
    match direction {
        TraversalDirection::Outgoing if relation.from == *current => Some(&relation.to),
        TraversalDirection::Incoming if relation.to == *current => Some(&relation.from),
        TraversalDirection::Both if relation.from == *current => Some(&relation.to),
        TraversalDirection::Both if relation.to == *current => Some(&relation.from),
        _ => None,
    }
}

struct Walk {
    plan: TraversalPlan,
    frontier: Vec<RuntimeRef>,
    visited: BTreeSet<RuntimeRef>,
    depth: u16,
    done: bool,
}

/// What the previous read step loaded.
#[derive(Clone, Copy)]
enum Loaded {
    Start,
    Adjacency,
    Identities,
}

/// Selects each read step's ranges from the authenticated changes of the
/// previous one. Every traversal advances in lockstep: adjacency of the
/// frontier, then identity histories of the relations it names, then the
/// next frontier. A range already read, or covered by a static family range
/// of the same read, is never requested again.
pub(crate) struct AdjacencyExpansion<'a> {
    scope: &'a ScopeId,
    valid_at: u64,
    known_at_cursor: u64,
    static_sources: &'a [RuntimeVersionedSource],
    walks: Vec<Walk>,
    stage: Loaded,
    loaded: BTreeMap<u64, RuntimeChange>,
    /// Relation identities named by a loaded version at each endpoint.
    outgoing: BTreeMap<(RuntimeType, RuntimeRef), BTreeSet<RuntimeRef>>,
    incoming: BTreeMap<(RuntimeType, RuntimeRef), BTreeSet<RuntimeRef>>,
    adjacency_read: BTreeSet<(RuntimeType, RuntimeRef, RuntimeAdjacencyDirection)>,
    identities_read: BTreeSet<RuntimeRef>,
}

impl<'a> AdjacencyExpansion<'a> {
    pub(crate) fn new(
        scope: &'a ScopeId,
        valid_at: u64,
        known_at_cursor: u64,
        static_sources: &'a [RuntimeVersionedSource],
        traversals: &[TraversalPlan],
    ) -> Self {
        Self {
            scope,
            valid_at,
            known_at_cursor,
            static_sources,
            walks: traversals
                .iter()
                .map(|plan| Walk {
                    plan: plan.clone(),
                    frontier: Vec::new(),
                    visited: BTreeSet::new(),
                    depth: 0,
                    done: false,
                })
                .collect(),
            stage: Loaded::Start,
            loaded: BTreeMap::new(),
            outgoing: BTreeMap::new(),
            incoming: BTreeMap::new(),
            adjacency_read: BTreeSet::new(),
            identities_read: BTreeSet::new(),
        }
    }

    /// Accepts one authenticated step and returns the next step's ranges; an
    /// empty selection ends the read.
    pub(crate) fn next(&mut self, step: &[RuntimeChange]) -> Vec<RuntimeVersionedSource> {
        for change in step {
            if let RuntimeMutation::Relation { relation } = &change.mutation {
                for (index, endpoint) in [
                    (&mut self.outgoing, &relation.from),
                    (&mut self.incoming, &relation.to),
                ] {
                    index
                        .entry((relation.reference.kind.clone(), endpoint.clone()))
                        .or_default()
                        .insert(relation.reference.clone());
                }
            }
            self.loaded.insert(change.cursor, change.clone());
        }
        loop {
            let requests = match self.stage {
                Loaded::Start => {
                    self.begin();
                    self.stage = Loaded::Adjacency;
                    self.adjacency_requests()
                }
                Loaded::Adjacency => {
                    self.stage = Loaded::Identities;
                    self.identity_requests()
                }
                Loaded::Identities => {
                    self.advance();
                    self.stage = Loaded::Adjacency;
                    self.adjacency_requests()
                }
            };
            if !requests.is_empty() || self.walks.iter().all(|walk| walk.done) {
                return requests;
            }
        }
    }

    fn begin(&mut self) {
        let (records, _) = active_graph_values(
            self.loaded.values(),
            self.scope,
            self.valid_at,
            self.known_at_cursor,
        );
        for walk in &mut self.walks {
            if records.contains_key(&walk.plan.start) && walk.plan.max_depth > 0 {
                walk.frontier = vec![walk.plan.start.clone()];
                walk.visited = BTreeSet::from([walk.plan.start.clone()]);
            } else {
                walk.done = true;
            }
        }
    }

    fn adjacency_requests(&mut self) -> Vec<RuntimeVersionedSource> {
        let mut requests = Vec::new();
        for walk in self.walks.iter().filter(|walk| !walk.done) {
            for node in &walk.frontier {
                for direction in walk.plan.adjacency_directions() {
                    let key = (walk.plan.relation.clone(), node.clone(), *direction);
                    if self.adjacency_read.insert(key) {
                        requests.push(RuntimeVersionedSource::Adjacency {
                            relation: walk.plan.relation.clone(),
                            node: node.clone(),
                            direction: *direction,
                        });
                    }
                }
            }
        }
        requests
    }

    fn incident<'w>(
        &'w self,
        walk: &'w Walk,
        node: &'w RuntimeRef,
    ) -> impl Iterator<Item = &'w RuntimeRef> + 'w {
        let key = (walk.plan.relation.clone(), node.clone());
        walk.plan
            .adjacency_directions()
            .iter()
            .filter_map(move |direction| match direction {
                RuntimeAdjacencyDirection::Outgoing => self.outgoing.get(&key),
                RuntimeAdjacencyDirection::Incoming => self.incoming.get(&key),
            })
            .flatten()
    }

    fn identity_requests(&mut self) -> Vec<RuntimeVersionedSource> {
        let mut named = BTreeSet::new();
        for walk in self.walks.iter().filter(|walk| !walk.done) {
            for node in &walk.frontier {
                named.extend(self.incident(walk, node).cloned());
            }
        }
        let mut requests = Vec::new();
        for reference in named {
            let source = RuntimeVersionedSource::Identity {
                model: RuntimeLogicalModel::GraphRelation,
                reference: reference.clone(),
            };
            let covered = self
                .static_sources
                .iter()
                .any(|available| version_source_covers(available, &source));
            if !covered && self.identities_read.insert(reference) {
                requests.push(source);
            }
        }
        requests
    }

    /// Every relation named at the frontier now has its complete history
    /// loaded, so its state at the valid time decides the next frontier.
    fn advance(&mut self) {
        let (_, relations) = active_graph_values(
            self.loaded.values(),
            self.scope,
            self.valid_at,
            self.known_at_cursor,
        );
        let mut walks = std::mem::take(&mut self.walks);
        for walk in walks.iter_mut().filter(|walk| !walk.done) {
            let mut next = Vec::new();
            let mut reached = walk.visited.clone();
            for node in &walk.frontier {
                for reference in self.incident(walk, node) {
                    let Some(relation) = relations.get(reference) else {
                        continue;
                    };
                    if let Some(successor) = successor(relation, node, walk.plan.direction) {
                        if reached.insert(successor.clone()) {
                            next.push(successor.clone());
                        }
                    }
                }
            }
            walk.visited = reached;
            walk.depth += 1;
            if walk.depth >= walk.plan.max_depth || next.is_empty() {
                walk.done = true;
                walk.frontier.clear();
            } else {
                walk.frontier = next;
            }
        }
        self.walks = walks;
    }
}

/// Breadth-first rows over the authenticated changes. Each node's incident
/// relations are visited in relation-reference order, so the result equals a
/// scan of every relation of the type in that order.
#[allow(clippy::too_many_arguments)]
pub(crate) fn traversal_rows(
    changes: &[RuntimeChange],
    scope: &ScopeId,
    relation_kind: &RuntimeType,
    start: &RuntimeRef,
    direction: TraversalDirection,
    max_depth: u16,
    valid_at: u64,
    known_at_cursor: u64,
) -> Vec<QueryRow> {
    let (records, relations) = active_graph_values(changes, scope, valid_at, known_at_cursor);
    if !records.contains_key(start) {
        return Vec::new();
    }
    let mut incident = BTreeMap::<RuntimeRef, Vec<&RuntimeRelation>>::new();
    for relation in relations
        .values()
        .filter(|relation| &relation.reference.kind == relation_kind)
    {
        let mut endpoints = match direction {
            TraversalDirection::Outgoing => vec![&relation.from],
            TraversalDirection::Incoming => vec![&relation.to],
            TraversalDirection::Both => vec![&relation.from, &relation.to],
        };
        endpoints.dedup();
        for endpoint in endpoints {
            incident.entry(endpoint.clone()).or_default().push(relation);
        }
    }
    let mut visited = BTreeSet::from([start.clone()]);
    let mut queue = VecDeque::from([(start.clone(), 0_u16, vec![start.clone()])]);
    let mut rows = Vec::new();
    while let Some((current, depth, path)) = queue.pop_front() {
        if depth >= max_depth {
            continue;
        }
        for relation in incident.get(&current).into_iter().flatten() {
            let Some(next) = successor(relation, &current, direction) else {
                continue;
            };
            if !visited.insert(next.clone()) {
                continue;
            }
            let next_depth = depth + 1;
            let mut next_path = path.clone();
            next_path.push(next.clone());
            let mut values = BTreeMap::new();
            values.insert(
                "depth".into(),
                RuntimeValue::Unsigned(u64::from(next_depth)),
            );
            values.insert("node_kind".into(), string(next.kind.to_string()));
            values.insert("node_id".into(), string(next.id.to_string()));
            values.insert(
                "relation_kind".into(),
                string(relation.reference.kind.to_string()),
            );
            values.insert(
                "relation_id".into(),
                string(relation.reference.id.to_string()),
            );
            values.insert("from_kind".into(), string(relation.from.kind.to_string()));
            values.insert("from_id".into(), string(relation.from.id.to_string()));
            values.insert("to_kind".into(), string(relation.to.kind.to_string()));
            values.insert("to_id".into(), string(relation.to.id.to_string()));
            values.insert(
                "path".into(),
                RuntimeValue::List(
                    next_path
                        .iter()
                        .map(|node| string(format!("{}:{}", node.kind, node.id)))
                        .collect(),
                ),
            );
            rows.push(QueryRow {
                identity: format!(
                    "traversal:{}:{}:{}:{}:{}:{}",
                    relation_kind, start.kind, start.id, next_depth, relation.reference.id, next.id
                ),
                values,
            });
            queue.push_back((next.clone(), next_depth, next_path));
        }
    }
    rows
}
