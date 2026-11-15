// Code generated from rrd-contract; DO NOT EDIT.

package rrd

// OpenAPI SHA-256: 80cc50ab077aa773e47d53135dcd12095240af88129fd84bf0ccbe81ca4ed4cb

const EndpointOpenAPIDocumentSHA256 = "80cc50ab077aa773e47d53135dcd12095240af88129fd84bf0ccbe81ca4ed4cb"

type OperationID string

const (
	OperationAttunementCancel             OperationID = "attunement-cancel"
	OperationAttunementJobList            OperationID = "attunement-job-list"
	OperationAttunementPause              OperationID = "attunement-pause"
	OperationAttunementRead               OperationID = "attunement-read"
	OperationAttunementReceiptRead        OperationID = "attunement-receipt-read"
	OperationAttunementRecoverExpired     OperationID = "attunement-recover-expired"
	OperationAttunementReplanAccept       OperationID = "attunement-replan-accept"
	OperationAttunementReplanPreview      OperationID = "attunement-replan-preview"
	OperationAttunementReplanReceiptRead  OperationID = "attunement-replan-receipt-read"
	OperationAttunementResume             OperationID = "attunement-resume"
	OperationAttunementStart              OperationID = "attunement-start"
	OperationAttunementStep               OperationID = "attunement-step"
	OperationAuditExport                  OperationID = "audit-export"
	OperationAuditRead                    OperationID = "audit-read"
	OperationBackupCreate                 OperationID = "backup-create"
	OperationBackupList                   OperationID = "backup-list"
	OperationCapabilitiesRead             OperationID = "capabilities-read"
	OperationChangefeedFollow             OperationID = "changefeed-follow"
	OperationChangefeedRead               OperationID = "changefeed-read"
	OperationColdHistoryRead              OperationID = "cold-history-read"
	OperationContextAssemble              OperationID = "context-assemble"
	OperationDataSchemaRead               OperationID = "data-schema-read"
	OperationDiagnosticsRead              OperationID = "diagnostics-read"
	OperationDiagnosticsStorageCursor     OperationID = "diagnostics-storage-cursor"
	OperationDiagnosticsStorageRead       OperationID = "diagnostics-storage-read"
	OperationEmbeddingBuiltinActivate     OperationID = "embedding-builtin-activate"
	OperationEndpointCatalogue            OperationID = "endpoint-catalogue"
	OperationEndpointConfigurationApply   OperationID = "endpoint-configuration-apply"
	OperationEndpointConfigurationPreview OperationID = "endpoint-configuration-preview"
	OperationEndpointConfigurationRead    OperationID = "endpoint-configuration-read"
	OperationEngramRead                   OperationID = "engram-read"
	OperationFlightLaunch                 OperationID = "flight-launch"
	OperationFlightList                   OperationID = "flight-list"
	OperationFlightRead                   OperationID = "flight-read"
	OperationHealthLive                   OperationID = "health-live"
	OperationHealthReady                  OperationID = "health-ready"
	OperationKnowledgeTreeList            OperationID = "knowledge-tree-list"
	OperationKnowledgeTreeReplace         OperationID = "knowledge-tree-replace"
	OperationObservationDeliver           OperationID = "observation-deliver"
	OperationObservationPrepare           OperationID = "observation-prepare"
	OperationObservationRead              OperationID = "observation-read"
	OperationOpenapiRead                  OperationID = "openapi-read"
	OperationQueryExecute                 OperationID = "query-execute"
	OperationQueryExport                  OperationID = "query-export"
	OperationQueryIndexEnsure             OperationID = "query-index-ensure"
	OperationQueryIndexList               OperationID = "query-index-list"
	OperationQueryLivePoll                OperationID = "query-live-poll"
	OperationRestoreCreate                OperationID = "restore-create"
	OperationSessionClose                 OperationID = "session-close"
	OperationSessionCreate                OperationID = "session-create"
	OperationSessionRenew                 OperationID = "session-renew"
	OperationSubscriptionClose            OperationID = "subscription-close"
	OperationSubscriptionOpen             OperationID = "subscription-open"
	OperationTaskAssessmentExecute        OperationID = "task-assessment-execute"
	OperationTaskAssessmentRead           OperationID = "task-assessment-read"
	OperationTaskAttemptRead              OperationID = "task-attempt-read"
	OperationTaskBudgetConfigure          OperationID = "task-budget-configure"
	OperationTaskBudgetRead               OperationID = "task-budget-read"
	OperationTaskBudgetResume             OperationID = "task-budget-resume"
	OperationTaskBudgetStop               OperationID = "task-budget-stop"
	OperationTaskClose                    OperationID = "task-close"
	OperationTaskEmbeddingExecute         OperationID = "task-embedding-execute"
	OperationTaskEndpointPropose          OperationID = "task-endpoint-propose"
	OperationTaskList                     OperationID = "task-list"
	OperationTaskOpen                     OperationID = "task-open"
	OperationTaskRead                     OperationID = "task-read"
	OperationTaskWorkerExecute            OperationID = "task-worker-execute"
	OperationTransactionAbort             OperationID = "transaction-abort"
	OperationTransactionBegin             OperationID = "transaction-begin"
	OperationTransactionCommit            OperationID = "transaction-commit"
	OperationTransactionPreview           OperationID = "transaction-preview"
	OperationVectorCollectionEnsure       OperationID = "vector-collection-ensure"
	OperationVectorCollectionList         OperationID = "vector-collection-list"
	OperationVectorIndexEnsure            OperationID = "vector-index-ensure"
	OperationVectorPayloadIndexDelete     OperationID = "vector-payload-index-delete"
	OperationVectorPayloadIndexEnsure     OperationID = "vector-payload-index-ensure"
	OperationVectorPointRetrieve          OperationID = "vector-point-retrieve"
	OperationVectorPointScroll            OperationID = "vector-point-scroll"
	OperationVectorQuantizationActivate   OperationID = "vector-quantization-activate"
	OperationVectorQuantizationBuild      OperationID = "vector-quantization-build"
	OperationVectorQuantizationList       OperationID = "vector-quantization-list"
	OperationVectorQuantizationRetire     OperationID = "vector-quantization-retire"
	OperationVectorSearch                 OperationID = "vector-search"
)

