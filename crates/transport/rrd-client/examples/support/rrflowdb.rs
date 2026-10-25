//! Application code shared by the runnable example and its real-server tests.
//! Every database effect goes through the public client; no storage is opened.
use rrd_client::{RequestOptions, RrdClient, Session};
use rrd_contract::{
    transaction_operation_sha256, AbortTransaction, BeginTransaction, CanonicalId, CommitReceipt,
    CommitTransaction, DataLogicalModel, DataPropertySchema, DataRecordSchema, DataReference,
    DataRelationSchema, DataSchemaMode, DataSchemaRegistry, DataTableSchema, DataValueType,
    ExecuteQuery, QueryBudget, QueryResult, QueryValue, ReadDataSchema, TransactionLease,
    TransactionMutation,
};
use std::collections::{BTreeMap, BTreeSet};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
pub const PROJECT: &str = "devpulse_project";
pub const TASK: &str = "devpulse_task";
pub const OWNS: &str = "devpulse_owns";

pub fn id(value: &str) -> CanonicalId {
    CanonicalId::new(value).expect("static example identifier")
}

pub fn reference(kind: &str, value: &str) -> DataReference {
    DataReference {
        kind: id(kind),
        id: id(value),
    }
}

pub fn options(key: &str) -> rrd_client::Result<RequestOptions> {
    RequestOptions::mutation(&format!("request-{key}"), &format!("operation-{key}"), key)
}

pub fn read_options(key: &str) -> rrd_client::Result<RequestOptions> {
    RequestOptions::read(&format!("request-{key}"), &format!("operation-{key}"))
}

pub async fn begin(client: &RrdClient, session: &Session, key: &str) -> Result<TransactionLease> {
    Ok(client
        .begin_transaction(
            session,
            BeginTransaction {
                scope: id("data"),
                timeout_ms: 30_000,
            },
            options(key)?,
        )
        .await?)
}

pub async fn commit(
    client: &RrdClient,
    session: &Session,
    transaction: &TransactionLease,
    mutations: Vec<TransactionMutation>,
    key: &str,
) -> Result<CommitReceipt> {
    let mut request_options = options(key)?;
    request_options.deadline_unix_ms = Some(transaction.expires_at_unix_ms);
    Ok(client
        .commit_transaction(
            session,
            &transaction.transaction_id,
            CommitTransaction {
                operation_sha256: transaction_operation_sha256(&mutations),
                mutations,
            },
            request_options,
        )
        .await?)
}

pub async fn abort(
    client: &RrdClient,
    session: &Session,
    transaction: &TransactionLease,
    key: &str,
) -> Result<()> {
    client
        .abort_transaction(
            session,
            &transaction.transaction_id,
            AbortTransaction {},
            options(key)?,
        )
        .await?;
    Ok(())
}

pub fn extend_schema(mut schema: DataSchemaRegistry) -> Result<DataSchemaRegistry> {
    if [PROJECT, TASK, OWNS]
        .iter()
        .any(|kind| schema.tables.contains_key(&id(kind)))
    {
        return Err("example tables already exist; use read or update instead of init".into());
    }
    schema.revision = schema
        .revision
        .checked_add(1)
        .ok_or("schema revision overflow")?;
    schema.migration = "add DevPulse local database example tables".into();
    for (kind, fields, unique) in [
        (
            PROJECT,
            vec!["slug", "name"],
            BTreeSet::from(["slug".into()]),
        ),
        (TASK, vec!["title", "status"], BTreeSet::new()),
    ] {
        schema.tables.insert(
            id(kind),
            DataTableSchema {
                model: DataLogicalModel::Relational,
                mode: DataSchemaMode::Strict,
                properties: BTreeMap::new(),
                allow_additional_properties: false,
            },
        );
        schema.records.insert(
            id(kind),
            DataRecordSchema {
                properties: fields
                    .into_iter()
                    .map(|field| {
                        (
                            field.into(),
                            DataPropertySchema {
                                value_type: DataValueType::String,
                                required: true,
                            },
                        )
                    })
                    .collect(),
                allow_additional_properties: false,
                unique_properties: unique,
            },
        );
    }
    schema.tables.insert(
        id(OWNS),
        DataTableSchema {
            model: DataLogicalModel::GraphRelation,
            mode: DataSchemaMode::Strict,
            properties: BTreeMap::new(),
            allow_additional_properties: false,
        },
    );
    schema.relations.insert(
        id(OWNS),
        DataRelationSchema {
            from: BTreeSet::from([id(PROJECT)]),
            to: BTreeSet::from([id(TASK)]),
            unique_pair: true,
            max_incoming: Some(1),
            ..Default::default()
        },
    );
    Ok(schema)
}

pub fn project_record(record_id: &str, slug: &str, at: u64) -> TransactionMutation {
    TransactionMutation::PutRecord {
        reference: reference(PROJECT, record_id),
        valid_from: at,
        valid_to: None,
        properties: BTreeMap::from([
            ("slug".into(), QueryValue::String(slug.into())),
            ("name".into(), QueryValue::String("Local AI project".into())),
        ]),
    }
}

pub fn task_record(title: &str, status: &str, at: u64) -> TransactionMutation {
    TransactionMutation::PutRecord {
        reference: reference(TASK, "first-task"),
        valid_from: at,
        valid_to: None,
        properties: BTreeMap::from([
            ("title".into(), QueryValue::String(title.into())),
            ("status".into(), QueryValue::String(status.into())),
        ]),
    }
}

