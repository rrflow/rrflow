//! Frozen canonical identities of an organised schema registry (H-11a).

use rrd_core::{
    RuntimeCategorySchema, RuntimeCategoryTargets, RuntimeCommit, RuntimeDomainSchema,
    RuntimeLogicalModel, RuntimeMutation, RuntimeSchemaRegistry, RuntimeTableSchema,
    RuntimeTagCardinality, RuntimeTagRef, RuntimeTagSchema, RuntimeType, ScopeId,
};
use std::collections::{BTreeMap, BTreeSet};

fn kind(value: &str) -> RuntimeType {
    RuntimeType::new(value).unwrap()
}

fn organised(invoice_domain: &str) -> RuntimeSchemaRegistry {
    let mut registry = RuntimeSchemaRegistry::empty(2, "organise billing");
    for name in ["invoice", "customer", "rrflow-seat"] {
        registry.tables.insert(
            kind(name),
            RuntimeTableSchema::schemaless(RuntimeLogicalModel::Document),
        );
    }
    let organisation = &mut registry.organisation;
    organisation.domains = BTreeMap::from([
        (
            kind("billing"),
            RuntimeDomainSchema {
                title: "Billing".into(),
                description: "Invoices, payments and customers".into(),
                parent: None,
            },
        ),
        (
            kind("invoices"),
            RuntimeDomainSchema {
                title: "Invoices".into(),
                description: String::new(),
                parent: Some(kind("billing")),
            },
        ),
    ]);
    organisation.categories = BTreeMap::from([(
        kind("lifecycle"),
        RuntimeCategorySchema {
            title: "Lifecycle".into(),
            description: String::new(),
            targets: RuntimeCategoryTargets::KindsAndRecords,
            cardinality: RuntimeTagCardinality::Single,
            open: false,
            tags: BTreeMap::from([(
                kind("active"),
                RuntimeTagSchema {
                    title: "Active".into(),
                    description: String::new(),
                },
            )]),
        },
    )]);
    organisation.kind_domains = BTreeMap::from([
        (kind("invoice"), kind(invoice_domain)),
        (kind("customer"), kind("billing")),
    ]);
    organisation.collection_domains = BTreeMap::from([(kind("invoice-text"), kind("invoices"))]);
    organisation.kind_tags = BTreeMap::from([(
        kind("invoice"),
        BTreeSet::from([RuntimeTagRef {
            category: kind("lifecycle"),
            tag: kind("active"),
        }]),
    )]);
    registry.validate().unwrap();
    registry
}

fn commit_digest(registry: RuntimeSchemaRegistry) -> String {
    let commit = RuntimeCommit {
        scope: ScopeId::new("instance:golden").unwrap(),
        at: 1_100,
        actor: "agent:golden".into(),
        expected_cursor: 7,
        mutations: vec![RuntimeMutation::Schema { registry }],
    };
    commit.validate().unwrap();
    commit.digest()
}

fn contract() -> serde_json::Value {
    let registry = organised("invoices");
    serde_json::json!({
        "comment": "H-11a organised registry; changes require an explicit wire-version decision",
        "registry": registry,
        "commit_digest": commit_digest(registry.clone()),
    })
}

#[test]
fn organised_registry_matches_checked_in_vector() {
    let actual = contract();
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/fixtures/engram-organisation-v1.json"
    );
    if std::env::var_os("GOLDEN_WRITE").is_some() {
        std::fs::write(
            path,
            format!("{}\n", serde_json::to_string_pretty(&actual).unwrap()),
        )
        .unwrap();
        return;
    }
    let expected: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn organisation_is_part_of_the_commit_identity() {
    let invoices = commit_digest(organised("invoices"));
    assert_eq!(invoices, commit_digest(organised("invoices")));
    assert_ne!(invoices, commit_digest(organised("billing")));

    let mut unorganised = organised("invoices");
    unorganised.organisation = Default::default();
    assert_ne!(invoices, commit_digest(unorganised));
}
