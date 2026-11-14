//! Estate organisation, public schemas, data retirement, retrieval and embeddings.

use super::*;

#[test]
pub(super) fn unified_catalogue_is_strict_round_trippable_and_rejects_mode_confusion() {
    let document = CanonicalId::new("document").unwrap();
    let mut registry = DataSchemaRegistry {
        revision: 1,
        migration: "freeze public catalogue".into(),
        organisation: DataEstateOrganisation {
            domains: BTreeMap::from([(
                CanonicalId::new("knowledge").unwrap(),
                DataDomainSchema {
                    title: "Knowledge".into(),
                    description: String::new(),
                    parent: None,
                },
            )]),
            kind_domains: BTreeMap::from([(
                document.clone(),
                CanonicalId::new("knowledge").unwrap(),
            )]),
            ..DataEstateOrganisation::default()
        },
        tables: BTreeMap::from([
            (
                document.clone(),
                DataTableSchema {
                    model: DataLogicalModel::Document,
                    mode: DataSchemaMode::Strict,
                    properties: BTreeMap::new(),
                    allow_additional_properties: false,
                },
            ),
            (
                CanonicalId::new("kv").unwrap(),
                DataTableSchema {
                    model: DataLogicalModel::KeyValue,
                    mode: DataSchemaMode::Schemaless,
                    properties: BTreeMap::new(),
                    allow_additional_properties: false,
                },
            ),
        ]),
        records: BTreeMap::from([(document.clone(), DataRecordSchema::default())]),
        relations: BTreeMap::new(),
        events: BTreeMap::new(),
    };
    let mutation = TransactionMutation::PutSchema {
        registry: registry.clone(),
    };
    mutation.validate().unwrap();
    let encoded = serde_json::to_value(&mutation).unwrap();
    let decoded: TransactionMutation = serde_json::from_value(encoded).unwrap();
    assert_eq!(decoded, mutation);

    registry.tables.get_mut(&document).unwrap().mode = DataSchemaMode::Schemaless;
    assert!(TransactionMutation::PutSchema { registry }
        .validate()
        .is_err());
}

pub(super) fn organised_schema(organisation: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "mutation": "put_schema",
        "registry": {
            "revision": 1,
            "migration": "organise the estate",
            "organisation": organisation,
            "tables": {
                "invoice": {"model": "document", "mode": "schemaless"},
                "rrflow-seat": {"model": "document", "mode": "schemaless"}
            }
        }
    })
}

pub(super) fn put_schema_error(organisation: serde_json::Value) -> String {
    let mutation: TransactionMutation =
        serde_json::from_value(organised_schema(organisation)).unwrap();
    mutation.validate().unwrap_err().to_string()
}

