//! H-11a engram organisation through the public engine operations.

use super::*;
use std::collections::BTreeMap;

use rrd_contract::{
    ChangeMutationSnapshot, DataCategorySchema, DataCategoryTargets, DataDomainSchema,
    DataLogicalModel, DataReference, DataSchemaMode, DataSchemaRegistry, DataTableSchema,
    DataTagCardinality, DataTagReference, DataTagSchema, ReadChangefeed, ReadDataSchema,
    ReadDataSnapshot,
};

fn canonical(value: &str) -> CanonicalId {
    CanonicalId::new(value).unwrap()
}

fn scope() -> String {
    format!("instance:{}", instance().as_str())
}

fn domain(title: &str, parent: Option<&str>) -> DataDomainSchema {
    DataDomainSchema {
        title: title.into(),
        description: String::new(),
        parent: parent.map(canonical),
    }
}

fn document_table() -> DataTableSchema {
    DataTableSchema {
        model: DataLogicalModel::Document,
        mode: DataSchemaMode::Schemaless,
        properties: BTreeMap::new(),
        allow_additional_properties: false,
    }
}

/// Revision 1 places `invoice` in `billing`; revision 2 moves it to the
/// nested `invoices` domain and tags the kind.
fn registry(revision: u64) -> DataSchemaRegistry {
    let mut registry = DataSchemaRegistry {
        revision,
        migration: format!("organise billing, revision {revision}"),
        organisation: DataEngramOrganisation::default(),
        tables: BTreeMap::from([
            (canonical("invoice"), document_table()),
            (canonical("customer"), document_table()),
        ]),
        records: BTreeMap::new(),
        relations: BTreeMap::new(),
        events: BTreeMap::new(),
    };
    let organisation = &mut registry.organisation;
    organisation.domains = BTreeMap::from([
        (canonical("billing"), domain("Billing", None)),
        (canonical("invoices"), domain("Invoices", Some("billing"))),
    ]);
    if revision == 1 {
        organisation.kind_domains = BTreeMap::from([(canonical("invoice"), canonical("billing"))]);
    } else {
        organisation.kind_domains = BTreeMap::from([(canonical("invoice"), canonical("invoices"))]);
        organisation.categories = BTreeMap::from([(
            canonical("lifecycle"),
            DataCategorySchema {
                title: "Lifecycle".into(),
                description: String::new(),
                targets: DataCategoryTargets::KindsAndRecords,
                cardinality: DataTagCardinality::Single,
                open: false,
                tags: BTreeMap::from([(
                    canonical("active"),
                    DataTagSchema {
                        title: "Active".into(),
                        description: String::new(),
                    },
                )]),
            },
        )]);
        organisation.kind_tags = BTreeMap::from([(
            canonical("invoice"),
            [DataTagReference {
                category: canonical("lifecycle"),
                tag: canonical("active"),
            }]
            .into(),
        )]);
    }
    registry
}

fn commit(
    engine: &RrdEngine,
    lease: &rrd_contract::SessionLease,
    mutations: Vec<TransactionMutation>,
    at: u64,
    suffix: &str,
) -> super::super::Result<rrd_contract::CommitReceipt> {
    let transaction = engine.begin_transaction(
        &lease.session_id,
        &lease.token,
        &BeginTransaction {
            scope: canonical("data"),
            timeout_ms: 1_000,
        },
        &mutation_context(
            &id(&format!("organisation-begin-{suffix}")),
            &format!("request-organisation-begin-{suffix}"),
            &format!("operation-organisation-begin-{suffix}"),
        ),
        at,
    )?;
    let request = CommitTransaction {
        operation_sha256: rrd_contract::transaction_operation_sha256(&mutations),
        mutations,
    };
    engine.commit_transaction(
        &lease.session_id,
        &lease.token,
        &transaction.transaction_id,
        &id(&format!("organisation-commit-{suffix}")),
        &request,
        at,
        &format!("request-organisation-commit-{suffix}"),
        &format!("operation-organisation-commit-{suffix}"),
    )
}

