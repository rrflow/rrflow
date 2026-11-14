use super::*;
use crate::{RuntimeId, RuntimeRef};

fn reference(kind: &str, id: &str) -> RuntimeRef {
    RuntimeRef {
        kind: RuntimeType::new(kind).unwrap(),
        id: RuntimeId::new(id).unwrap(),
    }
}

#[test]
fn canonical_table_map_is_required_and_never_inferred() {
    let prompt = RuntimeType::new("prompt").unwrap();
    let mut registry = RuntimeSchemaRegistry::empty(1, "reject missing table authority");
    registry
        .records
        .insert(prompt, RuntimeRecordSchema::default());

    assert!(registry
        .validate()
        .unwrap_err()
        .to_string()
        .contains("table"));
}

#[test]
fn serialized_schema_cannot_omit_the_canonical_table_map() {
    let serialized = serde_json::json!({
        "revision": 1,
        "migration": "reject omitted table authority",
        "records": {
            "prompt": {}
        },
        "relations": {},
        "events": {}
    });

    assert!(serde_json::from_value::<RuntimeSchemaRegistry>(serialized).is_err());
}

#[test]
fn paired_table_authoring_rejects_cross_family_reclassification() {
    let kind = RuntimeType::new("shared").unwrap();
    let mut registry = RuntimeSchemaRegistry::empty(1, "reject family reclassification");
    registry
        .define_record_table(
            kind.clone(),
            RuntimeLogicalModel::Relational,
            RuntimeRecordSchema::default(),
        )
        .unwrap();

    assert!(registry
        .define_relation_table(kind.clone(), RuntimeRelationSchema::default())
        .is_err());
    assert!(registry
        .define_event_table(
            kind,
            RuntimeLogicalModel::Event,
            RuntimeEventSchema::default(),
        )
        .is_err());
    assert!(registry
        .define_record_table(
            RuntimeType::new("vector").unwrap(),
            RuntimeLogicalModel::Vector,
            RuntimeRecordSchema::default(),
        )
        .is_err());
}

#[test]
fn required_properties_and_endpoint_types_fail_closed() {
    let mut registry = RuntimeSchemaRegistry::empty(1, "bootstrap");
    let prompt = RuntimeType::new("prompt").unwrap();
    registry
        .define_record_table(
            prompt.clone(),
            RuntimeLogicalModel::Relational,
            RuntimeRecordSchema {
                properties: BTreeMap::from([(
                    "text".into(),
                    RuntimePropertySchema::required(RuntimeValueType::String),
                )]),
                ..RuntimeRecordSchema::default()
            },
        )
        .unwrap();
    registry
        .define_relation_table(
            RuntimeType::new("caused").unwrap(),
            RuntimeRelationSchema {
                from: BTreeSet::from([prompt.clone()]),
                to: BTreeSet::from([prompt]),
                unique_pair: true,
                ..RuntimeRelationSchema::default()
            },
        )
        .unwrap();
    let record = RuntimeRecord {
        reference: reference("prompt", "p1"),
        valid_from: 1,
        valid_to: None,
        properties: RuntimeProperties::new(),
    };
    assert!(registry
        .validate_objects(
            [&RuntimeMutation::Record { record }],
            std::iter::empty(),
            std::iter::empty()
        )
        .unwrap_err()
        .to_string()
        .contains("missing required property"));
}

#[test]
fn cardinality_is_measured_across_temporal_overlap() {
    let mut registry = RuntimeSchemaRegistry::empty(1, "bootstrap");
    let prompt = RuntimeType::new("prompt").unwrap();
    registry
        .define_record_table(
            prompt.clone(),
            RuntimeLogicalModel::Relational,
            RuntimeRecordSchema::default(),
        )
        .unwrap();
    registry
        .define_relation_table(
            RuntimeType::new("next").unwrap(),
            RuntimeRelationSchema {
                from: BTreeSet::from([prompt.clone()]),
                to: BTreeSet::from([prompt]),
                max_outgoing: Some(1),
                ..RuntimeRelationSchema::default()
            },
        )
        .unwrap();
    let relation = |id: &str, to: &str, from: u64, until: Option<u64>| RuntimeRelation {
        reference: reference("next", id),
        from: reference("prompt", "p1"),
        to: reference("prompt", to),
        valid_from: from,
        valid_to: until,
        properties: RuntimeProperties::new(),
    };
    let first = relation("a", "p2", 1, Some(5));
    let non_overlapping = relation("b", "p3", 5, None);
    registry
        .validate_objects(
            std::iter::empty(),
            std::iter::empty(),
            [&first, &non_overlapping],
        )
        .unwrap();
    let overlapping = relation("c", "p4", 4, None);
    assert!(registry
        .validate_objects(
            std::iter::empty(),
            std::iter::empty(),
            [&first, &overlapping]
        )
        .is_err());
}

