//! Live schema checks, catalog capture, revisions and SQL helpers.

use super::*;

#[derive(Serialize)]
pub(super) struct CatalogEvidence {
    pub(super) extension_version: String,
    pub(super) relation_oid: u32,
    pub(super) columns: Vec<(i16, String, String)>,
    pub(super) indexes: Vec<IndexEvidence>,
}

pub(super) enum LiveSyncOperation {
    Upsert(PgvectorSyncPayload),
    Delete,
}

#[derive(Serialize)]
pub(super) struct IndexEvidence {
    pub(super) name: String,
    pub(super) access_method: String,
    pub(super) definition: String,
}

pub(super) fn descriptor() -> OperatorAdapterDescriptor {
    let all = BTreeSet::from([
        ScoreMetric::Cosine,
        ScoreMetric::Dot,
        ScoreMetric::Euclidean,
        ScoreMetric::Manhattan,
    ]);
    OperatorAdapterDescriptor {
        contract_version: OPERATOR_KNOWLEDGE_CONTRACT_VERSION,
        adapter: "pgvector".into(),
        implementation_digest: digest::sha256_hex(PGVECTOR_LIVE_IMPLEMENTATION_VERSION.as_bytes()),
        max_dimensions: 2_000,
        vector_kinds: BTreeSet::from([OperatorVectorKind::Dense]),
        search_capabilities: BTreeMap::from([
            (OperatorAccessPath::Exact, all.clone()),
            (OperatorAccessPath::Hnsw, all),
            (
                OperatorAccessPath::IvfFlat,
                BTreeSet::from([
                    ScoreMetric::Cosine,
                    ScoreMetric::Dot,
                    ScoreMetric::Euclidean,
                ]),
            ),
        ]),
        supports_tenant_filter: true,
        supports_payload_filter: false,
        supports_stable_revision: true,
    }
}

pub(super) fn verify_live_schema<C: GenericClient>(
    client: &mut C,
    deployment: &PgvectorDeployment,
    binding: &OperatorKnowledgeBinding,
) -> Result<()> {
    verify_source_identity(client, deployment, binding)?;
    let catalog = capture_catalog(client, deployment)?;
    let vector_type = catalog
        .columns
        .iter()
        .find(|(_, name, _)| name == &deployment.relation.vector_column)
        .map(|(_, _, kind)| kind.as_str());
    let cursor_type = catalog
        .columns
        .iter()
        .find(|(_, name, _)| name == &deployment.source_cursor_column)
        .map(|(_, _, kind)| kind.as_str());
    let required = [
        &deployment.relation.id_column,
        &deployment.relation.subject_column,
        &deployment.relation.tenant_column,
        &deployment.model_digest_column,
    ];
    if !matches!(vector_type, Some(kind) if kind.starts_with("vector("))
        || cursor_type != Some("bigint")
        || required
            .iter()
            .any(|name| !catalog.columns.iter().any(|(_, column, _)| column == *name))
    {
        return invalid(
            "pgvector live relation is missing its bound vector/model/tenant/cursor shape",
        );
    }
    let revision_sql = format!(
        "SELECT stable_revision FROM {} WHERE project_digest = $1",
        qualified(&deployment.control_schema, &deployment.revision_table)
    );
    if client
        .query_opt(&revision_sql, &[&project_digest(binding)])
        .map_err(|error| pg_error("verify pgvector project revision", error))?
        .is_none()
    {
        return invalid("pgvector project revision is not installed");
    }
    Ok(())
}

