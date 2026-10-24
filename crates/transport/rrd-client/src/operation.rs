//! Typed public operations, envelope construction, and outcome correlation.

use crate::error::{contract, decode};
use crate::{Error, Result, RrdClient, Session};
use hyper::{Method, StatusCode};
use rrd_contract::{
    AbortTransaction, AssembleContext, AuditExport, AuditPage, BeginTransaction, CanonicalId,
    ChangefeedFollowResult, ChangefeedPage, CloseTask, CommitReceipt, CommitTransaction,
    ContextPacket, CorrelationId, CreateInstanceBackup, CreateInstanceBackupResult,
    DiagnosticSnapshot, EnsureVectorCollection, EnsureVectorCollectionResult, EstateSnapshot,
    ExecuteQuery, ExportAudit, FollowChangefeed, InstanceBackupCatalogueSnapshot,
    LaunchPromptFlight, LaunchPromptFlightResult, ListInstanceBackups, ListPromptFlights,
    ListTasks, ListVectorCollections, OpenTask, PreviewTransaction, PromptFlightPage,
    PromptFlightReadResult, QueryResult, ReadAudit, ReadChangefeed, ReadDiagnosticSnapshot,
    ReadEstate, ReadPromptFlight, ReadTask, RequestContext, RequestEnvelope, ResourceId,
    ResourceKind, ResourcePath, ResponseEnvelope, ResponseOutcome, RestoreInstanceBackup,
    RestoreInstanceBackupResult, RetrieveVectorPoints, ScrollVectorPoints, SearchVectors, TaskPage,
    TaskReadResult, TaskResult, TransactionLease, TransactionPreview,
    VectorCollectionCatalogueSnapshot, VectorPointBatch, VectorPointPage, VectorSearchResult,
    PROTOCOL, PROTOCOL_VERSION,
};
use serde::de::DeserializeOwned;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestOptions {
    pub request_id: CorrelationId,
    pub operation_id: CorrelationId,
    pub idempotency_key: Option<CorrelationId>,
    pub deadline_unix_ms: Option<u64>,
}

impl RequestOptions {
    pub fn read(request_id: &str, operation_id: &str) -> Result<Self> {
        Self::new(request_id, operation_id, None, None)
    }

    pub fn mutation(request_id: &str, operation_id: &str, idempotency_key: &str) -> Result<Self> {
        Self::new(request_id, operation_id, Some(idempotency_key), None)
    }

    pub fn new(
        request_id: &str,
        operation_id: &str,
        idempotency_key: Option<&str>,
        deadline_unix_ms: Option<u64>,
    ) -> Result<Self> {
        let options = Self {
            request_id: CorrelationId::new(request_id).map_err(contract)?,
            operation_id: CorrelationId::new(operation_id).map_err(contract)?,
            idempotency_key: idempotency_key
                .map(CorrelationId::new)
                .transpose()
                .map_err(contract)?,
            deadline_unix_ms,
        };
        options.context().validate(false).map_err(contract)?;
        Ok(options)
    }

    pub(crate) fn context(&self) -> RequestContext {
        RequestContext {
            request_id: self.request_id.clone(),
            operation_id: self.operation_id.clone(),
            idempotency_key: self.idempotency_key.clone(),
            deadline_unix_ms: self.deadline_unix_ms,
        }
    }
}

impl RrdClient {
    /// Discover the native catalogue through the authenticated operation
    /// descriptor. Requires the instance-wide diagnostics-read grant.
    pub async fn read_data_schema(
        &self,
        session: &Session,
        request: rrd_contract::ReadDataSchema,
        options: RequestOptions,
    ) -> Result<rrd_contract::DataSchemaSnapshot> {
        request.validate().map_err(contract)?;
        let expected_scope = request.scope.clone();
        let catalogue = rrd_contract::endpoint_catalogue();
        let endpoint = catalogue
            .endpoints
            .iter()
            .find(|endpoint| endpoint.operation.as_str() == "data-schema-read")
            .ok_or_else(|| Error::Contract("data schema operation is unavailable".into()))?;
        let method = match endpoint.method {
            rrd_contract::HttpMethod::Get => Method::GET,
            rrd_contract::HttpMethod::Post => Method::POST,
            rrd_contract::HttpMethod::Delete => Method::DELETE,
        };
        let result: rrd_contract::DataSchemaSnapshot = self
            .session_call(
                method,
                &endpoint.path,
                session,
                request,
                options,
                endpoint.mutation,
            )
            .await?;
        result.validate().map_err(contract)?;
        if result.scope != expected_scope {
            return Err(Error::ResponseIdentityMismatch);
        }
        Ok(result)
    }

