//! What a search reads decides its speed: a profile of exact search puts 87% of its time in
//! authenticating rows and under 1% in scoring. So a search an active HNSW generation can serve
//! must read that generation's own candidates and the changes since it, and not the corpus. This
//! holds that through a real client and server, by the read evidence each search returns.
use rrd_client::{ClientConfig, RequestOptions, RrdClient};
use rrd_contract::*;
use rrd_core::digest;
use rrd_engine::RrdEngine;
use rrd_security::{
    Action, Principal, PrincipalKind, ResourceGrant, SecurityRepository, SecurityState,
    SECURITY_FORMAT,
};
use rrd_server::RrdHttpServer;
use rrd_store::RrflowKvStore;
use std::collections::BTreeMap;
use std::time::Duration;

const DIMENSIONS: usize = 8;
const BATCH: usize = 512;
const BUCKETS: u64 = 4;
const SMALL: usize = 256;
const LARGE: usize = 2_048;

fn mutation(id: &str) -> RequestOptions {
    RequestOptions::mutation(id, id, id).unwrap()
}
fn read(id: &str) -> RequestOptions {
    RequestOptions::read(id, id).unwrap()
}
fn canonical(value: &str) -> CanonicalId {
    CanonicalId::new(value).unwrap()
}
fn data_reference(kind: &str, id: &str) -> DataReference {
    DataReference {
        kind: canonical(kind),
        id: canonical(id),
    }
}

/// Deterministic and distinct, because a graph over repeated vectors says little about recall.
fn values(seed: usize) -> Vec<f32> {
    (0..DIMENSIONS)
        .map(|dimension| {
            let mut state = (seed as u64 + 1)
                .wrapping_mul(0x9E37_79B9_7F4A_7C15)
                .wrapping_add((dimension as u64 + 1).wrapping_mul(0xC2B2_AE3D_27D4_EB4F));
            state ^= state >> 29;
            state = state.wrapping_mul(0xBF58_476D_1CE4_E5B9);
            state ^= state >> 32;
            (state % 2_000) as f32 / 1_000.0 - 1.0
        })
        .collect()
}

fn schema() -> DataSchemaRegistry {
    let table = |model| DataTableSchema {
        model,
        mode: DataSchemaMode::Schemaless,
        properties: BTreeMap::new(),
        allow_additional_properties: false,
    };
    DataSchemaRegistry {
        revision: 1,
        migration: "install vector read path fixture".into(),
        organisation: DataEngramOrganisation::default(),
        tables: BTreeMap::from([
            (canonical("document"), table(DataLogicalModel::Document)),
            (canonical("embedding"), table(DataLogicalModel::Vector)),
        ]),
        records: BTreeMap::new(),
        relations: BTreeMap::new(),
        events: BTreeMap::new(),
    }
}

fn point(index: usize) -> [TransactionMutation; 2] {
    let id = format!("p{index}");
    [
        TransactionMutation::PutRecord {
            reference: data_reference("document", &id),
            valid_from: 1,
            valid_to: None,
            properties: DataProperties::new(),
        },
        TransactionMutation::PutVector {
            reference: data_reference("embedding", &id),
            subject: data_reference("document", &id),
            collection_id: canonical("documents"),
            vector_name: canonical("body"),
            field: canonical("body-embedding"),
            valid_from: 1,
            valid_to: None,
            value: DataVectorValue::Dense {
                values: values(index),
            },
            provenance: None,
            properties: DataProperties::from([
                (
                    "bucket".into(),
                    QueryValue::Unsigned(index as u64 % BUCKETS),
                ),
                ("unindexed".into(), QueryValue::Unsigned(index as u64 % 2)),
            ]),
        },
    ]
}

fn principal(name: &str, resource: &ResourcePath) -> Principal {
    Principal {
        id: canonical(name),
        kind: PrincipalKind::Service,
        credential_sha256: digest::sha256_hex(name.as_bytes()),
        credential_revision: 1,
        not_before_unix_ms: 1,
        expires_at_unix_ms: u64::MAX,
        disabled: false,
        role_ids: Default::default(),
        grants: [
            Action::SessionCreate,
            Action::TransactionBegin,
            Action::TransactionCommit,
            Action::VectorCollectionEnsure,
            Action::VectorPayloadIndexEnsure,
            Action::VectorSearch,
        ]
        .into_iter()
        .map(|action| ResourceGrant {
            action,
            resource_prefix: resource.clone(),
            data_policy: None,
        })
        .collect(),
    }
}

