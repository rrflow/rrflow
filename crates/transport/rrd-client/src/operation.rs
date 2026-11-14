//! Typed public operations, envelope construction, and outcome correlation.

mod attunement;
mod configuration;
mod data;
mod knowledge;
mod operations;
mod tasks;
mod vectors;

use crate::error::{contract, decode};

use crate::operations as generated;

use crate::{Error, Result, RrdClient, Session};

use hyper::{Method, StatusCode};

use rrd_contract::{
    AbortTransaction, ActivateVectorQuantizationArtifact, ActivateVectorQuantizationArtifactResult,
    AssembleContext, AuditExport, AuditPage, BeginTransaction, BuildVectorQuantizationArtifact,
    BuildVectorQuantizationArtifactResult, CanonicalId, ChangefeedFollowResult, ChangefeedPage,
    CloseTask, CommitReceipt, CommitTransaction, ContextPacket, CorrelationId,
    CreateInstanceBackup, CreateInstanceBackupResult, DeleteVectorPayloadIndex,
    DeleteVectorPayloadIndexResult, DiagnosticSnapshot, EnsureVectorCollection,
    EnsureVectorCollectionResult, EnsureVectorIndex, EnsureVectorIndexResult,
    EnsureVectorPayloadIndex, EnsureVectorPayloadIndexResult, EstateSnapshot, ExecuteQuery,
    ExportAudit, FollowChangefeed, InstanceBackupCatalogueSnapshot, KnowledgeTreeCatalogue,
    LaunchPromptFlight, LaunchPromptFlightResult, ListInstanceBackups, ListKnowledgeTrees,
    ListPromptFlights, ListTasks, ListVectorCollections, ListVectorQuantizationArtifacts,
    ListVectorQuantizationArtifactsResult, OpenTask, PreviewTransaction, PromptFlightPage,
    PromptFlightReadResult, QueryResult, ReadAudit, ReadChangefeed, ReadDiagnosticSnapshot,
    ReadEstate, ReadPromptFlight, ReadTask, ReplaceKnowledgeTrees, ReplaceKnowledgeTreesResult,
    RequestContext, RequestEnvelope, ResourceId, ResourceKind, ResourcePath, ResponseEnvelope,
    ResponseOutcome, RestoreInstanceBackup, RestoreInstanceBackupResult,
    RetireVectorQuantizationArtifact, RetireVectorQuantizationArtifactResult, RetrieveVectorPoints,
    ScrollVectorPoints, SearchVectors, TaskPage, TaskReadResult, TaskResult, TransactionLease,
    TransactionPreview, VectorCollectionCatalogueSnapshot, VectorPointBatch, VectorPointPage,
    VectorSearchResult, PROTOCOL, PROTOCOL_VERSION,
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
