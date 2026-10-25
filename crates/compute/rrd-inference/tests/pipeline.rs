use rrd_contract::{
    router_model_handshake_sha256, RouteDecisionKind, RouterBackendDescriptor,
    RouterModelHandshake, RouterModelManifest,
};
use rrd_core::{
    digest, ReadStamp, RuntimeCommit, RuntimeId, RuntimeLogicalModel, RuntimeMutation,
    RuntimeProperties, RuntimeRecord, RuntimeRecordSchema, RuntimeRef, RuntimeSchemaRegistry,
    RuntimeTableSchema, RuntimeType, RuntimeValue, ScopeId, VectorCollectionAddress, VectorValue,
};
use rrd_inference::{
    load_router_model_after_handshake, EmbeddingBackend, EmbeddingBackendRegistry,
    EmbeddingCoordinator, EmbeddingJob, EmbeddingRequest, EmbeddingSourceReader,
    EmbeddingSourceSnapshot, ExecutionTarget, FeatureHashBackend, InferenceTrustBoundary,
    NetworkPolicy, NetworkRequirement, RouterModelArtifacts, EMBEDDING_CONTRACT_VERSION,
};
use rrd_store::{RrflowMxStore, StorageEngine};
use std::collections::BTreeSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

const ROUTER_MODEL_BYTES: &[u8] = b"golden router model bytes";
const ROUTER_TOKENIZER_BYTES: &[u8] = b"golden tokenizer bytes";
const ROUTER_RUNTIME_BYTES: &[u8] = b"golden runtime bytes";
const ROUTER_GRAMMAR_BYTES: &[u8] = b"golden deterministic grammar bytes";

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct GoldenRouterModel {
    manifest: RouterModelManifest,
    backend: RouterBackendDescriptor,
    handshake: RouterModelHandshake,
}

fn golden_router_model() -> GoldenRouterModel {
    serde_json::from_str(include_str!(
        "../../../transport/rrd-contract/fixtures/model-manifest-v1.json"
    ))
    .unwrap()
}

struct SequenceReader {
    snapshots: Vec<EmbeddingSourceSnapshot>,
    reads: usize,
}

impl EmbeddingSourceReader for SequenceReader {
    fn read(&mut self, _source: &RuntimeRef) -> rrd_core::Result<EmbeddingSourceSnapshot> {
        let snapshot = self.snapshots[self.reads.min(self.snapshots.len() - 1)].clone();
        self.reads += 1;
        Ok(snapshot)
    }
}

fn job(bytes: &[u8], backend: &FeatureHashBackend) -> EmbeddingJob {
    let scope = ScopeId::new("instance:embedding").unwrap();
    EmbeddingJob {
        contract_version: EMBEDDING_CONTRACT_VERSION,
        id: RuntimeId::new("job-1").unwrap(),
        scope: scope.clone(),
        read: ReadStamp::new(scope, None, 0, 7, Some("11".repeat(32))).unwrap(),
        source: RuntimeRef::new("document", "source-1").unwrap(),
        expected_source_digest: digest::sha256_hex(bytes),
        target: RuntimeRef::new("embedding", "source-1-body").unwrap(),
        subject: RuntimeRef::new("document", "source-1").unwrap(),
        collection: VectorCollectionAddress {
            collection_id: "documents".into(),
            vector_name: "body".into(),
        },
        field: "body".into(),
        valid_from: 10,
        valid_to: None,
        model: backend.descriptor().model.clone(),
        network_policy: NetworkPolicy::Deny,
        requested_at: 11,
        properties: RuntimeProperties::new(),
    }
}

