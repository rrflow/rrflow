use super::*;
use rrd_contract::{
    BindingKind, BindingLimits, CredentialReference, DeclaredFootprint, DeploymentForm,
    DeploymentProfile, EndpointBinding, EndpointLocator, EndpointPresentation, GpuRequirement,
    InstalledEstateIdentity, KnowledgePath, KnowledgePathBinding, KnowledgeTree,
    KnowledgeTreeCatalogue, ListKnowledgeTrees, ReplaceKnowledgeTrees, SessionLease,
    StorageProfileKind, WorkloadClass, DEPLOYMENT_PROFILE_CONTRACT_VERSION,
};
use std::path::Path;
use std::sync::{Arc, Barrier};

fn canonical(value: &str) -> CanonicalId {
    CanonicalId::new(value).unwrap()
}

fn project() -> CanonicalId {
    canonical("knowledge-project")
}

fn bind_project(engine: &mut RrdEngine, root: &Path) {
    let configuration = engine.estate_configuration().clone();
    engine
        .bind_installed_state(
            InstalledEstateIdentity {
                project_id: project(),
                estate_id: canonical("knowledge-estate"),
                instance_id: instance(),
            },
            DeploymentProfile {
                contract_version: DEPLOYMENT_PROFILE_CONTRACT_VERSION,
                deployment_form: DeploymentForm::Embedded,
                storage_profile: StorageProfileKind::RrflowKv,
                endpoint_presentation: EndpointPresentation::InProcess,
            },
            configuration,
            rrd_store::filesystem::AnchoredPath::from_path(root).unwrap(),
        )
        .unwrap();
}

// This embedded-engine fixture explicitly binds identity; installation and
// authenticated HTTP roundtrips have their own real-installation tests.
fn knowledge_engine() -> (tempfile::TempDir, RrdEngine, SessionLease) {
    let (root, mut engine) = isolated_engine();
    bind_project(&mut engine, root.path());
    let lease = engine
        .create_session(
            &session_request(9_000, 2),
            &id("knowledge-session"),
            1_000,
            "request-knowledge-session",
            "operation-knowledge-session",
        )
        .unwrap();
    (root, engine, lease)
}

fn list_request() -> ListKnowledgeTrees {
    ListKnowledgeTrees {
        project_id: project(),
        scope: format!("instance:{}", instance()),
    }
}

fn replacement(revision: u64, name: &str) -> ReplaceKnowledgeTrees {
    ReplaceKnowledgeTrees {
        project_id: project(),
        scope: list_request().scope,
        expected_revision: revision,
        trees: vec![KnowledgeTree {
            id: canonical("research"),
            name: name.into(),
            paths: vec![KnowledgePath {
                id: canonical("sources"),
                name: "Sources".into(),
                parent_id: None,
                binding: KnowledgePathBinding::Group,
            }],
        }],
    }
}

fn read(engine: &RrdEngine, lease: &SessionLease) -> KnowledgeTreeCatalogue {
    engine
        .list_knowledge_trees(
            &lease.session_id,
            &lease.token,
            &list_request(),
            1_500,
            &format!("request-knowledge-list-{}", lease.session_id.as_str()),
            &format!("operation-knowledge-list-{}", lease.session_id.as_str()),
        )
        .unwrap()
}

fn write(
    engine: &RrdEngine,
    lease: &SessionLease,
    request: &ReplaceKnowledgeTrees,
    key: &str,
) -> super::super::Result<rrd_contract::ReplaceKnowledgeTreesResult> {
    engine.replace_knowledge_trees(
        &lease.session_id,
        &lease.token,
        request,
        &mutation_context(
            &id(key),
            &format!("request-knowledge-write-{key}"),
            &format!("operation-knowledge-write-{key}"),
        ),
        1_400,
    )
}