#[test]
fn one_catalogue_names_every_logical_model_and_schema_mode() {
    let mut registry = RuntimeSchemaRegistry::empty(1, "freeze unified catalogue");
    let models = [
        ("documents", RuntimeLogicalModel::Document),
        ("rows", RuntimeLogicalModel::Relational),
        ("nodes", RuntimeLogicalModel::GraphNode),
        ("edges", RuntimeLogicalModel::GraphRelation),
        ("entries", RuntimeLogicalModel::KeyValue),
        ("vectors", RuntimeLogicalModel::Vector),
        ("events", RuntimeLogicalModel::Event),
        ("samples", RuntimeLogicalModel::TimeSeries),
        ("places", RuntimeLogicalModel::Geo),
        ("objects", RuntimeLogicalModel::Object),
        ("claims", RuntimeLogicalModel::ReasoningClaim),
        ("reasoning", RuntimeLogicalModel::ReasoningRecord),
        ("reasoning_events", RuntimeLogicalModel::ReasoningEvent),
        ("lifecycles", RuntimeLogicalModel::LifecycleRecord),
        ("lifecycle_events", RuntimeLogicalModel::LifecycleEvent),
    ];
    for (kind, model) in models {
        registry.tables.insert(
            RuntimeType::new(kind).unwrap(),
            RuntimeTableSchema::schemaless(model),
        );
    }
    registry.validate().unwrap();
    assert_eq!(registry.catalogue_tables().unwrap().len(), models.len());
    assert_eq!(
        registry.tables[&RuntimeType::new("documents").unwrap()].mode,
        RuntimeSchemaMode::Schemaless
    );
}

#[test]
fn strict_and_schemaless_or_conflicting_model_families_cannot_be_confused() {
    let prompt = RuntimeType::new("prompt").unwrap();
    let mut registry = RuntimeSchemaRegistry::empty(1, "reject ambiguous schema");
    registry
        .records
        .insert(prompt.clone(), RuntimeRecordSchema::default());
    registry.tables.insert(
        prompt.clone(),
        RuntimeTableSchema::schemaless(RuntimeLogicalModel::Document),
    );
    assert!(registry
        .validate()
        .unwrap_err()
        .to_string()
        .contains("schemaless"));

    registry.tables.insert(
        prompt,
        RuntimeTableSchema::strict(RuntimeLogicalModel::GraphRelation),
    );
    assert!(registry
        .validate()
        .unwrap_err()
        .to_string()
        .contains("not Record"));
}

fn kind(value: &str) -> RuntimeType {
    RuntimeType::new(value).unwrap()
}

fn domain(title: &str, parent: Option<&str>) -> RuntimeDomainSchema {
    RuntimeDomainSchema {
        title: title.into(),
        description: String::new(),
        parent: parent.map(kind),
    }
}

fn component_category(cardinality: RuntimeTagCardinality) -> RuntimeCategorySchema {
    RuntimeCategorySchema {
        title: "Component".into(),
        description: String::new(),
        targets: RuntimeCategoryTargets::KindsAndRecords,
        cardinality,
        open: false,
        tags: BTreeMap::from([
            (
                kind("storage"),
                RuntimeTagSchema {
                    title: "Storage".into(),
                    description: String::new(),
                },
            ),
            (
                kind("query"),
                RuntimeTagSchema {
                    title: "Query".into(),
                    description: String::new(),
                },
            ),
        ]),
    }
}

fn tag(category: &str, value: &str) -> RuntimeTagRef {
    RuntimeTagRef {
        category: kind(category),
        tag: kind(value),
    }
}