#[test]
fn offline_generation_is_deterministic_normalized_and_provenance_bound() {
    let bytes = b"RRD makes source-grounded reasoning durable";
    let mut backend = FeatureHashBackend::new(64, 7).unwrap();
    let job = job(bytes, &backend);
    let mut expected_job_identity = b"rrflow-embedding-job-v1\0".to_vec();
    expected_job_identity.extend_from_slice(&serde_json::to_vec(&job).unwrap());
    assert_eq!(
        job.digest().unwrap(),
        digest::sha256_hex(&expected_job_identity)
    );
    let snapshot =
        EmbeddingSourceSnapshot::for_bytes(job.source.clone(), "text/plain", bytes.to_vec())
            .unwrap();
    let mut reader = SequenceReader {
        snapshots: vec![snapshot],
        reads: 0,
    };
    let prepared = EmbeddingCoordinator::prepare(&job, &mut reader, &mut backend).unwrap();
    prepared.validate().unwrap();
    assert_eq!(reader.reads, 2);
    let VectorValue::Dense { values } = &prepared.vector.value else {
        panic!("feature hash must produce dense vectors")
    };
    let norm = values
        .iter()
        .map(|value| f64::from(*value).powi(2))
        .sum::<f64>();
    assert!((norm - 1.0).abs() <= 1e-6);
    assert_eq!(
        prepared.vector.provenance.as_ref().unwrap().source_digest,
        digest::sha256_hex(bytes)
    );
    let transaction = prepared.transaction(&job, "agent:embedding", 12).unwrap();
    assert_eq!(transaction.read, job.read);
    assert_eq!(transaction.commit.expected_cursor, 7);
}

#[test]
fn source_change_during_inference_fails_before_a_mutation_exists() {
    let original = b"original source";
    let changed = b"changed source";
    let mut backend = FeatureHashBackend::new(32, 9).unwrap();
    let job = job(original, &backend);
    let mut reader = SequenceReader {
        snapshots: vec![
            EmbeddingSourceSnapshot::for_bytes(job.source.clone(), "text/plain", original.to_vec())
                .unwrap(),
            EmbeddingSourceSnapshot::for_bytes(job.source.clone(), "text/plain", changed.to_vec())
                .unwrap(),
        ],
        reads: 0,
    };
    let error = EmbeddingCoordinator::prepare(&job, &mut reader, &mut backend).unwrap_err();
    assert!(error.to_string().contains("changed during inference"));
}

struct MalformedBackend {
    descriptor: rrd_inference::EmbeddingBackendDescriptor,
}

struct CountingBackend {
    descriptor: rrd_inference::EmbeddingBackendDescriptor,
    calls: Arc<AtomicUsize>,
}

impl EmbeddingBackend for CountingBackend {
    fn descriptor(&self) -> &rrd_inference::EmbeddingBackendDescriptor {
        &self.descriptor
    }

    fn embed(&mut self, _request: &EmbeddingRequest) -> rrd_core::Result<VectorValue> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let mut values = vec![0.0; self.descriptor.model.dimensions as usize];
        values[0] = 1.0;
        Ok(VectorValue::Dense { values })
    }
}

fn embedding_request(id: &str, bytes: &[u8]) -> EmbeddingRequest {
    EmbeddingRequest {
        job_id: RuntimeId::new(id).unwrap(),
        job_digest: digest::sha256_hex(format!("job:{id}").as_bytes()),
        source_digest: digest::sha256_hex(bytes),
        media_type: "text/plain".into(),
        bytes: bytes.to_vec(),
    }
}

