use super::*;
use crate::ServiceErrorKind;
use rrd_inference::{EmbeddingBackend, EmbeddingBackendDescriptor, EmbeddingRequest};
use std::sync::mpsc;
use std::time::Duration;

struct BlockedBackend {
    descriptor: EmbeddingBackendDescriptor,
    inner: rrd_inference::FeatureHashBackend,
    entered: mpsc::Sender<()>,
    release: mpsc::Receiver<()>,
}

impl EmbeddingBackend for BlockedBackend {
    fn descriptor(&self) -> &EmbeddingBackendDescriptor {
        &self.descriptor
    }

    fn embed(&mut self, request: &EmbeddingRequest) -> rrd_core::Result<rrd_core::VectorValue> {
        self.entered.send(()).unwrap();
        self.release.recv_timeout(Duration::from_secs(20)).unwrap();
        self.inner.embed(request)
    }
}

#[test]
fn blocked_embedding_does_not_lock_model_catalogue() {
    let (_root, engine) = isolated_engine();
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let inner = rrd_inference::FeatureHashBackend::new(32, 912).unwrap();
    let mut descriptor = inner.descriptor().clone();
    descriptor.id = "test:blocked:cpu:v1".into();
    let backend = BlockedBackend {
        descriptor,
        inner,
        entered: entered_tx,
        release: release_rx,
    };
    let backend_id = backend.descriptor().id.clone();
    engine.install_embedding_backend(backend).unwrap();
    let lease = engine
        .create_session(
            &session_request(9_000, 4),
            &id("admission-session"),
            10,
            "admission-create",
            "admission-create-op",
        )
        .unwrap();
    let request = GenerateEmbeddings {
        scope: format!("instance:{}", instance()),
        backend_id,
        network_policy: EmbeddingNetworkPolicy::Deny,
        inputs: vec![EmbeddingInput {
            id: CanonicalId::new("input").unwrap(),
            media_type: "text/plain".into(),
            bytes: b"bounded embedding admission".to_vec(),
        }],
    };
    std::thread::scope(|threads| {
        let worker = threads.spawn(|| {
            engine.generate_embeddings(
                &lease.session_id,
                &lease.token,
                &request,
                20,
                "admission-generate",
                "admission-generate-op",
            )
        });
        entered_rx.recv_timeout(Duration::from_secs(10)).unwrap();
        let (listed_tx, listed_rx) = mpsc::channel();
        let (engine, lease, request) = (&engine, &lease, &request);
        threads.spawn(move || {
            let result = (|| -> std::result::Result<_, ServiceError> {
                engine.install_feature_hash_embedding(32, 913)?;
                let catalogue = engine.list_embedding_models(
                    &lease.session_id,
                    &lease.token,
                    &ListEmbeddingModels {
                        scope: request.scope.clone(),
                    },
                    21,
                    "admission-list",
                    "admission-list-op",
                )?;
                let mut other = request.clone();
                other.backend_id = catalogue
                    .backends
                    .iter()
                    .find(|backend| backend.id != request.backend_id)
                    .unwrap()
                    .id
                    .clone();
                for (index, denied) in [request, &other].into_iter().enumerate() {
                    let error = engine
                        .generate_embeddings(
                            &lease.session_id,
                            &lease.token,
                            denied,
                            22,
                            &format!("busy-request-{index}"),
                            &format!("busy-operation-{index}"),
                        )
                        .unwrap_err();
                    assert!(matches!(error, ServiceError::InferenceBusy));
                    assert_eq!(error.kind(), ServiceErrorKind::ResourceExhausted);
                    assert!(error.retryable());
                }
                Ok((catalogue, other))
            })();
            listed_tx.send(result).unwrap();
        });
        let listed_before_release = listed_rx.recv_timeout(Duration::from_secs(2));
        // Always release the controlled backend before asserting, including on
        // the old lock-held implementation, so the regression cannot hang CI.
        release_tx.send(()).unwrap();
        let completed = worker.join().unwrap().unwrap();
        assert_eq!(completed.registry_revision, 1);
        assert!(
            listed_before_release.is_ok(),
            "model listing was blocked by embedding execution"
        );
        let (catalogue, other) = listed_before_release.unwrap().unwrap();
        assert_eq!(catalogue.registry_revision, 2);
        assert_eq!(catalogue.backends.len(), 2);
        let next = engine
            .generate_embeddings(
                &lease.session_id,
                &lease.token,
                &other,
                23,
                "after-release-request",
                "after-release-operation",
            )
            .unwrap();
        assert_eq!(next.registry_revision, 2);
    });
}

