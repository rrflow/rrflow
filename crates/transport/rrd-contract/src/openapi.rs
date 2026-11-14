//! The OpenAPI document derived from the endpoint catalogue.

use crate::*;
use schemars::JsonSchema;

/// Deterministic OpenAPI 3.1 projection of the authoritative RRD v1 endpoint
/// catalogue and Rust wire types.
///
/// This document is the language-neutral generator input for supported SDKs;
/// it is not a separately maintained description of the protocol.
pub fn openapi_document() -> Result<serde_json::Value> {
    let catalogue = endpoint_catalogue();
    catalogue.validate()?;
    let signal_catalogue = serde_json::from_str::<serde_json::Value>(
        generated_signal_catalogue::SIGNAL_CATALOGUE_JSON,
    )
    .map_err(|error| ContractError(format!("generated signal catalogue is invalid: {error}")))?;
    let signal_catalogue_sha256 = generated_signal_catalogue::SIGNAL_CATALOGUE_SHA256;
    let query_value_schema = openapi_query_value_schema()?;
    let vector_payload_filter_schema = openapi_vector_payload_filter_schema()?;
    let mut websocket_frame_schema = schema_json::<WebSocketFrame>();
    rebase_local_schema_refs(
        &mut websocket_frame_schema,
        "#/components/schemas/WebSocketFrame",
    );
    let mut paths = serde_json::Map::new();
    for descriptor in &catalogue.endpoints {
        let method = match descriptor.method {
            HttpMethod::Get => "get",
            HttpMethod::Post => "post",
            HttpMethod::Delete => "delete",
        };
        let mut operation = serde_json::Map::new();
        operation.insert(
            "operationId".into(),
            serde_json::Value::String(descriptor.operation.as_str().into()),
        );
        operation.insert(
            "summary".into(),
            serde_json::Value::String(descriptor.summary.clone()),
        );
        operation.insert(
            "description".into(),
            serde_json::Value::String(descriptor.description.clone()),
        );
        operation.insert(
            "tags".into(),
            serde_json::json!([descriptor.family.as_str()]),
        );
        operation.insert(
            "x-rrd-action".into(),
            serde_json::to_value(descriptor.action)
                .map_err(|error| ContractError(error.to_string()))?,
        );
        operation.insert(
            "x-rrd-mutation".into(),
            serde_json::Value::Bool(descriptor.mutation),
        );
        operation.insert(
            "x-rrd-retry".into(),
            serde_json::to_value(descriptor.retry)
                .map_err(|error| ContractError(error.to_string()))?,
        );
        operation.insert(
            "security".into(),
            match descriptor.authentication {
                EndpointAuthentication::Public => serde_json::json!([]),
                EndpointAuthentication::ApiKey => serde_json::json!([{"rrdApiKey": []}]),
                EndpointAuthentication::SessionBearer => {
                    serde_json::json!([{"rrdBearer": []}])
                }
            },
        );
        let parameters = openapi_parameters(descriptor);
        if !parameters.is_empty() {
            operation.insert("parameters".into(), serde_json::Value::Array(parameters));
        }
        if descriptor.method != HttpMethod::Get {
            let mut request_schema = request_envelope_schema(&descriptor.request_type)?;
            rebase_local_schema_refs(
                &mut request_schema,
                &openapi_schema_pointer(
                    &descriptor.path,
                    method,
                    "requestBody/content/application~1json/schema",
                ),
            );
            operation.insert(
                "requestBody".into(),
                serde_json::json!({
                    "required": true,
                    "content": {
                        "application/json": {
                            "schema": request_schema
                        }
                    }
                }),
            );
        }
        let response_schema = response_envelope_schema(&descriptor.response_type)?;
        let mut success_schema = response_schema.clone();
        rebase_local_schema_refs(
            &mut success_schema,
            &openapi_schema_pointer(
                &descriptor.path,
                method,
                "responses/200/content/application~1json/schema",
            ),
        );
        let mut error_schema = response_schema;
        rebase_local_schema_refs(
            &mut error_schema,
            &openapi_schema_pointer(
                &descriptor.path,
                method,
                "responses/default/content/application~1json/schema",
            ),
        );
        operation.insert(
            "responses".into(),
            serde_json::json!({
                "200": {
                    "description": "Typed RRD response",
                    "content": {"application/json": {"schema": success_schema}}
                },
                "default": {
                    "description": "Typed RRD error response",
                    "content": {"application/json": {"schema": error_schema}}
                }
            }),
        );
        let path = paths
            .entry(descriptor.path.clone())
            .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
        path.as_object_mut()
            .expect("OpenAPI path item is constructed as an object")
            .insert(method.into(), serde_json::Value::Object(operation));
    }

    Ok(canonical_json(serde_json::json!({
        "openapi": "3.1.0",
        "info": {
            "title": "RRFlow Durable Runtime API",
            "version": format!("{PROTOCOL_VERSION}.0.0")
        },
        "servers": [{"url": "http://127.0.0.1:9477"}],
        "tags": catalogue
            .families
            .iter()
            .map(|family| serde_json::json!({
                "name": family.family.as_str(),
                "description": family.description,
                "x-rrd-title": family.title
            }))
            .collect::<Vec<_>>(),
        "paths": paths,
        "components": {
            "schemas": {
                "QueryValue": query_value_schema,
                "WebSocketFrame": websocket_frame_schema,
                "VectorPayloadFilter": vector_payload_filter_schema
            },
            "securitySchemes": {
                "rrdApiKey": {
                    "type": "apiKey",
                    "in": "header",
                    "name": "Authorization",
                    "description": "Authorization: ApiKey <credential>; X-RRD-Principal is also required"
                },
                "rrdBearer": {
                    "type": "http",
                    "scheme": "bearer",
                    "description": "Session bearer; X-RRD-Session is also required"
                }
            }
        },
        "x-rrd-protocol": PROTOCOL,
        "x-rrd-protocol-version": PROTOCOL_VERSION,
        "x-rrd-endpoint-count": catalogue.endpoints.len(),
        "x-rrd-signal-catalogue": signal_catalogue,
        "x-rrd-signal-catalogue-sha256": signal_catalogue_sha256,
        "x-rrd-websocket-endpoint-count": catalogue.websocket_endpoints.len(),
        "x-rrd-websocket-endpoints": catalogue.websocket_endpoints
    })))
}