/// Two project kinds and one engine-owned kind, organised into a small tree.
fn organised() -> RuntimeSchemaRegistry {
    let mut registry = RuntimeSchemaRegistry::empty(1, "organise the estate");
    for name in ["invoice", "customer", "rrflow-seat"] {
        registry.tables.insert(
            kind(name),
            RuntimeTableSchema::schemaless(RuntimeLogicalModel::Document),
        );
    }
    registry.organisation.domains = BTreeMap::from([
        (kind("billing"), domain("Billing", None)),
        (kind("invoices"), domain("Invoices", Some("billing"))),
    ]);
    registry.organisation.kind_domains = BTreeMap::from([(kind("invoice"), kind("invoices"))]);
    registry.organisation.categories = BTreeMap::from([(
        kind("component"),
        component_category(RuntimeTagCardinality::Single),
    )]);
    registry.organisation.kind_tags = BTreeMap::from([(
        kind("invoice"),
        BTreeSet::from([tag("component", "storage")]),
    )]);
    registry
}

fn rejection(registry: &RuntimeSchemaRegistry) -> String {
    registry.validate().unwrap_err().to_string()
}

#[test]
fn every_kind_and_collection_belongs_to_exactly_one_domain() {
    let registry = organised();
    registry.validate().unwrap();
    let organisation = &registry.organisation;
    assert_eq!(organisation.domain_of_kind(&kind("invoice")), "invoices");
    assert_eq!(
        organisation.domain_of_kind(&kind("customer")),
        DEFAULT_DOMAIN
    );
    assert_eq!(
        organisation.domain_of_kind(&kind("rrflow-seat")),
        ENGINE_DOMAIN
    );
    assert_eq!(
        organisation.domain_of_collection("documents"),
        DEFAULT_DOMAIN
    );
    assert_eq!(
        organisation.domain_of_collection("rrflow-public-embedding"),
        ENGINE_DOMAIN
    );
    assert!(organisation.has_domain("billing"));
    assert!(organisation.has_domain(DEFAULT_DOMAIN));
    assert!(!organisation.has_domain("shipping"));
}

#[test]
fn domains_reject_unknown_parents_cycles_and_excess_depth() {
    let mut registry = organised();
    registry
        .organisation
        .domains
        .insert(kind("orphan"), domain("Orphan", Some("missing")));
    assert!(rejection(&registry).contains("unknown parent"));

    let mut registry = organised();
    registry
        .organisation
        .domains
        .insert(kind("billing"), domain("Billing", Some("invoices")));
    assert!(rejection(&registry).contains("cycle") || rejection(&registry).contains("ancestor"));

    let mut registry = organised();
    let mut parent = None;
    for level in 0..=MAX_DOMAIN_DEPTH {
        let id = format!("level-{level}");
        registry
            .organisation
            .domains
            .insert(kind(&id), domain("Level", parent.as_deref()));
        parent = Some(id);
    }
    assert!(rejection(&registry).contains("deeper than"));

    let mut registry = organised();
    for index in 0..MAX_DOMAINS {
        registry
            .organisation
            .domains
            .insert(kind(&format!("domain-{index}")), domain("Many", None));
    }
    assert!(rejection(&registry).contains("at most"));
}

#[test]
fn built_in_and_engine_domains_cannot_be_redefined_or_borrowed() {
    for reserved in [DEFAULT_DOMAIN, ENGINE_DOMAIN, "rrflow-private"] {
        let mut registry = organised();
        registry
            .organisation
            .domains
            .insert(kind(reserved), domain("Reserved", None));
        assert!(rejection(&registry).contains("reserved"), "{reserved}");
    }

    let mut registry = organised();
    registry
        .organisation
        .domains
        .insert(kind("inside-engine"), domain("Inside", Some(ENGINE_DOMAIN)));
    assert!(rejection(&registry).contains("inside rrflow"));

    let mut registry = organised();
    registry
        .organisation
        .kind_domains
        .insert(kind("rrflow-seat"), kind("billing"));
    assert!(rejection(&registry).contains("always belongs to rrflow"));

    let mut registry = organised();
    registry
        .organisation
        .kind_domains
        .insert(kind("customer"), kind(ENGINE_DOMAIN));
    assert!(rejection(&registry).contains("cannot be placed in rrflow"));

    let mut registry = organised();
    registry
        .organisation
        .collection_domains
        .insert(kind("rrflow-public-embedding"), kind("billing"));
    assert!(rejection(&registry).contains("always belongs to rrflow"));
}