#[test]
pub(super) fn estate_organisation_is_validated_at_the_public_boundary() {
    let billing = serde_json::json!({"billing": {"title": "Billing"}});
    let accepted: TransactionMutation =
        serde_json::from_value(organised_schema(serde_json::json!({
            "domains": {
                "billing": {"title": "Billing", "description": "Invoices and payments"},
                "invoices": {"title": "Invoices", "parent": "billing"}
            },
            "categories": {
                "lifecycle": {
                    "title": "Lifecycle",
                    "targets": "kinds_and_records",
                    "cardinality": "single",
                    "tags": {"active": {"title": "Active"}}
                }
            },
            "kind_domains": {"invoice": "invoices"},
            "collection_domains": {"invoice-text": "invoices"},
            "kind_tags": {"invoice": [{"category": "lifecycle", "tag": "active"}]}
        })))
        .unwrap();
    accepted.validate().unwrap();
    let TransactionMutation::PutSchema { registry } = &accepted else {
        unreachable!()
    };
    let invoice = CanonicalId::new("invoice").unwrap();
    let seat = CanonicalId::new("rrflow-seat").unwrap();
    assert_eq!(registry.organisation.domain_of_kind(&invoice), "invoices");
    assert_eq!(registry.organisation.domain_of_kind(&seat), "rrflow");

    for (organisation, expected) in [
        (
            serde_json::json!({"domains": {"default": {"title": "Default"}}}),
            "built in or reserved",
        ),
        (
            serde_json::json!({"domains": {"rrflow-private": {"title": "Private"}}}),
            "built in or reserved",
        ),
        (
            serde_json::json!({"domains": {"billing": {"title": "Billing", "parent": "rrflow"}}}),
            "inside rrflow",
        ),
        (
            serde_json::json!({"domains": {"billing": {"title": "Billing", "parent": "missing"}}}),
            "unknown parent",
        ),
        (
            serde_json::json!({"domains": {
                "a": {"title": "A", "parent": "b"},
                "b": {"title": "B", "parent": "a"}
            }}),
            "cycle",
        ),
        (
            serde_json::json!({"domains": {"billing": {"title": " "}}}),
            "needs a title",
        ),
        (
            serde_json::json!({"domains": billing.clone(), "kind_domains": {"invoice": "shipping"}}),
            "unknown domain",
        ),
        (
            serde_json::json!({"domains": billing.clone(), "kind_domains": {"invoice": "default"}}),
            "omit the entry",
        ),
        (
            serde_json::json!({"domains": billing.clone(), "kind_domains": {"customer": "billing"}}),
            "unregistered table type",
        ),
        (
            serde_json::json!({"domains": billing.clone(), "kind_domains": {"rrflow-seat": "billing"}}),
            "always belongs to rrflow",
        ),
        (
            serde_json::json!({"kind_domains": {"invoice": "rrflow"}}),
            "cannot be placed in rrflow",
        ),
        (
            serde_json::json!({"kind_tags": {"invoice": [{"category": "lifecycle", "tag": "active"}]}}),
            "unknown category",
        ),
        (
            serde_json::json!({
                "categories": {"lifecycle": {
                    "title": "Lifecycle", "targets": "records", "cardinality": "single",
                    "tags": {"active": {"title": "Active"}}
                }},
                "kind_tags": {"invoice": [{"category": "lifecycle", "tag": "active"}]}
            }),
            "does not tag table types",
        ),
        (
            serde_json::json!({
                "categories": {"lifecycle": {
                    "title": "Lifecycle", "targets": "kinds", "cardinality": "single",
                    "tags": {"active": {"title": "Active"}, "retired": {"title": "Retired"}}
                }},
                "kind_tags": {"invoice": [
                    {"category": "lifecycle", "tag": "active"},
                    {"category": "lifecycle", "tag": "retired"}
                ]}
            }),
            "single-tag category",
        ),
    ] {
        let error = put_schema_error(organisation.clone());
        assert!(error.contains(expected), "{organisation}: {error}");
    }

    // The retired namespace/database label is no longer part of the contract.
    let mut legacy = organised_schema(serde_json::json!({}));
    legacy["registry"]["catalogue"] =
        serde_json::json!({"namespace": "default", "database": "default"});
    assert!(serde_json::from_value::<TransactionMutation>(legacy).is_err());

    // A default organisation is omitted, so existing registries serialize unchanged.
    let mut unorganised = organised_schema(serde_json::json!({}));
    unorganised["registry"]
        .as_object_mut()
        .unwrap()
        .remove("organisation");
    let decoded: TransactionMutation = serde_json::from_value(unorganised).unwrap();
    let encoded = serde_json::to_value(&decoded).unwrap();
    assert!(encoded["registry"].get("organisation").is_none());
}

#[test]
pub(super) fn public_schema_requires_one_explicit_nonempty_table_map() {
    let document = CanonicalId::new("document").unwrap();
    let registry = DataSchemaRegistry {
        revision: 1,
        migration: "reject inferred public table models".into(),
        organisation: DataEstateOrganisation::default(),
        tables: BTreeMap::new(),
        records: BTreeMap::from([(document, DataRecordSchema::default())]),
        relations: BTreeMap::new(),
        events: BTreeMap::new(),
    };
    assert!(TransactionMutation::PutSchema {
        registry: registry.clone()
    }
    .validate()
    .is_err());

    let mut encoded = serde_json::to_value(TransactionMutation::PutSchema { registry }).unwrap();
    encoded["registry"]
        .as_object_mut()
        .unwrap()
        .remove("tables");
    assert!(serde_json::from_value::<TransactionMutation>(encoded).is_err());
}

#[test]
pub(super) fn data_retirement_targets_are_model_typed_and_event_cursor_addressed() {
    let document = DataTarget::Reference {
        reference: DataReference {
            kind: CanonicalId::new("document").unwrap(),
            id: CanonicalId::new("alpha").unwrap(),
        },
    };
    TransactionMutation::RetireData {
        model: DataLogicalModel::Document,
        target: document.clone(),
        effective_at: 10,
    }
    .validate()
    .unwrap();
    let event = DataTarget::Event {
        kind: CanonicalId::new("observed").unwrap(),
        cursor: 7,
    };
    let retirement = TransactionMutation::RetireData {
        model: DataLogicalModel::Event,
        target: event,
        effective_at: 10,
    };
    retirement.validate().unwrap();
    assert_eq!(
        serde_json::from_value::<TransactionMutation>(serde_json::to_value(&retirement).unwrap())
            .unwrap(),
        retirement
    );
    assert!(TransactionMutation::RetireData {
        model: DataLogicalModel::Event,
        target: document,
        effective_at: 10,
    }
    .validate()
    .is_err());
    assert!(TransactionMutation::RetireData {
        model: DataLogicalModel::Vector,
        target: DataTarget::Event {
            kind: CanonicalId::new("observed").unwrap(),
            cursor: 7,
        },
        effective_at: 10,
    }
    .validate()
    .is_err());
    assert!(ReadDataSnapshot {
        valid_at: 10,
        max_storage_keys: 1,
    }
    .validate()
    .is_ok());
    assert!(ReadDataSnapshot {
        valid_at: 0,
        max_storage_keys: 0,
    }
    .validate()
    .is_err());
}