    pub async fn execute_query(
        &self,
        session: &Session,
        request: ExecuteQuery,
        options: RequestOptions,
    ) -> Result<QueryResult> {
        self.session_call(Method::POST, "/v1/query", session, request, options, false)
            .await
    }

    pub async fn read_endpoint_configuration(
        &self,
        session: &Session,
        request: rrd_contract::ReadEndpointConfiguration,
        options: RequestOptions,
    ) -> Result<rrd_contract::EndpointConfiguration> {
        let catalogue = rrd_contract::endpoint_catalogue();
        let endpoint = catalogue
            .endpoints
            .iter()
            .find(|e| e.operation.as_str() == "endpoint-configuration-read")
            .ok_or_else(|| {
                Error::Contract("endpoint configuration operation is unavailable".into())
            })?;
        let result: rrd_contract::EndpointConfiguration = self
            .session_call(
                Method::POST,
                &endpoint.path,
                session,
                request,
                options,
                endpoint.mutation,
            )
            .await?;
        result.validate().map_err(contract)?;
        if result.identity.instance_id != self.instance {
            return Err(Error::ResponseIdentityMismatch);
        }
        Ok(result)
    }

    pub async fn preview_endpoint_configuration(
        &self,
        session: &Session,
        request: rrd_contract::PreviewEndpointConfiguration,
        options: RequestOptions,
    ) -> Result<rrd_contract::EndpointConfigurationPlan> {
        request.validate().map_err(contract)?;
        let expected_revision = request.expected_revision;
        let expected_profile = request
            .profile
            .clone()
            .map(|p| rrd_contract::EndpointProfile::from_input(expected_revision + 1, p))
            .transpose()
            .map_err(contract)?;
        let catalogue = rrd_contract::endpoint_catalogue();
        let endpoint = catalogue
            .endpoints
            .iter()
            .find(|e| e.operation.as_str() == "endpoint-configuration-preview")
            .ok_or_else(|| {
                Error::Contract("endpoint configuration operation is unavailable".into())
            })?;
        let result: rrd_contract::EndpointConfigurationPlan = self
            .session_call(
                Method::POST,
                &endpoint.path,
                session,
                request,
                options,
                endpoint.mutation,
            )
            .await?;
        result.validate().map_err(contract)?;
        if result.identity.instance_id != self.instance {
            return Err(Error::ResponseIdentityMismatch);
        }
        if result.expected_revision != expected_revision || result.profile != expected_profile {
            return Err(Error::ResponseIdentityMismatch);
        }
        Ok(result)
    }

    pub async fn apply_endpoint_configuration(
        &self,
        session: &Session,
        request: rrd_contract::ApplyEndpointConfiguration,
        options: RequestOptions,
    ) -> Result<rrd_contract::EndpointConfiguration> {
        request.validate().map_err(contract)?;
        let expected_plan = request.plan.clone();
        let catalogue = rrd_contract::endpoint_catalogue();
        let endpoint = catalogue
            .endpoints
            .iter()
            .find(|e| e.operation.as_str() == "endpoint-configuration-apply")
            .ok_or_else(|| {
                Error::Contract("endpoint configuration operation is unavailable".into())
            })?;
        let result: rrd_contract::EndpointConfiguration = self
            .session_call(
                Method::POST,
                &endpoint.path,
                session,
                request,
                options,
                endpoint.mutation,
            )
            .await?;
        result.validate().map_err(contract)?;
        if result.identity.instance_id != self.instance {
            return Err(Error::ResponseIdentityMismatch);
        }
        if result.identity != expected_plan.identity
            || result.revision != expected_plan.expected_revision + 1
            || result.profile != expected_plan.profile
            || result.last_applied_plan_sha256.as_ref() != Some(&expected_plan.plan_sha256)
        {
            return Err(Error::ResponseIdentityMismatch);
        }
        Ok(result)
    }

