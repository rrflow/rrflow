//! The pgvector deployment, sync payloads and the live adapter.

use super::*;

pub const PGVECTOR_LIVE_IMPLEMENTATION_VERSION: &str = "rrflow-pgvector-live-v1";

/// Non-secret deployment identity for one pgvector relation and RRD control
/// schema. Connection strings, passwords, and root certificates never enter
/// this serializable contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PgvectorDeployment {
    pub contract_version: u16,
    pub relation: PgvectorRelation,
    pub model_digest_column: String,
    pub source_cursor_column: String,
    pub control_schema: String,
    pub metadata_table: String,
    pub revision_table: String,
    pub applied_work_table: String,
}

impl PgvectorDeployment {
    pub fn validate(&self) -> Result<()> {
        validate_version(self.contract_version)?;
        self.relation.validate()?;
        for (kind, value) in [
            ("pgvector model digest column", &self.model_digest_column),
            ("pgvector source cursor column", &self.source_cursor_column),
            ("pgvector control schema", &self.control_schema),
            ("pgvector metadata table", &self.metadata_table),
            ("pgvector revision table", &self.revision_table),
            ("pgvector applied-work table", &self.applied_work_table),
        ] {
            validate_pg_identifier(kind, value)?;
        }
        let columns = [
            self.relation.id_column.as_str(),
            self.relation.subject_column.as_str(),
            self.relation.vector_column.as_str(),
            self.relation.tenant_column.as_str(),
            self.model_digest_column.as_str(),
            self.source_cursor_column.as_str(),
        ];
        if columns
            .iter()
            .enumerate()
            .any(|(index, value)| columns[..index].contains(value))
        {
            return invalid("pgvector deployment columns must be distinct");
        }
        let tables = [
            self.metadata_table.as_str(),
            self.revision_table.as_str(),
            self.applied_work_table.as_str(),
        ];
        if tables
            .iter()
            .enumerate()
            .any(|(index, value)| tables[..index].contains(value))
        {
            return invalid("pgvector control tables must be distinct");
        }
        Ok(())
    }

    pub fn digest(&self) -> Result<String> {
        self.validate()?;
        content_digest(b"rrflow-pgvector-deployment-v1\0", self)
    }
}

/// Canonical payload for applying one vector-family outbox item to pgvector.
/// The surrounding `OperatorSyncWork` content-addresses these exact bytes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PgvectorSyncPayload {
    pub contract_version: u16,
    pub external_id: String,
    pub subject_id: String,
    pub source_cursor: u64,
    pub vector: Vec<f32>,
}

impl PgvectorSyncPayload {
    pub fn validate(&self, dimensions: u32) -> Result<()> {
        validate_version(self.contract_version)?;
        validate_name("pgvector external identity", &self.external_id)?;
        validate_name("pgvector subject identity", &self.subject_id)?;
        if self.source_cursor == 0 {
            return invalid("pgvector sync source cursor must be non-zero");
        }
        if self.vector.len() != dimensions as usize
            || self.vector.is_empty()
            || self.vector.iter().any(|value| !value.is_finite())
        {
            return invalid("pgvector sync vector differs from the bound finite model space");
        }
        Ok(())
    }