#[test]
fn configured_embedding_deadline_returns_while_worker_keeps_its_permit() {
    interrupted_embedding_keeps_its_permit(false);
}

#[test]
fn cancelled_embedding_returns_while_worker_keeps_its_permit() {
    interrupted_embedding_keeps_its_permit(true);
}

fn interrupted_embedding_keeps_its_permit(cancel: bool) {
    let (_root, mut engine) = isolated_engine();
    let mut configuration = rrd_contract::EstateConfiguration::default();
    configuration.reasoning.max_step_elapsed_ms = if cancel { 5_000 } else { 250 };
    configuration.configuration_sha256 =
        rrd_contract::estate_configuration_sha256(&configuration).unwrap();
    configuration.validate().unwrap();
    engine.estate_configuration = configuration;
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let inner = rrd_inference::FeatureHashBackend::new(32, 914).unwrap();
    let descriptor = inner.descriptor().clone();
    let backend_id = descriptor.id.clone();
    engine
        .install_embedding_backend(BlockedBackend {
            descriptor,
            inner,
            entered: entered_tx,
            release: release_rx,
        })
        .unwrap();
    let lease = engine
        .create_session(
            &session_request(9_000, 4),
            &id("deadline-session"),
            10,
            "deadline-create",
            "deadline-create-op",
        )
        .unwrap();
    let request = GenerateEmbeddings {
        scope: format!("instance:{}", instance()),
        backend_id,
        network_policy: EmbeddingNetworkPolicy::Deny,
        inputs: vec![EmbeddingInput {
            id: CanonicalId::new("input").unwrap(),
            media_type: "text/plain".into(),
            bytes: b"configured deadline".to_vec(),
        }],
    };
    std::thread::scope(|threads| {
        let (done_tx, done_rx) = mpsc::channel();
        let cancellation = rrd_inference::EmbeddingCancellation::default();
        let caller_cancellation = cancellation.clone();
        let (engine, lease, request) = (&engine, &lease, &request);
        threads.spawn(move || {
            done_tx
                .send(engine.generate_embeddings_with_cancellation(
                    &lease.session_id,
                    &lease.token,
                    request,
                    20,
                    "deadline-call",
                    "deadline-call-op",
                    &caller_cancellation,
                ))
                .unwrap();
        });
        entered_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        if cancel {
            cancellation.cancel();
        }
        let observed = done_rx.recv_timeout(Duration::from_secs(2));
        let capacity_held = matches!(
            engine.generate_embeddings(
                &lease.session_id,
                &lease.token,
                request,
                21,
                "deadline-next",
                "deadline-next-op"
            ),
            Err(ServiceError::InferenceBusy)
        );
        release_tx.send(()).unwrap();
        let error = observed
            .expect("configured deadline did not return before backend release")
            .unwrap_err();
        if cancel {
            assert!(matches!(
                error,
                ServiceError::InferenceCancelled {
                    execution_may_continue: true
                }
            ));
            assert_eq!(error.kind(), ServiceErrorKind::FailedPrecondition);
        } else {
            assert!(matches!(
                error,
                ServiceError::InferenceDeadlineExceeded {
                    execution_may_continue: true
                }
            ));
            assert_eq!(error.kind(), ServiceErrorKind::DeadlineExceeded);
        }
        assert!(!error.retryable());
        assert!(capacity_held);
        let until = std::time::Instant::now() + Duration::from_secs(2);
        loop {
            match engine
                .embedding_backends
                .lock()
                .unwrap()
                .try_acquire(&request.backend_id)
            {
                Ok(lease) => {
                    drop(lease);
                    break;
                }
                Err(rrd_inference::EmbeddingDispatchError::Busy)
                    if std::time::Instant::now() < until =>
                {
                    std::thread::sleep(Duration::from_millis(1));
                }
                _ => panic!("late worker did not release admission"),
            }
        }
    });
}