    pub async fn export_query(
        &self,
        session: &Session,
        request: rrd_contract::ExportQuery,
        options: RequestOptions,
    ) -> Result<rrd_contract::QueryExport> {
        request.validate().map_err(contract)?;
        let scope = request.query.scope.clone();
        let limit = request.max_output_bytes;
        let catalogue = rrd_contract::endpoint_catalogue();
        let endpoint = catalogue
            .endpoints
            .iter()
            .find(|endpoint| endpoint.operation.as_str() == "query-export")
            .ok_or_else(|| Error::Contract("query export operation is unavailable".into()))?;
        let method = match endpoint.method {
            rrd_contract::HttpMethod::Get => Method::GET,
            rrd_contract::HttpMethod::Post => Method::POST,
            rrd_contract::HttpMethod::Delete => Method::DELETE,
        };
        let result: rrd_contract::QueryExport = self
            .session_call(
                method,
                &endpoint.path,
                session,
                request,
                options,
                endpoint.mutation,
            )
            .await?;
        result.decode_artifact().map_err(contract)?;
        if result.source.scope != scope || result.artifact_bytes > limit {
            return Err(Error::ResponseIdentityMismatch);
        }
        Ok(result)
    }

    pub async fn begin_transaction(
        &self,
        session: &Session,
        request: BeginTransaction,
        options: RequestOptions,
    ) -> Result<TransactionLease> {
        self.session_call(
            Method::POST,
            "/v1/transactions",
            session,
            request,
            options,
            true,
        )
        .await
    }

    pub async fn preview_transaction(
        &self,
        session: &Session,
        transaction: &CorrelationId,
        request: PreviewTransaction,
        options: RequestOptions,
    ) -> Result<TransactionPreview> {
        self.session_call(
            Method::POST,
            &format!("/v1/transactions/{}/preview", transaction.as_str()),
            session,
            request,
            options,
            true,
        )
        .await
    }

    pub async fn commit_transaction(
        &self,
        session: &Session,
        transaction: &CorrelationId,
        request: CommitTransaction,
        options: RequestOptions,
    ) -> Result<CommitReceipt> {
        self.session_call(
            Method::POST,
            &format!("/v1/transactions/{}/commit", transaction.as_str()),
            session,
            request,
            options,
            true,
        )
        .await
    }

    pub async fn abort_transaction(
        &self,
        session: &Session,
        transaction: &CorrelationId,
        request: AbortTransaction,
        options: RequestOptions,
    ) -> Result<TransactionLease> {
        self.session_call(
            Method::DELETE,
            &format!("/v1/transactions/{}", transaction.as_str()),
            session,
            request,
            options,
            true,
        )
        .await
    }

    pub async fn search_vectors(
        &self,
        session: &Session,
        request: SearchVectors,
        options: RequestOptions,
    ) -> Result<VectorSearchResult> {
        self.session_call(
            Method::POST,
            "/v1/vector/search",
            session,
            request,
            options,
            false,
        )
        .await
    }

    pub async fn ensure_vector_collection(
        &self,
        session: &Session,
        request: EnsureVectorCollection,
        options: RequestOptions,
    ) -> Result<EnsureVectorCollectionResult> {
        self.session_call(
            Method::POST,
            "/v1/vector/collections/ensure",
            session,
            request,
            options,
            true,
        )
        .await
    }

