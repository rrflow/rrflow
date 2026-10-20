use rrd_core::{digest, Error, RuntimeId, VectorValue};
use rrd_inference::{
    EmbeddingBackend, EmbeddingBackendDescriptor, EmbeddingBackendRegistry, EmbeddingDispatchError,
    EmbeddingRequest, FeatureHashBackend, NetworkPolicy,
};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Barrier};

fn request() -> EmbeddingRequest {
    let bytes = b"source for bounded execution".to_vec();
    EmbeddingRequest {
        job_id: RuntimeId::new("admission-input").unwrap(),
        job_digest: digest::sha256_hex(b"job"),
        source_digest: digest::sha256_hex(&bytes),
        media_type: "text/plain".into(),
        bytes,
    }
}

fn install(registry: &mut EmbeddingBackendRegistry, seed: u64) -> EmbeddingBackendDescriptor {
    let backend = FeatureHashBackend::new(32, seed).unwrap();
    let mut descriptor = backend.descriptor().clone();
    descriptor.id = format!("test:healthy:{seed}");
    registry
        .install(Box::new(FaultBackend {
            descriptor: descriptor.clone(),
            inner: backend,
            calls: Arc::new(AtomicUsize::new(0)),
            failure: Failure::Healthy,
        }))
        .unwrap();
    descriptor
}

#[test]
fn selection_pins_revision_and_drop_releases_the_single_registry_slot() {
    let mut registry = EmbeddingBackendRegistry::default();
    let first = install(&mut registry, 1);
    let execution = registry.try_acquire(&first.id).unwrap();
    let second = install(&mut registry, 2);
    assert_eq!(registry.revision(), 2);
    assert_eq!(registry.descriptors().len(), 2);
    assert_eq!(execution.descriptor(), &first);
    for id in [&first.id, &second.id] {
        assert!(matches!(
            registry.try_acquire(id),
            Err(EmbeddingDispatchError::Busy)
        ));
    }
    let result = execution
        .embed_batch(&first.model, NetworkPolicy::Deny, &[request()])
        .unwrap();
    assert_eq!(result.registry_revision, 1);
    assert_eq!(result.backend, first);
    let abandoned = registry.try_acquire(&second.id).unwrap();
    drop(abandoned);
    let result = registry
        .try_acquire(&second.id)
        .unwrap()
        .embed_batch(&second.model, NetworkPolicy::Deny, &[request()])
        .unwrap();
    assert_eq!(result.registry_revision, 2);
}

#[test]
fn simultaneous_admission_never_creates_a_second_execution_lease() {
    let mut registry = EmbeddingBackendRegistry::default();
    let descriptor = install(&mut registry, 3);
    let barrier = Barrier::new(8);
    std::thread::scope(|threads| {
        let workers = (0..8)
            .map(|_| {
                threads.spawn(|| {
                    barrier.wait();
                    let lease = registry.try_acquire(&descriptor.id);
                    // Keep the successful lease alive until every contender attempted
                    // admission. This asserts exclusion without timing assumptions.
                    barrier.wait();
                    match lease {
                        Ok(lease) => {
                            drop(lease);
                            1
                        }
                        Err(EmbeddingDispatchError::Busy) => 0,
                        Err(error) => panic!("unexpected admission error: {error}"),
                    }
                })
            })
            .collect::<Vec<_>>();
        assert_eq!(
            workers
                .into_iter()
                .map(|worker| worker.join().unwrap())
                .sum::<u32>(),
            1
        );
    });
    assert!(registry.try_acquire(&descriptor.id).is_ok());
}

#[test]
fn validation_denial_releases_admission_and_unknown_id_never_reserves() {
    let mut registry = EmbeddingBackendRegistry::default();
    let descriptor = install(&mut registry, 4);
    assert!(matches!(
        registry.try_acquire("missing"),
        Err(EmbeddingDispatchError::Rejected(_))
    ));
    let mut wrong = descriptor.model.clone();
    wrong.revision = "different".into();
    assert!(registry
        .try_acquire(&descriptor.id)
        .unwrap()
        .embed_batch(&wrong, NetworkPolicy::Deny, &[request()])
        .is_err());
    let mut corrupt = request();
    corrupt.bytes.push(0);
    assert!(registry
        .try_acquire(&descriptor.id)
        .unwrap()
        .embed_batch(&descriptor.model, NetworkPolicy::Deny, &[corrupt])
        .is_err());
    assert!(registry
        .try_acquire(&descriptor.id)
        .unwrap()
        .embed_batch(&descriptor.model, NetworkPolicy::Deny, &[request()])
        .is_ok());
}