var endpoints = map[OperationID]Endpoint{
	OperationAttunementCancel:             {Method: "POST", Path: "/v1/attunement/cancel", Authentication: "session_bearer", Mutation: true},
	OperationAttunementJobList:            {Method: "POST", Path: "/v1/attunement/jobs/list", Authentication: "session_bearer", Mutation: false},
	OperationAttunementPause:              {Method: "POST", Path: "/v1/attunement/pause", Authentication: "session_bearer", Mutation: true},
	OperationAttunementRead:               {Method: "POST", Path: "/v1/attunement/read", Authentication: "session_bearer", Mutation: false},
	OperationAttunementReceiptRead:        {Method: "POST", Path: "/v1/attunement/receipt/read", Authentication: "session_bearer", Mutation: false},
	OperationAttunementRecoverExpired:     {Method: "POST", Path: "/v1/attunement/recover-expired", Authentication: "session_bearer", Mutation: true},
	OperationAttunementReplanAccept:       {Method: "POST", Path: "/v1/attunement/replan/accept", Authentication: "session_bearer", Mutation: true},
	OperationAttunementReplanPreview:      {Method: "POST", Path: "/v1/attunement/replan/preview", Authentication: "session_bearer", Mutation: false},
	OperationAttunementReplanReceiptRead:  {Method: "POST", Path: "/v1/attunement/replan/receipt/read", Authentication: "session_bearer", Mutation: false},
	OperationAttunementResume:             {Method: "POST", Path: "/v1/attunement/resume", Authentication: "session_bearer", Mutation: true},
	OperationAttunementStart:              {Method: "POST", Path: "/v1/attunement/start", Authentication: "session_bearer", Mutation: true},
	OperationAttunementStep:               {Method: "POST", Path: "/v1/attunement/step", Authentication: "session_bearer", Mutation: true},
	OperationAuditExport:                  {Method: "POST", Path: "/v1/audit/export", Authentication: "session_bearer", Mutation: false},
	OperationAuditRead:                    {Method: "POST", Path: "/v1/audit/read", Authentication: "session_bearer", Mutation: false},
	OperationBackupCreate:                 {Method: "POST", Path: "/v1/backups", Authentication: "session_bearer", Mutation: true},
	OperationBackupList:                   {Method: "POST", Path: "/v1/backups/list", Authentication: "session_bearer", Mutation: false},
	OperationCapabilitiesRead:             {Method: "GET", Path: "/v1/capabilities", Authentication: "public", Mutation: false},
	OperationChangefeedFollow:             {Method: "POST", Path: "/v1/changes/follow", Authentication: "session_bearer", Mutation: false},
	OperationChangefeedRead:               {Method: "POST", Path: "/v1/changes/read", Authentication: "session_bearer", Mutation: false},
	OperationColdHistoryRead:              {Method: "POST", Path: "/v1/storage/cold-history/read", Authentication: "session_bearer", Mutation: false},
	OperationContextAssemble:              {Method: "POST", Path: "/v1/context/assemble", Authentication: "session_bearer", Mutation: false},
	OperationDataSchemaRead:               {Method: "POST", Path: "/v1/data/schema", Authentication: "session_bearer", Mutation: false},
	OperationDiagnosticsRead:              {Method: "POST", Path: "/v1/diagnostics/read", Authentication: "session_bearer", Mutation: false},
	OperationDiagnosticsStorageCursor:     {Method: "POST", Path: "/v1/diagnostics/storage/cursor", Authentication: "session_bearer", Mutation: false},
	OperationDiagnosticsStorageRead:       {Method: "POST", Path: "/v1/diagnostics/storage/read", Authentication: "session_bearer", Mutation: false},
	OperationEmbeddingBuiltinActivate:     {Method: "POST", Path: "/v1/embedding/builtin/activate", Authentication: "session_bearer", Mutation: true},
	OperationEndpointCatalogue:            {Method: "GET", Path: "/v1/schema/endpoints", Authentication: "public", Mutation: false},
	OperationEndpointConfigurationApply:   {Method: "POST", Path: "/v1/configuration/endpoints/apply", Authentication: "session_bearer", Mutation: true},
	OperationEndpointConfigurationPreview: {Method: "POST", Path: "/v1/configuration/endpoints/preview", Authentication: "session_bearer", Mutation: false},
	OperationEndpointConfigurationRead:    {Method: "POST", Path: "/v1/configuration/endpoints/read", Authentication: "session_bearer", Mutation: false},
	OperationEngramRead:                   {Method: "POST", Path: "/v1/engrams/{engram}/read", Authentication: "session_bearer", Mutation: false},
	OperationFlightLaunch:                 {Method: "POST", Path: "/v1/flights", Authentication: "session_bearer", Mutation: true},
	OperationFlightList:                   {Method: "POST", Path: "/v1/flights/list", Authentication: "session_bearer", Mutation: false},
	OperationFlightRead:                   {Method: "POST", Path: "/v1/flights/read", Authentication: "session_bearer", Mutation: false},
	OperationHealthLive:                   {Method: "GET", Path: "/v1/health/live", Authentication: "public", Mutation: false},
	OperationHealthReady:                  {Method: "GET", Path: "/v1/health/ready", Authentication: "public", Mutation: false},
	OperationKnowledgeTreeList:            {Method: "POST", Path: "/v1/knowledge/trees/list", Authentication: "session_bearer", Mutation: false},
	OperationKnowledgeTreeReplace:         {Method: "POST", Path: "/v1/knowledge/trees/replace", Authentication: "session_bearer", Mutation: true},
	OperationObservationDeliver:           {Method: "POST", Path: "/v1/observations/deliver", Authentication: "session_bearer", Mutation: true},
	OperationObservationPrepare:           {Method: "POST", Path: "/v1/observations/prepare", Authentication: "session_bearer", Mutation: true},
	OperationObservationRead:              {Method: "POST", Path: "/v1/observations/read", Authentication: "session_bearer", Mutation: false},
	OperationOpenapiRead:                  {Method: "GET", Path: "/v1/schema/openapi", Authentication: "public", Mutation: false},
	OperationQueryExecute:                 {Method: "POST", Path: "/v1/query", Authentication: "session_bearer", Mutation: false},
	OperationQueryExport:                  {Method: "POST", Path: "/v1/query/export", Authentication: "session_bearer", Mutation: false},
	OperationQueryIndexEnsure:             {Method: "POST", Path: "/v1/query/indexes/ensure", Authentication: "session_bearer", Mutation: true},
	OperationQueryIndexList:               {Method: "POST", Path: "/v1/query/indexes/list", Authentication: "session_bearer", Mutation: false},
	OperationQueryLivePoll:                {Method: "POST", Path: "/v1/query/live/poll", Authentication: "session_bearer", Mutation: false},
	OperationRestoreCreate:                {Method: "POST", Path: "/v1/restores", Authentication: "session_bearer", Mutation: true},
	OperationSessionClose:                 {Method: "DELETE", Path: "/v1/sessions/{session}", Authentication: "session_bearer", Mutation: true},
	OperationSessionCreate:                {Method: "POST", Path: "/v1/sessions", Authentication: "api_key", Mutation: true},
	OperationSessionRenew:                 {Method: "POST", Path: "/v1/sessions/{session}/renew", Authentication: "session_bearer", Mutation: true},
	OperationSubscriptionClose:            {Method: "POST", Path: "/v1/subscriptions/close", Authentication: "session_bearer", Mutation: true},
	OperationSubscriptionOpen:             {Method: "POST", Path: "/v1/subscriptions/open", Authentication: "session_bearer", Mutation: true},
	OperationTaskAssessmentExecute:        {Method: "POST", Path: "/v1/tasks/assessment/execute", Authentication: "session_bearer", Mutation: true},
	OperationTaskAssessmentRead:           {Method: "POST", Path: "/v1/tasks/assessment/read", Authentication: "session_bearer", Mutation: false},
	OperationTaskAttemptRead:              {Method: "POST", Path: "/v1/tasks/attempt/read", Authentication: "session_bearer", Mutation: false},
	OperationTaskBudgetConfigure:          {Method: "POST", Path: "/v1/tasks/budget/configure", Authentication: "session_bearer", Mutation: true},
	OperationTaskBudgetRead:               {Method: "POST", Path: "/v1/tasks/budget/read", Authentication: "session_bearer", Mutation: false},
	OperationTaskBudgetResume:             {Method: "POST", Path: "/v1/tasks/budget/resume", Authentication: "session_bearer", Mutation: true},
	OperationTaskBudgetStop:               {Method: "POST", Path: "/v1/tasks/budget/stop", Authentication: "session_bearer", Mutation: true},
	OperationTaskClose:                    {Method: "POST", Path: "/v1/tasks/close", Authentication: "session_bearer", Mutation: true},
	OperationTaskEmbeddingExecute:         {Method: "POST", Path: "/v1/tasks/embedding/execute", Authentication: "session_bearer", Mutation: true},
	OperationTaskEndpointPropose:          {Method: "POST", Path: "/v1/tasks/endpoint/propose", Authentication: "session_bearer", Mutation: true},
	OperationTaskList:                     {Method: "POST", Path: "/v1/tasks/list", Authentication: "session_bearer", Mutation: false},
	OperationTaskOpen:                     {Method: "POST", Path: "/v1/tasks", Authentication: "session_bearer", Mutation: true},
	OperationTaskRead:                     {Method: "POST", Path: "/v1/tasks/read", Authentication: "session_bearer", Mutation: false},
	OperationTaskWorkerExecute:            {Method: "POST", Path: "/v1/tasks/worker/execute", Authentication: "session_bearer", Mutation: true},
	OperationTransactionAbort:             {Method: "DELETE", Path: "/v1/transactions/{transaction}", Authentication: "session_bearer", Mutation: true},
	OperationTransactionBegin:             {Method: "POST", Path: "/v1/transactions", Authentication: "session_bearer", Mutation: true},
	OperationTransactionCommit:            {Method: "POST", Path: "/v1/transactions/{transaction}/commit", Authentication: "session_bearer", Mutation: true},
	OperationTransactionPreview:           {Method: "POST", Path: "/v1/transactions/{transaction}/preview", Authentication: "session_bearer", Mutation: true},
	OperationVectorCollectionEnsure:       {Method: "POST", Path: "/v1/vector/collections/ensure", Authentication: "session_bearer", Mutation: true},
	OperationVectorCollectionList:         {Method: "POST", Path: "/v1/vector/collections/list", Authentication: "session_bearer", Mutation: false},
	OperationVectorIndexEnsure:            {Method: "POST", Path: "/v1/vector/indexes/ensure", Authentication: "session_bearer", Mutation: true},
	OperationVectorPayloadIndexDelete:     {Method: "POST", Path: "/v1/vector/payload-indexes/delete", Authentication: "session_bearer", Mutation: true},
	OperationVectorPayloadIndexEnsure:     {Method: "POST", Path: "/v1/vector/payload-indexes/ensure", Authentication: "session_bearer", Mutation: true},
	OperationVectorPointRetrieve:          {Method: "POST", Path: "/v1/vector/points/retrieve", Authentication: "session_bearer", Mutation: false},
	OperationVectorPointScroll:            {Method: "POST", Path: "/v1/vector/points/scroll", Authentication: "session_bearer", Mutation: false},
	OperationVectorQuantizationActivate:   {Method: "POST", Path: "/v1/vector/quantizations/activate", Authentication: "session_bearer", Mutation: true},
	OperationVectorQuantizationBuild:      {Method: "POST", Path: "/v1/vector/quantizations/build", Authentication: "session_bearer", Mutation: true},
	OperationVectorQuantizationList:       {Method: "POST", Path: "/v1/vector/quantizations/list", Authentication: "session_bearer", Mutation: false},
	OperationVectorQuantizationRetire:     {Method: "POST", Path: "/v1/vector/quantizations/retire", Authentication: "session_bearer", Mutation: true},
	OperationVectorSearch:                 {Method: "POST", Path: "/v1/vector/search", Authentication: "session_bearer", Mutation: false},
}
