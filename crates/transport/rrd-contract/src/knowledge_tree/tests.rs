use super::*;
use crate::{BindingKind, BindingLimits, CredentialReference, DeclaredFootprint, GpuRequirement};

fn id(value: &str) -> CanonicalId {
    CanonicalId::new(value).unwrap()
}

fn group(value: &str, parent: Option<&str>) -> KnowledgePath {
    KnowledgePath {
        id: id(value),
        name: value.into(),
        parent_id: parent.map(id),
        binding: KnowledgePathBinding::Group,
    }
}

fn native(value: &str, parent: Option<&str>) -> KnowledgePath {
    KnowledgePath {
        binding: KnowledgePathBinding::Native {
            collection_id: id("documents"),
            vector_name: id("semantic"),
        },
        ..group(value, parent)
    }
}

fn request() -> ReplaceKnowledgeTrees {
    ReplaceKnowledgeTrees {
        project_id: id("project-alpha"),
        scope: "development".into(),
        expected_revision: 0,
        trees: vec![KnowledgeTree {
            id: id("research"),
            name: "Research 知識".into(),
            paths: vec![group("papers", None), native("embeddings", Some("papers"))],
        }],
    }
}

fn external() -> KnowledgePathBinding {
    KnowledgePathBinding::External {
        provider_id: id("postgres-pgvector"),
        endpoint: Box::new(EndpointBinding {
            id: id("vector-postgres"),
            class: WorkloadClass::VectorStore,
            kind: BindingKind::SelfHosted,
            endpoint: EndpointLocator::Url {
                url: "postgresql://localhost:5432/knowledge".into(),
            },
            service_id: id("knowledge-postgres"),
            manifest_sha256: None,
            credential: Some(CredentialReference {
                provider: id("local-vault"),
                name: "knowledge/postgres".into(),
            }),
            placement: Some(id("local-machine")),
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
        namespace: Some("Public".into()),
        index: None,
        collection: Some("ResearchDocuments".into()),
    }
}

#[test]
fn project_directory_paths_are_portable_and_project_scoped() {
    for relative_path in [None, Some("docs/research papers"), Some("src/knowledge")] {
        let binding = KnowledgePathBinding::ProjectDirectory {
            relative_path: relative_path.map(str::to_owned),
        };
        binding.validate().unwrap();
        let encoded = serde_json::to_string(&binding).unwrap();
        let decoded: KnowledgePathBinding = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, binding);
    }
    for value in [
        "",
        ".",
        "..",
        "../private",
        "/absolute",
        "C:/outside",
        "dir:stream",
        "a\\b",
        "a//b",
        "a/../b",
        "a/b.",
        "a/b ",
        "a/NUL.txt",
        "docs/COM\u{b9}",
        "com\u{b2}.txt",
        "Lpt\u{b3}",
    ] {
        let binding = KnowledgePathBinding::ProjectDirectory {
            relative_path: Some(value.into()),
        };
        assert!(
            binding.validate().is_err(),
            "unsafe project directory accepted: {value:?}"
        );
    }
}

#[test]
fn knowledge_tree_group_and_native_roundtrip_keeps_human_names() {
    let catalogue = request().into_catalogue().unwrap();
    catalogue.validate().unwrap();
    assert_eq!(catalogue.revision, 1);
    assert_eq!(catalogue.trees[0].name, "Research 知識");
    let json = serde_json::to_string(&catalogue).unwrap();
    let decoded: KnowledgeTreeCatalogue = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded, catalogue);
    assert!(json.contains("\"kind\":\"group\""));
}

#[test]
fn knowledge_tree_rejects_duplicate_ids_names_and_cross_tree_paths() {
    let mut duplicate_tree = request();
    duplicate_tree.trees.push(duplicate_tree.trees[0].clone());
    assert!(duplicate_tree.validate().is_err());
    let mut duplicate_name = request();
    let mut tree = duplicate_name.trees[0].clone();
    tree.id = id("another-tree");
    tree.paths.clear();
    duplicate_name.trees.push(tree);
    assert!(duplicate_name.validate().is_err());
    let mut duplicate_path = request();
    duplicate_path.trees[0]
        .paths
        .push(native("embeddings", None));
    assert!(duplicate_path.validate().is_err());
    let mut cross_tree = request();
    cross_tree.trees.push(KnowledgeTree {
        id: id("second"),
        name: "Second".into(),
        paths: vec![group("papers", None)],
    });
    assert!(cross_tree.validate().is_err());
    let mut siblings = request();
    let mut path = native("second-leaf", Some("papers"));
    path.name = "embeddings".into();
    siblings.trees[0].paths.push(path);
    assert!(siblings.validate().is_err());
}