#[test]
fn knowledge_catalogue_and_exact_receipt_survive_engine_restart() {
    let (root, engine, lease) = knowledge_engine();
    assert_eq!(read(&engine, &lease).revision, 0);
    let request = replacement(0, "Research 知識");
    let result = write(&engine, &lease, &request, "knowledge-first").unwrap();
    assert!(!result.idempotent_replay);
    assert_eq!(result.catalogue.revision, 1);
    let entries: Vec<_> = engine
        .storage
        .control()
        .journal_since(0, 64)
        .unwrap()
        .into_iter()
        .filter(|entry| entry.action.starts_with("knowledge_trees."))
        .collect();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].action, "knowledge_trees.replaced");
    assert_eq!(entries[1].action, "knowledge_trees.receipted");
    assert_eq!(entries[1].sequence, entries[0].sequence + 1);
    assert_eq!(entries[0].operation_id, entries[1].operation_id);
    drop(engine);

    let mut reopened = RrdEngine::open_existing(root.path(), instance(), TOKEN_KEY).unwrap();
    bind_project(&mut reopened, root.path());
    assert_eq!(read(&reopened, &lease), result.catalogue);
    let before = reopened.storage.control().sequence().unwrap();
    let replay = write(&reopened, &lease, &request, "knowledge-first").unwrap();
    assert!(replay.idempotent_replay);
    assert_eq!(replay.catalogue, result.catalogue);
    assert_eq!(reopened.storage.control().sequence().unwrap(), before);
}

#[test]
fn knowledge_saturated_admission_preserves_catalogue_and_releases_for_retry() {
    let (_root, mut engine, lease) = knowledge_engine();
    engine
        .estate_configuration
        .storage
        .transaction_admission_requests = 1;
    let permit = engine
        .admit_transaction(&(), "request-held", "operation-held")
        .unwrap();
    let request = replacement(0, "Admitted knowledge");
    assert!(matches!(
        write(&engine, &lease, &request, "bounded-knowledge"),
        Err(ServiceError::StorageAdmission(_))
    ));
    assert_eq!(read(&engine, &lease).revision, 0);
    assert!(engine
        .storage
        .control()
        .journal_since(0, 128)
        .unwrap()
        .iter()
        .all(|entry| !entry.action.starts_with("knowledge_trees.")));
    assert_eq!(
        engine
            .transaction_admission_snapshot()
            .unwrap()
            .outstanding_requests,
        1
    );
    drop(permit);
    let saved = write(&engine, &lease, &request, "bounded-knowledge").unwrap();
    assert!(!saved.idempotent_replay);
    assert_eq!(read(&engine, &lease), saved.catalogue);
    assert_eq!(
        engine
            .transaction_admission_snapshot()
            .unwrap()
            .outstanding_requests,
        0
    );
    let replay = write(&engine, &lease, &request, "bounded-knowledge").unwrap();
    assert!(replay.idempotent_replay);
    assert_eq!(replay.catalogue, saved.catalogue);
}

#[test]
fn knowledge_stale_revision_and_changed_replay_cannot_replace_current_content() {
    let (_root, engine, lease) = knowledge_engine();
    let first_request = replacement(0, "First");
    let first = write(&engine, &lease, &first_request, "first").unwrap();
    let second = write(&engine, &lease, &replacement(1, "Second"), "second").unwrap();
    let before = engine.storage.control().sequence().unwrap();
    let replay = write(&engine, &lease, &first_request, "first").unwrap();
    assert!(replay.idempotent_replay);
    assert_eq!(replay.catalogue, first.catalogue);
    assert_eq!(read(&engine, &lease), second.catalogue);
    assert!(matches!(
        write(&engine, &lease, &replacement(0, "Changed"), "first"),
        Err(ServiceError::IdempotencyConflict)
    ));
    assert!(matches!(
        write(&engine, &lease, &replacement(0, "Stale"), "new-key"),
        Err(ServiceError::StorageConflict(_))
    ));
    let mut moved = replacement(2, "Moved");
    moved.trees[0].id = canonical("other-tree");
    assert!(matches!(
        write(&engine, &lease, &moved, "moved-path"),
        Err(ServiceError::Contract(_))
    ));
    assert_eq!(read(&engine, &lease), second.catalogue);
    assert_eq!(engine.storage.control().sequence().unwrap(), before);
}