/// Normalize object insertion order so public protocol bytes are independent
/// of workspace feature unification. In particular, DataFusion enables
/// `serde_json/preserve_order`; RRD must not emit a different OpenAPI digest
/// merely because another workspace package activated that representation.
pub(crate) fn canonical_json(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Array(values) => {
            serde_json::Value::Array(values.into_iter().map(canonical_json).collect())
        }
        serde_json::Value::Object(values) => {
            let mut entries = values.into_iter().collect::<Vec<_>>();
            entries.sort_by(|left, right| left.0.cmp(&right.0));
            let mut canonical = serde_json::Map::with_capacity(entries.len());
            for (name, value) in entries {
                canonical.insert(name, canonical_json(value));
            }
            serde_json::Value::Object(canonical)
        }
        scalar => scalar,
    }
}

pub(crate) fn openapi_schema_pointer(path: &str, method: &str, suffix: &str) -> String {
    let escaped = path.replace('~', "~0").replace('/', "~1");
    format!("#/paths/{escaped}/{method}/{suffix}")
}

pub(crate) fn rebase_local_schema_refs(value: &mut serde_json::Value, schema_pointer: &str) {
    match value {
        serde_json::Value::Array(values) => {
            for value in values {
                rebase_local_schema_refs(value, schema_pointer);
            }
        }
        serde_json::Value::Object(object) => {
            if let Some(serde_json::Value::String(reference)) = object.get_mut("$ref") {
                if reference == "#" || reference == "#/$defs/QueryValue" {
                    *reference = "#/components/schemas/QueryValue".into();
                } else if reference == "#/$defs/VectorPayloadFilter" {
                    *reference = "#/components/schemas/VectorPayloadFilter".into();
                } else if let Some(suffix) = reference.strip_prefix("#/") {
                    *reference = format!("{schema_pointer}/{suffix}");
                }
            }
            for value in object.values_mut() {
                rebase_local_schema_refs(value, schema_pointer);
            }
            let remove_definitions =
                if let Some(serde_json::Value::Object(definitions)) = object.get_mut("$defs") {
                    definitions.remove("QueryValue");
                    definitions.remove("VectorPayloadFilter");
                    definitions.is_empty()
                } else {
                    false
                };
            if remove_definitions {
                object.remove("$defs");
            }
        }
        _ => {}
    }
}