enum Failure {
    Healthy,
    Error,
    Panic,
    DescriptorDrift,
}

struct FaultBackend {
    descriptor: EmbeddingBackendDescriptor,
    inner: FeatureHashBackend,
    calls: Arc<AtomicUsize>,
    failure: Failure,
}

impl EmbeddingBackend for FaultBackend {
    fn descriptor(&self) -> &EmbeddingBackendDescriptor {
        &self.descriptor
    }
    fn embed(&mut self, request: &EmbeddingRequest) -> rrd_core::Result<VectorValue> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        match self.failure {
            Failure::Healthy => self.inner.embed(request),
            Failure::Error => Err(Error::InvalidRuntime {
                reason: "controlled adapter error".into(),
            }),
            Failure::Panic => panic!("controlled adapter panic"),
            Failure::DescriptorDrift => {
                self.descriptor.model.revision = "changed-during-execution".into();
                self.inner.embed(request)
            }
        }
    }
}

fn faulty(
    registry: &mut EmbeddingBackendRegistry,
    failure: Failure,
) -> (EmbeddingBackendDescriptor, Arc<AtomicUsize>) {
    let inner = FeatureHashBackend::new(32, 5).unwrap();
    let descriptor = inner.descriptor().clone();
    let calls = Arc::new(AtomicUsize::new(0));
    registry
        .install(Box::new(FaultBackend {
            descriptor: descriptor.clone(),
            inner,
            calls: calls.clone(),
            failure,
        }))
        .unwrap();
    (descriptor, calls)
}

#[test]
fn backend_error_releases_admission_for_another_backend() {
    let mut registry = EmbeddingBackendRegistry::default();
    let (fault, calls) = faulty(&mut registry, Failure::Error);
    let healthy = install(&mut registry, 6);
    assert!(registry
        .try_acquire(&fault.id)
        .unwrap()
        .embed_batch(&fault.model, NetworkPolicy::Deny, &[request()])
        .is_err());
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(registry
        .try_acquire(&healthy.id)
        .unwrap()
        .embed_batch(&healthy.model, NetworkPolicy::Deny, &[request()])
        .is_ok());
}

#[test]
fn panic_releases_capacity_and_poisoning_is_confined_to_the_failed_backend() {
    let mut registry = EmbeddingBackendRegistry::default();
    let (fault, calls) = faulty(&mut registry, Failure::Panic);
    let healthy = install(&mut registry, 7);
    let execution = registry.try_acquire(&fault.id).unwrap();
    let model = fault.model.clone();
    assert!(std::thread::spawn(move || execution.embed_batch(
        &model,
        NetworkPolicy::Deny,
        &[request()]
    ))
    .join()
    .is_err());
    assert_eq!(registry.descriptors().len(), 2);
    assert!(matches!(
        registry.try_acquire(&fault.id).unwrap().embed_batch(
            &fault.model,
            NetworkPolicy::Deny,
            &[request()]
        ),
        Err(EmbeddingDispatchError::UnhealthyBackend)
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(registry
        .try_acquire(&healthy.id)
        .unwrap()
        .embed_batch(&healthy.model, NetworkPolicy::Deny, &[request()])
        .is_ok());
}

#[test]
fn descriptor_drift_rejects_output_and_further_execution_without_rewriting_catalogue() {
    let mut registry = EmbeddingBackendRegistry::default();
    let (descriptor, calls) = faulty(&mut registry, Failure::DescriptorDrift);
    for _ in 0..2 {
        let error = registry
            .try_acquire(&descriptor.id)
            .unwrap()
            .embed_batch(&descriptor.model, NetworkPolicy::Deny, &[request()])
            .unwrap_err();
        assert!(error.to_string().contains("descriptor changed"));
    }
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(registry.descriptor(&descriptor.id), Some(&descriptor));
}