    pub async fn list_vector_collections(
        &self,
        session: &Session,
        request: ListVectorCollections,
        options: RequestOptions,
    ) -> Result<VectorCollectionCatalogueSnapshot> {
        self.session_call(
            Method::POST,
            "/v1/vector/collections/list",
            session,
            request,
            options,
            false,
        )
        .await
    }

    pub async fn scroll_vector_points(
        &self,
        session: &Session,
        request: ScrollVectorPoints,
        options: RequestOptions,
    ) -> Result<VectorPointPage> {
        self.session_call(
            Method::POST,
            "/v1/vector/points/scroll",
            session,
            request,
            options,
            false,
        )
        .await
    }

    pub async fn retrieve_vector_points(
        &self,
        session: &Session,
        request: RetrieveVectorPoints,
        options: RequestOptions,
    ) -> Result<VectorPointBatch> {
        self.session_call(
            Method::POST,
            "/v1/vector/points/retrieve",
            session,
            request,
            options,
            false,
        )
        .await
    }

    pub async fn read_changefeed(
        &self,
        session: &Session,
        request: ReadChangefeed,
        options: RequestOptions,
    ) -> Result<ChangefeedPage> {
        self.session_call(
            Method::POST,
            "/v1/changes/read",
            session,
            request,
            options,
            false,
        )
        .await
    }

    pub async fn follow_changefeed(
        &self,
        session: &Session,
        request: FollowChangefeed,
        options: RequestOptions,
    ) -> Result<ChangefeedFollowResult> {
        self.session_call(
            Method::POST,
            "/v1/changes/follow",
            session,
            request,
            options,
            false,
        )
        .await
    }

    pub async fn create_backup(
        &self,
        session: &Session,
        request: CreateInstanceBackup,
        options: RequestOptions,
    ) -> Result<CreateInstanceBackupResult> {
        self.session_call(Method::POST, "/v1/backups", session, request, options, true)
            .await
    }

    pub async fn list_backups(
        &self,
        session: &Session,
        request: ListInstanceBackups,
        options: RequestOptions,
    ) -> Result<InstanceBackupCatalogueSnapshot> {
        self.session_call(
            Method::POST,
            "/v1/backups/list",
            session,
            request,
            options,
            false,
        )
        .await
    }

    pub async fn restore_backup(
        &self,
        session: &Session,
        request: RestoreInstanceBackup,
        options: RequestOptions,
    ) -> Result<RestoreInstanceBackupResult> {
        self.session_call(
            Method::POST,
            "/v1/restores",
            session,
            request,
            options,
            true,
        )
        .await
    }

    pub async fn read_estate(
        &self,
        session: &Session,
        estate: CanonicalId,
        request: ReadEstate,
        options: RequestOptions,
    ) -> Result<EstateSnapshot> {
        let resource = ResourcePath {
            segments: vec![
                ResourceId::new(ResourceKind::Estate, estate.as_str().to_owned())
                    .map_err(contract)?,
                ResourceId::new(ResourceKind::Instance, self.instance.as_str().to_owned())
                    .map_err(contract)?,
            ],
        };
        self.session_call_resource(
            Method::POST,
            &format!("/v1/estates/{estate}/read"),
            session,
            resource,
            request,
            options,
            false,
        )
        .await
    }

    pub async fn read_audit(
        &self,
        session: &Session,
        request: ReadAudit,
        options: RequestOptions,
    ) -> Result<AuditPage> {
        let page: AuditPage = self
            .session_call(
                Method::POST,
                "/v1/audit/read",
                session,
                request,
                options,
                false,
            )
            .await?;
        page.validate().map_err(contract)?;
        Ok(page)
    }

    pub async fn export_audit(
        &self,
        session: &Session,
        request: ExportAudit,
        options: RequestOptions,
    ) -> Result<AuditExport> {
        let export: AuditExport = self
            .session_call(
                Method::POST,
                "/v1/audit/export",
                session,
                request,
                options,
                false,
            )
            .await?;
        export.validate().map_err(contract)?;
        Ok(export)
    }