#[test]
fn feature_hash_rejects_zero_norm_before_returning_output_and_releases_admission() {
    use rrd_inference::{EmbeddingCancellation, EmbeddingExecutionControl};
    use std::time::{Duration, Instant};
    let mut backend = FeatureHashBackend::new(32, 44).unwrap();
    let bad = embedding_request("zero", b"source-1");
    let good = embedding_request("nonzero", b"source0");
    // The two tokens hash to bucket 20 with opposite signs. Direct callers
    // must receive a failure, not rely on a later registry validation layer.
    let error = backend.embed(&bad).unwrap_err();
    assert!(error.to_string().contains("no finite nonzero norm"));
    assert!(backend.embed_batch(&[good.clone(), bad.clone()]).is_err());
    let descriptor = backend.descriptor().clone();
    let mut registry = EmbeddingBackendRegistry::default();
    registry.install(Box::new(backend)).unwrap();
    let error = registry
        .try_acquire(&descriptor.id)
        .unwrap()
        .embed_batch_with_control(
            descriptor.model.clone(),
            NetworkPolicy::Deny,
            vec![bad],
            EmbeddingExecutionControl::new(
                Instant::now() + Duration::from_secs(5),
                EmbeddingCancellation::default(),
            ),
        )
        .unwrap_err();
    assert!(error.to_string().contains("no finite nonzero norm"));
    let recovered = registry
        .try_acquire(&descriptor.id)
        .unwrap()
        .embed_batch(&descriptor.model, NetworkPolicy::Deny, &[good])
        .unwrap();
    let VectorValue::Dense { values } = &recovered.vectors[0] else {
        panic!("expected dense vector")
    };
    assert_eq!(values.len(), 32);
    assert!(values.iter().all(|v| v.is_finite()));
    let norm: f64 = values.iter().map(|v| f64::from(*v).powi(2)).sum();
    assert!((norm - 1.0).abs() < 1e-4);
}

impl EmbeddingBackend for MalformedBackend {
    fn descriptor(&self) -> &rrd_inference::EmbeddingBackendDescriptor {
        &self.descriptor
    }

    fn embed(&mut self, _request: &EmbeddingRequest) -> rrd_core::Result<VectorValue> {
        Ok(VectorValue::Dense {
            values: vec![1.0, 0.0],
        })
    }
}

#[test]
fn malformed_output_shape_fails_closed() {
    let bytes = b"shape mismatch";
    let baseline = FeatureHashBackend::new(32, 5).unwrap();
    let job = job(bytes, &baseline);
    let snapshot =
        EmbeddingSourceSnapshot::for_bytes(job.source.clone(), "text/plain", bytes.to_vec())
            .unwrap();
    let mut reader = SequenceReader {
        snapshots: vec![snapshot],
        reads: 0,
    };
    let mut backend = MalformedBackend {
        descriptor: baseline.descriptor().clone(),
    };
    assert!(EmbeddingCoordinator::prepare(&job, &mut reader, &mut backend).is_err());
}

#[test]
fn offline_policy_and_exact_model_identity_are_enforced_before_inference() {
    let bytes = b"policy enforcement";
    let baseline = FeatureHashBackend::new(32, 5).unwrap();
    let job = job(bytes, &baseline);
    let snapshot =
        EmbeddingSourceSnapshot::for_bytes(job.source.clone(), "text/plain", bytes.to_vec())
            .unwrap();
    let mut reader = SequenceReader {
        snapshots: vec![snapshot.clone()],
        reads: 0,
    };
    let mut remote_descriptor = baseline.descriptor().clone();
    remote_descriptor.execution = ExecutionTarget::Remote {
        provider: "example".into(),
    };
    remote_descriptor.network = NetworkRequirement::Required;
    remote_descriptor.trust = InferenceTrustBoundary::RemoteProvider {
        provider: "example".into(),
    };
    let mut remote = MalformedBackend {
        descriptor: remote_descriptor,
    };
    let error = EmbeddingCoordinator::prepare(&job, &mut reader, &mut remote).unwrap_err();
    assert!(error.to_string().contains("denies the network"));
    assert_eq!(reader.reads, 0);

    let mut reader = SequenceReader {
        snapshots: vec![snapshot],
        reads: 0,
    };
    let mut wrong_descriptor = baseline.descriptor().clone();
    wrong_descriptor.model.revision = "other".into();
    let mut wrong_model = MalformedBackend {
        descriptor: wrong_descriptor,
    };
    let error = EmbeddingCoordinator::prepare(&job, &mut reader, &mut wrong_model).unwrap_err();
    assert!(error.to_string().contains("requested model"));
    assert_eq!(reader.reads, 0);
}

