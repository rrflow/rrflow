//! E-01 acceptance: directed, typed, depth-bounded traversal equals the exact
//! graph oracle, and its physical evidence follows the visited edges rather
//! than the estate.
//!
//! The oracle is the breadth-first scan of every relation of the traversed
//! type, in relation-reference order, over the complete authenticated change
//! log. It shares no code with the native adjacency reader.

use rrd_core::{
    ReadStamp, RuntimeChange, RuntimeCommit, RuntimeLogicalModel, RuntimeMutation,
    RuntimeProperties, RuntimeRecord, RuntimeRecordSchema, RuntimeRef, RuntimeRelation,
    RuntimeRelationSchema, RuntimeRetirement, RuntimeSchemaRegistry, RuntimeType, RuntimeValue,
    ScopeId,
};
use rrd_query::{
    parse, ExecutionBudget, Parameters, PhysicalOperator, QueryExecution, StampedQueryPipeline,
};
use rrd_store::{
    RrflowKvStore, RrflowMxStore, RuntimeReadAccessPath, RuntimeReadBudget, RuntimeVersionedSource,
    StorageEngine,
};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

const NODES: usize = 10;
const ACTOR: &str = "test:graph-traversal";

type Rows = BTreeMap<String, BTreeMap<String, RuntimeValue>>;

fn scope() -> ScopeId {
    ScopeId::new("project:graph-traversal").unwrap()
}

fn kind(name: &str) -> RuntimeType {
    RuntimeType::new(name).unwrap()
}

fn node(id: &str) -> RuntimeRef {
    RuntimeRef::new("node", id).unwrap()
}

/// Deterministic xorshift64* stream; seeds make every corpus reproducible.
struct Stream(u64);

impl Stream {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    fn below(&mut self, bound: u64) -> u64 {
        self.next() % bound
    }

    fn chance(&mut self, percent: u64) -> bool {
        self.below(100) < percent
    }
}

fn schema() -> RuntimeSchemaRegistry {
    let mut registry = RuntimeSchemaRegistry::empty(1, "graph traversal oracle");
    registry
        .define_record_table(
            kind("node"),
            RuntimeLogicalModel::Relational,
            RuntimeRecordSchema::default(),
        )
        .unwrap();
    for relation in ["link", "other"] {
        registry
            .define_relation_table(
                kind(relation),
                RuntimeRelationSchema {
                    from: BTreeSet::from([kind("node")]),
                    to: BTreeSet::from([kind("node")]),
                    allow_additional_properties: true,
                    ..RuntimeRelationSchema::default()
                },
            )
            .unwrap();
    }
    registry
}

fn record(id: &str, valid_to: Option<u64>) -> RuntimeMutation {
    RuntimeMutation::Record {
        record: RuntimeRecord {
            reference: node(id),
            valid_from: 0,
            valid_to,
            properties: RuntimeProperties::new(),
        },
    }
}

fn commit<E: StorageEngine>(engine: &E, at: u64, mutations: Vec<RuntimeMutation>) -> u64 {
    let expected_cursor = engine.runtime().cursor().unwrap();
    engine
        .runtime()
        .commit(&RuntimeCommit {
            scope: scope(),
            at,
            actor: ACTOR.into(),
            expected_cursor,
            mutations,
        })
        .unwrap()
        .last_cursor
}