    pub async fn read_diagnostic_snapshot(
        &self,
        session: &Session,
        request: ReadDiagnosticSnapshot,
        options: RequestOptions,
    ) -> Result<DiagnosticSnapshot> {
        let snapshot: DiagnosticSnapshot = self
            .session_call(
                Method::POST,
                "/v1/diagnostics/read",
                session,
                request,
                options,
                false,
            )
            .await?;
        snapshot.validate().map_err(contract)?;
        Ok(snapshot)
    }

    pub async fn inspect_storage_cursor(
        &self,
        session: &Session,
        request: rrd_contract::InspectStorageCursor,
        options: RequestOptions,
    ) -> Result<rrd_contract::StorageCursorSnapshot> {
        let result: rrd_contract::StorageCursorSnapshot = self
            .session_call(
                Method::POST,
                "/v1/diagnostics/storage/cursor",
                session,
                request,
                options,
                false,
            )
            .await?;
        result.validate().map_err(contract)?;
        Ok(result)
    }

    /// Bounded physical observations without materializing graph history.
    pub async fn read_storage_diagnostics(
        &self,
        session: &Session,
        request: rrd_contract::ReadStorageDiagnostics,
        options: RequestOptions,
    ) -> Result<rrd_contract::DiagnosticStorageSnapshot> {
        let snapshot: rrd_contract::DiagnosticStorageSnapshot = self
            .session_call(
                Method::POST,
                "/v1/diagnostics/storage/read",
                session,
                request,
                options,
                false,
            )
            .await?;
        snapshot.validate().map_err(contract)?;
        Ok(snapshot)
    }

    pub async fn assemble_context(
        &self,
        session: &Session,
        request: AssembleContext,
        options: RequestOptions,
    ) -> Result<ContextPacket> {
        let packet: ContextPacket = self
            .session_call(
                Method::POST,
                "/v1/context/assemble",
                session,
                request,
                options,
                false,
            )
            .await?;
        packet.validate().map_err(contract)?;
        Ok(packet)
    }

    pub async fn launch_prompt_flight(
        &self,
        session: &Session,
        request: LaunchPromptFlight,
        options: RequestOptions,
    ) -> Result<LaunchPromptFlightResult> {
        let result: LaunchPromptFlightResult = self
            .session_call(Method::POST, "/v1/flights", session, request, options, true)
            .await?;
        result.validate().map_err(contract)?;
        Ok(result)
    }

    pub async fn list_prompt_flights(
        &self,
        session: &Session,
        request: ListPromptFlights,
        options: RequestOptions,
    ) -> Result<PromptFlightPage> {
        let page: PromptFlightPage = self
            .session_call(
                Method::POST,
                "/v1/flights/list",
                session,
                request,
                options,
                false,
            )
            .await?;
        page.validate().map_err(contract)?;
        Ok(page)
    }

    pub async fn read_prompt_flight(
        &self,
        session: &Session,
        request: ReadPromptFlight,
        options: RequestOptions,
    ) -> Result<PromptFlightReadResult> {
        let result: PromptFlightReadResult = self
            .session_call(
                Method::POST,
                "/v1/flights/read",
                session,
                request,
                options,
                false,
            )
            .await?;
        result.validate().map_err(contract)?;
        Ok(result)
    }

    pub async fn open_task(
        &self,
        session: &Session,
        request: OpenTask,
        options: RequestOptions,
    ) -> Result<TaskResult> {
        let result: TaskResult = self
            .session_call(Method::POST, "/v1/tasks", session, request, options, true)
            .await?;
        result.validate().map_err(contract)?;
        Ok(result)
    }

    pub async fn close_task(
        &self,
        session: &Session,
        request: CloseTask,
        options: RequestOptions,
    ) -> Result<TaskResult> {
        let result: TaskResult = self
            .session_call(
                Method::POST,
                "/v1/tasks/close",
                session,
                request,
                options,
                true,
            )
            .await?;
        result.validate().map_err(contract)?;
        Ok(result)
    }