#[test]
fn registry_batches_exact_models_and_rejects_duplicate_or_corrupt_input() {
    let backend = FeatureHashBackend::new(32, 41).unwrap();
    let descriptor = backend.descriptor().clone();
    let mut registry = EmbeddingBackendRegistry::default();
    assert_eq!(registry.install(Box::new(backend)).unwrap(), 1);
    assert_eq!(registry.revision(), 1);
    assert_eq!(registry.descriptors(), vec![descriptor.clone()]);

    let requests = vec![
        embedding_request("batch-a", b"alpha source"),
        embedding_request("batch-b", b"beta source"),
    ];
    let result = registry
        .try_acquire(&descriptor.id)
        .unwrap()
        .embed_batch(&descriptor.model, NetworkPolicy::Deny, &requests)
        .unwrap();
    assert_eq!(result.registry_revision, 1);
    assert_eq!(result.backend, descriptor);
    assert_eq!(result.vectors.len(), 2);

    assert!(registry
        .install(Box::new(FeatureHashBackend::new(32, 41).unwrap()))
        .unwrap_err()
        .to_string()
        .contains("already installed"));
    let mut corrupt = embedding_request("corrupt", b"original");
    corrupt.bytes = b"changed".to_vec();
    assert!(registry
        .try_acquire(&result.backend.id)
        .unwrap()
        .embed_batch(&result.backend.model, NetworkPolicy::Deny, &[corrupt],)
        .unwrap_err()
        .to_string()
        .contains("source digest"));
}

#[test]
fn registry_enforces_provider_trust_and_resource_limits_before_dispatch() {
    let baseline = FeatureHashBackend::new(32, 43).unwrap();
    let mut remote = baseline.descriptor().clone();
    remote.id = "provider:test:v1".into();
    remote.execution = ExecutionTarget::Remote {
        provider: "test-provider".into(),
    };
    remote.network = NetworkRequirement::Required;
    remote.trust = InferenceTrustBoundary::RemoteProvider {
        provider: "test-provider".into(),
    };
    let calls = Arc::new(AtomicUsize::new(0));
    let mut registry = EmbeddingBackendRegistry::default();
    registry
        .install(Box::new(CountingBackend {
            descriptor: remote.clone(),
            calls: Arc::clone(&calls),
        }))
        .unwrap();
    let requests = vec![embedding_request("provider-a", b"provider input")];
    assert!(registry
        .try_acquire(&remote.id)
        .unwrap()
        .embed_batch(&remote.model, NetworkPolicy::Deny, &requests,)
        .unwrap_err()
        .to_string()
        .contains("denies the network"));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(
        registry
            .try_acquire(&remote.id)
            .unwrap()
            .embed_batch(&remote.model, NetworkPolicy::Allow, &requests,)
            .unwrap()
            .vectors
            .len(),
        1
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);

    let mut bounded = baseline.descriptor().clone();
    bounded.id = "bounded:test:v1".into();
    bounded.resources.maximum_batch_inputs = 1;
    let bounded_calls = Arc::new(AtomicUsize::new(0));
    registry
        .install(Box::new(CountingBackend {
            descriptor: bounded.clone(),
            calls: Arc::clone(&bounded_calls),
        }))
        .unwrap();
    assert!(registry
        .try_acquire(&bounded.id)
        .unwrap()
        .embed_batch(
            &bounded.model,
            NetworkPolicy::Deny,
            &[
                embedding_request("bounded-a", b"first"),
                embedding_request("bounded-b", b"second"),
            ],
        )
        .unwrap_err()
        .to_string()
        .contains("input count"));
    assert_eq!(bounded_calls.load(Ordering::SeqCst), 0);
}