pub(super) fn verify_source_identity<C: GenericClient>(
    client: &mut C,
    deployment: &PgvectorDeployment,
    binding: &OperatorKnowledgeBinding,
) -> Result<()> {
    let sql = format!(
        "SELECT contract_version, source_identity_digest FROM {} WHERE singleton = true",
        qualified(&deployment.control_schema, &deployment.metadata_table)
    );
    let row = client
        .query_opt(&sql, &[])
        .map_err(|error| pg_error("read pgvector source identity", error))?
        .ok_or_else(|| Error::InvalidRuntime {
            reason: "pgvector source identity is not installed".into(),
        })?;
    let version: i32 = row.get(0);
    let source: String = row.get(1);
    if version != i32::from(OPERATOR_KNOWLEDGE_CONTRACT_VERSION)
        || source != binding.source_identity_digest
    {
        return invalid("pgvector live source identity differs from the project binding");
    }
    Ok(())
}

pub(super) fn capture_catalog<C: GenericClient>(
    client: &mut C,
    deployment: &PgvectorDeployment,
) -> Result<CatalogEvidence> {
    let extension_version: String = client
        .query_opt(
            "SELECT extversion FROM pg_extension WHERE extname = 'vector'",
            &[],
        )
        .map_err(|error| pg_error("read pgvector extension version", error))?
        .ok_or_else(|| Error::InvalidRuntime {
            reason: "pgvector extension is not installed".into(),
        })?
        .get(0);
    require_pgvector_08(&extension_version)?;
    let relation = client
        .query_opt(
            "SELECT c.oid FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace WHERE n.nspname = $1 AND c.relname = $2",
            &[&deployment.relation.schema, &deployment.relation.relation],
        )
        .map_err(|error| pg_error("resolve pgvector relation", error))?
        .ok_or_else(|| Error::InvalidRuntime {
            reason: "bound pgvector relation does not exist".into(),
        })?;
    let relation_oid: u32 = relation.get(0);
    let columns = client
        .query(
            "SELECT attnum, attname, format_type(atttypid, atttypmod) FROM pg_attribute WHERE attrelid = $1 AND attnum > 0 AND NOT attisdropped ORDER BY attnum",
            &[&relation_oid],
        )
        .map_err(|error| pg_error("inspect pgvector columns", error))?
        .into_iter()
        .map(|row| (row.get(0), row.get(1), row.get(2)))
        .collect();
    let indexes = client
        .query(
            "SELECT ic.relname, am.amname, pg_get_indexdef(i.indexrelid) FROM pg_index i JOIN pg_class ic ON ic.oid = i.indexrelid JOIN pg_am am ON am.oid = ic.relam WHERE i.indrelid = $1 ORDER BY ic.relname",
            &[&relation_oid],
        )
        .map_err(|error| pg_error("inspect pgvector indexes", error))?
        .into_iter()
        .map(|row| IndexEvidence {
            name: row.get(0),
            access_method: row.get(1),
            definition: row.get(2),
        })
        .collect();
    Ok(CatalogEvidence {
        extension_version,
        relation_oid,
        columns,
        indexes,
    })
}

pub(super) fn capture_revision<C: GenericClient>(
    client: &mut C,
    deployment: &PgvectorDeployment,
    binding: &OperatorKnowledgeBinding,
    catalog: &CatalogEvidence,
) -> Result<OperatorSourceRevision> {
    let row = client
        .query_one(
            "SELECT pg_current_snapshot()::text, pg_current_wal_lsn()::text",
            &[],
        )
        .map_err(|error| pg_error("capture pgvector snapshot", error))?;
    let snapshot: String = row.get(0);
    let wal_lsn: String = row.get(1);
    let revision_sql = format!(
        "SELECT stable_revision FROM {} WHERE project_digest = $1",
        qualified(&deployment.control_schema, &deployment.revision_table)
    );
    let stable_revision: i64 = client
        .query_one(&revision_sql, &[&project_digest(binding)])
        .map_err(|error| pg_error("capture pgvector stable revision", error))?
        .get(0);
    Ok(OperatorSourceRevision {
        contract_version: OPERATOR_KNOWLEDGE_CONTRACT_VERSION,
        adapter: binding.adapter.clone(),
        project_id: binding.project_id.clone(),
        source_identity_digest: binding.source_identity_digest.clone(),
        snapshot_digest: digest::sha256_hex(snapshot.as_bytes()),
        catalog_digest: content_digest(b"rrflow-pgvector-catalog-v1\0", catalog)?,
        stable_revision: Some(stable_revision.to_string()),
        wal_lsn_digest: Some(digest::sha256_hex(wal_lsn.as_bytes())),
    })
}