#[test]
fn knowledge_tree_rejects_cycles_missing_parents_and_non_group_parents() {
    for paths in [
        vec![group("self-cycle", Some("self-cycle"))],
        vec![group("left", Some("right")), group("right", Some("left"))],
        vec![group("orphan", Some("missing"))],
        vec![native("leaf", None), native("child", Some("leaf"))],
    ] {
        let mut candidate = request();
        candidate.trees[0].paths = paths;
        assert!(candidate.validate().is_err());
    }
    let mut other_tree = request();
    other_tree.trees.push(KnowledgeTree {
        id: id("other"),
        name: "Other".into(),
        paths: vec![group("wrong-tree-parent", Some("papers"))],
    });
    assert!(other_tree.validate().is_err());
}

#[test]
fn knowledge_tree_depth_budget_is_inclusive() {
    let mut candidate = request();
    candidate.trees[0].paths = (0..MAX_KNOWLEDGE_PATH_DEPTH)
        .map(|index| {
            group(
                &format!("node-{index}"),
                (index > 0)
                    .then(|| format!("node-{}", index - 1))
                    .as_deref(),
            )
        })
        .collect();
    candidate.validate().unwrap();
    candidate.trees[0].paths.push(group(
        "too-deep",
        Some(&format!("node-{}", MAX_KNOWLEDGE_PATH_DEPTH - 1)),
    ));
    assert!(candidate.validate().is_err());
}

#[test]
fn knowledge_tree_enforces_tree_path_and_total_count_budgets() {
    let mut candidate = request();
    candidate.trees = (0..=MAX_KNOWLEDGE_TREES)
        .map(|index| KnowledgeTree {
            id: id(&format!("tree-{index}")),
            name: format!("Tree {index}"),
            paths: Vec::new(),
        })
        .collect();
    assert!(candidate.validate().is_err());
    candidate.trees = vec![KnowledgeTree {
        id: id("large-tree"),
        name: "Large tree".into(),
        paths: (0..=MAX_KNOWLEDGE_PATHS_PER_TREE)
            .map(|index| group(&format!("path-{index}"), None))
            .collect(),
    }];
    assert!(candidate.validate().is_err());
    candidate.trees = (0..9)
        .map(|tree_index| KnowledgeTree {
            id: id(&format!("tree-{tree_index}")),
            name: format!("Tree {tree_index}"),
            paths: (0..MAX_KNOWLEDGE_PATHS_PER_TREE)
                .map(|path_index| group(&format!("path-{tree_index}-{path_index}"), None))
                .collect(),
        })
        .collect();
    assert!(candidate.validate().is_err());
}

#[test]
fn knowledge_tree_checks_encoded_budget_after_individual_field_bounds() {
    let mut candidate = request();
    candidate.trees = (0..3)
        .map(|tree_index| KnowledgeTree {
            id: id(&format!("tree-{tree_index}")),
            name: format!("Tree {tree_index}"),
            paths: (0..MAX_KNOWLEDGE_PATHS_PER_TREE)
                .map(|path_index| {
                    let mut path = native(&format!("path-{tree_index}-{path_index}"), None);
                    path.binding = external();
                    if let KnowledgePathBinding::External { endpoint, .. } = &mut path.binding {
                        endpoint.endpoint = EndpointLocator::Url {
                            url: format!("https://example.test/{}", "p".repeat(1800)),
                        };
                    }
                    path
                })
                .collect(),
        })
        .collect();
    let error = candidate.validate().unwrap_err();
    assert!(error.to_string().contains("encoded byte budget"));
}

#[test]
fn knowledge_tree_rejects_unsafe_names_and_counts_utf8_bytes() {
    for name in [
        "",
        " ",
        " leading",
        "trailing ",
        "line\nbreak",
        "spoof\u{202e}name",
    ] {
        let mut candidate = request();
        candidate.trees[0].name = name.into();
        assert!(candidate.validate().is_err());
    }
    let mut candidate = request();
    candidate.trees[0].name = "知".repeat(MAX_KNOWLEDGE_NAME_BYTES / 3 + 1);
    assert!(candidate.validate().is_err());
}

#[test]
fn knowledge_tree_external_configuration_reuses_endpoint_credential_rules() {
    external().validate().unwrap();
    let mut anonymous = external();
    if let KnowledgePathBinding::External { endpoint, .. } = &mut anonymous {
        endpoint.credential = None;
    }
    anonymous.validate().unwrap();
    if let KnowledgePathBinding::External { endpoint, .. } = &mut anonymous {
        endpoint.kind = BindingKind::Api;
        endpoint.placement = None;
    }
    assert!(anonymous.validate().is_err());
    let mut invalid_credential = external();
    if let KnowledgePathBinding::External { endpoint, .. } = &mut invalid_credential {
        endpoint.credential.as_mut().unwrap().name = "secret value".into();
    }
    assert!(invalid_credential.validate().is_err());
    let mut wrong_class = external();
    if let KnowledgePathBinding::External { endpoint, .. } = &mut wrong_class {
        endpoint.class = WorkloadClass::CanonicalStore;
    }
    assert!(wrong_class.validate().is_err());
    let mut no_destination = external();
    if let KnowledgePathBinding::External {
        namespace,
        collection,
        ..
    } = &mut no_destination
    {
        *namespace = None;
        *collection = None;
    }
    assert!(no_destination.validate().is_err());
}