pub(crate) fn openapi_query_value_schema() -> Result<serde_json::Value> {
    let generated = serde_json::to_value(schemars::schema_for!(QueryValue))
        .map_err(|error| ContractError(error.to_string()))?;
    let definition = generated
        .get("$defs")
        .and_then(serde_json::Value::as_object)
        .and_then(|definitions| definitions.get("QueryValue"))
        .cloned();
    let mut schema = definition.unwrap_or(generated);
    rebase_local_schema_refs(&mut schema, "#/components/schemas/QueryValue");
    Ok(schema)
}

pub(crate) fn openapi_vector_payload_filter_schema() -> Result<serde_json::Value> {
    let generated = serde_json::to_value(schemars::schema_for!(VectorPayloadFilter))
        .map_err(|error| ContractError(error.to_string()))?;
    let definition = generated
        .get("$defs")
        .and_then(serde_json::Value::as_object)
        .and_then(|definitions| definitions.get("VectorPayloadFilter"))
        .cloned();
    let mut schema = definition.unwrap_or(generated);
    rebase_local_schema_refs(&mut schema, "#/components/schemas/VectorPayloadFilter");
    Ok(schema)
}

pub(crate) fn openapi_parameters(descriptor: &EndpointDescriptor) -> Vec<serde_json::Value> {
    let mut parameters = Vec::new();
    for name in ["engram", "session", "transaction"] {
        if descriptor.path.contains(&format!("{{{name}}}")) {
            parameters.push(serde_json::json!({
                "name": name,
                "in": "path",
                "required": true,
                "schema": {"type": "string", "minLength": 1, "maxLength": MAX_ID_BYTES}
            }));
        }
    }
    match descriptor.authentication {
        EndpointAuthentication::Public => {}
        EndpointAuthentication::ApiKey => parameters.push(serde_json::json!({
            "name": "X-RRD-Principal",
            "in": "header",
            "required": true,
            "schema": {"type": "string", "minLength": 1, "maxLength": MAX_ID_BYTES}
        })),
        EndpointAuthentication::SessionBearer => parameters.push(serde_json::json!({
            "name": "X-RRD-Session",
            "in": "header",
            "required": true,
            "schema": {"type": "string", "minLength": 1, "maxLength": MAX_ID_BYTES}
        })),
    }
    parameters
}

pub(crate) fn schema_json<T: JsonSchema>() -> serde_json::Value {
    let schema = schemars::generate::SchemaSettings::draft2020_12()
        .with(|settings| settings.inline_subschemas = true)
        .into_generator()
        .into_root_schema_for::<T>();
    serde_json::to_value(schema).expect("JsonSchema output must serialize as JSON")
}

