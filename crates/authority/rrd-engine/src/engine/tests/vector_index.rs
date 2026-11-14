use super::*;
use rrd_contract::{
    ActivateVectorQuantizationArtifact, BackupCoverageSnapshot, BuildVectorQuantizationArtifact,
    CreateInstanceBackup, DataEmbeddingProvenance, DataLogicalModel, DataProperties,
    DataPropertySchema, DataRecordSchema, DataReference, DataSchemaMode, DataSchemaRegistry,
    DataTableSchema, DataTarget, DataValueType, DataVectorNormalization, DataVectorValue,
    DeleteVectorCollection, DeleteVectorPayloadIndex, EnsureQueryIndex, EnsureVectorIndex,
    EnsureVectorPayloadIndex, ExecuteRetrievalQuery, HybridFusion, ListVectorCollections,
    ListVectorPayloadIndexes, ListVectorQuantizationArtifacts, QueryBudget, QueryIndexKind,
    QueryValue, ReadAccessPath, ReadEvidence, RestoreInstanceBackup,
    RetireVectorQuantizationArtifact, RetrievalContextPair, RetrievalFusion, RetrievalOutput,
    RetrievalPrefetch, RetrievalQuery, RetrievalRecommendStrategy, RetrievalRerankStage,
    RetrievalResultShape, RetrievalVectorExample, RetrieveVectorPoints, SearchHybrid,
    SearchVectors, VectorEmbeddingModel, VectorIndexBuildPolicy, VectorIndexBuildTarget,
    VectorIndexConfiguration, VectorIndexDifferentialStatus, VectorIndexMaintenanceMode,
    VectorPayloadCondition, VectorPayloadFilter, VectorPayloadIndexKind, VectorPayloadOperator,
    VectorProductCompression, VectorQuantizationArtifactState, VectorQuantizationBits,
    VectorQuantizationMethod, VectorSearchMode, VectorSearchQuery,
};
use std::collections::BTreeMap;

mod catalogue_reads;
mod collection_moves;
mod hnsw_read_path;
mod incremental_generations;
mod index_identities;
mod point_reads;
mod quantization_replay;
mod turboquant_overlay;

fn assert_direct_read_evidence(
    selected_versions: u64,
    evidence: &ReadEvidence,
    expected_paths: &[ReadAccessPath],
) {
    assert!(selected_versions > 0);
    evidence.validate().unwrap();
    for expected in expected_paths {
        assert!(
            evidence.paths.iter().any(|path| path.path == *expected),
            "missing direct read evidence for {expected:?}"
        );
    }
}

fn reference(kind: &str, id: &str) -> DataReference {
    DataReference {
        kind: CanonicalId::new(kind).unwrap(),
        id: CanonicalId::new(id).unwrap(),
    }
}

fn vector_mutation(id: &str, values: Vec<f32>) -> TransactionMutation {
    TransactionMutation::PutVector {
        reference: reference("embedding", id),
        subject: reference("document", id),
        collection_id: CanonicalId::new("documents").unwrap(),
        vector_name: CanonicalId::new("body").unwrap(),
        field: CanonicalId::new("body-embedding").unwrap(),
        valid_from: 1,
        valid_to: None,
        value: DataVectorValue::Dense { values },
        provenance: None,
        properties: DataProperties::new(),
    }
}

fn document_mutation(id: &str, body: &str) -> TransactionMutation {
    TransactionMutation::PutRecord {
        reference: reference("document", id),
        valid_from: 1,
        valid_to: None,
        properties: DataProperties::from([("body".into(), QueryValue::String(body.into()))]),
    }
}

