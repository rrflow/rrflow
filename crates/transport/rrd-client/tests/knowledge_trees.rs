//! Public authenticated knowledge-tree configuration: real HTTP and durable engine.
//! Uses only this test's temporary installed estate; no external provider is invoked.

use rrd_client::{ClientConfig, Error, RequestOptions, RrdClient, Session};
use rrd_contract::{
    CanonicalId, CreateSession, ErrorCode, InstallationManagedPathKind, InstallationTargetKind,
    KnowledgePath, KnowledgePathBinding, KnowledgeTree, ListKnowledgeTrees, ReplaceKnowledgeTrees,
    ResourceId, ResourceKind, ResourcePath, SessionLimits,
};
use rrd_core::digest;
use rrd_engine::RrdEngine;
use rrd_security::{Action, Principal, PrincipalKind, ResourceGrant, SecurityRepository};
use rrd_server::RrdHttpServer;
use rrd_store::RrflowKvStore;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn read_options(label: &str) -> RequestOptions {
    RequestOptions::read(&format!("request-{label}"), &format!("operation-{label}")).unwrap()
}

fn write_options(label: &str, key: &str) -> RequestOptions {
    let deadline = u64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis(),
    )
    .unwrap()
        + 30_000;
    RequestOptions::new(
        &format!("request-{label}"),
        &format!("operation-{label}"),
        Some(key),
        Some(deadline),
    )
    .unwrap()
}

async fn session(client: &RrdClient, principal: &str, key: &str) -> Session {
    client
        .create_session(
            CanonicalId::new(principal).unwrap(),
            key,
            CreateSession {
                limits: SessionLimits {
                    idle_timeout_ms: 60_000,
                    absolute_timeout_ms: 300_000,
                    max_open_transactions: 1,
                },
            },
            write_options(principal, principal),
        )
        .await
        .unwrap()
}

fn principal(id: &str, credential: &[u8], instance: &CanonicalId, actions: &[Action]) -> Principal {
    Principal {
        id: CanonicalId::new(id).unwrap(),
        kind: PrincipalKind::Service,
        credential_sha256: digest::sha256_hex(credential),
        credential_revision: 1,
        not_before_unix_ms: 1,
        expires_at_unix_ms: u64::MAX,
        disabled: false,
        role_ids: Default::default(),
        grants: actions
            .iter()
            .map(|action| ResourceGrant {
                action: *action,
                resource_prefix: ResourcePath {
                    segments: vec![
                        ResourceId::new(ResourceKind::Instance, instance.as_str()).unwrap()
                    ],
                },
                data_policy: None,
            })
            .collect(),
    }
}

fn assert_code(error: Error, expected: ErrorCode) {
    match error {
        Error::Api { error, .. } => assert_eq!(error.code, expected),
        other => panic!("expected public {expected:?}, got {other:?}"),
    }
}