pub(super) fn live_search_sql(deployment: &PgvectorDeployment, metric: ScoreMetric) -> String {
    let relation = qualified(&deployment.relation.schema, &deployment.relation.relation);
    let id = quote_identifier(&deployment.relation.id_column);
    let subject = quote_identifier(&deployment.relation.subject_column);
    let vector = quote_identifier(&deployment.relation.vector_column);
    let tenant = quote_identifier(&deployment.relation.tenant_column);
    let model = quote_identifier(&deployment.model_digest_column);
    let cursor = quote_identifier(&deployment.source_cursor_column);
    let (operator, score) = match metric {
        ScoreMetric::Cosine => ("<=>", "1.0 - distance"),
        ScoreMetric::Dot => ("<#>", "distance * -1.0"),
        ScoreMetric::Euclidean => ("<->", "distance * -1.0"),
        ScoreMetric::Manhattan => ("<+>", "distance * -1.0"),
    };
    format!(
        "SELECT external_id, subject_id, {score} AS score FROM (SELECT {id}::text AS external_id, {subject}::text AS subject_id, {vector} {operator} $1::text::vector AS distance FROM {relation} WHERE {tenant}::text = $2 AND {model}::text = $3 AND {cursor} <= $4 ORDER BY {vector} {operator} $1::text::vector LIMIT $5) AS rrd_ranked ORDER BY distance ASC, external_id ASC"
    )
}

pub(super) fn selected_path<'a>(
    explain: &Value,
    catalog: &'a CatalogEvidence,
) -> (OperatorAccessPath, Option<&'a IndexEvidence>) {
    for index in &catalog.indexes {
        if explain_contains_index(explain, &index.name) {
            return match index.access_method.as_str() {
                "hnsw" => (OperatorAccessPath::Hnsw, Some(index)),
                "ivfflat" => (OperatorAccessPath::IvfFlat, Some(index)),
                _ => continue,
            };
        }
    }
    (OperatorAccessPath::Exact, None)
}

pub(super) fn explain_contains_index(value: &Value, expected: &str) -> bool {
    match value {
        Value::Object(map) => {
            map.get("Index Name").and_then(Value::as_str) == Some(expected)
                || map
                    .values()
                    .any(|value| explain_contains_index(value, expected))
        }
        Value::Array(values) => values
            .iter()
            .any(|value| explain_contains_index(value, expected)),
        _ => false,
    }
}

pub(super) fn deny_newer_row(
    transaction: &mut Transaction<'_>,
    deployment: &PgvectorDeployment,
    tenant: &str,
    external_id: &str,
    source_cursor: u64,
) -> Result<()> {
    let sql = format!(
        "SELECT {} FROM {} WHERE {}::text = $1 AND {}::text = $2 FOR UPDATE",
        quote_identifier(&deployment.source_cursor_column),
        qualified(&deployment.relation.schema, &deployment.relation.relation),
        quote_identifier(&deployment.relation.tenant_column),
        quote_identifier(&deployment.relation.id_column),
    );
    let current: Option<i64> = transaction
        .query_opt(&sql, &[&tenant, &external_id])
        .map_err(|error| pg_error("check pgvector row freshness", error))?
        .map(|row| row.get(0));
    let cursor = i64::try_from(source_cursor).map_err(|_| Error::InvalidRuntime {
        reason: "pgvector source cursor does not fit bigint".into(),
    })?;
    if current.is_some_and(|current| current > cursor) {
        return invalid("pgvector sync work is older than the external row");
    }
    Ok(())
}

