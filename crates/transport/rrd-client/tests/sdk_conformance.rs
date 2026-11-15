use rrd_client::{is_unauthenticated, ClientConfig, Error, RequestOptions, RrdClient};
use rrd_contract::{
    FollowChangefeed, ReadChangefeed, SdkConformanceCorpus, SessionEndState, TransactionState,
};
use rrd_core::digest;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct HarnessManifest {
    format_version: u16,
    corpus_sha256: String,
    corpus_path: PathBuf,
    base_url: String,
    retry_base_urls: BTreeMap<String, String>,
    incompatible_version_url: String,
    shutdown_path: PathBuf,
}

fn address(url: &str) -> SocketAddr {
    url.strip_prefix("http://")
        .expect("conformance URL is loopback HTTP")
        .trim_end_matches('/')
        .parse()
        .expect("conformance URL contains a socket address")
}

fn mutation(step: &str) -> RequestOptions {
    RequestOptions::mutation(
        &format!("rust-{step}-request"),
        &format!("rust-{step}-operation"),
        &format!("rust-{step}-key"),
    )
    .unwrap()
}

fn read(step: &str) -> RequestOptions {
    RequestOptions::read(
        &format!("rust-{step}-request"),
        &format!("rust-{step}-operation"),
    )
    .unwrap()
}