#[tokio::test]
async fn public_knowledge_trees_authenticate_compare_replay_and_survive_restart() {
    let temporary = tempfile::tempdir().unwrap();
    let project = temporary.path().join("disposable-knowledge-project");
    std::fs::create_dir_all(&project).unwrap();
    let executable = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let preview = RrdEngine::plan_installation(
        &project,
        InstallationTargetKind::ExistingProject,
        &digest::sha256_hex(b"knowledge-tree-installed-test"),
        "default",
        None,
        &executable,
    )
    .unwrap();
    RrdEngine::apply_installation(
        &project,
        InstallationTargetKind::ExistingProject,
        &preview,
        &preview.installation.plan_sha256,
        &executable,
    )
    .unwrap();
    let identity = preview.installation.target.clone();
    let scope = format!("instance:{}", identity.instance_id);
    let storage_root = project.join(
        &preview
            .installation
            .managed_path(InstallationManagedPathKind::StorageRoot)
            .relative_path,
    );
    let storage = RrflowKvStore::open(&storage_root).unwrap();
    let repository = SecurityRepository::new(&storage, identity.instance_id.clone());
    let mut security = repository.load().unwrap().unwrap();
    let revision = security.revision;
    security.revision += 1;
    for principal in [
        principal(
            "knowledge-owner",
            b"test-owner-key",
            &identity.instance_id,
            &[
                Action::SessionCreate,
                Action::KnowledgeTreeRead,
                Action::KnowledgeTreeWrite,
            ],
        ),
        principal(
            "knowledge-denied",
            b"test-denied-key",
            &identity.instance_id,
            &[Action::SessionCreate],
        ),
        principal(
            "knowledge-reader",
            b"test-reader-key",
            &identity.instance_id,
            &[Action::SessionCreate, Action::KnowledgeTreeRead],
        ),
    ] {
        security.principals.insert(principal.id.clone(), principal);
    }
    repository
        .replace(
            revision,
            security,
            2,
            "knowledge-public-test",
            "request-test-security",
            "operation-test-security",
        )
        .unwrap();
    drop(storage);

    let server = RrdHttpServer::bind(
        RrdEngine::open_installed(&project, &executable).unwrap(),
        "127.0.0.1:0".parse().unwrap(),
    )
    .unwrap();
    let client = RrdClient::connect_local(
        server.local_addr(),
        identity.instance_id.clone(),
        ClientConfig {
            request_timeout: Duration::from_secs(30),
            ..ClientConfig::default()
        },
    )
    .unwrap();
    let (shutdown, receiver) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(server.serve_until(async move {
        let _ = receiver.await;
    }));
    let capabilities = client.capabilities().await.unwrap();
    let knowledge = capabilities
        .capabilities
        .iter()
        .find(|capability| capability.name.as_str() == "knowledge-tree-catalogue")
        .unwrap();
    assert_eq!(knowledge.contract_version, 1);
    assert_eq!(
        knowledge.status,
        rrd_contract::CapabilityStatus::Experimental
    );
    assert_eq!(
        knowledge.limits[&CanonicalId::new("max-catalogue-bytes").unwrap()],
        rrd_contract::MAX_KNOWLEDGE_CATALOGUE_BYTES as u64
    );
    for operation in ["knowledge-tree-list", "knowledge-tree-replace"] {
        assert!(capabilities
            .product_capabilities
            .capabilities
            .iter()
            .any(|capability| capability.id == operation));
    }
    let owner = session(&client, "knowledge-owner", "test-owner-key").await;
    let denied = session(&client, "knowledge-denied", "test-denied-key").await;
    let reader = session(&client, "knowledge-reader", "test-reader-key").await;
    let list = ListKnowledgeTrees {
        project_id: identity.project_id.clone(),
        scope: scope.clone(),
    };
    let empty = client
        .list_knowledge_trees(&owner, list.clone(), read_options("initial"))
        .await
        .unwrap();
    assert_eq!(empty.revision, 0);
    assert!(empty.trees.is_empty());
    assert_code(
        client
            .list_knowledge_trees(&denied, list.clone(), read_options("denied-read"))
            .await
            .unwrap_err(),
        ErrorCode::PermissionDenied,
    );

    let tree = KnowledgeTree {
        id: CanonicalId::new("personal-knowledge").unwrap(),
        name: "Project knowledge".into(),
        paths: vec![
            KnowledgePath {
                id: CanonicalId::new("root").unwrap(),
                name: "Knowledge".into(),
                parent_id: None,
                binding: KnowledgePathBinding::Group,
            },
            KnowledgePath {
                id: CanonicalId::new("research").unwrap(),
                name: "Research".into(),
                parent_id: Some(CanonicalId::new("root").unwrap()),
                binding: KnowledgePathBinding::Group,
            },
        ],
    };
    let replacement = ReplaceKnowledgeTrees {
        project_id: identity.project_id.clone(),
        scope: scope.clone(),
        expected_revision: 0,
        trees: vec![tree],
    };
    assert_code(
        client
            .replace_knowledge_trees(
                &reader,
                replacement.clone(),
                write_options("reader-write", "reader-write"),
            )
            .await
            .unwrap_err(),
        ErrorCode::PermissionDenied,
    );
    let saved = client
        .replace_knowledge_trees(
            &owner,
            replacement.clone(),
            write_options("first-write", "knowledge-write"),
        )
        .await
        .unwrap();
    assert_eq!(saved.catalogue.revision, 1);
    assert_eq!(saved.catalogue.project_id, identity.project_id);
    assert_eq!(
        saved.catalogue,
        replacement.clone().into_catalogue().unwrap()
    );
    assert!(!saved.idempotent_replay);

    let replay = client
        .replace_knowledge_trees(
            &owner,
            replacement.clone(),
            write_options("replay", "knowledge-write"),
        )
        .await
        .unwrap();
    assert!(replay.idempotent_replay);
    assert_eq!(replay.catalogue, saved.catalogue);
    let mut changed = replacement.clone();
    changed.trees[0].name = "Changed content".into();
    assert_code(
        client
            .replace_knowledge_trees(
                &owner,
                changed,
                write_options("key-conflict", "knowledge-write"),
            )
            .await
            .unwrap_err(),
        ErrorCode::Conflict,
    );
    assert_code(
        client
            .replace_knowledge_trees(
                &owner,
                replacement.clone(),
                write_options("stale-revision", "stale-write"),
            )
            .await
            .unwrap_err(),
        ErrorCode::Conflict,
    );
    assert_eq!(
        client
            .list_knowledge_trees(&reader, list.clone(), read_options("after-denials"))
            .await
            .unwrap(),
        saved.catalogue
    );

    drop(client);
    shutdown.send(()).unwrap();
    task.await.unwrap().unwrap();
    let server = RrdHttpServer::bind(
        RrdEngine::open_installed(&project, &executable).unwrap(),
        "127.0.0.1:0".parse().unwrap(),
    )
    .unwrap();
    let client = RrdClient::connect_local(
        server.local_addr(),
        identity.instance_id,
        ClientConfig::default(),
    )
    .unwrap();
    let (shutdown, receiver) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(server.serve_until(async move {
        let _ = receiver.await;
    }));
    let capabilities = client.capabilities().await.unwrap();
    let knowledge = capabilities
        .capabilities
        .iter()
        .find(|capability| capability.name.as_str() == "knowledge-tree-catalogue")
        .unwrap();
    assert_eq!(knowledge.contract_version, 1);
    assert_eq!(
        knowledge.status,
        rrd_contract::CapabilityStatus::Experimental
    );
    assert_eq!(
        knowledge.limits[&CanonicalId::new("max-catalogue-bytes").unwrap()],
        rrd_contract::MAX_KNOWLEDGE_CATALOGUE_BYTES as u64
    );
    for operation in ["knowledge-tree-list", "knowledge-tree-replace"] {
        assert!(capabilities
            .product_capabilities
            .capabilities
            .iter()
            .any(|capability| capability.id == operation));
    }
    let owner = session(&client, "knowledge-owner", "test-owner-key").await;
    assert_eq!(
        client
            .list_knowledge_trees(&owner, list.clone(), read_options("reopened"))
            .await
            .unwrap(),
        saved.catalogue
    );
    let replay = client
        .replace_knowledge_trees(
            &owner,
            replacement,
            write_options("reopened-replay", "knowledge-write"),
        )
        .await
        .unwrap();
    assert!(replay.idempotent_replay);
    assert_eq!(replay.catalogue, saved.catalogue);
    drop(client);
    shutdown.send(()).unwrap();
    task.await.unwrap().unwrap();
}