pub(super) fn apply_vector(
    transaction: &mut Transaction<'_>,
    deployment: &PgvectorDeployment,
    binding: &OperatorKnowledgeBinding,
    tenant_value: &str,
    payload: &PgvectorSyncPayload,
) -> Result<()> {
    let tenant = quote_identifier(&deployment.relation.tenant_column);
    let id = quote_identifier(&deployment.relation.id_column);
    let subject = quote_identifier(&deployment.relation.subject_column);
    let vector = quote_identifier(&deployment.relation.vector_column);
    let model = quote_identifier(&deployment.model_digest_column);
    let cursor = quote_identifier(&deployment.source_cursor_column);
    let relation = qualified(&deployment.relation.schema, &deployment.relation.relation);
    let sql = format!(
        "INSERT INTO {relation} ({tenant}, {id}, {subject}, {vector}, {model}, {cursor}) VALUES ($1, $2, $3, $4::text::vector, $5, $6) ON CONFLICT ({tenant}, {id}) DO UPDATE SET {subject} = EXCLUDED.{subject}, {vector} = EXCLUDED.{vector}, {model} = EXCLUDED.{model}, {cursor} = EXCLUDED.{cursor}"
    );
    let vector = vector_literal(&payload.vector);
    let source_cursor =
        i64::try_from(payload.source_cursor).map_err(|_| Error::InvalidRuntime {
            reason: "pgvector source cursor does not fit bigint".into(),
        })?;
    transaction
        .execute(
            &sql,
            &[
                &tenant_value,
                &payload.external_id,
                &payload.subject_id,
                &vector,
                &binding.model.digest,
                &source_cursor,
            ],
        )
        .map_err(|error| pg_error("apply pgvector row", error))?;
    Ok(())
}

pub(super) fn delete_vector(
    transaction: &mut Transaction<'_>,
    deployment: &PgvectorDeployment,
    tenant_value: &str,
    external_id: &str,
    source_cursor: u64,
) -> Result<()> {
    let relation = qualified(&deployment.relation.schema, &deployment.relation.relation);
    let tenant = quote_identifier(&deployment.relation.tenant_column);
    let id = quote_identifier(&deployment.relation.id_column);
    let cursor = quote_identifier(&deployment.source_cursor_column);
    let sql = format!(
        "DELETE FROM {relation} WHERE {tenant}::text = $1 AND {id}::text = $2 AND {cursor} <= $3"
    );
    let source_cursor = i64::try_from(source_cursor).map_err(|_| Error::InvalidRuntime {
        reason: "pgvector source cursor does not fit bigint".into(),
    })?;
    transaction
        .execute(&sql, &[&tenant_value, &external_id, &source_cursor])
        .map_err(|error| pg_error("delete pgvector row", error))?;
    Ok(())
}

pub(super) fn bump_project_revision(
    transaction: &mut Transaction<'_>,
    deployment: &PgvectorDeployment,
    binding: &OperatorKnowledgeBinding,
) -> Result<i64> {
    let sql = format!(
        "UPDATE {} SET stable_revision = stable_revision + 1 WHERE project_digest = $1 RETURNING stable_revision",
        qualified(&deployment.control_schema, &deployment.revision_table)
    );
    transaction
        .query_opt(&sql, &[&project_digest(binding)])
        .map_err(|error| pg_error("advance pgvector project revision", error))?
        .map(|row| row.get(0))
        .ok_or_else(|| Error::InvalidRuntime {
            reason: "pgvector project revision disappeared".into(),
        })
}