fn bucket_filter() -> VectorPayloadFilter {
    VectorPayloadFilter::Condition {
        condition: VectorPayloadCondition {
            property: canonical("bucket"),
            operator: VectorPayloadOperator::Equals {
                value: QueryValue::Unsigned(2),
            },
        },
    }
}

/// The storage work of a search, apart from the accumulator proof, whose inclusion paths grow with
/// the logarithm of the engram's leaves.
type StoragePaths = Vec<(ReadAccessPath, u64, u64, u64, u64)>;

#[derive(Debug, PartialEq)]
struct Reads {
    access_path: String,
    selected_versions: u64,
    canonical_candidates: u64,
    storage: StoragePaths,
}

#[derive(Debug)]
struct Measured {
    plain: Reads,
    filtered: Reads,
    plain_proof: u64,
    plain_keys: u64,
    refusal: String,
}

fn reads(result: &VectorSearchResult) -> (Reads, u64) {
    let storage = result
        .read_evidence
        .paths
        .iter()
        .filter(|path| path.path != ReadAccessPath::AccumulatorProof)
        .map(|path| {
            (
                path.path,
                path.point_reads,
                path.range_scans,
                path.keys_examined,
                path.values_decoded,
            )
        })
        .collect();
    let proof = result
        .read_evidence
        .paths
        .iter()
        .find(|path| path.path == ReadAccessPath::AccumulatorProof)
        .map_or(0, |path| path.keys_examined);
    (
        Reads {
            access_path: result.access_path.to_string(),
            selected_versions: result.selected_versions,
            canonical_candidates: result.resources.canonical_candidates,
            storage,
        },
        proof,
    )
}

fn neighbors(result: &VectorSearchResult) -> Vec<String> {
    result
        .hits
        .iter()
        .map(|hit| hit.reference.id.to_string())
        .collect()
}