#[test]
fn knowledge_tree_rejects_secret_urls_without_echoing_values() {
    for url in [
        "https://operator:do-not-print@example.test/vectors",
        "https://example.test/vectors?api_key=do-not-print",
        "https://example.test/vectors#do-not-print",
        "https://operator%40example.test/vectors",
    ] {
        let mut binding = external();
        if let KnowledgePathBinding::External { endpoint, .. } = &mut binding {
            endpoint.endpoint = EndpointLocator::Url { url: url.into() };
        }
        let error = binding.validate().unwrap_err();
        assert!(!error.to_string().contains("do-not-print"));
    }
    let mut in_process = external();
    if let KnowledgePathBinding::External { endpoint, .. } = &mut in_process {
        endpoint.kind = BindingKind::Local;
        endpoint.endpoint = EndpointLocator::InProcess {
            component: id("not-external"),
        };
    }
    assert!(in_process.validate().is_err());
}

#[test]
fn knowledge_tree_deserialization_rejects_unknown_secret_and_identity_fields() {
    let mut value = serde_json::to_value(request()).unwrap();
    value["password"] = serde_json::json!("do-not-store");
    assert!(serde_json::from_value::<ReplaceKnowledgeTrees>(value).is_err());
    let mut value = serde_json::to_value(external()).unwrap();
    value["endpoint"]["credential"]["value"] = serde_json::json!("do-not-store");
    assert!(serde_json::from_value::<KnowledgePathBinding>(value).is_err());
    let mut value = serde_json::to_value(request()).unwrap();
    value["trees"][0]["id"] = serde_json::json!("Not Canonical");
    assert!(serde_json::from_value::<ReplaceKnowledgeTrees>(value).is_err());
}

#[test]
fn knowledge_tree_canonical_digest_binds_scope_project_revision_and_content() {
    let first = request().into_catalogue().unwrap();
    let mut reordered = first.clone();
    reordered.trees[0].paths.reverse();
    assert_eq!(first.sha256().unwrap(), reordered.sha256().unwrap());
    for changed in [
        {
            let mut value = first.clone();
            value.scope = "another".into();
            value
        },
        {
            let mut value = first.clone();
            value.project_id = id("project-beta");
            value
        },
        {
            let mut value = first.clone();
            value.revision += 1;
            value
        },
        {
            let mut value = first.clone();
            value.trees[0].name = "Changed".into();
            value
        },
    ] {
        assert_ne!(first.sha256().unwrap(), changed.sha256().unwrap());
    }
    let mut two_trees = request();
    two_trees.trees.push(KnowledgeTree {
        id: id("a-first"),
        name: "First".into(),
        paths: Vec::new(),
    });
    let catalogue = two_trees.into_catalogue().unwrap();
    assert_eq!(catalogue.trees[0].id, id("a-first"));
    let mut reversed = catalogue.clone();
    reversed.trees.reverse();
    assert_eq!(catalogue.sha256().unwrap(), reversed.sha256().unwrap());
}

#[test]
fn knowledge_tree_revision_and_identity_replacement_checks_are_explicit() {
    let previous = request().into_catalogue().unwrap();
    let mut next = request();
    next.expected_revision = 1;
    next.trees[0].name = "Renamed".into();
    next.validate_against(&previous).unwrap();
    let mut stale = next.clone();
    stale.expected_revision = 0;
    assert!(stale.validate_against(&previous).is_err());
    let mut wrong_project = next.clone();
    wrong_project.project_id = id("project-beta");
    assert!(wrong_project.validate_against(&previous).is_err());
    let mut wrong_scope = next.clone();
    wrong_scope.scope = "another".into();
    assert!(wrong_scope.validate_against(&previous).is_err());
    let mut moved_identity = next.clone();
    moved_identity.trees[0].id = id("different-tree");
    assert!(moved_identity.validate_against(&previous).is_err());
    let mut overflow = next;
    overflow.expected_revision = u64::MAX;
    assert!(overflow.validate().is_err());
}

#[test]
fn knowledge_tree_initial_and_empty_replacement_are_versioned_and_scoped() {
    let initial = KnowledgeTreeCatalogue::empty(id("project-alpha"), "development".into()).unwrap();
    assert_eq!(initial.revision, 0);
    let mut candidate = request();
    candidate.validate_against(&initial).unwrap();
    candidate.trees.clear();
    let empty = candidate.into_catalogue().unwrap();
    assert_eq!(empty.revision, 1);
    empty.validate().unwrap();
    let mut invalid_initial = request().into_catalogue().unwrap();
    invalid_initial.revision = 0;
    assert!(invalid_initial.validate().is_err());
    let mut future = empty;
    future.contract_version += 1;
    assert!(future.validate().is_err());
    for scope in ["", "with\0nul", &"s".repeat(crate::MAX_ID_BYTES + 1)] {
        assert!(ListKnowledgeTrees {
            project_id: id("project-alpha"),
            scope: scope.into()
        }
        .validate()
        .is_err());
    }
}