#[test]
fn knowledge_configuration_requires_exact_installed_project_and_instance_scope() {
    let (root, mut engine) = isolated_engine();
    let lease = engine
        .create_session(
            &session_request(9_000, 2),
            &id("unbound-knowledge-session"),
            1_000,
            "request-unbound",
            "operation-unbound",
        )
        .unwrap();
    let list = |request: &ListKnowledgeTrees| {
        engine.list_knowledge_trees(
            &lease.session_id,
            &lease.token,
            request,
            1_200,
            "request-bound-check",
            "operation-bound-check",
        )
    };
    assert!(matches!(
        list(&list_request()),
        Err(ServiceError::ProjectBindingMismatch)
    ));
    assert!(matches!(
        write(&engine, &lease, &replacement(0, "Unbound"), "unbound"),
        Err(ServiceError::ProjectBindingMismatch)
    ));
    bind_project(&mut engine, root.path());
    for (project_id, scope, is_project_error) in [
        (canonical("different-project"), list_request().scope, true),
        (project(), "instance:different-instance".into(), false),
    ] {
        let request = ListKnowledgeTrees { project_id, scope };
        let result = engine.list_knowledge_trees(
            &lease.session_id,
            &lease.token,
            &request,
            1_200,
            "request-isolation",
            "operation-isolation",
        );
        if is_project_error {
            assert!(matches!(result, Err(ServiceError::ProjectBindingMismatch)));
        } else {
            assert!(matches!(result, Err(ServiceError::WrongScope)));
        }
        let mut change = replacement(0, "Wrong binding");
        change.project_id = request.project_id;
        change.scope = request.scope;
        assert!(write(&engine, &lease, &change, "wrong-binding").is_err());
    }
    assert_eq!(read(&engine, &lease).revision, 0);
}

#[test]
fn knowledge_native_paths_require_real_collection_and_named_vector() {
    let (_root, engine, lease) = knowledge_engine();
    let mut request = replacement(0, "Native");
    request.trees[0].paths[0].binding = KnowledgePathBinding::Native {
        collection_id: canonical("documents"),
        vector_name: canonical("semantic"),
    };
    let missing = write(&engine, &lease, &request, "native-path").unwrap_err();
    assert!(matches!(missing, ServiceError::Contract(_)));
    assert!(missing.to_string().contains("unknown native collection"));
    assert_eq!(read(&engine, &lease).revision, 0);
    engine
        .ensure_vector_collection(
            &lease.session_id,
            &lease.token,
            &EnsureVectorCollection {
                scope: list_request().scope,
                collection_id: canonical("documents"),
                vectors: vec![NamedVectorDefinition {
                    name: canonical("semantic"),
                    field: canonical("body-embedding"),
                    kind: VectorValueKind::Dense,
                    dimensions: 3,
                    metric: VectorSearchMetric::Cosine,
                    embedding_model: None,
                    memory_tier: VectorMemoryTier::Cached,
                }],
            },
            &mutation_context(
                &id("native-collection"),
                "request-vector",
                "operation-vector",
            ),
            1_300,
        )
        .unwrap();
    let mut unknown_vector = request.clone();
    if let KnowledgePathBinding::Native { vector_name, .. } =
        &mut unknown_vector.trees[0].paths[0].binding
    {
        *vector_name = canonical("not-present");
    }
    let missing = write(&engine, &lease, &unknown_vector, "native-path").unwrap_err();
    assert!(matches!(missing, ServiceError::Contract(_)));
    assert!(missing.to_string().contains("unknown vector"));
    assert_eq!(read(&engine, &lease).revision, 0);
    let saved = write(&engine, &lease, &request, "native-path").unwrap();
    assert_eq!(saved.catalogue.revision, 1);
    assert_eq!(saved.catalogue.trees[0].paths, request.trees[0].paths);
}

#[test]
fn knowledge_external_path_persists_configuration_without_provider_execution() {
    let (_root, engine, lease) = knowledge_engine();
    let mut request = replacement(0, "Configured provider");
    request.trees[0].paths[0].binding = KnowledgePathBinding::External {
        provider_id: canonical("explicit-provider"),
        endpoint: Box::new(EndpointBinding {
            id: canonical("provider-endpoint"),
            class: WorkloadClass::VectorStore,
            kind: BindingKind::SelfHosted,
            endpoint: EndpointLocator::Url {
                url: "http://127.0.0.1:1/vectors".into(),
            },
            service_id: canonical("vector-service"),
            manifest_sha256: None,
            credential: Some(CredentialReference {
                provider: canonical("operator-vault"),
                name: "knowledge/vector-service".into(),
            }),
            placement: Some(canonical("local-machine")),
            footprint: DeclaredFootprint {
                memory_bytes: 1024,
                gpu: GpuRequirement::None,
            },
            limits: BindingLimits {
                max_concurrency: 1,
                timeout_ms: 1000,
                max_input_bytes: 1024,
            },
        }),
        namespace: Some("Research".into()),
        index: None,
        collection: Some("Documents".into()),
    };
    let saved = write(&engine, &lease, &request, "external-config").unwrap();
    assert_eq!(saved.catalogue.trees[0].paths, request.trees[0].paths);
    assert_eq!(read(&engine, &lease), saved.catalogue);
    let actions: Vec<_> = engine
        .storage
        .control()
        .journal_since(0, 64)
        .unwrap()
        .into_iter()
        .map(|entry| entry.action)
        .collect();
    assert!(!actions
        .iter()
        .any(|action| action.contains("provider") || action.contains("search")));
}