    pub async fn list_tasks(
        &self,
        session: &Session,
        request: ListTasks,
        options: RequestOptions,
    ) -> Result<TaskPage> {
        let page: TaskPage = self
            .session_call(
                Method::POST,
                "/v1/tasks/list",
                session,
                request,
                options,
                false,
            )
            .await?;
        page.validate().map_err(contract)?;
        Ok(page)
    }

    pub async fn read_task(
        &self,
        session: &Session,
        request: ReadTask,
        options: RequestOptions,
    ) -> Result<TaskReadResult> {
        let result: TaskReadResult = self
            .session_call(
                Method::POST,
                "/v1/tasks/read",
                session,
                request,
                options,
                false,
            )
            .await?;
        result.validate().map_err(contract)?;
        Ok(result)
    }

    pub(crate) async fn session_call<T, O>(
        &self,
        method: Method,
        path: &str,
        session: &Session,
        request: T,
        options: RequestOptions,
        mutation: bool,
    ) -> Result<O>
    where
        T: Serialize,
        O: DeserializeOwned,
    {
        self.session_call_resource(
            method,
            path,
            session,
            self.instance_resource()?,
            request,
            options,
            mutation,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn session_call_resource<T, O>(
        &self,
        method: Method,
        path: &str,
        session: &Session,
        resource: ResourcePath,
        request: T,
        options: RequestOptions,
        mutation: bool,
    ) -> Result<O>
    where
        T: Serialize,
        O: DeserializeOwned,
    {
        let expected = options.context();
        let envelope = self.envelope_for(resource, request, &options, mutation)?;
        let bytes = serde_json::to_vec(&envelope).map_err(decode)?;
        let authorization = format!("Bearer {}", session.token().as_str());
        let headers = [
            ("X-RRD-Session", session.session_id().as_str()),
            ("Authorization", authorization.as_str()),
        ];
        let response = self
            .send_raw(
                method,
                path,
                bytes,
                &headers,
                true,
                expected.deadline_unix_ms,
            )
            .await?;
        outcome(StatusCode::OK, response, Some(&expected))
    }

    pub(crate) fn envelope<T: Serialize>(
        &self,
        payload: T,
        options: &RequestOptions,
        mutation: bool,
    ) -> Result<RequestEnvelope<T>> {
        self.envelope_for(self.instance_resource()?, payload, options, mutation)
    }

    pub(crate) fn envelope_for<T: Serialize>(
        &self,
        resource: ResourcePath,
        payload: T,
        options: &RequestOptions,
        mutation: bool,
    ) -> Result<RequestEnvelope<T>> {
        let envelope = RequestEnvelope {
            protocol: PROTOCOL.into(),
            protocol_version: PROTOCOL_VERSION,
            context: options.context(),
            resource,
            payload,
        };
        envelope.validate(mutation).map_err(contract)?;
        Ok(envelope)
    }

    pub(crate) fn instance_resource(&self) -> Result<ResourcePath> {
        Ok(ResourcePath {
            segments: vec![ResourceId::new(
                ResourceKind::Instance,
                self.instance.as_str().to_owned(),
            )
            .map_err(contract)?],
        })
    }
}

pub(crate) fn outcome<T>(
    status: StatusCode,
    response: ResponseEnvelope<T>,
    expected: Option<&RequestContext>,
) -> Result<T> {
    if response.protocol != PROTOCOL || response.protocol_version != PROTOCOL_VERSION {
        return Err(Error::UnsupportedProtocol {
            protocol: response.protocol,
            version: response.protocol_version,
        });
    }
    if expected.is_some_and(|context| {
        response.request_id != context.request_id || response.operation_id != context.operation_id
    }) {
        return Err(Error::ResponseIdentityMismatch);
    }
    match response.outcome {
        ResponseOutcome::Ok { payload } if status.is_success() => Ok(payload),
        ResponseOutcome::Ok { .. } => Err(Error::Decode(
            "successful outcome used error HTTP status".into(),
        )),
        ResponseOutcome::Error { error } => Err(Error::Api { status, error }),
    }
}