pub(crate) fn request_envelope_schema(name: &str) -> Result<serde_json::Value> {
    let schema = match name {
        "ActivateBuiltinEmbedding" => schema_json::<RequestEnvelope<ActivateBuiltinEmbedding>>(),
        "ConfigureTaskBudget" => schema_json::<RequestEnvelope<ConfigureTaskBudget>>(),
        "ExecuteTaskEmbedding" => schema_json::<RequestEnvelope<ExecuteTaskEmbedding>>(),
        "EndpointSelectionRequest" => schema_json::<RequestEnvelope<EndpointSelectionRequest>>(),
        "TaskWorkerRequest" => schema_json::<RequestEnvelope<TaskWorkerRequest>>(),
        "TaskAssessmentRequest" => schema_json::<RequestEnvelope<TaskAssessmentRequest>>(),
        "PrepareObservationDelivery" => {
            schema_json::<RequestEnvelope<PrepareObservationDelivery>>()
        }
        "DeliverObservation" => schema_json::<RequestEnvelope<DeliverObservation>>(),
        "ReadObservationDelivery" => schema_json::<RequestEnvelope<ReadObservationDelivery>>(),
        "ReadTaskAssessment" => schema_json::<RequestEnvelope<ReadTaskAssessment>>(),
        "ReadTaskAttempt" => schema_json::<RequestEnvelope<ReadTaskAttempt>>(),
        "ReadTaskBudget" => schema_json::<RequestEnvelope<ReadTaskBudget>>(),
        "StopTaskBudget" => schema_json::<RequestEnvelope<StopTaskBudget>>(),
        "ResumeTaskBudget" => schema_json::<RequestEnvelope<ResumeTaskBudget>>(),
        "AbortTransaction" => schema_json::<RequestEnvelope<AbortTransaction>>(),
        "AssembleContext" => schema_json::<RequestEnvelope<AssembleContext>>(),
        "BeginTransaction" => schema_json::<RequestEnvelope<BeginTransaction>>(),
        "CloseSession" => schema_json::<RequestEnvelope<CloseSession>>(),
        "CommitTransaction" => schema_json::<RequestEnvelope<CommitTransaction>>(),
        "CreateInstanceBackup" => schema_json::<RequestEnvelope<CreateInstanceBackup>>(),
        "CreateSession" => schema_json::<RequestEnvelope<CreateSession>>(),
        "ExecuteQuery" => schema_json::<RequestEnvelope<ExecuteQuery>>(),
        "ReadEndpointConfiguration" => schema_json::<RequestEnvelope<ReadEndpointConfiguration>>(),
        "PreviewEndpointConfiguration" => {
            schema_json::<RequestEnvelope<PreviewEndpointConfiguration>>()
        }
        "ApplyEndpointConfiguration" => {
            schema_json::<RequestEnvelope<ApplyEndpointConfiguration>>()
        }
        "ExportQuery" => schema_json::<RequestEnvelope<ExportQuery>>(),
        "ExportAudit" => schema_json::<RequestEnvelope<ExportAudit>>(),
        "EnsureQueryIndex" => schema_json::<RequestEnvelope<EnsureQueryIndex>>(),
        "EnsureVectorCollection" => schema_json::<RequestEnvelope<EnsureVectorCollection>>(),
        "ListKnowledgeTrees" => schema_json::<RequestEnvelope<ListKnowledgeTrees>>(),
        "ReplaceKnowledgeTrees" => schema_json::<RequestEnvelope<ReplaceKnowledgeTrees>>(),
        "ListQueryIndexes" => schema_json::<RequestEnvelope<ListQueryIndexes>>(),
        "ListVectorCollections" => schema_json::<RequestEnvelope<ListVectorCollections>>(),
        "OpenSubscription" => schema_json::<RequestEnvelope<OpenSubscription>>(),
        "PollLiveQuery" => schema_json::<RequestEnvelope<PollLiveQuery>>(),
        "FollowChangefeed" => schema_json::<RequestEnvelope<FollowChangefeed>>(),
        "ListInstanceBackups" => schema_json::<RequestEnvelope<ListInstanceBackups>>(),
        "PreviewTransaction" => schema_json::<RequestEnvelope<PreviewTransaction>>(),
        "ReadAudit" => schema_json::<RequestEnvelope<ReadAudit>>(),
        "ReadChangefeed" => schema_json::<RequestEnvelope<ReadChangefeed>>(),
        "ReadColdHistory" => schema_json::<RequestEnvelope<ReadColdHistory>>(),
        "ReadDiagnosticSnapshot" => schema_json::<RequestEnvelope<ReadDiagnosticSnapshot>>(),
        "InspectStorageCursor" => schema_json::<RequestEnvelope<InspectStorageCursor>>(),
        "ReadStorageDiagnostics" => schema_json::<RequestEnvelope<ReadStorageDiagnostics>>(),
        "ReadDataSchema" => schema_json::<RequestEnvelope<ReadDataSchema>>(),
        "LaunchPromptFlight" => schema_json::<RequestEnvelope<LaunchPromptFlight>>(),
        "ListPromptFlights" => schema_json::<RequestEnvelope<ListPromptFlights>>(),
        "ReadPromptFlight" => schema_json::<RequestEnvelope<ReadPromptFlight>>(),
        "OpenTask" => schema_json::<RequestEnvelope<OpenTask>>(),
        "CloseTask" => schema_json::<RequestEnvelope<CloseTask>>(),
        "ListTasks" => schema_json::<RequestEnvelope<ListTasks>>(),
        "ReadTask" => schema_json::<RequestEnvelope<ReadTask>>(),
        "ReadAttunement" => schema_json::<RequestEnvelope<ReadAttunement>>(),
        "ListAttunementJobs" => schema_json::<RequestEnvelope<ListAttunementJobs>>(),
        "StartAttunement" => schema_json::<RequestEnvelope<StartAttunement>>(),
        "StepAttunement" => schema_json::<RequestEnvelope<StepAttunement>>(),
        "PauseAttunement" => schema_json::<RequestEnvelope<PauseAttunement>>(),
        "RecoverExpiredAttunement" => schema_json::<RequestEnvelope<RecoverExpiredAttunement>>(),
        "ResumeAttunementWithLease" => schema_json::<RequestEnvelope<ResumeAttunementWithLease>>(),
        "PreviewAttunementReplan" => schema_json::<RequestEnvelope<PreviewAttunementReplan>>(),
        "AcceptAttunementReplan" => schema_json::<RequestEnvelope<AcceptAttunementReplan>>(),
        "ReadAttunementReceipt" => schema_json::<RequestEnvelope<ReadAttunementReceipt>>(),
        "CancelAttunement" => schema_json::<RequestEnvelope<CancelAttunement>>(),
        "ReadEstate" => schema_json::<RequestEnvelope<ReadEstate>>(),
        "RenewSession" => schema_json::<RequestEnvelope<RenewSession>>(),
        "RestoreInstanceBackup" => schema_json::<RequestEnvelope<RestoreInstanceBackup>>(),
        "RetrieveVectorPoints" => schema_json::<RequestEnvelope<RetrieveVectorPoints>>(),
        "ScrollVectorPoints" => schema_json::<RequestEnvelope<ScrollVectorPoints>>(),
        "SearchVectors" => schema_json::<RequestEnvelope<SearchVectors>>(),
        "EnsureVectorIndex" => schema_json::<RequestEnvelope<EnsureVectorIndex>>(),
        "EnsureVectorPayloadIndex" => schema_json::<RequestEnvelope<EnsureVectorPayloadIndex>>(),
        "DeleteVectorPayloadIndex" => schema_json::<RequestEnvelope<DeleteVectorPayloadIndex>>(),
        "BuildVectorQuantizationArtifact" => {
            schema_json::<RequestEnvelope<BuildVectorQuantizationArtifact>>()
        }
        "ListVectorQuantizationArtifacts" => {
            schema_json::<RequestEnvelope<ListVectorQuantizationArtifacts>>()
        }
        "ActivateVectorQuantizationArtifact" => {
            schema_json::<RequestEnvelope<ActivateVectorQuantizationArtifact>>()
        }
        "RetireVectorQuantizationArtifact" => {
            schema_json::<RequestEnvelope<RetireVectorQuantizationArtifact>>()
        }
        "CloseSubscription" => schema_json::<RequestEnvelope<CloseSubscription>>(),
        _ => return invalid(format!("no public request schema for {name}")),
    };
    Ok(schema)
}