#[test]
fn knowledge_read_and_write_require_exact_grants_and_denials_are_audited() {
    let (root, mut engine) = isolated_engine();
    bind_project(&mut engine, root.path());
    let resource = ResourcePath {
        segments: vec![ResourceId::new(ResourceKind::Instance, instance().as_str()).unwrap()],
    };
    let principals = [
        (
            "writer",
            vec![
                SecurityAction::SessionCreate,
                SecurityAction::KnowledgeTreeRead,
                SecurityAction::KnowledgeTreeWrite,
            ],
        ),
        (
            "reader",
            vec![
                SecurityAction::SessionCreate,
                SecurityAction::KnowledgeTreeRead,
            ],
        ),
        ("denied", vec![SecurityAction::SessionCreate]),
    ]
    .into_iter()
    .map(|(name, actions)| {
        let principal_id = canonical(name);
        (
            principal_id.clone(),
            Principal {
                id: principal_id,
                kind: PrincipalKind::Service,
                credential_sha256: digest::sha256_hex(name.as_bytes()),
                credential_revision: 1,
                not_before_unix_ms: 1,
                expires_at_unix_ms: u64::MAX,
                disabled: false,
                role_ids: Default::default(),
                grants: actions
                    .into_iter()
                    .map(|action| ResourceGrant {
                        action,
                        resource_prefix: resource.clone(),
                        data_policy: None,
                    })
                    .collect(),
            },
        )
    })
    .collect();
    SecurityRepository::new(&engine.storage, instance())
        .initialize(
            SecurityState {
                format_version: rrd_security::SECURITY_FORMAT,
                revision: 1,
                principals,
                roles: Default::default(),
                identity_bindings: Default::default(),
                jwt_issuers: Default::default(),
            },
            1,
            "knowledge-security",
            "request-security",
            "operation-security",
        )
        .unwrap();
    let session = |name: &str| {
        engine
            .create_authenticated_session(
                &canonical(name),
                name.as_bytes(),
                &session_request(9_000, 2),
                &id(&format!("session-{name}")),
                1_000,
                &format!("request-{name}"),
                &format!("operation-{name}"),
            )
            .unwrap()
    };
    let writer = session("writer");
    let reader = session("reader");
    let denied = session("denied");
    let first = write(
        &engine,
        &writer,
        &replacement(0, "Authorized"),
        "authorized",
    )
    .unwrap();
    assert_eq!(read(&engine, &reader), first.catalogue);
    let before = engine.storage.control().journal_since(0, 128).unwrap();
    assert!(matches!(
        engine.list_knowledge_trees(
            &denied.session_id,
            &denied.token,
            &list_request(),
            1_500,
            "request-denied-read",
            "operation-denied-read",
        ),
        Err(ServiceError::PermissionDenied)
    ));
    assert!(matches!(
        write(&engine, &denied, &replacement(1, "Denied"), "denied-write"),
        Err(ServiceError::PermissionDenied)
    ));
    assert!(matches!(
        write(
            &engine,
            &reader,
            &replacement(1, "Reader write"),
            "reader-write"
        ),
        Err(ServiceError::PermissionDenied)
    ));
    let after = engine.storage.control().journal_since(0, 128).unwrap();
    assert_eq!(after.len(), before.len() + 6);
    assert!(after[before.len()..]
        .iter()
        .all(|entry| !entry.action.starts_with("knowledge_trees.")));
    let audits = SecurityRepository::new(&engine.storage, instance())
        .audit_since(0, 128)
        .unwrap();
    for action in [
        SecurityAction::KnowledgeTreeRead,
        SecurityAction::KnowledgeTreeWrite,
    ] {
        assert!(audits
            .records
            .iter()
            .any(|(_, record)| record.action == action
                && record.phase == AuditPhase::Completed
                && record.decision == AuditDecision::Denied));
    }
    assert_eq!(read(&engine, &writer), first.catalogue);
}

