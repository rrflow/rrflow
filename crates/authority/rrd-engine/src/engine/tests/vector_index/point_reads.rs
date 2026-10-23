use super::*;

fn canonical(value: &str) -> CanonicalId {
    CanonicalId::new(value).unwrap()
}

fn retrieve(
    engine: &RrdEngine,
    lease: &rrd_contract::SessionLease,
    collection: &str,
    valid_at: u64,
    max_keys: u64,
    now: u64,
) -> rrd_contract::VectorPointBatch {
    engine
        .retrieve_vector_points(
            &lease.session_id,
            &lease.token,
            &RetrieveVectorPoints {
                scope: format!("instance:{}", instance()),
                collection_id: canonical(collection),
                vector_name: canonical("body"),
                valid_at,
                references: vec![
                    reference("embedding", "missing"),
                    reference("embedding", "gpu-a"),
                ],
                max_storage_keys: max_keys,
            },
            now,
            "point-read",
            "point-read",
        )
        .unwrap()
}
fn vector_reads(batch: &rrd_contract::VectorPointBatch) -> (u64, u64) {
    batch
        .read_evidence
        .paths
        .iter()
        .filter(|p| p.path == ReadAccessPath::VectorVersions)
        .fold((0, 0), |sum, p| {
            (sum.0 + p.keys_examined, sum.1 + p.values_decoded)
        })
}

#[test]
fn addressed_point_reads_exclude_unrelated_growth_and_preserve_temporal_reopen() {
    let root = tempfile::tempdir().unwrap();
    let engine = RrdEngine::open(root.path(), instance(), TOKEN_KEY).unwrap();
    let lease = prepare_gpu_index_fixture(&engine);
    let first = retrieve(&engine, &lease, "documents", 1, 10_000, 350);
    let mut unrelated = Vec::new();
    for n in 0..96 {
        let key = format!("unrelated-{n}");
        unrelated.push(document_mutation(&key, "unrelated corpus"));
        unrelated.push(vector_mutation(&key, vec![0.25, 0.75]));
    }
    commit_vectors(&engine, &lease, 400, unrelated);
    // Fixed admission must fund the requested histories and their proofs, not
    // every vector in the estate. The old whole-family path exhausts it.
    let after = retrieve(&engine, &lease, "documents", 1, 128, 450);
    assert_eq!(after.points, first.points);
    assert_eq!(after.missing, vec![reference("embedding", "missing")]);
    assert_eq!(after.selected_versions, 1);
    assert_eq!(vector_reads(&after), vector_reads(&first));
    assert_eq!(vector_reads(&after).1, 1);
    let mut changed = vector_mutation("gpu-a", vec![0.5, 0.5]);
    if let TransactionMutation::PutVector { valid_from, .. } = &mut changed {
        *valid_from = 2;
    }
    commit_vectors(&engine, &lease, 500, vec![changed]);
    assert_eq!(
        retrieve(&engine, &lease, "documents", 1, 128, 550).points,
        first.points
    );
    let updated = retrieve(&engine, &lease, "documents", 2, 128, 551);
    assert_eq!(updated.selected_versions, 2);
    assert_eq!(
        updated.points[0].value,
        DataVectorValue::Dense {
            values: vec![0.5, 0.5]
        }
    );
    commit_vectors(
        &engine,
        &lease,
        600,
        vec![TransactionMutation::RetireData {
            model: DataLogicalModel::Vector,
            target: DataTarget::Reference {
                reference: reference("embedding", "gpu-a"),
            },
            effective_at: 3,
        }],
    );
    let retired = retrieve(&engine, &lease, "documents", 3, 128, 650);
    assert!(retired.points.is_empty());
    assert_eq!(
        retired.missing,
        vec![
            reference("embedding", "missing"),
            reference("embedding", "gpu-a")
        ]
    );
    assert_eq!(retired.selected_versions, 3);
    // Retirement adds one authoritative version. Before its effective time the
    // value remains visible, now attributed to that newly committed version.
    let mut retired_visible = updated.points.clone();
    retired_visible[0].source_cursor = updated.known_at_cursor + 1;
    assert_eq!(
        retrieve(&engine, &lease, "documents", 2, 128, 660).points,
        retired_visible
    );
    drop(engine);
    let reopened = RrdEngine::open(root.path(), instance(), TOKEN_KEY).unwrap();
    assert_eq!(
        retrieve(&reopened, &lease, "documents", 1, 128, 700).points,
        first.points
    );
    assert_eq!(
        retrieve(&reopened, &lease, "documents", 2, 128, 701).points,
        retired_visible
    );
    assert!(retrieve(&reopened, &lease, "documents", 3, 128, 702)
        .points
        .is_empty());
    let request = RetrieveVectorPoints {
        scope: format!("instance:{}", instance()),
        collection_id: canonical("documents"),
        vector_name: canonical("body"),
        valid_at: 2,
        references: vec![reference("embedding", "gpu-a")],
        max_storage_keys: 1,
    };
    let error = reopened
        .retrieve_vector_points(
            &lease.session_id,
            &lease.token,
            &request,
            850,
            "point-budget",
            "point-budget",
        )
        .unwrap_err();
    assert_eq!(error.kind(), crate::ServiceErrorKind::ResourceExhausted);
}

#[test]
fn addressed_point_reads_do_not_resurrect_an_older_collection_address() {
    let root = tempfile::tempdir().unwrap();
    let engine = RrdEngine::open(root.path(), instance(), TOKEN_KEY).unwrap();
    let lease = prepare_gpu_index_fixture(&engine);
    engine
        .ensure_vector_collection(
            &lease.session_id,
            &lease.token,
            &EnsureVectorCollection {
                scope: format!("instance:{}", instance()),
                collection_id: canonical("archive"),
                vectors: vec![NamedVectorDefinition {
                    name: canonical("body"),
                    field: canonical("body-embedding"),
                    kind: VectorValueKind::Dense,
                    dimensions: 2,
                    metric: VectorSearchMetric::Dot,
                    embedding_model: None,
                    memory_tier: VectorMemoryTier::Cached,
                }],
            },
            &mutation_context(
                &id("archive-collection"),
                "archive-collection",
                "archive-collection",
            ),
            400,
        )
        .unwrap();
    let original = retrieve(&engine, &lease, "documents", 1, 10_000, 450);
    let mut moved = vector_mutation("gpu-a", vec![0.2, 0.8]);
    if let TransactionMutation::PutVector {
        collection_id,
        valid_from,
        ..
    } = &mut moved
    {
        *collection_id = canonical("archive");
        *valid_from = 2;
    }
    commit_vectors(&engine, &lease, 500, vec![moved]);
    assert_eq!(
        retrieve(&engine, &lease, "documents", 1, 128, 550).points,
        original.points
    );
    assert!(retrieve(&engine, &lease, "documents", 2, 128, 551)
        .points
        .is_empty());
    let archive = retrieve(&engine, &lease, "archive", 2, 128, 552);
    assert_eq!(archive.points.len(), 1);
    assert_eq!(
        archive.points[0].value,
        DataVectorValue::Dense {
            values: vec![0.2, 0.8]
        }
    );
    drop(engine);
    let reopened = RrdEngine::open(root.path(), instance(), TOKEN_KEY).unwrap();
    assert!(retrieve(&reopened, &lease, "documents", 2, 128, 600)
        .points
        .is_empty());
    assert_eq!(
        retrieve(&reopened, &lease, "archive", 2, 128, 601).points,
        archive.points
    );
}