#[test]
pub(super) fn recursive_retrieval_contract_is_strict_bounded_and_multimodal() {
    let nearest = |using: &str, query: VectorSearchQuery| RetrievalQuery::Nearest {
        using: CanonicalId::new(using).unwrap(),
        query,
        filter: None,
        mode: VectorSearchMode::Exact,
    };
    let request = ExecuteRetrievalQuery {
        scope: "instance:alpha".into(),
        valid_at: 10,
        collection_id: CanonicalId::new("documents").unwrap(),
        query: RetrievalQuery::Fusion {
            prefetch: vec![
                RetrievalPrefetch {
                    query: Box::new(nearest(
                        "dense",
                        VectorSearchQuery::Dense {
                            values: vec![1.0, 0.0],
                        },
                    )),
                    limit: 4,
                },
                RetrievalPrefetch {
                    query: Box::new(nearest(
                        "sparse",
                        VectorSearchQuery::Sparse {
                            dimensions: 8,
                            indices: vec![1, 6],
                            values: vec![0.5, 0.75],
                        },
                    )),
                    limit: 4,
                },
            ],
            fusion: RetrievalFusion::ReciprocalRank {
                rank_constant: 60,
                weights_millionths: vec![1_000_000, 750_000],
            },
        },
        result_shape: RetrievalResultShape::Groups {
            property: CanonicalId::new("category").unwrap(),
            max_groups: 2,
            hits_per_group: 2,
        },
        limit: 2,
        candidate_limit: 4,
        max_storage_keys: 10_000,
    };
    request.validate().unwrap();
    let encoded = serde_json::to_value(&request).unwrap();
    assert_eq!(
        serde_json::from_value::<ExecuteRetrievalQuery>(encoded.clone()).unwrap(),
        request
    );
    let mut unknown = encoded;
    unknown
        .as_object_mut()
        .unwrap()
        .insert("middleware".into(), serde_json::json!(true));
    assert!(serde_json::from_value::<ExecuteRetrievalQuery>(unknown).is_err());

    let mut wrong_weights = request.clone();
    let RetrievalQuery::Fusion { fusion, .. } = &mut wrong_weights.query else {
        unreachable!()
    };
    let RetrievalFusion::ReciprocalRank {
        weights_millionths, ..
    } = fusion;
    weights_millionths.pop();
    assert!(wrong_weights.validate().is_err());

    let mut oversized_prefetch = request;
    let RetrievalQuery::Fusion { prefetch, .. } = &mut oversized_prefetch.query else {
        unreachable!()
    };
    prefetch[0].limit = 5;
    assert!(oversized_prefetch.validate().is_err());
}

#[test]
pub(super) fn public_embedding_contract_is_bounded_strict_and_collection_addressed() {
    let input = EmbeddingInput {
        id: CanonicalId::new("query-text").unwrap(),
        media_type: "text/plain".into(),
        bytes: b"source-grounded inference".to_vec(),
    };
    let batch = GenerateEmbeddings {
        scope: "instance:alpha".into(),
        backend_id: "rrflow:feature-hash:cpu:v1".into(),
        network_policy: EmbeddingNetworkPolicy::Deny,
        inputs: vec![input.clone()],
    };
    batch.validate().unwrap();
    let encoded = serde_json::to_value(&batch).unwrap();
    assert_eq!(
        serde_json::from_value::<GenerateEmbeddings>(encoded.clone()).unwrap(),
        batch
    );
    let mut unknown = encoded;
    unknown["provider_api_key"] = serde_json::json!("must-not-cross-contract");
    assert!(serde_json::from_value::<GenerateEmbeddings>(unknown).is_err());

    let request = EmbedAndSearchVectors {
        scope: "instance:alpha".into(),
        backend_id: batch.backend_id,
        network_policy: EmbeddingNetworkPolicy::Deny,
        input,
        valid_at: 10,
        collection_id: CanonicalId::new("documents").unwrap(),
        vector_name: CanonicalId::new("body").unwrap(),
        filter: None,
        top_k: 4,
        mode: VectorSearchMode::Exact,
        max_storage_keys: 10_000,
    };
    request.validate().unwrap();
    let mut invalid = request;
    invalid.input.bytes.clear();
    assert!(invalid.validate().is_err());
}