#[test]
fn knowledge_receipt_cannot_bypass_invalid_or_expired_session() {
    let (_root, engine, lease) = knowledge_engine();
    let request = replacement(0, "Original");
    write(&engine, &lease, &request, "protected-receipt").unwrap();
    let before = engine.storage.control().sequence().unwrap();
    let context = mutation_context(
        &id("protected-receipt"),
        "request-replay",
        "operation-replay",
    );
    assert!(matches!(
        engine.replace_knowledge_trees(
            &lease.session_id,
            &id("wrong-token"),
            &request,
            &context,
            1_500,
        ),
        Err(ServiceError::Unauthenticated)
    ));
    assert_eq!(engine.storage.control().sequence().unwrap(), before);
    assert!(matches!(
        engine
            .replace_knowledge_trees(&lease.session_id, &lease.token, &request, &context, 10_001,),
        Err(ServiceError::SessionExpired)
    ));
    let after = engine.storage.control().journal_since(before, 64).unwrap();
    assert!(after
        .iter()
        .all(|entry| !entry.action.starts_with("knowledge_trees.")));
}

#[test]
fn knowledge_corrupt_persisted_content_never_becomes_an_empty_success() {
    let (_root, engine, lease) = knowledge_engine();
    write(
        &engine,
        &lease,
        &replacement(0, "Original"),
        "before-corruption",
    )
    .unwrap();
    let entry = engine
        .storage
        .control()
        .journal_since(0, 64)
        .unwrap()
        .into_iter()
        .find(|entry| entry.action == "knowledge_trees.replaced")
        .unwrap();
    let expected = engine.storage.control().get(&entry.key).unwrap();
    let mut state: Value = serde_json::from_slice(expected.as_ref().unwrap()).unwrap();
    state["catalogue"]["trees"][0]["name"] = serde_json::json!("Corrupt replacement");
    engine
        .storage
        .control()
        .commit(&ControlTransition {
            key: entry.key,
            expected,
            replacement: Some(serde_json::to_vec(&state).unwrap()),
            at: 1_450,
            actor: "test-fixture".into(),
            action: "test.corrupt_knowledge".into(),
            request_id: "request-corrupt".into(),
            operation_id: "operation-corrupt".into(),
        })
        .unwrap();
    let result = engine.list_knowledge_trees(
        &lease.session_id,
        &lease.token,
        &list_request(),
        1_500,
        "request-corrupt-read",
        "operation-corrupt-read",
    );
    assert!(matches!(result, Err(ServiceError::Storage(_))));
    assert!(matches!(
        write(
            &engine,
            &lease,
            &replacement(1, "Cannot overwrite"),
            "after-corruption"
        ),
        Err(ServiceError::Storage(_))
    ));
}