fn session(engine: &RrdEngine, suffix: &str) -> rrd_contract::SessionLease {
    engine
        .create_session(
            &session_request(5_000, 4),
            &id(&format!("organisation-session-{suffix}")),
            1_000,
            &format!("request-organisation-session-{suffix}"),
            &format!("operation-organisation-session-{suffix}"),
        )
        .unwrap()
}

fn current_schema(
    engine: &RrdEngine,
    lease: &rrd_contract::SessionLease,
    suffix: &str,
) -> DataSchemaRegistry {
    let snapshot = engine
        .read_data_schema(
            &lease.session_id,
            &lease.token,
            &ReadDataSchema {
                scope: scope(),
                max_storage_keys: 64,
            },
            1_500,
            &format!("request-organisation-schema-{suffix}"),
            &format!("operation-organisation-schema-{suffix}"),
        )
        .unwrap();
    snapshot.validate().unwrap();
    snapshot.schema.unwrap()
}

#[test]
fn organisation_commits_moves_reads_back_and_survives_reopen() {
    let (root, engine) = isolated_engine();
    let lease = session(&engine, "first");
    let invoice = DataReference {
        kind: canonical("invoice"),
        id: canonical("alpha"),
    };
    commit(
        &engine,
        &lease,
        vec![
            TransactionMutation::PutSchema {
                registry: registry(1),
            },
            TransactionMutation::PutRecord {
                reference: invoice.clone(),
                valid_from: 1_100,
                valid_to: None,
                properties: BTreeMap::new(),
            },
        ],
        1_100,
        "install",
    )
    .unwrap();
    let installed = current_schema(&engine, &lease, "installed");
    assert_eq!(installed, registry(1));
    assert_eq!(
        installed.organisation.domain_of_kind(&canonical("invoice")),
        "billing"
    );
    assert_eq!(
        installed
            .organisation
            .domain_of_kind(&canonical("customer")),
        "default"
    );

    // Moving the kind is one schema revision; no record mutation is written.
    commit(
        &engine,
        &lease,
        vec![TransactionMutation::PutSchema {
            registry: registry(2),
        }],
        1_200,
        "move",
    )
    .unwrap();
    let moved = current_schema(&engine, &lease, "moved");
    assert_eq!(moved, registry(2));
    assert_eq!(
        moved.organisation.domain_of_kind(&canonical("invoice")),
        "invoices"
    );

    // Each committed registry, and so each organisation, stays readable at
    // its own cursor.
    let changes = engine
        .read_changefeed(
            &lease.session_id,
            &lease.token,
            &ReadChangefeed {
                scope: scope(),
                after_cursor: 0,
                limit: 16,
            },
            1_300,
            "request-organisation-changes",
            "operation-organisation-changes",
        )
        .unwrap();
    let schemas = changes
        .changes
        .iter()
        .filter_map(|change| match &change.mutation {
            ChangeMutationSnapshot::Data {
                mutation: TransactionMutation::PutSchema { registry },
            } => Some(registry.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(schemas, vec![registry(1), registry(2)]);
    let records = changes
        .changes
        .iter()
        .filter(|change| {
            matches!(
                &change.mutation,
                ChangeMutationSnapshot::Data {
                    mutation: TransactionMutation::PutRecord { .. }
                }
            )
        })
        .count();
    assert_eq!(records, 1, "moving a kind rewrites no record");

    let snapshot = engine
        .read_data_snapshot(
            &lease.session_id,
            &lease.token,
            &ReadDataSnapshot {
                valid_at: 1_250,
                max_storage_keys: 64,
            },
            1_300,
            "request-organisation-snapshot",
            "operation-organisation-snapshot",
        )
        .unwrap();
    assert_eq!(snapshot.schema_revision, 2);
    assert!(snapshot.entries.iter().any(|entry| entry.target
        == rrd_contract::DataTarget::Reference {
            reference: invoice.clone()
        }));

    drop(engine);
    let reopened = RrdEngine::open(root.path(), instance(), TOKEN_KEY).unwrap();
    let lease = session(&reopened, "reopened");
    assert_eq!(current_schema(&reopened, &lease, "reopened"), registry(2));
}

#[test]
fn an_invalid_organisation_commits_nothing() {
    let (_root, engine) = isolated_engine();
    let lease = session(&engine, "invalid");
    commit(
        &engine,
        &lease,
        vec![TransactionMutation::PutSchema {
            registry: registry(1),
        }],
        1_100,
        "valid",
    )
    .unwrap();
    let mut unknown = registry(2);
    unknown
        .organisation
        .kind_domains
        .insert(canonical("customer"), canonical("shipping"));
    assert!(commit(
        &engine,
        &lease,
        vec![TransactionMutation::PutSchema { registry: unknown }],
        1_200,
        "unknown-domain",
    )
    .is_err());
    assert_eq!(current_schema(&engine, &lease, "unchanged"), registry(1));
}

/// One named way to break a valid organisation.
type Breakage = (&'static str, Box<dyn Fn(&mut DataSchemaRegistry)>);

#[test]
fn public_and_kernel_validators_reject_the_same_organisations() {
    let cases: Vec<Breakage> = vec![
        (
            "built-in domain",
            Box::new(|r| {
                r.organisation
                    .domains
                    .insert(canonical("default"), domain("Default", None));
            }),
        ),
        (
            "reserved domain",
            Box::new(|r| {
                r.organisation
                    .domains
                    .insert(canonical("rrflow-private"), domain("Private", None));
            }),
        ),
        (
            "parent rrflow",
            Box::new(|r| {
                r.organisation
                    .domains
                    .insert(canonical("inner"), domain("Inner", Some("rrflow")));
            }),
        ),
        (
            "unknown parent",
            Box::new(|r| {
                r.organisation
                    .domains
                    .insert(canonical("inner"), domain("Inner", Some("missing")));
            }),
        ),
        (
            "cycle",
            Box::new(|r| {
                r.organisation
                    .domains
                    .insert(canonical("billing"), domain("Billing", Some("invoices")));
            }),
        ),
        (
            "empty title",
            Box::new(|r| {
                r.organisation
                    .domains
                    .insert(canonical("blank"), domain(" ", None));
            }),
        ),
        (
            "unknown member domain",
            Box::new(|r| {
                r.organisation
                    .kind_domains
                    .insert(canonical("customer"), canonical("shipping"));
            }),
        ),
        (
            "redundant default",
            Box::new(|r| {
                r.organisation
                    .kind_domains
                    .insert(canonical("customer"), canonical("default"));
            }),
        ),
        (
            "unregistered kind",
            Box::new(|r| {
                r.organisation
                    .kind_domains
                    .insert(canonical("supplier"), canonical("billing"));
            }),
        ),
        (
            "kind placed in rrflow",
            Box::new(|r| {
                r.organisation
                    .kind_domains
                    .insert(canonical("customer"), canonical("rrflow"));
            }),
        ),
        (
            "engine-owned collection moved",
            Box::new(|r| {
                r.organisation
                    .collection_domains
                    .insert(canonical("rrflow-public-embedding"), canonical("billing"));
            }),
        ),
        (
            "unknown tag",
            Box::new(|r| {
                r.organisation.kind_tags.insert(
                    canonical("customer"),
                    [DataTagReference {
                        category: canonical("lifecycle"),
                        tag: canonical("retired"),
                    }]
                    .into(),
                );
            }),
        ),
        (
            "empty kind tags",
            Box::new(|r| {
                r.organisation
                    .kind_tags
                    .insert(canonical("customer"), Default::default());
            }),
        ),
    ];
    let valid = registry(2);
    TransactionMutation::PutSchema {
        registry: valid.clone(),
    }
    .validate()
    .unwrap();
    super::super::runtime_schema(&valid)
        .unwrap()
        .validate()
        .unwrap();
    for (name, apply) in cases {
        let mut registry = valid.clone();
        apply(&mut registry);
        assert!(
            TransactionMutation::PutSchema {
                registry: registry.clone()
            }
            .validate()
            .is_err(),
            "public validator accepted {name}"
        );
        assert!(
            super::super::runtime_schema(&registry)
                .and_then(|runtime| runtime
                    .validate()
                    .map_err(|error| ServiceError::Contract(error.to_string())))
                .is_err(),
            "kernel validator accepted {name}"
        );
    }
}