async fn measure(points: usize) -> Measured {
    let root = tempfile::tempdir().unwrap();
    let instance = canonical("readpath");
    let scope = format!("instance:{instance}");
    let storage = RrflowKvStore::open(root.path()).unwrap();
    let resource = ResourcePath {
        segments: vec![ResourceId::new(ResourceKind::Instance, "readpath").unwrap()],
    };
    let builder = principal("builder", &resource);
    SecurityRepository::new(&storage, instance.clone())
        .initialize(
            SecurityState {
                format_version: SECURITY_FORMAT,
                revision: 1,
                principals: BTreeMap::from([(builder.id.clone(), builder)]),
                roles: BTreeMap::new(),
                identity_bindings: BTreeMap::new(),
                jwt_issuers: BTreeMap::new(),
            },
            1,
            "bootstrap",
            "bootstrap",
            "bootstrap",
        )
        .unwrap();
    drop(storage);
    let engine = RrdEngine::open(root.path(), instance.clone(), [0x6c; 32]).unwrap();
    let server = RrdHttpServer::bind(engine, "127.0.0.1:0".parse().unwrap()).unwrap();
    let client = RrdClient::connect_local(
        server.local_addr(),
        instance.clone(),
        ClientConfig {
            // A build runs inside its one request.
            request_timeout: Duration::from_secs(300),
            ..ClientConfig::default()
        },
    )
    .unwrap();
    let (shutdown, receiver) = tokio::sync::oneshot::channel();
    let running = tokio::spawn(server.serve_until(async move {
        let _ = receiver.await;
    }));
    let session = client
        .create_session(
            canonical("builder"),
            "builder",
            CreateSession {
                limits: SessionLimits {
                    idle_timeout_ms: 600_000,
                    absolute_timeout_ms: 900_000,
                    max_open_transactions: 1,
                },
            },
            mutation("session"),
        )
        .await
        .unwrap();
    client
        .ensure_vector_collection(
            &session,
            EnsureVectorCollection {
                scope: scope.clone(),
                collection_id: canonical("documents"),
                vectors: vec![NamedVectorDefinition {
                    name: canonical("body"),
                    field: canonical("body-embedding"),
                    kind: VectorValueKind::Dense,
                    dimensions: DIMENSIONS as u32,
                    metric: VectorSearchMetric::Cosine,
                    embedding_model: None,
                    memory_tier: VectorMemoryTier::Cached,
                }],
            },
            mutation("collection"),
        )
        .await
        .unwrap();
    client
        .ensure_vector_payload_index(
            &session,
            EnsureVectorPayloadIndex {
                scope: scope.clone(),
                collection_id: canonical("documents"),
                field: canonical("bucket"),
                kind: VectorPayloadIndexKind::Unsigned,
            },
            mutation("payload-index"),
        )
        .await
        .unwrap();

    let mut batches = vec![vec![TransactionMutation::PutSchema { registry: schema() }]];
    for start in (0..points).step_by(BATCH) {
        batches.push(
            (start..(start + BATCH).min(points))
                .flat_map(point)
                .collect(),
        );
    }
    for (ordinal, mutations) in batches.into_iter().enumerate() {
        let transaction = client
            .begin_transaction(
                &session,
                BeginTransaction {
                    scope: canonical("data"),
                    timeout_ms: 300_000,
                },
                mutation(&format!("begin-{ordinal}")),
            )
            .await
            .unwrap();
        let key = format!("commit-{ordinal}");
        client
            .commit_transaction(
                &session,
                &transaction.transaction_id,
                CommitTransaction {
                    operation_sha256: transaction_operation_sha256(&mutations),
                    mutations,
                },
                RequestOptions::new(
                    &key,
                    &key,
                    Some(&key),
                    Some(
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_millis() as u64
                            + 300_000,
                    ),
                )
                .unwrap(),
            )
            .await
            .unwrap();
    }
    client
        .ensure_vector_index(
            &session,
            EnsureVectorIndex {
                scope: scope.clone(),
                collection_id: canonical("documents"),
                vector_name: canonical("body"),
                configuration: VectorIndexConfiguration::Hnsw {
                    m: 16,
                    ef_construction: 128,
                    max_level: 6,
                    seed: 17,
                    filter_properties: vec![canonical("bucket")],
                },
                build_policy: VectorIndexBuildPolicy::Cpu,
                max_storage_keys: 1_000_000,
            },
            mutation("index"),
        )
        .await
        .unwrap();

    let search = |filter, mode, max_storage_keys| SearchVectors {
        scope: scope.clone(),
        valid_at: 2,
        collection_id: Some(canonical("documents")),
        vector_name: Some(canonical("body")),
        field: None,
        query: VectorSearchQuery::Dense {
            values: values(points + 9),
        },
        filter,
        metric: None,
        top_k: 5,
        mode,
        max_storage_keys,
    };
    let approximate = VectorSearchMode::RequireApproximate {
        exact_rerank: 32,
        ef_search: 128,
    };

    let plain = client
        .search_vectors(
            &session,
            search(None, approximate, 1_000_000),
            read("search-plain"),
        )
        .await
        .unwrap();
    let filtered = client
        .search_vectors(
            &session,
            search(Some(bucket_filter()), approximate, 1_000_000),
            read("search-filtered"),
        )
        .await
        .unwrap();
    for (label, served, filter) in [
        ("plain", &plain, None),
        ("filtered", &filtered, Some(bucket_filter())),
    ] {
        let exact = client
            .search_vectors(
                &session,
                search(filter, VectorSearchMode::Exact, 1_000_000),
                read(&format!("exact-{label}")),
            )
            .await
            .unwrap();
        assert_eq!(served.access_path, canonical("hnsw"), "{points} {label}");
        assert!(!served.exact, "{points} {label}");
        assert_eq!(exact.hits.len(), 5, "{points} {label}");
        assert_eq!(neighbors(served), neighbors(&exact), "{points} {label}");
        assert_eq!(
            served.hits.iter().map(|hit| hit.score).collect::<Vec<_>>(),
            exact.hits.iter().map(|hit| hit.score).collect::<Vec<_>>(),
            "{points} {label}: reranked exactly"
        );
    }

    // With a rerank narrower than the graph can fill exactly, this is the real oracle: the same
    // graph, read two ways, proposes the same identities and reranks them to the same answer.
    for query in 0..20 {
        let question = |mode| {
            let mut request = search(None, mode, 1_000_000);
            request.query = VectorSearchQuery::Dense {
                values: values(points + 100 + query),
            };
            request
        };
        let generation = client
            .search_vectors(
                &session,
                question(VectorSearchMode::RequireApproximate {
                    exact_rerank: 8,
                    ef_search: 8,
                }),
                read(&format!("narrow-generation-{query}")),
            )
            .await
            .unwrap();
        let general = client
            .search_vectors(
                &session,
                question(VectorSearchMode::AllowApproximate {
                    exact_rerank: 8,
                    ef_search: 8,
                }),
                read(&format!("narrow-general-{query}")),
            )
            .await
            .unwrap();
        assert_eq!(general.access_path, canonical("hnsw"), "{points} {query}");
        assert!(
            generation.selected_versions < general.selected_versions,
            "{points} {query}"
        );
        assert_eq!(
            neighbors(&generation),
            neighbors(&general),
            "{points} {query}"
        );
        assert_eq!(
            generation
                .hits
                .iter()
                .map(|hit| hit.score)
                .collect::<Vec<_>>(),
            general.hits.iter().map(|hit| hit.score).collect::<Vec<_>>(),
            "{points} {query}"
        );
    }

    // No generation covers this property, so nothing can satisfy the request. It is told so, under
    // a budget far below what reading the corpus would take.
    let refused = client
        .search_vectors(
            &session,
            search(
                Some(VectorPayloadFilter::Condition {
                    condition: VectorPayloadCondition {
                        property: canonical("unindexed"),
                        operator: VectorPayloadOperator::Equals {
                            value: QueryValue::Unsigned(1),
                        },
                    },
                }),
                approximate,
                128,
            ),
            read("search-refused"),
        )
        .await
        .unwrap_err();

    shutdown.send(()).unwrap();
    running.await.unwrap().unwrap();

    let (plain_reads, plain_proof) = reads(&plain);
    let (filtered_reads, _) = reads(&filtered);
    eprintln!(
        "{points} points: plain keys {}, plain {plain_reads:?}, proof {plain_proof}; filtered keys {}, filtered {filtered_reads:?}; refusal {refused}",
        plain.read_evidence.keys_examined, filtered.read_evidence.keys_examined,
    );
    Measured {
        plain: plain_reads,
        filtered: filtered_reads,
        plain_proof,
        plain_keys: plain.read_evidence.keys_examined,
        refusal: refused.to_string(),
    }
}

#[tokio::test]
async fn an_hnsw_search_reads_what_it_reranks_and_not_the_corpus() {
    let small = measure(SMALL).await;
    let large = measure(LARGE).await;

    assert_eq!(small.plain, large.plain, "a plain search's work stays flat");
    assert_eq!(
        small.filtered, large.filtered,
        "a filtered search's work stays flat"
    );
    assert!(
        large.plain.canonical_candidates <= 32,
        "it hydrates only what it reranks: {:?}",
        large.plain
    );
    // Eight times the points adds three levels to each inclusion path, and no more.
    assert!(
        large.plain_proof < small.plain_proof * 2,
        "the proof grows with the logarithm of the engram: {} then {}",
        small.plain_proof,
        large.plain_proof
    );
    assert!(
        large.plain_keys < small.plain_keys * 2,
        "{} then {}",
        small.plain_keys,
        large.plain_keys
    );
}

#[tokio::test]
async fn a_search_no_generation_can_satisfy_is_refused_without_reading_the_corpus() {
    for points in [SMALL, LARGE] {
        let measured = measure(points).await;
        assert!(
            measured
                .refusal
                .contains("no vector access path satisfies the request contract"),
            "{points} points got {}",
            measured.refusal
        );
    }
}
