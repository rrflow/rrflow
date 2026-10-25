use rrd_core::{digest, RuntimeId, VectorValue};
use rrd_inference::{
    EmbeddingBackend, EmbeddingBackendDescriptor, EmbeddingBackendRegistry, EmbeddingDispatchError,
    EmbeddingRequest, FeatureHashBackend, NetworkPolicy,
};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

fn request() -> EmbeddingRequest {
    let bytes = b"deadline input".to_vec();
    EmbeddingRequest {
        job_id: RuntimeId::new("input").unwrap(),
        job_digest: digest::sha256_hex(b"job"),
        source_digest: digest::sha256_hex(&bytes),
        media_type: "text/plain".into(),
        bytes,
    }
}

struct Controlled {
    inner: FeatureHashBackend,
    calls: Arc<AtomicUsize>,
    entered: mpsc::Sender<()>,
    release: mpsc::Receiver<()>,
    panic: bool,
}

impl EmbeddingBackend for Controlled {
    fn descriptor(&self) -> &EmbeddingBackendDescriptor {
        self.inner.descriptor()
    }
    fn embed(&mut self, request: &EmbeddingRequest) -> rrd_core::Result<VectorValue> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let _ = self.entered.send(());
        self.release.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(!self.panic, "controlled worker panic");
        self.inner.embed(request)
    }
}

#[test]
fn expired_admission_never_dispatches_and_releases_capacity() {
    let mut registry = EmbeddingBackendRegistry::default();
    let (entered, entered_rx) = mpsc::channel();
    let (_release, release_rx) = mpsc::channel();
    let calls = Arc::new(AtomicUsize::new(0));
    let backend = Controlled {
        inner: FeatureHashBackend::new(32, 1).unwrap(),
        calls: calls.clone(),
        entered,
        release: release_rx,
        panic: false,
    };
    let descriptor = backend.descriptor().clone();
    registry.install(Box::new(backend)).unwrap();
    let error = registry
        .try_acquire(&descriptor.id)
        .unwrap()
        .embed_batch_until(
            descriptor.model,
            NetworkPolicy::Deny,
            vec![request()],
            Instant::now(),
        )
        .unwrap_err();
    assert!(matches!(
        error,
        EmbeddingDispatchError::DeadlineExceeded {
            execution_may_continue: false
        }
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert!(entered_rx.try_recv().is_err());
    assert!(registry.try_acquire(&descriptor.id).is_ok());
}

#[test]
fn blocked_worker_times_out_without_releasing_capacity_or_accepting_late_output() {
    let mut registry = EmbeddingBackendRegistry::default();
    let (entered, entered_rx) = mpsc::channel();
    let (release, release_rx) = mpsc::channel();
    let calls = Arc::new(AtomicUsize::new(0));
    let backend = Controlled {
        inner: FeatureHashBackend::new(32, 2).unwrap(),
        calls: calls.clone(),
        entered,
        release: release_rx,
        panic: false,
    };
    let descriptor = backend.descriptor().clone();
    registry.install(Box::new(backend)).unwrap();
    let lease = registry.try_acquire(&descriptor.id).unwrap();
    let model = descriptor.model.clone();
    let (done, result) = mpsc::channel();
    let caller = std::thread::spawn(move || {
        done.send(lease.embed_batch_until(
            model,
            NetworkPolicy::Deny,
            vec![request()],
            Instant::now() + Duration::from_millis(250),
        ))
        .unwrap();
    });
    entered_rx.recv_timeout(Duration::from_secs(2)).unwrap();
    let timed = result.recv_timeout(Duration::from_secs(2));
    let held = matches!(
        registry.try_acquire(&descriptor.id),
        Err(EmbeddingDispatchError::Busy)
    );
    // Always release before assertions so a broken implementation cannot hang CI.
    release.send(()).unwrap();
    caller.join().unwrap();
    assert!(held, "timeout released an executing adapter's permit");
    assert!(matches!(
        timed.unwrap().unwrap_err(),
        EmbeddingDispatchError::DeadlineExceeded {
            execution_may_continue: true
        }
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let until = Instant::now() + Duration::from_secs(2);
    loop {
        match registry.try_acquire(&descriptor.id) {
            Ok(lease) => {
                drop(lease);
                break;
            }
            Err(EmbeddingDispatchError::Busy) if Instant::now() < until => std::thread::yield_now(),
            _ => panic!("completed worker did not release capacity"),
        }
    }
}

#[test]
fn timely_real_backend_preserves_provenance_and_validation() {
    let mut registry = EmbeddingBackendRegistry::default();
    let backend = FeatureHashBackend::new(32, 3).unwrap();
    let descriptor = backend.descriptor().clone();
    registry.install(Box::new(backend)).unwrap();
    let expected = registry
        .try_acquire(&descriptor.id)
        .unwrap()
        .embed_batch(&descriptor.model, NetworkPolicy::Deny, &[request()])
        .unwrap();
    let actual = registry
        .try_acquire(&descriptor.id)
        .unwrap()
        .embed_batch_until(
            descriptor.model.clone(),
            NetworkPolicy::Deny,
            vec![request()],
            Instant::now() + Duration::from_secs(5),
        )
        .unwrap();
    assert_eq!(actual, expected);
    let mut corrupt = request();
    corrupt.bytes.push(0);
    assert!(matches!(
        registry
            .try_acquire(&descriptor.id)
            .unwrap()
            .embed_batch_until(
                descriptor.model,
                NetworkPolicy::Deny,
                vec![corrupt],
                Instant::now() + Duration::from_secs(5)
            ),
        Err(EmbeddingDispatchError::Rejected(_))
    ));
    assert!(registry.try_acquire(&descriptor.id).is_ok());
}

#[test]
fn worker_panic_is_reported_and_does_not_leak_admission() {
    let mut registry = EmbeddingBackendRegistry::default();
    let (entered, _entered_rx) = mpsc::channel();
    let (release, release_rx) = mpsc::channel();
    let backend = Controlled {
        inner: FeatureHashBackend::new(32, 4).unwrap(),
        calls: Arc::new(AtomicUsize::new(0)),
        entered,
        release: release_rx,
        panic: true,
    };
    let descriptor = backend.descriptor().clone();
    registry.install(Box::new(backend)).unwrap();
    release.send(()).unwrap();
    let error = registry
        .try_acquire(&descriptor.id)
        .unwrap()
        .embed_batch_until(
            descriptor.model,
            NetworkPolicy::Deny,
            vec![request()],
            Instant::now() + Duration::from_secs(5),
        )
        .unwrap_err();
    assert!(matches!(error, EmbeddingDispatchError::WorkerUnavailable));
    assert!(registry.try_acquire(&descriptor.id).is_ok());
}
