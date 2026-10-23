use rrd_core::{digest, RuntimeId, VectorValue};
use rrd_inference::{
    EmbeddingBackend, EmbeddingBackendDescriptor, EmbeddingBackendRegistry, EmbeddingCancellation,
    EmbeddingDispatchError, EmbeddingExecutionControl, EmbeddingRequest, FeatureHashBackend,
    NetworkPolicy,
};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

fn request() -> EmbeddingRequest {
    let bytes = b"cooperative source".to_vec();
    EmbeddingRequest {
        job_id: RuntimeId::new("source").unwrap(),
        job_digest: digest::sha256_hex(b"job"),
        source_digest: digest::sha256_hex(&bytes),
        media_type: "text/plain".into(),
        bytes,
    }
}

struct Backend {
    inner: FeatureHashBackend,
    calls: Arc<AtomicUsize>,
    entered: mpsc::Sender<()>,
    release: mpsc::Receiver<()>,
    cooperative: bool,
}

impl EmbeddingBackend for Backend {
    fn descriptor(&self) -> &EmbeddingBackendDescriptor {
        self.inner.descriptor()
    }
    fn embed(&mut self, request: &EmbeddingRequest) -> rrd_core::Result<VectorValue> {
        self.inner.embed(request)
    }
    fn embed_batch_controlled(
        &mut self,
        requests: &[EmbeddingRequest],
        control: &EmbeddingExecutionControl,
    ) -> Result<Vec<VectorValue>, EmbeddingDispatchError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.entered.send(()).unwrap();
        if self.cooperative {
            loop {
                control.check()?;
                std::thread::sleep(Duration::from_millis(1));
            }
        }
        self.release.recv_timeout(Duration::from_secs(5)).unwrap();
        // Deliberately return output without checking cancellation. The owner
        // must still reject it and retain the permit while this call blocks.
        self.inner.embed_batch(requests).map_err(Into::into)
    }
}

#[test]
fn cancellation_is_sticky_and_precedes_dispatch() {
    let mut registry = EmbeddingBackendRegistry::default();
    let (entered, observed) = mpsc::channel();
    let (_release, release) = mpsc::channel();
    let calls = Arc::new(AtomicUsize::new(0));
    let backend = Backend {
        inner: FeatureHashBackend::new(32, 7).unwrap(),
        calls: calls.clone(),
        entered,
        release,
        cooperative: true,
    };
    let descriptor = backend.descriptor().clone();
    registry.install(Box::new(backend)).unwrap();
    let cancellation = EmbeddingCancellation::default();
    cancellation.clone().cancel();
    assert!(cancellation.is_cancelled());
    for _ in 0..2 {
        assert!(matches!(
            registry
                .try_acquire(&descriptor.id)
                .unwrap()
                .embed_batch_with_control(
                    descriptor.model.clone(),
                    NetworkPolicy::Deny,
                    vec![request()],
                    EmbeddingExecutionControl::new(
                        Instant::now() + Duration::from_secs(5),
                        cancellation.clone()
                    )
                ),
            Err(EmbeddingDispatchError::Cancelled {
                execution_may_continue: false
            })
        ));
    }
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert!(observed.try_recv().is_err());
    assert!(registry.try_acquire(&descriptor.id).is_ok());
}

#[test]
fn cancelling_cooperative_and_noncooperative_work_preserves_actual_ownership() {
    for cooperative in [true, false] {
        let mut registry = EmbeddingBackendRegistry::default();
        let (entered, observed) = mpsc::channel();
        let (release, released) = mpsc::channel();
        let calls = Arc::new(AtomicUsize::new(0));
        let backend = Backend {
            inner: FeatureHashBackend::new(32, 9).unwrap(),
            calls: calls.clone(),
            entered,
            release: released,
            cooperative,
        };
        let descriptor = backend.descriptor().clone();
        registry.install(Box::new(backend)).unwrap();
        let cancellation = EmbeddingCancellation::default();
        let control = EmbeddingExecutionControl::new(
            Instant::now() + Duration::from_secs(5),
            cancellation.clone(),
        );
        let lease = registry.try_acquire(&descriptor.id).unwrap();
        let model = descriptor.model.clone();
        let (done, result) = mpsc::channel();
        let caller = std::thread::spawn(move || {
            done.send(lease.embed_batch_with_control(
                model,
                NetworkPolicy::Deny,
                vec![request()],
                control,
            ))
            .unwrap();
        });
        observed.recv_timeout(Duration::from_secs(2)).unwrap();
        cancellation.cancel();
        let stopped = result.recv_timeout(Duration::from_secs(2));
        let retained = matches!(
            registry.try_acquire(&descriptor.id),
            Err(EmbeddingDispatchError::Busy)
        );
        if !cooperative {
            release.send(()).unwrap();
        }
        caller.join().unwrap();
        assert!(matches!(
            stopped.unwrap().unwrap_err(),
            EmbeddingDispatchError::Cancelled { .. }
        ));
        if !cooperative {
            assert!(retained);
        }
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        let until = Instant::now() + Duration::from_secs(2);
        loop {
            match registry.try_acquire(&descriptor.id) {
                Ok(lease) => {
                    drop(lease);
                    break;
                }
                Err(EmbeddingDispatchError::Busy) if Instant::now() < until => {
                    std::thread::sleep(Duration::from_millis(1))
                }
                _ => panic!("worker failed to release admission"),
            }
        }
    }
}

#[test]
fn real_feature_hash_preserves_output_and_observes_cancelled_control() {
    let mut backend = FeatureHashBackend::new(32, 11).unwrap();
    let expected = backend.embed_batch(&[request()]).unwrap();
    let cancellation = EmbeddingCancellation::default();
    let control = EmbeddingExecutionControl::new(
        Instant::now() + Duration::from_secs(5),
        cancellation.clone(),
    );
    assert_eq!(
        backend
            .embed_batch_controlled(&[request()], &control)
            .unwrap(),
        expected
    );
    cancellation.cancel();
    assert!(matches!(
        backend.embed_batch_controlled(&[request()], &control),
        Err(EmbeddingDispatchError::Cancelled {
            execution_may_continue: false
        })
    ));
}