/// A seeded temporal graph: two relation types, self-loops, bounded and
/// open validity, revisions that move endpoints or valid time backwards,
/// retirements, re-puts after retirement, and node records that expire or
/// are retired. Returns the commit cursors in order.
fn commit_random_graph<E: StorageEngine>(engine: &E, seed: u64) -> Vec<u64> {
    let mut stream = Stream(seed.wrapping_mul(0x9e37_79b9_7f4a_7c15) | 1);
    let mut nodes = vec![RuntimeMutation::Schema { registry: schema() }];
    let mut node_valid_to = Vec::new();
    for index in 0..NODES {
        let valid_to = stream.chance(20).then(|| 100 + stream.below(200));
        node_valid_to.push(valid_to);
        nodes.push(record(&format!("n{index}"), valid_to));
    }
    let mut cursors = vec![commit(engine, 1, nodes)];
    // Current relation per identity, plus the greatest valid_from it has had.
    let mut current = BTreeMap::<RuntimeRef, (RuntimeRelation, u64)>::new();
    let mut retired = Vec::<(RuntimeRef, u64)>::new();
    let mut next_id = 0_u64;
    for round in 0..8_u64 {
        let mut mutations = Vec::new();
        let mut touched = BTreeSet::new();
        for _ in 0..(3 + stream.below(6)) {
            let action = stream.below(100);
            if action < 25 && !current.is_empty() {
                // Revise an existing relation: move endpoints and valid time.
                let index = usize::try_from(stream.below(current.len() as u64)).unwrap();
                let reference = current.keys().nth(index).unwrap().clone();
                if !touched.insert(reference.clone()) {
                    continue;
                }
                let (mut relation, mut greatest) = current[&reference].clone();
                relation.from = node(&format!("n{}", stream.below(NODES as u64)));
                relation.to = node(&format!("n{}", stream.below(NODES as u64)));
                relation.valid_from = stream.below(300);
                relation.valid_to = stream
                    .chance(20)
                    .then(|| relation.valid_from + 1 + stream.below(150));
                greatest = greatest.max(relation.valid_from);
                mutations.push(RuntimeMutation::Relation {
                    relation: relation.clone(),
                });
                current.insert(reference, (relation, greatest));
            } else if action < 40 && !current.is_empty() {
                // Retire a relation whose current version is open-ended.
                let index = usize::try_from(stream.below(current.len() as u64)).unwrap();
                let reference = current.keys().nth(index).unwrap().clone();
                let (relation, greatest) = current[&reference].clone();
                if relation.valid_to.is_some() || !touched.insert(reference.clone()) {
                    continue;
                }
                mutations.push(RuntimeMutation::Retire {
                    retirement: RuntimeRetirement {
                        model: RuntimeLogicalModel::GraphRelation,
                        reference: reference.clone(),
                        effective_at: greatest + 1 + stream.below(50),
                    },
                });
                current.remove(&reference);
                retired.push((reference, greatest));
            } else if action < 48 && !retired.is_empty() {
                // Re-put a retired identity with fresh endpoints.
                let index = usize::try_from(stream.below(retired.len() as u64)).unwrap();
                let (reference, greatest) = retired.remove(index);
                if !touched.insert(reference.clone()) {
                    retired.push((reference, greatest));
                    continue;
                }
                let relation = RuntimeRelation {
                    reference: reference.clone(),
                    from: node(&format!("n{}", stream.below(NODES as u64))),
                    to: node(&format!("n{}", stream.below(NODES as u64))),
                    valid_from: stream.below(300),
                    valid_to: None,
                    properties: RuntimeProperties::new(),
                };
                let greatest = greatest.max(relation.valid_from);
                mutations.push(RuntimeMutation::Relation {
                    relation: relation.clone(),
                });
                current.insert(reference, (relation, greatest));
            } else {
                let relation_kind = if stream.chance(80) { "link" } else { "other" };
                let from = stream.below(NODES as u64);
                let to = if stream.chance(10) {
                    from
                } else {
                    stream.below(NODES as u64)
                };
                let valid_from = stream.below(300);
                let relation = RuntimeRelation {
                    reference: RuntimeRef::new(relation_kind, format!("r{next_id}")).unwrap(),
                    from: node(&format!("n{from}")),
                    to: node(&format!("n{to}")),
                    valid_from,
                    valid_to: stream
                        .chance(20)
                        .then(|| valid_from + 1 + stream.below(150)),
                    properties: RuntimeProperties::new(),
                };
                next_id += 1;
                touched.insert(relation.reference.clone());
                mutations.push(RuntimeMutation::Relation {
                    relation: relation.clone(),
                });
                current.insert(relation.reference.clone(), (relation, valid_from));
            }
        }
        if !mutations.is_empty() {
            cursors.push(commit(engine, 10 + round, mutations));
        }
    }
    let mut retirements = Vec::new();
    for (index, valid_to) in node_valid_to.iter().enumerate() {
        if valid_to.is_none() && stream.chance(25) {
            retirements.push(RuntimeMutation::Retire {
                retirement: RuntimeRetirement {
                    model: RuntimeLogicalModel::Relational,
                    reference: node(&format!("n{index}")),
                    effective_at: 150 + stream.below(200),
                },
            });
        }
    }
    if !retirements.is_empty() {
        cursors.push(commit(engine, 100, retirements));
    }
    cursors
}