#[test]
fn transaction_cas_rejects_a_runtime_source_change_after_inference() {
    let bytes = b"source at captured cursor";
    let mut backend = FeatureHashBackend::new(32, 17).unwrap();
    let mut job = job(bytes, &backend);
    let engine = RrflowMxStore::default();
    let mut properties = RuntimeProperties::new();
    properties.insert(
        "content_digest".into(),
        RuntimeValue::Digest(digest::sha256_hex(bytes)),
    );
    engine
        .runtime()
        .commit(&RuntimeCommit {
            scope: job.scope.clone(),
            at: 10,
            actor: "test:source".into(),
            expected_cursor: 0,
            mutations: vec![
                RuntimeMutation::Schema {
                    registry: {
                        let mut registry =
                            RuntimeSchemaRegistry::empty(1, "embedding source test schema");
                        registry
                            .define_record_table(
                                RuntimeType::new("document").unwrap(),
                                RuntimeLogicalModel::Relational,
                                RuntimeRecordSchema {
                                    allow_additional_properties: true,
                                    ..RuntimeRecordSchema::default()
                                },
                            )
                            .unwrap();
                        registry.tables.insert(
                            RuntimeType::new("embedding").unwrap(),
                            RuntimeTableSchema::schemaless(RuntimeLogicalModel::Vector),
                        );
                        registry
                    },
                },
                RuntimeMutation::Record {
                    record: RuntimeRecord {
                        reference: job.source.clone(),
                        valid_from: 10,
                        valid_to: None,
                        properties,
                    },
                },
            ],
        })
        .unwrap();
    job.read = engine.runtime().read_stamp(&job.scope).unwrap();
    let snapshot =
        EmbeddingSourceSnapshot::for_bytes(job.source.clone(), "text/plain", bytes.to_vec())
            .unwrap();
    let mut reader = SequenceReader {
        snapshots: vec![snapshot],
        reads: 0,
    };
    let prepared = EmbeddingCoordinator::prepare(&job, &mut reader, &mut backend).unwrap();

    let mut changed = RuntimeProperties::new();
    changed.insert(
        "content_digest".into(),
        RuntimeValue::Digest(digest::sha256_hex(b"changed after inference")),
    );
    engine
        .runtime()
        .commit(&RuntimeCommit {
            scope: job.scope.clone(),
            at: 12,
            actor: "test:source".into(),
            expected_cursor: job.read.commit_cursor,
            mutations: vec![RuntimeMutation::Record {
                record: RuntimeRecord {
                    reference: job.source.clone(),
                    valid_from: 10,
                    valid_to: None,
                    properties: changed,
                },
            }],
        })
        .unwrap();

    let transaction = prepared.transaction(&job, "agent:embedding", 13).unwrap();
    assert!(engine
        .runtime()
        .commit_data_transaction(&transaction)
        .is_err());
}