    pub fn canonical_bytes(&self, dimensions: u32) -> Result<Vec<u8>> {
        self.validate(dimensions)?;
        serde_json::to_vec(self).map_err(|error| Error::InvalidRuntime {
            reason: format!("encode canonical pgvector sync payload: {error}"),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PgvectorDeletePayload {
    pub contract_version: u16,
    pub external_id: String,
    pub source_cursor: u64,
}

impl PgvectorDeletePayload {
    pub fn validate(&self) -> Result<()> {
        validate_version(self.contract_version)?;
        validate_name("pgvector delete identity", &self.external_id)?;
        if self.source_cursor == 0 {
            return invalid("pgvector delete source cursor must be non-zero");
        }
        Ok(())
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>> {
        self.validate()?;
        serde_json::to_vec(self).map_err(|error| Error::InvalidRuntime {
            reason: format!("encode canonical pgvector delete payload: {error}"),
        })
    }
}

/// A synchronous live adapter. It deliberately owns one PostgreSQL client so
/// each trait call has one transaction and cannot accidentally interleave
/// another project's statements on the same session.
pub struct PgvectorLiveAdapter {
    client: Client,
    deployment: PgvectorDeployment,
    binding: OperatorKnowledgeBinding,
    binding_digest: String,
    tenant: String,
    descriptor: OperatorAdapterDescriptor,
}

impl PgvectorLiveAdapter {
    /// Connects with certificate- and hostname-validating native TLS. The
    /// connection configuration must explicitly require TLS; `prefer` is
    /// rejected because it can downgrade to plaintext.
    pub fn connect_tls(
        params: &str,
        root_certificate_pem: Option<&[u8]>,
        deployment: PgvectorDeployment,
        binding: OperatorKnowledgeBinding,
        tenant: impl Into<String>,
    ) -> Result<Self> {
        let config = require_tls_config(params)?;
        let mut connector = TlsConnector::builder();
        if let Some(pem) = root_certificate_pem {
            let certificate = Certificate::from_pem(pem)
                .map_err(|error| runtime_error("parse pgvector root certificate", error))?;
            connector.add_root_certificate(certificate);
        }
        let connector = connector
            .build()
            .map_err(|error| runtime_error("build pgvector TLS connector", error))?;
        let client = config
            .connect(MakeTlsConnector::new(connector))
            .map_err(|error| pg_error("connect pgvector TLS session", error))?;
        Self::from_client(client, deployment, binding, tenant)
    }

    /// Adopts an already-authenticated client. This is also the explicit seam
    /// used by local endpoint tests; production callers should prefer
    /// `connect_tls`.
    pub fn from_client(
        mut client: Client,
        deployment: PgvectorDeployment,
        binding: OperatorKnowledgeBinding,
        tenant: impl Into<String>,
    ) -> Result<Self> {
        deployment.validate()?;
        binding.validate()?;
        let tenant = tenant.into();
        validate_name("pgvector tenant value", &tenant)?;
        if binding.adapter != "pgvector"
            || deployment.digest()? != binding.config_digest
            || deployment.relation.digest()? != binding.relation_digest
            || digest::sha256_hex(tenant.as_bytes()) != binding.tenant_digest
        {
            return invalid("pgvector deployment or tenant differs from the immutable binding");
        }
        verify_live_schema(&mut client, &deployment, &binding)?;
        let descriptor = descriptor();
        let binding_digest = binding.digest()?;
        Ok(Self {
            client,
            deployment,
            binding,
            binding_digest,
            tenant,
            descriptor,
        })
    }

    /// Installs only RRD's control schema and the project revision row. The
    /// operator knowledge relation remains application-owned. Existing source
    /// identity cannot be overwritten by a different deployment.
    pub fn install_control_schema(
        client: &mut Client,
        deployment: &PgvectorDeployment,
        binding: &OperatorKnowledgeBinding,
    ) -> Result<()> {
        deployment.validate()?;
        binding.validate()?;
        if binding.adapter != "pgvector"
            || deployment.digest()? != binding.config_digest
            || deployment.relation.digest()? != binding.relation_digest
        {
            return invalid("pgvector control migration differs from the immutable binding");
        }
        let schema = quote_identifier(&deployment.control_schema);
        let metadata = qualified(&deployment.control_schema, &deployment.metadata_table);
        let revision = qualified(&deployment.control_schema, &deployment.revision_table);
        let applied = qualified(&deployment.control_schema, &deployment.applied_work_table);
        let ddl = format!(
            "CREATE SCHEMA IF NOT EXISTS {schema};\
             CREATE TABLE IF NOT EXISTS {metadata} (singleton boolean PRIMARY KEY CHECK (singleton), contract_version integer NOT NULL, source_identity_digest text NOT NULL);\
             CREATE TABLE IF NOT EXISTS {revision} (project_digest text PRIMARY KEY, stable_revision bigint NOT NULL CHECK (stable_revision >= 0));\
             CREATE TABLE IF NOT EXISTS {applied} (work_id text PRIMARY KEY, project_digest text NOT NULL, payload_digest text NOT NULL, source_cursor bigint NOT NULL CHECK (source_cursor > 0), revision jsonb NOT NULL);"
        );
        let mut transaction = client
            .build_transaction()
            .isolation_level(IsolationLevel::Serializable)
            .start()
            .map_err(|error| pg_error("start pgvector control migration", error))?;
        transaction
            .batch_execute(&ddl)
            .map_err(|error| pg_error("install pgvector control schema", error))?;
        let metadata_sql = format!(
            "INSERT INTO {metadata} (singleton, contract_version, source_identity_digest) VALUES (true, $1, $2) ON CONFLICT (singleton) DO NOTHING"
        );
        transaction
            .execute(
                &metadata_sql,
                &[
                    &i32::from(OPERATOR_KNOWLEDGE_CONTRACT_VERSION),
                    &binding.source_identity_digest,
                ],
            )
            .map_err(|error| pg_error("seed pgvector source identity", error))?;
        verify_source_identity(&mut transaction, deployment, binding)?;
        let revision_sql = format!(
            "INSERT INTO {revision} (project_digest, stable_revision) VALUES ($1, 0) ON CONFLICT (project_digest) DO NOTHING"
        );
        transaction
            .execute(&revision_sql, &[&project_digest(binding)])
            .map_err(|error| pg_error("seed pgvector project revision", error))?;
        transaction
            .commit()
            .map_err(|error| pg_error("commit pgvector control migration", error))
    }

    pub fn into_client(self) -> Client {
        self.client
    }
}

impl OperatorKnowledgeAdapter for PgvectorLiveAdapter {
    fn descriptor(&self) -> &OperatorAdapterDescriptor {
        &self.descriptor
    }

    fn search(
        &mut self,
        binding: &OperatorKnowledgeBinding,
        request: &OperatorSearchRequest,
    ) -> Result<OperatorSearchResult> {
        if binding.digest()? != self.binding_digest {
            return invalid("live pgvector adapter is bound to another project");
        }
        let vector = match &request.search.query {
            VectorQuery::Dense { values } => vector_literal(values),
            _ => return invalid("live pgvector v1 accepts dense vectors only"),
        };
        let started = Instant::now();
        let deployment = self.deployment.clone();
        let stored_binding = self.binding.clone();
        let tenant = self.tenant.clone();
        let mut transaction = self
            .client
            .build_transaction()
            .isolation_level(IsolationLevel::RepeatableRead)
            .read_only(true)
            .start()
            .map_err(|error| pg_error("start pgvector repeatable-read search", error))?;
        verify_source_identity(&mut transaction, &deployment, &stored_binding)?;
        let portable_plan = deployment
            .relation
            .build_search(request.search.metric, &request.controls)?;
        for (name, value) in &portable_plan.settings {
            transaction
                .query_one("SELECT set_config($1, $2, true)", &[name, value])
                .map_err(|error| pg_error("apply pgvector local search control", error))?;
        }
        let sql = live_search_sql(&deployment, request.search.metric);
        let explain_sql = format!("EXPLAIN (FORMAT JSON) {sql}");
        let top_k = i64::try_from(request.search.top_k).map_err(|_| Error::InvalidRuntime {
            reason: "pgvector top_k does not fit bigint".into(),
        })?;
        let read_cursor = i64::try_from(request.search.read.commit_cursor).map_err(|_| {
            Error::InvalidRuntime {
                reason: "pgvector read cursor does not fit bigint".into(),
            }
        })?;
        let parameters: [&(dyn postgres::types::ToSql + Sync); 5] = [
            &vector,
            &tenant,
            &stored_binding.model.digest,
            &read_cursor,
            &top_k,
        ];
        let explain: Value = transaction
            .query_one(&explain_sql, &parameters)
            .map_err(|error| pg_error("explain pgvector search", error))?
            .get(0);
        let catalog = capture_catalog(&mut transaction, &deployment)?;
        let (selected_path, selected_index) = selected_path(&explain, &catalog);
        let rows = transaction
            .query(&sql, &parameters)
            .map_err(|error| pg_error("execute pgvector search", error))?;
        let revision = capture_revision(&mut transaction, &deployment, &stored_binding, &catalog)?;
        transaction
            .commit()
            .map_err(|error| pg_error("commit pgvector search snapshot", error))?;
        let fallback_reason_digest =
            (selected_path != request.controls.requested_path).then(|| {
                digest::sha256_hex(
                    format!(
                        "pgvector planner selected {selected_path:?} instead of {:?}",
                        request.controls.requested_path
                    )
                    .as_bytes(),
                )
            });
        let index_digest =
            selected_index.map(|index| digest::sha256_hex(index.definition.as_bytes()));
        let plan_digest = content_digest(b"rrflow-pgvector-explain-v1\0", &explain)?;
        Ok(OperatorSearchResult {
            contract_version: OPERATOR_KNOWLEDGE_CONTRACT_VERSION,
            request_digest: request.digest()?,
            revision,
            plan: OperatorPlanEvidence {
                selected_path,
                plan_digest,
                index_digest,
                fallback_reason_digest,
                controls: request.controls.clone(),
                filter_applied_after_ann: selected_path != OperatorAccessPath::Exact,
                ordering_exact: selected_path == OperatorAccessPath::Exact
                    || request.controls.iterative_scan != IterativeScanMode::RelaxedOrder,
                candidates_examined: None,
            },
            hits: rows
                .into_iter()
                .map(|row| OperatorSearchHit {
                    external_id: row.get(0),
                    subject_id: row.get(1),
                    score: row.get(2),
                })
                .collect(),
            elapsed_micros: started.elapsed().as_micros().try_into().unwrap_or(u64::MAX),
        })
    }
}

impl OperatorKnowledgeWriter for PgvectorLiveAdapter {
    fn descriptor(&self) -> &OperatorAdapterDescriptor {
        &self.descriptor
    }

    fn apply(
        &mut self,
        binding: &OperatorKnowledgeBinding,
        work: &OperatorSyncWork,
        payload: &[u8],
    ) -> Result<OperatorSyncReceipt> {
        if binding.digest()? != self.binding_digest {
            return invalid("live pgvector writer is bound to another project");
        }
        let (external_id, source_cursor, operation) = match work.operation {
            OperatorSyncOperation::UpsertVector => {
                let payload: PgvectorSyncPayload =
                    serde_json::from_slice(payload).map_err(|error| Error::InvalidRuntime {
                        reason: format!("decode canonical pgvector sync payload: {error}"),
                    })?;
                payload.validate(binding.dimensions)?;
                (
                    payload.external_id.clone(),
                    payload.source_cursor,
                    LiveSyncOperation::Upsert(payload),
                )
            }
            OperatorSyncOperation::DeleteVector => {
                let payload: PgvectorDeletePayload =
                    serde_json::from_slice(payload).map_err(|error| Error::InvalidRuntime {
                        reason: format!("decode canonical pgvector delete payload: {error}"),
                    })?;
                payload.validate()?;
                (
                    payload.external_id.clone(),
                    payload.source_cursor,
                    LiveSyncOperation::Delete,
                )
            }
        };
        if source_cursor != work.source_cursor {
            return invalid("pgvector payload cursor differs from its outbox work");
        }
        let deployment = self.deployment.clone();
        let stored_binding = self.binding.clone();
        let tenant = self.tenant.clone();
        let mut transaction = self
            .client
            .build_transaction()
            .isolation_level(IsolationLevel::Serializable)
            .start()
            .map_err(|error| pg_error("start pgvector sync transaction", error))?;
        verify_source_identity(&mut transaction, &deployment, &stored_binding)?;
        transaction
            .query_one(
                "SELECT pg_advisory_xact_lock(hashtextextended($1, 0))",
                &[&project_digest(&stored_binding)],
            )
            .map_err(|error| pg_error("lock pgvector project sync lane", error))?;
        if let Some(revision) =
            load_applied_revision(&mut transaction, &deployment, &stored_binding, work)?
        {
            transaction
                .commit()
                .map_err(|error| pg_error("commit pgvector idempotent replay", error))?;
            return Ok(OperatorSyncReceipt {
                contract_version: OPERATOR_KNOWLEDGE_CONTRACT_VERSION,
                work_id: work.id.clone(),
                revision,
                applied_now: false,
                idempotent_replay: true,
            });
        }
        deny_newer_row(
            &mut transaction,
            &deployment,
            &tenant,
            &external_id,
            work.source_cursor,
        )?;
        match operation {
            LiveSyncOperation::Upsert(payload) => apply_vector(
                &mut transaction,
                &deployment,
                &stored_binding,
                &tenant,
                &payload,
            )?,
            LiveSyncOperation::Delete => delete_vector(
                &mut transaction,
                &deployment,
                &tenant,
                &external_id,
                source_cursor,
            )?,
        }
        bump_project_revision(&mut transaction, &deployment, &stored_binding)?;
        let catalog = capture_catalog(&mut transaction, &deployment)?;
        let revision = capture_revision(&mut transaction, &deployment, &stored_binding, &catalog)?;
        store_applied_revision(
            &mut transaction,
            &deployment,
            &stored_binding,
            work,
            &revision,
        )?;
        transaction
            .commit()
            .map_err(|error| pg_error("commit pgvector sync transaction", error))?;
        Ok(OperatorSyncReceipt {
            contract_version: OPERATOR_KNOWLEDGE_CONTRACT_VERSION,
            work_id: work.id.clone(),
            revision,
            applied_now: true,
            idempotent_replay: false,
        })
    }
}