pub(super) fn load_applied_revision(
    transaction: &mut Transaction<'_>,
    deployment: &PgvectorDeployment,
    binding: &OperatorKnowledgeBinding,
    work: &OperatorSyncWork,
) -> Result<Option<OperatorSourceRevision>> {
    let sql = format!(
        "SELECT project_digest, payload_digest, source_cursor, revision FROM {} WHERE work_id = $1",
        qualified(&deployment.control_schema, &deployment.applied_work_table)
    );
    let Some(row) = transaction
        .query_opt(&sql, &[&work.id])
        .map_err(|error| pg_error("read pgvector applied work", error))?
    else {
        return Ok(None);
    };
    let project: String = row.get(0);
    let payload: String = row.get(1);
    let source_cursor: i64 = row.get(2);
    let revision: Value = row.get(3);
    if project != project_digest(binding)
        || payload != work.payload_digest
        || source_cursor != i64::try_from(work.source_cursor).unwrap_or(-1)
    {
        return invalid("pgvector idempotency identity collides with different work");
    }
    let revision = serde_json::from_value(revision).map_err(|error| Error::InvalidRuntime {
        reason: format!("decode pgvector stored revision: {error}"),
    })?;
    Ok(Some(revision))
}

pub(super) fn store_applied_revision(
    transaction: &mut Transaction<'_>,
    deployment: &PgvectorDeployment,
    binding: &OperatorKnowledgeBinding,
    work: &OperatorSyncWork,
    revision: &OperatorSourceRevision,
) -> Result<()> {
    let sql = format!(
        "INSERT INTO {} (work_id, project_digest, payload_digest, source_cursor, revision) VALUES ($1, $2, $3, $4, $5)",
        qualified(&deployment.control_schema, &deployment.applied_work_table)
    );
    let source_cursor = i64::try_from(work.source_cursor).map_err(|_| Error::InvalidRuntime {
        reason: "pgvector source cursor does not fit bigint".into(),
    })?;
    let revision = serde_json::to_value(revision).map_err(|error| Error::InvalidRuntime {
        reason: format!("encode pgvector stored revision: {error}"),
    })?;
    transaction
        .execute(
            &sql,
            &[
                &work.id,
                &project_digest(binding),
                &work.payload_digest,
                &source_cursor,
                &revision,
            ],
        )
        .map_err(|error| pg_error("record pgvector applied work", error))?;
    Ok(())
}

pub(super) fn require_pgvector_08(version: &str) -> Result<()> {
    let mut parts = version.split('.');
    let major = parts.next().and_then(|part| part.parse::<u32>().ok());
    let minor = parts.next().and_then(|part| part.parse::<u32>().ok());
    if !matches!((major, minor), (Some(major), Some(minor)) if major > 0 || minor >= 8) {
        return invalid("live adapter requires pgvector 0.8 or newer");
    }
    Ok(())
}

pub(super) fn require_tls_config(params: &str) -> Result<Config> {
    let config = Config::from_str(params).map_err(|error| pg_error("parse TLS config", error))?;
    if config.get_ssl_mode() != SslMode::Require {
        return invalid("pgvector TLS connection must set sslmode=require");
    }
    Ok(config)
}

pub(super) fn vector_literal(values: &[f32]) -> String {
    let mut literal = String::from("[");
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            literal.push(',');
        }
        literal.push_str(&value.to_string());
    }
    literal.push(']');
    literal
}

pub(super) fn project_digest(binding: &OperatorKnowledgeBinding) -> String {
    digest::sha256_hex(binding.project_id.as_bytes())
}

pub(super) fn qualified(schema: &str, relation: &str) -> String {
    format!(
        "{}.{}",
        quote_identifier(schema),
        quote_identifier(relation)
    )
}

pub(super) fn pg_error(context: &'static str, error: postgres::Error) -> Error {
    if let Some(database) = error.as_db_error() {
        return Error::InvalidRuntime {
            reason: format!(
                "{context}: PostgreSQL {}: {}",
                database.code().code(),
                database.message()
            ),
        };
    }
    runtime_error(context, error)
}

pub(super) fn runtime_error(context: &'static str, error: impl std::fmt::Display) -> Error {
    Error::InvalidRuntime {
        reason: format!("{context}: {error}"),
    }
}