#[test]
fn router_loader_receives_bytes_only_after_exact_manifest_handshake_admission() {
    let golden = golden_router_model();
    let calls = AtomicUsize::new(0);
    let loaded = load_router_model_after_handshake(
        &golden.manifest,
        &golden.backend,
        &golden.handshake,
        RouterModelArtifacts::new(
            ROUTER_MODEL_BYTES,
            ROUTER_TOKENIZER_BYTES,
            ROUTER_RUNTIME_BYTES,
            ROUTER_GRAMMAR_BYTES,
        ),
        |admission| {
            calls.fetch_add(1, Ordering::SeqCst);
            assert_eq!(
                admission.manifest().manifest_sha256,
                golden.manifest.manifest_sha256
            );
            assert_eq!(
                admission.backend().descriptor_sha256,
                golden.backend.descriptor_sha256
            );
            assert_eq!(
                admission.handshake().handshake_sha256,
                golden.handshake.handshake_sha256
            );
            assert_eq!(admission.artifacts().model(), ROUTER_MODEL_BYTES);
            assert_eq!(admission.artifacts().tokenizer(), ROUTER_TOKENIZER_BYTES);
            assert_eq!(admission.artifacts().runtime(), ROUTER_RUNTIME_BYTES);
            assert_eq!(admission.artifacts().grammar(), ROUTER_GRAMMAR_BYTES);
            Ok("loaded")
        },
    )
    .unwrap();
    assert_eq!(loaded, "loaded");
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn every_router_artifact_is_verified_before_the_loader_is_called() {
    let golden = golden_router_model();
    let calls = AtomicUsize::new(0);
    let mut corrupt_model = ROUTER_MODEL_BYTES.to_vec();
    corrupt_model[0] ^= 0xff;
    let mut corrupt_tokenizer = ROUTER_TOKENIZER_BYTES.to_vec();
    corrupt_tokenizer[0] ^= 0xff;
    let mut corrupt_runtime = ROUTER_RUNTIME_BYTES.to_vec();
    corrupt_runtime[0] ^= 0xff;
    let mut corrupt_grammar = ROUTER_GRAMMAR_BYTES.to_vec();
    corrupt_grammar[0] ^= 0xff;

    let cases = [
        RouterModelArtifacts::new(
            &ROUTER_MODEL_BYTES[..ROUTER_MODEL_BYTES.len() - 1],
            ROUTER_TOKENIZER_BYTES,
            ROUTER_RUNTIME_BYTES,
            ROUTER_GRAMMAR_BYTES,
        ),
        RouterModelArtifacts::new(
            &corrupt_model,
            ROUTER_TOKENIZER_BYTES,
            ROUTER_RUNTIME_BYTES,
            ROUTER_GRAMMAR_BYTES,
        ),
        RouterModelArtifacts::new(
            ROUTER_MODEL_BYTES,
            &corrupt_tokenizer,
            ROUTER_RUNTIME_BYTES,
            ROUTER_GRAMMAR_BYTES,
        ),
        RouterModelArtifacts::new(
            ROUTER_MODEL_BYTES,
            ROUTER_TOKENIZER_BYTES,
            &corrupt_runtime,
            ROUTER_GRAMMAR_BYTES,
        ),
        RouterModelArtifacts::new(
            ROUTER_MODEL_BYTES,
            ROUTER_TOKENIZER_BYTES,
            ROUTER_RUNTIME_BYTES,
            &corrupt_grammar,
        ),
    ];
    for artifacts in cases {
        let result = load_router_model_after_handshake(
            &golden.manifest,
            &golden.backend,
            &golden.handshake,
            artifacts,
            |_| {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok(())
            },
        );
        assert!(result.is_err());
    }
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[test]
fn contract_model_tokenizer_schema_resource_and_runtime_drift_precede_loading() {
    let golden = golden_router_model();
    let calls = AtomicUsize::new(0);
    let artifacts = RouterModelArtifacts::new(
        ROUTER_MODEL_BYTES,
        ROUTER_TOKENIZER_BYTES,
        ROUTER_RUNTIME_BYTES,
        ROUTER_GRAMMAR_BYTES,
    );
    let mut cases = Vec::new();

    let mut changed = golden.handshake.clone();
    changed.contract_version += 1;
    seal_router_handshake(&mut changed);
    cases.push(changed);

    let mut changed = golden.handshake.clone();
    changed.model.sha256 = "60".repeat(32);
    seal_router_handshake(&mut changed);
    cases.push(changed);

    let mut changed = golden.handshake.clone();
    changed.tokenizer.sha256 = "61".repeat(32);
    seal_router_handshake(&mut changed);
    cases.push(changed);

    let mut changed = golden.handshake.clone();
    changed.routing_schema_sha256 = "62".repeat(32);
    seal_router_handshake(&mut changed);
    cases.push(changed);

    let mut changed = golden.handshake.clone();
    changed.decisions = BTreeSet::from([RouteDecisionKind::SelectRecipe]);
    seal_router_handshake(&mut changed);
    cases.push(changed);

    let mut changed = golden.handshake.clone();
    changed.model_limits.maximum_resident_bytes -= 1;
    seal_router_handshake(&mut changed);
    cases.push(changed);

    let mut changed = golden.handshake.clone();
    changed.runtime.abi_revision += 1;
    seal_router_handshake(&mut changed);
    cases.push(changed);

    for handshake in &cases {
        let result = load_router_model_after_handshake(
            &golden.manifest,
            &golden.backend,
            handshake,
            artifacts,
            |_| {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok(())
            },
        );
        assert!(result.is_err());
    }
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

fn seal_router_handshake(handshake: &mut RouterModelHandshake) {
    handshake.handshake_sha256 = router_model_handshake_sha256(handshake).unwrap();
}