pub fn seed_mutations(schema: DataSchemaRegistry, at: u64) -> Vec<TransactionMutation> {
    vec![
        TransactionMutation::PutSchema { registry: schema },
        project_record("local-project", "local-ai", at),
        task_record("Build with rrflowDB", "open", at),
        TransactionMutation::PutRelation {
            reference: reference(OWNS, "project-first-task"),
            from: reference(PROJECT, "local-project"),
            to: reference(TASK, "first-task"),
            valid_from: at,
            valid_to: None,
            properties: BTreeMap::new(),
        },
    ]
}

pub async fn initialize(
    client: &RrdClient,
    session: &Session,
    at: u64,
    key: &str,
) -> Result<CommitReceipt> {
    let transaction = begin(client, session, &format!("{key}-begin")).await?;
    let result = async {
        let snapshot = client
            .read_data_schema(
                session,
                ReadDataSchema {
                    scope: format!("instance:{}", client.instance_id()),
                    max_storage_keys: 4096,
                },
                read_options(&format!("{key}-schema"))?,
            )
            .await?;
        if snapshot.known_at_cursor != transaction.read_cursor {
            return Err(
                "database changed while planning schema; retry with a new operation key".into(),
            );
        }
        let schema = extend_schema(snapshot.schema.ok_or("installed schema is absent")?)?;
        commit(
            client,
            session,
            &transaction,
            seed_mutations(schema, at),
            &format!("{key}-commit"),
        )
        .await
    }
    .await;
    if result.is_err() {
        // Cleanup failure does not turn a failed initialization into success.
        let _ = abort(client, session, &transaction, &format!("{key}-abort")).await;
    }
    result
}

pub async fn query(
    client: &RrdClient,
    session: &Session,
    query: String,
    at: u64,
    key: &str,
) -> Result<QueryResult> {
    Ok(client
        .execute_query(
            session,
            ExecuteQuery {
                scope: format!("instance:{}", client.instance_id()),
                query,
                parameters: BTreeMap::from([
                    ("at".into(), QueryValue::Unsigned(at)),
                    ("project".into(), QueryValue::String("local-ai".into())),
                ]),
                budget: QueryBudget {
                    max_storage_keys: 4096,
                    max_rows: 64,
                    max_output_bytes: 65_536,
                    max_batch_rows: 64,
                    max_memory_bytes: 16 * 1024 * 1024,
                    max_spill_bytes: 16 * 1024 * 1024,
                    max_elapsed_ms: 10_000,
                },
            },
            read_options(key)?,
        )
        .await?)
}

pub async fn relationships(
    client: &RrdClient,
    session: &Session,
    at: u64,
    known: u64,
    key: &str,
) -> Result<QueryResult> {
    query(client, session, format!(
        "FROM record:{PROJECT} JOIN relation:{OWNS} ON id = from_id AT VALID $at KNOWN {known} WHERE left.slug = $project PROJECT left.name, right.to_id"
    ), at, key).await
}

pub async fn tasks(
    client: &RrdClient,
    session: &Session,
    at: u64,
    key: &str,
) -> Result<QueryResult> {
    query(
        client,
        session,
        format!("FROM record:{TASK} AT VALID $at KNOWN HEAD PROJECT id, title, status"),
        at,
        key,
    )
    .await
}

/// A bounded application view, not another engine snapshot or lifecycle authority.
#[derive(Clone, serde::Serialize)]
pub struct DatabaseView {
    pub tasks: QueryResult,
    pub relationships: QueryResult,
}

pub async fn read_view(
    client: &RrdClient,
    session: &Session,
    at: u64,
    key: &str,
) -> Result<DatabaseView> {
    let tasks = tasks(client, session, at, &format!("{key}-tasks")).await?;
    let relationships = relationships(
        client,
        session,
        at,
        tasks.known_at_cursor,
        &format!("{key}-join"),
    )
    .await?;
    let view = DatabaseView {
        tasks,
        relationships,
    };
    view.validate(&format!("instance:{}", client.instance_id()))?;
    Ok(view)
}

impl DatabaseView {
    /// Never present partial or differently stamped queries as one complete view.
    pub fn validate(&self, scope: &str) -> Result<()> {
        for result in [&self.tasks, &self.relationships] {
            result.execution.read_evidence.validate()?;
            if result.scope != scope
                || result.execution.truncated
                || result.execution.returned_rows != result.rows.len() as u64
            {
                return Err("database view is incomplete or outside the requested scope".into());
            }
        }
        if self.tasks.known_at_cursor != self.relationships.known_at_cursor
            || self.tasks.schema_revision != self.relationships.schema_revision
        {
            return Err("database view has inconsistent read stamps".into());
        }
        Ok(())
    }

    /// The caller supplies the expected state independently of the returned rows.
    pub fn verify(&self, scope: &str, expected_status: &str) -> Result<()> {
        self.validate(scope)?;
        let expected_task = BTreeMap::from([
            ("id".into(), QueryValue::String("first-task".into())),
            (
                "title".into(),
                QueryValue::String("Build with rrflowDB".into()),
            ),
            ("status".into(), QueryValue::String(expected_status.into())),
        ]);
        let expected_relationship = BTreeMap::from([
            (
                "left.name".into(),
                QueryValue::String("Local AI project".into()),
            ),
            (
                "right.to_id".into(),
                QueryValue::String("first-task".into()),
            ),
        ]);
        if self.tasks.rows.len() != 1 || self.tasks.rows[0].values != expected_task {
            return Err("database task does not match the expected example state".into());
        }
        if self.relationships.rows.len() != 1
            || self.relationships.rows[0].values != expected_relationship
        {
            return Err("database relationship does not match the expected example state".into());
        }
        Ok(())
    }
}