#[test]
fn memberships_name_registered_kinds_known_domains_and_no_redundant_default() {
    let mut registry = organised();
    registry
        .organisation
        .kind_domains
        .insert(kind("unregistered"), kind("billing"));
    assert!(rejection(&registry).contains("unregistered table type"));

    let mut registry = organised();
    registry
        .organisation
        .kind_domains
        .insert(kind("customer"), kind("shipping"));
    assert!(rejection(&registry).contains("unknown domain"));

    let mut registry = organised();
    registry
        .organisation
        .kind_domains
        .insert(kind("customer"), kind(DEFAULT_DOMAIN));
    assert!(rejection(&registry).contains("omit the entry"));

    let mut registry = organised();
    registry
        .organisation
        .collection_domains
        .insert(kind("documents"), kind("billing"));
    registry.validate().unwrap();
    assert_eq!(
        registry.organisation.domain_of_collection("documents"),
        "billing"
    );
}

#[test]
fn kind_tags_follow_their_category_vocabulary_targets_and_cardinality() {
    let mut registry = organised();
    registry.organisation.kind_tags.insert(
        kind("customer"),
        BTreeSet::from([tag("component", "storage"), tag("component", "query")]),
    );
    assert!(rejection(&registry).contains("single-tag category"));

    registry.organisation.categories.insert(
        kind("component"),
        component_category(RuntimeTagCardinality::Multiple),
    );
    registry.validate().unwrap();

    let mut registry = organised();
    registry.organisation.kind_tags.insert(
        kind("customer"),
        BTreeSet::from([tag("component", "billing")]),
    );
    assert!(rejection(&registry).contains("unknown tag"));

    let mut registry = organised();
    registry.organisation.kind_tags.insert(
        kind("customer"),
        BTreeSet::from([tag("lifecycle", "draft")]),
    );
    assert!(rejection(&registry).contains("unknown category"));

    let mut registry = organised();
    let mut records_only = component_category(RuntimeTagCardinality::Single);
    records_only.targets = RuntimeCategoryTargets::Records;
    registry
        .organisation
        .categories
        .insert(kind("component"), records_only);
    assert!(rejection(&registry).contains("does not tag table types"));

    let mut registry = organised();
    registry
        .organisation
        .kind_tags
        .insert(kind("customer"), BTreeSet::new());
    assert!(rejection(&registry).contains("empty tag set"));
}

#[test]
fn organisation_labels_are_bounded_titled_text() {
    let mut registry = organised();
    registry
        .organisation
        .domains
        .insert(kind("untitled"), domain(" ", None));
    assert!(rejection(&registry).contains("needs a title"));

    let mut registry = organised();
    registry.organisation.domains.insert(
        kind("long"),
        domain(&"x".repeat(MAX_ORGANISATION_TITLE_BYTES + 1), None),
    );
    assert!(rejection(&registry).contains("needs a title"));

    let mut registry = organised();
    let mut described = domain("Described", None);
    described.description = "x".repeat(MAX_ORGANISATION_DESCRIPTION_BYTES + 1);
    registry
        .organisation
        .domains
        .insert(kind("described"), described);
    assert!(rejection(&registry).contains("description exceeds"));

    let mut registry = organised();
    registry
        .organisation
        .domains
        .insert(kind("control"), domain("Line\nbreak", None));
    assert!(rejection(&registry).contains("control characters"));
}

#[test]
fn default_organisation_is_omitted_from_the_persisted_registry() {
    let mut registry = organised();
    registry.organisation = RuntimeEstateOrganisation::default();
    let json = serde_json::to_value(&registry).unwrap();
    assert!(json.get("organisation").is_none());
    let organised = serde_json::to_value(organised()).unwrap();
    assert_eq!(
        organised["organisation"]["kind_domains"]["invoice"],
        serde_json::json!("invoices")
    );
}