pub(crate) fn response_envelope_schema(name: &str) -> Result<serde_json::Value> {
    let schema = match name {
        "EmbeddingModelCatalogue" => schema_json::<ResponseEnvelope<EmbeddingModelCatalogue>>(),
        "GenerateEmbeddingsResult" => schema_json::<ResponseEnvelope<GenerateEmbeddingsResult>>(),
        "TaskAttemptView" => schema_json::<ResponseEnvelope<TaskAttemptView>>(),
        "TaskAssessmentView" => schema_json::<ResponseEnvelope<TaskAssessmentView>>(),
        "TaskAssessmentOutput" => schema_json::<ResponseEnvelope<TaskAssessmentOutput>>(),
        "EndpointSelectionProposal" => schema_json::<ResponseEnvelope<EndpointSelectionProposal>>(),
        "TaskWorkerOutput" => schema_json::<ResponseEnvelope<TaskWorkerOutput>>(),
        "TaskBudgetView" => schema_json::<ResponseEnvelope<TaskBudgetView>>(),
        "AuditExport" => schema_json::<ResponseEnvelope<AuditExport>>(),
        "AuditPage" => schema_json::<ResponseEnvelope<AuditPage>>(),
        "ChangefeedFollowResult" => schema_json::<ResponseEnvelope<ChangefeedFollowResult>>(),
        "ChangefeedPage" => schema_json::<ResponseEnvelope<ChangefeedPage>>(),
        "ColdHistoryReadResult" => schema_json::<ResponseEnvelope<ColdHistoryReadResult>>(),
        "CommitReceipt" => schema_json::<ResponseEnvelope<CommitReceipt>>(),
        "ContextPacket" => schema_json::<ResponseEnvelope<ContextPacket>>(),
        "CloseSubscriptionResult" => schema_json::<ResponseEnvelope<CloseSubscriptionResult>>(),
        "CreateInstanceBackupResult" => {
            schema_json::<ResponseEnvelope<CreateInstanceBackupResult>>()
        }
        "DiagnosticSnapshot" => schema_json::<ResponseEnvelope<DiagnosticSnapshot>>(),
        "StorageCursorSnapshot" => schema_json::<ResponseEnvelope<StorageCursorSnapshot>>(),
        "DiagnosticStorageSnapshot" => schema_json::<ResponseEnvelope<DiagnosticStorageSnapshot>>(),
        "DataSchemaSnapshot" => schema_json::<ResponseEnvelope<DataSchemaSnapshot>>(),
        "LaunchPromptFlightResult" => schema_json::<ResponseEnvelope<LaunchPromptFlightResult>>(),
        "PromptFlightPage" => schema_json::<ResponseEnvelope<PromptFlightPage>>(),
        "PromptFlightReadResult" => schema_json::<ResponseEnvelope<PromptFlightReadResult>>(),
        "TaskResult" => schema_json::<ResponseEnvelope<TaskResult>>(),
        "TaskPage" => schema_json::<ResponseEnvelope<TaskPage>>(),
        "TaskReadResult" => schema_json::<ResponseEnvelope<TaskReadResult>>(),
        "ObservationDeliverySnapshot" => {
            schema_json::<ResponseEnvelope<ObservationDeliverySnapshot>>()
        }
        "ObservationDeliveryReadResult" => {
            schema_json::<ResponseEnvelope<ObservationDeliveryReadResult>>()
        }
        "AttunementReadResult" => schema_json::<ResponseEnvelope<AttunementReadResult>>(),
        "AttunementJobPage" => schema_json::<ResponseEnvelope<AttunementJobPage>>(),
        "AttunementControlResponse" => schema_json::<ResponseEnvelope<AttunementControlResponse>>(),
        "AttunementReplanPreview" => schema_json::<ResponseEnvelope<AttunementReplanPreview>>(),
        "AttunementReplanResult" => schema_json::<ResponseEnvelope<AttunementReplanResult>>(),
        "AttunementReplanReceiptReadResult" => {
            schema_json::<ResponseEnvelope<AttunementReplanReceiptReadResult>>()
        }
        "AttunementReceiptReadResult" => {
            schema_json::<ResponseEnvelope<AttunementReceiptReadResult>>()
        }
        "AttunementCancelResult" => schema_json::<ResponseEnvelope<AttunementCancelResult>>(),
        "EndpointCatalogue" => schema_json::<ResponseEnvelope<EndpointCatalogue>>(),
        "EstateSnapshot" => schema_json::<ResponseEnvelope<EstateSnapshot>>(),
        "InstanceBackupCatalogueSnapshot" => {
            schema_json::<ResponseEnvelope<InstanceBackupCatalogueSnapshot>>()
        }
        "KnowledgeTreeCatalogue" => schema_json::<ResponseEnvelope<KnowledgeTreeCatalogue>>(),
        "ReplaceKnowledgeTreesResult" => {
            schema_json::<ResponseEnvelope<ReplaceKnowledgeTreesResult>>()
        }
        "Liveness" => schema_json::<ResponseEnvelope<Liveness>>(),
        "LiveQueryDeltaResult" => schema_json::<ResponseEnvelope<LiveQueryDeltaResult>>(),
        "OpenApiDocument" => schema_json::<ResponseEnvelope<serde_json::Value>>(),
        "OpenSubscriptionResult" => schema_json::<ResponseEnvelope<OpenSubscriptionResult>>(),
        "QueryResult" => schema_json::<ResponseEnvelope<QueryResult>>(),
        "EndpointConfiguration" => schema_json::<ResponseEnvelope<EndpointConfiguration>>(),
        "EndpointConfigurationPlan" => schema_json::<ResponseEnvelope<EndpointConfigurationPlan>>(),
        "QueryExport" => schema_json::<ResponseEnvelope<QueryExport>>(),
        "EnsureQueryIndexResult" => schema_json::<ResponseEnvelope<EnsureQueryIndexResult>>(),
        "EnsureVectorCollectionResult" => {
            schema_json::<ResponseEnvelope<EnsureVectorCollectionResult>>()
        }
        "QueryIndexCatalogueSnapshot" => {
            schema_json::<ResponseEnvelope<QueryIndexCatalogueSnapshot>>()
        }
        "Readiness" => schema_json::<ResponseEnvelope<Readiness>>(),
        "RestoreInstanceBackupResult" => {
            schema_json::<ResponseEnvelope<RestoreInstanceBackupResult>>()
        }
        "ServiceCapabilities" => schema_json::<ResponseEnvelope<ServiceCapabilities>>(),
        "SessionLease" => schema_json::<ResponseEnvelope<SessionLease>>(),
        "SessionTermination" => schema_json::<ResponseEnvelope<SessionTermination>>(),
        "TransactionLease" => schema_json::<ResponseEnvelope<TransactionLease>>(),
        "TransactionPreview" => schema_json::<ResponseEnvelope<TransactionPreview>>(),
        "VectorPointBatch" => schema_json::<ResponseEnvelope<VectorPointBatch>>(),
        "VectorPointPage" => schema_json::<ResponseEnvelope<VectorPointPage>>(),
        "VectorCollectionCatalogueSnapshot" => {
            schema_json::<ResponseEnvelope<VectorCollectionCatalogueSnapshot>>()
        }
        "VectorSearchResult" => schema_json::<ResponseEnvelope<VectorSearchResult>>(),
        "EnsureVectorIndexResult" => schema_json::<ResponseEnvelope<EnsureVectorIndexResult>>(),
        "EnsureVectorPayloadIndexResult" => {
            schema_json::<ResponseEnvelope<EnsureVectorPayloadIndexResult>>()
        }
        "DeleteVectorPayloadIndexResult" => {
            schema_json::<ResponseEnvelope<DeleteVectorPayloadIndexResult>>()
        }
        "BuildVectorQuantizationArtifactResult" => {
            schema_json::<ResponseEnvelope<BuildVectorQuantizationArtifactResult>>()
        }
        "ListVectorQuantizationArtifactsResult" => {
            schema_json::<ResponseEnvelope<ListVectorQuantizationArtifactsResult>>()
        }
        "ActivateVectorQuantizationArtifactResult" => {
            schema_json::<ResponseEnvelope<ActivateVectorQuantizationArtifactResult>>()
        }
        "RetireVectorQuantizationArtifactResult" => {
            schema_json::<ResponseEnvelope<RetireVectorQuantizationArtifactResult>>()
        }
        _ => return invalid(format!("no public response schema for {name}")),
    };
    Ok(schema)
}