/// The pre-adjacency semantics, restated independently: replay every
/// version in cursor order, keep what is valid at `valid_at`, then scan all
/// relations of the type for each dequeued node.
#[allow(clippy::too_many_arguments)]
fn oracle_rows(
    log: &[RuntimeChange],
    relation_kind: &RuntimeType,
    start: &RuntimeRef,
    direction: &str,
    max_depth: u16,
    valid_at: u64,
    known_at: u64,
) -> Rows {
    let mut records = BTreeMap::<RuntimeRef, RuntimeRecord>::new();
    let mut relations = BTreeMap::<RuntimeRef, RuntimeRelation>::new();
    for change in log.iter().filter(|change| change.cursor <= known_at) {
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
    let valid = |valid_to: Option<u64>| valid_to.is_none_or(|end| valid_at < end);
    if !records
        .get(start)
        .is_some_and(|record| valid(record.valid_to))
    {
        return Rows::new();
    }
    let relations = relations
        .into_values()
        .filter(|relation| &relation.reference.kind == relation_kind && valid(relation.valid_to))
        .collect::<Vec<_>>();
    let text = |value: String| RuntimeValue::String(value);
    let mut visited = BTreeSet::from([start.clone()]);
    let mut queue = VecDeque::from([(start.clone(), 0_u16, vec![start.clone()])]);
    let mut rows = Rows::new();
    while let Some((current, depth, path)) = queue.pop_front() {
        if depth >= max_depth {
            continue;
        }
        for relation in &relations {
            let next = match direction {
                "OUTGOING" if relation.from == current => &relation.to,
                "INCOMING" if relation.to == current => &relation.from,
                "BOTH" if relation.from == current => &relation.to,
                "BOTH" if relation.to == current => &relation.from,
                _ => continue,
            };
            if !visited.insert(next.clone()) {
                continue;
            }
            let mut path = path.clone();
            path.push(next.clone());
            let values = BTreeMap::from([
                ("depth".into(), RuntimeValue::Unsigned(u64::from(depth + 1))),
                ("node_kind".into(), text(next.kind.to_string())),
                ("node_id".into(), text(next.id.to_string())),
                (
                    "relation_kind".into(),
                    text(relation.reference.kind.to_string()),
                ),
                (
                    "relation_id".into(),
                    text(relation.reference.id.to_string()),
                ),
                ("from_kind".into(), text(relation.from.kind.to_string())),
                ("from_id".into(), text(relation.from.id.to_string())),
                ("to_kind".into(), text(relation.to.kind.to_string())),
                ("to_id".into(), text(relation.to.id.to_string())),
                (
                    "path".into(),
                    RuntimeValue::List(
                        path.iter()
                            .map(|node| text(format!("{}:{}", node.kind, node.id)))
                            .collect(),
                    ),
                ),
            ]);
            rows.insert(
                format!(
                    "traversal:{relation_kind}:{}:{}:{}:{}:{}",
                    start.kind,
                    start.id,
                    depth + 1,
                    relation.reference.id,
                    next.id
                ),
                values,
            );
            queue.push_back((next.clone(), depth + 1, path));
        }
    }
    rows
}

fn run<E: StorageEngine>(engine: &E, read: &ReadStamp, text: &str) -> QueryExecution {
    let query = parse(text).unwrap();
    let run = StampedQueryPipeline::new(engine, read.clone())
        .unwrap()
        .run(&query, &Parameters::new(), &ExecutionBudget::default())
        .unwrap_or_else(|error| panic!("{text}: {error}"));
    assert!(
        matches!(
            run.plan.operators.as_slice(),
            [
                PhysicalOperator::AdjacencyTraversal { .. },
                PhysicalOperator::DataFusionEvaluate
            ]
        ),
        "{text}"
    );
    run.execution
}

fn rows(execution: &QueryExecution) -> Rows {
    execution
        .batches
        .iter()
        .flat_map(|batch| &batch.rows)
        .map(|row| (row.identity.clone(), row.values.clone()))
        .collect()
}

fn exercise<E: StorageEngine>(engine: &E, seed: u64) -> Vec<QueryExecution> {
    let cursors = commit_random_graph(engine, seed);
    let head = engine.runtime().read_stamp(&scope()).unwrap();
    let log = engine
        .runtime()
        .read_changes(&head, 0, usize::MAX)
        .unwrap()
        .changes;
    let middle = cursors[cursors.len() / 2];
    let mut executions = Vec::new();
    let mut non_empty = 0;
    for start in ["n0", "n1", "n2", "n3", "absent"] {
        for direction in ["OUTGOING", "INCOMING", "BOTH"] {
            for depth in [1_u16, 2, 4] {
                for valid_at in [0_u64, 120, 260, 500] {
                    for (known, known_at) in [
                        ("HEAD".to_owned(), head.commit_cursor),
                        (middle.to_string(), middle),
                    ] {
                        let text = format!(
                            "FROM traverse:link START node:{start} DIRECTION {direction} DEPTH {depth} AT VALID {valid_at} KNOWN {known} PROJECT *"
                        );
                        let execution = run(engine, &head, &text);
                        let expected = oracle_rows(
                            &log,
                            &kind("link"),
                            &node(start),
                            direction,
                            depth,
                            valid_at,
                            known_at,
                        );
                        assert_eq!(rows(&execution), expected, "seed {seed}: {text}");
                        non_empty += usize::from(!expected.is_empty());
                        executions.push(execution);
                    }
                }
            }
        }
    }
    assert!(
        non_empty > 0,
        "seed {seed} produced no reachable neighbourhood"
    );
    executions
}

#[test]
fn native_traversal_matches_the_exact_graph_oracle_on_every_engine() {
    for seed in 1..=4 {
        exercise(&RrflowMxStore::new(), seed);
    }
    let memory = exercise(&RrflowMxStore::new(), 5);
    let directory = tempfile::tempdir().unwrap();
    let kv = RrflowKvStore::open(directory.path()).unwrap();
    let durable = exercise(&kv, 5);
    assert_eq!(memory, durable);
}

fn active_nodes(log: &[RuntimeChange], valid_at: u64, known_at: u64) -> BTreeSet<String> {
    let mut records = BTreeMap::<RuntimeRef, RuntimeRecord>::new();
    for change in log.iter().filter(|change| change.cursor <= known_at) {
        match &change.mutation {
            RuntimeMutation::Record { record } if record.valid_from <= valid_at => {
                records.insert(record.reference.clone(), record.clone());
            }
            RuntimeMutation::Retire { retirement }
                if retirement.model.is_record_like() && retirement.effective_at <= valid_at =>
            {
                records.remove(&retirement.reference);
            }
            _ => {}
        }
    }
    records
        .into_values()
        .filter(|record| record.valid_to.is_none_or(|end| valid_at < end))
        .map(|record| record.reference.id.to_string())
        .collect()
}

/// A traversal on either side of an exact join shares one read with the
/// joined record family; the family range covers every record identity the
/// traversal would otherwise request.
#[test]
fn traversal_joins_expand_beside_static_family_ranges() {
    let engine = RrflowMxStore::new();
    commit_random_graph(&engine, 7);
    let head = engine.runtime().read_stamp(&scope()).unwrap();
    let log = engine
        .runtime()
        .read_changes(&head, 0, usize::MAX)
        .unwrap()
        .changes;
    let mut joined_rows = 0;
    for start in ["n0", "n1", "n2"] {
        for valid_at in [120_u64, 260] {
            let traversal = format!("traverse:link START node:{start} DIRECTION BOTH DEPTH 3");
            let oracle = oracle_rows(
                &log,
                &kind("link"),
                &node(start),
                "BOTH",
                3,
                valid_at,
                head.commit_cursor,
            );
            let nodes = active_nodes(&log, valid_at, head.commit_cursor);
            for traversal_left in [true, false] {
                let text = if traversal_left {
                    format!("FROM {traversal} JOIN record:node ON node_id = id AT VALID {valid_at} KNOWN HEAD PROJECT *")
                } else {
                    format!("FROM record:node JOIN {traversal} ON id = node_id AT VALID {valid_at} KNOWN HEAD PROJECT *")
                };
                let execution = run(&engine, &head, &text);
                let mut expected = BTreeMap::new();
                for (identity, values) in &oracle {
                    let RuntimeValue::String(node_id) = &values["node_id"] else {
                        unreachable!("oracle node ids are strings")
                    };
                    if !nodes.contains(node_id) {
                        continue;
                    }
                    let record = format!("record:node:{node_id}");
                    let (left, right, side) = if traversal_left {
                        (identity.as_str(), record.as_str(), "left.")
                    } else {
                        (record.as_str(), identity.as_str(), "right.")
                    };
                    let traversal_values = values
                        .iter()
                        .map(|(field, value)| (format!("{side}{field}"), value.clone()))
                        .collect::<BTreeMap<_, _>>();
                    expected.insert(
                        format!("join:{}:{left}{right}", left.len()),
                        traversal_values,
                    );
                }
                let side = if traversal_left { "left." } else { "right." };
                let actual = rows(&execution)
                    .into_iter()
                    .map(|(identity, values)| {
                        let traversal_values = values
                            .into_iter()
                            .filter(|(field, _)| field.starts_with(side))
                            .collect::<BTreeMap<_, _>>();
                        (identity, traversal_values)
                    })
                    .collect::<Rows>();
                assert_eq!(actual, expected, "{text}");
                joined_rows += expected.len();
            }
        }
    }
    assert!(joined_rows > 0);
}

/// A chain `a -> b -> c -> d` of `link` relations in an estate that also
/// holds `noise` disconnected `link` relations, `noise` `other` relations
/// leaving `a`, and `noise` unrelated nodes.
fn chain_estate<E: StorageEngine>(engine: &E, noise: usize) -> ReadStamp {
    let mut nodes = vec![RuntimeMutation::Schema { registry: schema() }];
    nodes.extend(["a", "b", "c", "d"].map(|id| record(id, None)));
    nodes.extend((0..=noise).map(|index| record(&format!("x{index}"), None)));
    commit(engine, 1, nodes);
    let edge =
        |relation_kind: &str, id: String, from: &str, to: String| RuntimeMutation::Relation {
            relation: RuntimeRelation {
                reference: RuntimeRef::new(relation_kind, id).unwrap(),
                from: node(from),
                to: node(&to),
                valid_from: 10,
                valid_to: None,
                properties: RuntimeProperties::new(),
            },
        };
    let mut relations = vec![
        edge("link", "a-b".into(), "a", "b".into()),
        edge("link", "b-c".into(), "b", "c".into()),
        edge("link", "c-d".into(), "c", "d".into()),
    ];
    for index in 0..noise {
        relations.push(edge(
            "link",
            format!("x{index}"),
            &format!("x{index}"),
            format!("x{}", index + 1),
        ));
        relations.push(edge(
            "other",
            format!("a-x{index}"),
            "a",
            format!("x{index}"),
        ));
    }
    commit(engine, 2, relations);
    engine.runtime().read_stamp(&scope()).unwrap()
}

fn path_keys(execution: &QueryExecution, path: RuntimeReadAccessPath) -> u64 {
    execution
        .read_evidence
        .paths
        .iter()
        .find(|entry| entry.path == path)
        .map_or(0, |entry| entry.keys_examined)
}

#[test]
fn traversal_evidence_follows_visited_edges_not_estate_size() {
    const NOISE: usize = 400;
    let text =
        "FROM traverse:link START node:a DIRECTION OUTGOING DEPTH 8 AT VALID 100 KNOWN HEAD PROJECT node_id, depth";
    let small = RrflowMxStore::new();
    let small_read = chain_estate(&small, 0);
    let small_run = run(&small, &small_read, text);
    let large = RrflowMxStore::new();
    let large_read = chain_estate(&large, NOISE);
    let large_run = run(&large, &large_read, text);

    assert_eq!(rows(&small_run), rows(&large_run));
    assert_eq!(large_run.returned_rows, 3);
    assert_eq!(small_run.selected_versions, large_run.selected_versions);
    for path in [
        RuntimeReadAccessPath::ReadStamp,
        RuntimeReadAccessPath::SchemaVersions,
        RuntimeReadAccessPath::RecordVersions,
        RuntimeReadAccessPath::RelationVersions,
        RuntimeReadAccessPath::AdjacencyVersions,
    ] {
        assert_eq!(
            path_keys(&small_run, path),
            path_keys(&large_run, path),
            "{path:?}"
        );
    }
    // One start identity, four frontier adjacency ranges (the last is
    // empty), and three relation identity histories.
    assert_eq!(
        path_keys(&large_run, RuntimeReadAccessPath::RecordVersions),
        1
    );
    assert_eq!(
        path_keys(&large_run, RuntimeReadAccessPath::AdjacencyVersions),
        3
    );
    assert_eq!(
        path_keys(&large_run, RuntimeReadAccessPath::RelationVersions),
        3
    );
    let adjacency_scans = large_run
        .read_evidence
        .paths
        .iter()
        .find(|entry| entry.path == RuntimeReadAccessPath::AdjacencyVersions)
        .unwrap()
        .range_scans;
    assert_eq!(adjacency_scans, 4);
    assert!(large_run.read_evidence.keys_examined < 100);

    // The family ranges the reader replaced grow with the estate.
    let family = large
        .runtime()
        .read_versioned(
            &large_read,
            &[
                RuntimeVersionedSource::Relations {
                    kind: Some(kind("link")),
                },
                RuntimeVersionedSource::Records { kind: None },
            ],
            RuntimeReadBudget::new(100_000).unwrap(),
        )
        .unwrap();
    assert!(family.evidence.keys_examined > 2 * NOISE as u64);
}