fn vector_fixture_schema(
    migration: impl Into<String>,
    document: DataRecordSchema,
) -> DataSchemaRegistry {
    let document_kind = CanonicalId::new("document").unwrap();
    let document_table = DataTableSchema {
        model: DataLogicalModel::Relational,
        mode: DataSchemaMode::Strict,
        properties: BTreeMap::new(),
        allow_additional_properties: false,
    };
    DataSchemaRegistry {
        revision: 1,
        migration: migration.into(),
        organisation: DataEstateOrganisation::default(),
        tables: BTreeMap::from([
            (document_kind.clone(), document_table),
            (
                CanonicalId::new("embedding").unwrap(),
                DataTableSchema {
                    model: DataLogicalModel::Vector,
                    mode: DataSchemaMode::Schemaless,
                    properties: BTreeMap::new(),
                    allow_additional_properties: false,
                },
            ),
        ]),
        records: BTreeMap::from([(document_kind, document)]),
        relations: BTreeMap::new(),
        events: BTreeMap::new(),
    }
}

fn commit_vectors(
    engine: &RrdEngine,
    lease: &rrd_contract::SessionLease,
    ordinal: u64,
    mutations: Vec<TransactionMutation>,
) {
    let transaction = engine
        .begin_transaction(
            &lease.session_id,
            &lease.token,
            &BeginTransaction {
                scope: CanonicalId::new("data").unwrap(),
                timeout_ms: 1_000,
            },
            &mutation_context(
                &id(&format!("begin-key-{ordinal}")),
                &format!("request-begin-{ordinal}"),
                &format!("operation-begin-{ordinal}"),
            ),
            ordinal,
        )
        .unwrap();
    let request = CommitTransaction {
        operation_sha256: rrd_contract::transaction_operation_sha256(&mutations),
        mutations,
    };
    engine
        .commit_transaction(
            &lease.session_id,
            &lease.token,
            &transaction.transaction_id,
            &id(&format!("commit-key-{ordinal}")),
            &request,
            ordinal + 1,
            &format!("request-commit-{ordinal}"),
            &format!("operation-commit-{ordinal}"),
        )
        .unwrap();
}

fn index_request() -> EnsureVectorIndex {
    EnsureVectorIndex {
        scope: format!("instance:{}", instance()),
        collection_id: CanonicalId::new("documents").unwrap(),
        vector_name: CanonicalId::new("body").unwrap(),
        configuration: VectorIndexConfiguration::Hnsw {
            m: 4,
            ef_construction: 8,
            max_level: 4,
            seed: 17,
            filter_properties: Vec::new(),
        },
        build_policy: VectorIndexBuildPolicy::Cpu,
        max_storage_keys: 10_000,
    }
}

fn search_request(mode: VectorSearchMode) -> SearchVectors {
    SearchVectors {
        scope: format!("instance:{}", instance()),
        valid_at: 1,
        collection_id: Some(CanonicalId::new("documents").unwrap()),
        vector_name: Some(CanonicalId::new("body").unwrap()),
        field: None,
        query: VectorSearchQuery::Dense {
            values: vec![1.0, 0.0],
        },
        filter: None,
        metric: None,
        top_k: 1,
        mode,
        max_storage_keys: 10_000,
    }
}

fn ensure_documents_memory_tier(
    engine: &RrdEngine,
    lease: &rrd_contract::SessionLease,
    memory_tier: VectorMemoryTier,
    operation: &str,
    now: u64,
) {
    engine
        .ensure_vector_collection(
            &lease.session_id,
            &lease.token,
            &EnsureVectorCollection {
                scope: format!("instance:{}", instance()),
                collection_id: CanonicalId::new("documents").unwrap(),
                vectors: vec![NamedVectorDefinition {
                    name: CanonicalId::new("body").unwrap(),
                    field: CanonicalId::new("body-embedding").unwrap(),
                    kind: VectorValueKind::Dense,
                    dimensions: 2,
                    metric: VectorSearchMetric::Dot,
                    embedding_model: None,
                    memory_tier,
                }],
            },
            &mutation_context(
                &id(&format!("{operation}-key")),
                &format!("{operation}-request"),
                &format!("{operation}-operation"),
            ),
            now,
        )
        .unwrap();
}

mod authorization;
mod backup_restore;
mod collection_administration;
mod gpu_hnsw;
mod memory_tiers;
mod persistent_retrieval;
mod quantization_lifecycle;
mod unified_retrieval;

use gpu_hnsw::*;
use quantization_lifecycle::*;