#[test]
fn knowledge_concurrent_replacements_have_one_cas_winner() {
    let (_root, engine, lease) = knowledge_engine();
    let engine = Arc::new(engine);
    let barrier = Arc::new(Barrier::new(2));
    let results = std::thread::scope(|scope| {
        let handles: Vec<_> = ["left", "right"]
            .into_iter()
            .map(|name| {
                let engine = Arc::clone(&engine);
                let barrier = Arc::clone(&barrier);
                let lease = lease.clone();
                scope.spawn(move || {
                    barrier.wait();
                    write(&engine, &lease, &replacement(0, name), name)
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect::<Vec<_>>()
    });
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|result| matches!(result, Err(ServiceError::StorageConflict(_))))
            .count(),
        1
    );
    assert_eq!(read(&engine, &lease).revision, 1);
    let entries = engine.storage.control().journal_since(0, 64).unwrap();
    assert_eq!(
        entries
            .iter()
            .filter(|entry| entry.action.starts_with("knowledge_trees."))
            .count(),
        2
    );
}

#[test]
fn knowledge_stale_write_is_rejected_after_a_different_engine_lifetime_commits() {
    let (root, first_engine, lease) = knowledge_engine();
    let stale = replacement(0, "Stale from the first engine");
    assert_eq!(read(&first_engine, &lease).revision, 0);
    drop(first_engine);

    let mut second_engine = RrdEngine::open_existing(root.path(), instance(), TOKEN_KEY).unwrap();
    bind_project(&mut second_engine, root.path());
    let winner = write(
        &second_engine,
        &lease,
        &replacement(0, "Second engine"),
        "second-engine",
    )
    .unwrap();
    drop(second_engine);

    let mut first_engine = RrdEngine::open_existing(root.path(), instance(), TOKEN_KEY).unwrap();
    bind_project(&mut first_engine, root.path());
    assert!(matches!(
        write(&first_engine, &lease, &stale, "stale-engine"),
        Err(ServiceError::StorageConflict(_))
    ));
    assert_eq!(read(&first_engine, &lease), winner.catalogue);
    // A failed stale operation did not reserve its idempotency key or partly
    // update the catalogue: the corrected request can use that same key.
    let corrected = write(
        &first_engine,
        &lease,
        &replacement(1, "Corrected"),
        "stale-engine",
    )
    .unwrap();
    assert_eq!(corrected.catalogue.revision, 2);
    assert!(!corrected.idempotent_replay);
}

#[test]
fn knowledge_empty_replacement_is_a_durable_removal_and_replay_does_not_restore() {
    let (root, engine, lease) = knowledge_engine();
    let original = replacement(0, "Original");
    write(&engine, &lease, &original, "before-clear").unwrap();
    let cleared = ReplaceKnowledgeTrees {
        trees: Vec::new(),
        ..replacement(1, "Unused")
    };
    let result = write(&engine, &lease, &cleared, "clear").unwrap();
    assert_eq!(result.catalogue.revision, 2);
    assert!(result.catalogue.trees.is_empty());
    assert_eq!(
        write(&engine, &lease, &original, "before-clear")
            .unwrap()
            .catalogue
            .revision,
        1
    );
    assert_eq!(read(&engine, &lease), result.catalogue);
    drop(engine);
    let mut reopened = RrdEngine::open_existing(root.path(), instance(), TOKEN_KEY).unwrap();
    bind_project(&mut reopened, root.path());
    assert_eq!(read(&reopened, &lease), result.catalogue);
}

#[test]
fn knowledge_receipt_budget_uses_revision_without_scanning_receipt_history() {
    let (_root, engine, lease) = knowledge_engine();
    let request = replacement(0, "Original accepted snapshot");
    let first = write(&engine, &lease, &request, "retained-receipt").unwrap();
    let entry = engine
        .storage
        .control()
        .journal_since(0, 64)
        .unwrap()
        .into_iter()
        .find(|entry| entry.action == "knowledge_trees.replaced")
        .unwrap();
    let expected = engine.storage.control().get(&entry.key).unwrap();
    let mut state: Value = serde_json::from_slice(expected.as_ref().unwrap()).unwrap();
    // A bounded checkpoint fixture tests exhaustion without creating 100,000
    // snapshots. Production revisions advance only with an atomic new receipt.
    let mut catalogue = first.catalogue.clone();
    catalogue.revision = 100_000;
    state["catalogue"] = serde_json::to_value(&catalogue).unwrap();
    state["catalogue_sha256"] = serde_json::json!(catalogue.sha256().unwrap());
    engine
        .storage
        .control()
        .commit(&ControlTransition {
            key: entry.key,
            expected,
            replacement: Some(serde_json::to_vec(&state).unwrap()),
            at: 1_450,
            actor: "test-fixture".into(),
            action: "test.knowledge_receipt_budget".into(),
            request_id: "request-budget-fixture".into(),
            operation_id: "operation-budget-fixture".into(),
        })
        .unwrap();
    let before = engine.storage.control().sequence().unwrap();
    assert!(matches!(
        write(
            &engine,
            &lease,
            &replacement(100_000, "Over budget"),
            "new-over-budget"
        ),
        Err(ServiceError::ConfigurationLimit(_))
    ));
    let replay = write(&engine, &lease, &request, "retained-receipt").unwrap();
    assert!(replay.idempotent_replay);
    assert_eq!(replay.catalogue, first.catalogue);
    assert_eq!(read(&engine, &lease), catalogue);
    assert_eq!(engine.storage.control().sequence().unwrap(), before);
}

#[test]
fn knowledge_idempotency_is_bound_to_the_authenticated_session() {
    let (_root, engine, first_session) = knowledge_engine();
    let second_session = engine
        .create_session(
            &session_request(9_000, 2),
            &id("knowledge-second-session"),
            1_000,
            "request-second-session",
            "operation-second-session",
        )
        .unwrap();
    let first_request = replacement(0, "First session");
    let first = write(&engine, &first_session, &first_request, "shared-client-key").unwrap();
    let second = write(
        &engine,
        &second_session,
        &replacement(1, "Second session"),
        "shared-client-key",
    )
    .unwrap();
    assert!(!second.idempotent_replay);
    assert_eq!(second.catalogue.revision, 2);
    let replay = write(&engine, &first_session, &first_request, "shared-client-key").unwrap();
    assert!(replay.idempotent_replay);
    assert_eq!(replay.catalogue, first.catalogue);
    assert_eq!(read(&engine, &second_session), second.catalogue);
}