// Ignored by default so an ordinary `cargo test` cannot count it as passed
// without a daemon; `scripts/ci/run_sdk_conformance.py` runs it by name.
#[tokio::test]
#[ignore = "requires the real shared SDK conformance daemon"]
async fn rust_sdk_passes_shared_real_daemon_conformance() {
    let manifest_path = std::env::var("RRD_SDK_CONFORMANCE_MANIFEST")
        .expect("real SDK conformance requires RRD_SDK_CONFORMANCE_MANIFEST");
    let manifest_bytes = std::fs::read(manifest_path).unwrap();
    let manifest: HarnessManifest = serde_json::from_slice(&manifest_bytes).unwrap();
    assert_eq!(manifest.format_version, 1);
    assert!(!manifest.shutdown_path.as_os_str().is_empty());
    let corpus_bytes = std::fs::read(&manifest.corpus_path).unwrap();
    assert_eq!(manifest.corpus_sha256, digest::sha256_hex(&corpus_bytes));
    let corpus: SdkConformanceCorpus = serde_json::from_slice(&corpus_bytes).unwrap();
    corpus.validate().unwrap();

    let retry_client = RrdClient::connect_local(
        address(&manifest.retry_base_urls["rust"]),
        corpus.identity.instance.clone(),
        ClientConfig {
            request_timeout: Duration::from_secs(5),
            max_attempts: corpus.expected.retry_attempts,
            websocket_limits: Default::default(),
        },
    )
    .unwrap();
    assert_eq!(
        retry_client.capabilities().await.unwrap().protocol_version,
        corpus.protocol_version
    );

    let incompatible = RrdClient::connect_local(
        address(&manifest.incompatible_version_url),
        corpus.identity.instance.clone(),
        ClientConfig::default(),
    )
    .unwrap()
    .capabilities()
    .await
    .unwrap_err();
    assert!(matches!(
        incompatible,
        Error::UnsupportedProtocol { version, .. }
            if version == corpus.incompatible_protocol_version
    ));

    let client = RrdClient::connect_local(
        address(&manifest.base_url),
        corpus.identity.instance.clone(),
        ClientConfig {
            request_timeout: Duration::from_secs(10),
            max_attempts: 2,
            websocket_limits: Default::default(),
        },
    )
    .unwrap();
    let capabilities = client.capabilities().await.unwrap();
    assert_eq!(capabilities.protocol, corpus.protocol);
    assert_eq!(
        client.endpoint_catalogue().await.unwrap().endpoints.len(),
        usize::from(corpus.expected.endpoint_count)
    );

    let wrong_key = client
        .create_session(
            corpus.identity.principal.clone(),
            "wrong-sdk-conformance-key",
            corpus.session.create.clone(),
            mutation("wrong-key"),
        )
        .await
        .unwrap_err();
    assert!(is_unauthenticated(&wrong_key));

    let session = client
        .create_session(
            corpus.identity.principal.clone(),
            &corpus.identity.api_key,
            corpus.session.create.clone(),
            mutation("session-create"),
        )
        .await
        .unwrap();
    let session = client
        .renew_session(
            &session,
            corpus.session.renew.clone(),
            mutation("session-renew"),
        )
        .await
        .unwrap();

    let cold = client
        .read_cold_history(
            &session,
            corpus.cold_history.read.clone(),
            read("cold-history"),
        )
        .await
        .unwrap();
    let generation = cold.generation.as_ref().expect("cold generation");
    assert_eq!(cold.scope, corpus.cold_history.read.scope);
    assert_eq!(generation.generation, 1);
    assert!(generation.through_cursor > generation.requested_after_cursor);
    assert!(!cold.changes.is_empty());
    assert_eq!(generation.rows, cold.changes.len() as u64);
    assert!(cold.changes.iter().all(|change| change.scope == cold.scope));
    assert!(cold
        .changes
        .iter()
        .any(|change| change.actor == corpus.cold_history.expected_actor));

    client
        .ensure_vector_collection(
            &session,
            corpus.vector.ensure.clone(),
            mutation("vector-ensure"),
        )
        .await
        .unwrap();

    let preview_transaction = client
        .begin_transaction(
            &session,
            corpus.transaction.preview_begin.clone(),
            mutation("preview-begin"),
        )
        .await
        .unwrap();
    let preview = client
        .preview_transaction(
            &session,
            &preview_transaction.transaction_id,
            corpus.transaction.preview.clone(),
            mutation("transaction-preview"),
        )
        .await
        .unwrap();
    assert_eq!(preview.transaction_id, preview_transaction.transaction_id);
    let aborted = client
        .abort_transaction(
            &session,
            &preview_transaction.transaction_id,
            corpus.transaction.abort.clone(),
            mutation("transaction-abort"),
        )
        .await
        .unwrap();
    assert_eq!(aborted.state, TransactionState::Aborted);

    let commit_transaction = client
        .begin_transaction(
            &session,
            corpus.transaction.commit_begin.clone(),
            mutation("commit-begin"),
        )
        .await
        .unwrap();
    let committed = client
        .commit_transaction(
            &session,
            &commit_transaction.transaction_id,
            corpus.transaction.commit.clone(),
            RequestOptions::new(
                "rust-transaction-commit-request",
                "rust-transaction-commit-operation",
                Some("rust-transaction-commit-key"),
                Some(
                    u64::try_from(
                        SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_millis(),
                    )
                    .unwrap()
                    .saturating_add(corpus.transaction.commit_deadline_timeout_ms),
                ),
            )
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        committed.operation_sha256,
        corpus.transaction.commit.operation_sha256
    );

    let query = client
        .execute_query(&session, corpus.query.clone(), read("query"))
        .await
        .unwrap();
    assert!(query
        .rows
        .iter()
        .any(|row| row.identity == corpus.expected.query_identity));
    let vectors = client
        .search_vectors(
            &session,
            corpus.vector.search.clone(),
            read("vector-search"),
        )
        .await
        .unwrap();
    assert!(vectors.hits.iter().any(|hit| {
        format!("{}:{}", hit.reference.kind, hit.reference.id) == corpus.expected.vector_reference
    }));

    // The quantization lifecycle: build a TurboQuant generation, activate the
    // generation this build returned (the runners share one engram), and
    // require it.
    let quantization = &corpus.vector.quantization;
    let built = client
        .build_vector_quantization(
            &session,
            quantization.build.clone(),
            mutation("quantization-build"),
        )
        .await
        .unwrap();
    assert_eq!(
        built.artifact.artifact_id,
        quantization.activate.artifact_id
    );
    client
        .activate_vector_quantization(
            &session,
            rrd_contract::ActivateVectorQuantizationArtifact {
                generation: built.artifact.generation,
                ..quantization.activate.clone()
            },
            mutation("quantization-activate"),
        )
        .await
        .unwrap();
    let quantized = client
        .search_vectors(
            &session,
            quantization.search.clone(),
            read("quantized-search"),
        )
        .await
        .unwrap();
    assert_eq!(quantized.access_path, corpus.expected.quantized_access_path);
    assert!(quantized.hits.iter().any(|hit| {
        format!("{}:{}", hit.reference.kind, hit.reference.id) == corpus.expected.vector_reference
    }));

    // The graph index and its payload index. The changefeed read below proves
    // the scope stays readable with an index in it.
    let index = &corpus.vector.index;
    client
        .ensure_vector_payload_index(
            &session,
            index.payload_index.clone(),
            mutation("payload-index-ensure"),
        )
        .await
        .unwrap();
    let built_index = client
        .ensure_vector_index(&session, index.ensure.clone(), mutation("index-ensure"))
        .await
        .unwrap();
    assert_eq!(built_index.index.kind, corpus.expected.indexed_access_path);
    let indexed = client
        .search_vectors(&session, index.search.clone(), read("indexed-search"))
        .await
        .unwrap();
    assert_eq!(indexed.access_path, corpus.expected.indexed_access_path);
    assert!(indexed.hits.iter().any(|hit| {
        format!("{}:{}", hit.reference.kind, hit.reference.id) == corpus.expected.indexed_reference
    }));
    client
        .ensure_vector_payload_index(
            &session,
            index.spare_payload_index.clone(),
            mutation("payload-index-spare"),
        )
        .await
        .unwrap();
    client
        .delete_vector_payload_index(
            &session,
            index.delete_payload_index.clone(),
            mutation("payload-index-delete"),
        )
        .await
        .unwrap();

    let changes = client
        .read_changefeed(
            &session,
            corpus.changefeed.read.clone(),
            read("changefeed-read"),
        )
        .await
        .unwrap();
    let follow_read = ReadChangefeed {
        after_cursor: changes.head_cursor,
        ..corpus.changefeed.read.clone()
    };
    let followed = client
        .follow_changefeed(
            &session,
            FollowChangefeed {
                read: follow_read.clone(),
                wait_timeout_ms: corpus.changefeed.follow_wait_timeout_ms,
            },
            read("changefeed-follow"),
        )
        .await
        .unwrap();
    assert!(followed.timed_out);
    let cancelled = tokio::time::timeout(
        Duration::from_millis(corpus.changefeed.cancel_after_ms),
        client.follow_changefeed(
            &session,
            FollowChangefeed {
                read: follow_read,
                wait_timeout_ms: corpus.changefeed.cancellation_wait_timeout_ms,
            },
            read("changefeed-cancel"),
        ),
    )
    .await;
    assert!(cancelled.is_err());

    let backup = client
        .create_backup(
            &session,
            corpus.backup.create.clone(),
            mutation("backup-create"),
        )
        .await
        .unwrap();
    let backup_prefix = format!("{}--", corpus.backup.create.label);
    let backup_suffix = backup.backup.label.strip_prefix(&backup_prefix).unwrap();
    assert_eq!(backup_suffix.len(), 16);
    assert!(backup_suffix.bytes().all(|byte| byte.is_ascii_hexdigit()));
    let backups = client
        .list_backups(&session, corpus.backup.list.clone(), read("backup-list"))
        .await
        .unwrap();
    assert!(backups
        .backups
        .iter()
        .any(|entry| entry.backup_sha256 == backup.backup.backup_sha256));
    let engram = client
        .read_engram(
            &session,
            corpus.identity.engram.clone(),
            corpus.engram.clone(),
            read("engram-read"),
        )
        .await
        .unwrap();
    assert_eq!(engram.revision, corpus.expected.engram_revision);

    let closed = client
        .close_session(&session, corpus.session.close, mutation("session-close"))
        .await
        .unwrap();
    assert_eq!(closed.state, SessionEndState::Closed);
    eprintln!(
        "SDK conformance OK: language=rust corpus_sha256={}",
        manifest.corpus_sha256
    );
}
