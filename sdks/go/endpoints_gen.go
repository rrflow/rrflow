// Code generated from rrd-contract; DO NOT EDIT.

package rrd

// OpenAPI SHA-256: 6a8d3e490eeb0e13a10465b752dee165223c7097e18ccd08acb79b38934e4650

const EndpointOpenAPIDocumentSHA256 = "6a8d3e490eeb0e13a10465b752dee165223c7097e18ccd08acb79b38934e4650"

type OperationID string

const (
	OperationAuditExport              OperationID = "audit-export"
	OperationAuditRead                OperationID = "audit-read"
	OperationBackupCreate             OperationID = "backup-create"
	OperationBackupList               OperationID = "backup-list"
	OperationCapabilitiesRead         OperationID = "capabilities-read"
	OperationChangefeedFollow         OperationID = "changefeed-follow"
	OperationChangefeedRead           OperationID = "changefeed-read"
	OperationContextAssemble          OperationID = "context-assemble"
	OperationDataSchemaRead           OperationID = "data-schema-read"
	OperationDiagnosticsRead          OperationID = "diagnostics-read"
	OperationDiagnosticsStorageCursor OperationID = "diagnostics-storage-cursor"
	OperationDiagnosticsStorageRead   OperationID = "diagnostics-storage-read"
	OperationEndpointCatalogue        OperationID = "endpoint-catalogue"
	OperationEstateRead               OperationID = "estate-read"
	OperationFlightLaunch             OperationID = "flight-launch"
	OperationFlightList               OperationID = "flight-list"
	OperationFlightRead               OperationID = "flight-read"
	OperationHealthLive               OperationID = "health-live"
	OperationHealthReady              OperationID = "health-ready"
	OperationOpenapiRead              OperationID = "openapi-read"
	OperationQueryExecute             OperationID = "query-execute"
	OperationQueryExport              OperationID = "query-export"
	OperationQueryIndexEnsure         OperationID = "query-index-ensure"
	OperationQueryIndexList           OperationID = "query-index-list"
	OperationQueryLivePoll            OperationID = "query-live-poll"
	OperationRestoreCreate            OperationID = "restore-create"
	OperationSessionClose             OperationID = "session-close"
	OperationSessionCreate            OperationID = "session-create"
	OperationSessionRenew             OperationID = "session-renew"
	OperationSubscriptionClose        OperationID = "subscription-close"
	OperationSubscriptionOpen         OperationID = "subscription-open"
	OperationTaskClose                OperationID = "task-close"
	OperationTaskList                 OperationID = "task-list"
	OperationTaskOpen                 OperationID = "task-open"
	OperationTaskRead                 OperationID = "task-read"
	OperationTransactionAbort         OperationID = "transaction-abort"
	OperationTransactionBegin         OperationID = "transaction-begin"
	OperationTransactionCommit        OperationID = "transaction-commit"
	OperationTransactionPreview       OperationID = "transaction-preview"
	OperationVectorCollectionEnsure   OperationID = "vector-collection-ensure"
	OperationVectorCollectionList     OperationID = "vector-collection-list"
	OperationVectorPointRetrieve      OperationID = "vector-point-retrieve"
	OperationVectorPointScroll        OperationID = "vector-point-scroll"
	OperationVectorSearch             OperationID = "vector-search"
)

var endpoints = map[OperationID]Endpoint{
	OperationAuditExport:              {Method: "POST", Path: "/v1/audit/export", Authentication: "session_bearer", Mutation: false},
	OperationAuditRead:                {Method: "POST", Path: "/v1/audit/read", Authentication: "session_bearer", Mutation: false},
	OperationBackupCreate:             {Method: "POST", Path: "/v1/backups", Authentication: "session_bearer", Mutation: true},
	OperationBackupList:               {Method: "POST", Path: "/v1/backups/list", Authentication: "session_bearer", Mutation: false},
	OperationCapabilitiesRead:         {Method: "GET", Path: "/v1/capabilities", Authentication: "public", Mutation: false},
	OperationChangefeedFollow:         {Method: "POST", Path: "/v1/changes/follow", Authentication: "session_bearer", Mutation: false},
	OperationChangefeedRead:           {Method: "POST", Path: "/v1/changes/read", Authentication: "session_bearer", Mutation: false},
	OperationContextAssemble:          {Method: "POST", Path: "/v1/context/assemble", Authentication: "session_bearer", Mutation: false},
	OperationDataSchemaRead:           {Method: "POST", Path: "/v1/data/schema", Authentication: "session_bearer", Mutation: false},
	OperationDiagnosticsRead:          {Method: "POST", Path: "/v1/diagnostics/read", Authentication: "session_bearer", Mutation: false},
	OperationDiagnosticsStorageCursor: {Method: "POST", Path: "/v1/diagnostics/storage/cursor", Authentication: "session_bearer", Mutation: false},
	OperationDiagnosticsStorageRead:   {Method: "POST", Path: "/v1/diagnostics/storage/read", Authentication: "session_bearer", Mutation: false},
	OperationEndpointCatalogue:        {Method: "GET", Path: "/v1/schema/endpoints", Authentication: "public", Mutation: false},
	OperationEstateRead:               {Method: "POST", Path: "/v1/estates/{estate}/read", Authentication: "session_bearer", Mutation: false},
	OperationFlightLaunch:             {Method: "POST", Path: "/v1/flights", Authentication: "session_bearer", Mutation: true},
	OperationFlightList:               {Method: "POST", Path: "/v1/flights/list", Authentication: "session_bearer", Mutation: false},
	OperationFlightRead:               {Method: "POST", Path: "/v1/flights/read", Authentication: "session_bearer", Mutation: false},
	OperationHealthLive:               {Method: "GET", Path: "/v1/health/live", Authentication: "public", Mutation: false},
	OperationHealthReady:              {Method: "GET", Path: "/v1/health/ready", Authentication: "public", Mutation: false},
	OperationOpenapiRead:              {Method: "GET", Path: "/v1/schema/openapi", Authentication: "public", Mutation: false},
	OperationQueryExecute:             {Method: "POST", Path: "/v1/query", Authentication: "session_bearer", Mutation: false},
	OperationQueryExport:              {Method: "POST", Path: "/v1/query/export", Authentication: "session_bearer", Mutation: false},
	OperationQueryIndexEnsure:         {Method: "POST", Path: "/v1/query/indexes/ensure", Authentication: "session_bearer", Mutation: true},
	OperationQueryIndexList:           {Method: "POST", Path: "/v1/query/indexes/list", Authentication: "session_bearer", Mutation: false},
	OperationQueryLivePoll:            {Method: "POST", Path: "/v1/query/live/poll", Authentication: "session_bearer", Mutation: false},
	OperationRestoreCreate:            {Method: "POST", Path: "/v1/restores", Authentication: "session_bearer", Mutation: true},
	OperationSessionClose:             {Method: "DELETE", Path: "/v1/sessions/{session}", Authentication: "session_bearer", Mutation: true},
	OperationSessionCreate:            {Method: "POST", Path: "/v1/sessions", Authentication: "api_key", Mutation: true},
	OperationSessionRenew:             {Method: "POST", Path: "/v1/sessions/{session}/renew", Authentication: "session_bearer", Mutation: true},
	OperationSubscriptionClose:        {Method: "POST", Path: "/v1/subscriptions/close", Authentication: "session_bearer", Mutation: true},
	OperationSubscriptionOpen:         {Method: "POST", Path: "/v1/subscriptions/open", Authentication: "session_bearer", Mutation: true},
	OperationTaskClose:                {Method: "POST", Path: "/v1/tasks/close", Authentication: "session_bearer", Mutation: true},
	OperationTaskList:                 {Method: "POST", Path: "/v1/tasks/list", Authentication: "session_bearer", Mutation: false},
	OperationTaskOpen:                 {Method: "POST", Path: "/v1/tasks", Authentication: "session_bearer", Mutation: true},
	OperationTaskRead:                 {Method: "POST", Path: "/v1/tasks/read", Authentication: "session_bearer", Mutation: false},
	OperationTransactionAbort:         {Method: "DELETE", Path: "/v1/transactions/{transaction}", Authentication: "session_bearer", Mutation: true},
	OperationTransactionBegin:         {Method: "POST", Path: "/v1/transactions", Authentication: "session_bearer", Mutation: true},
	OperationTransactionCommit:        {Method: "POST", Path: "/v1/transactions/{transaction}/commit", Authentication: "session_bearer", Mutation: true},
	OperationTransactionPreview:       {Method: "POST", Path: "/v1/transactions/{transaction}/preview", Authentication: "session_bearer", Mutation: true},
	OperationVectorCollectionEnsure:   {Method: "POST", Path: "/v1/vector/collections/ensure", Authentication: "session_bearer", Mutation: true},
	OperationVectorCollectionList:     {Method: "POST", Path: "/v1/vector/collections/list", Authentication: "session_bearer", Mutation: false},
	OperationVectorPointRetrieve:      {Method: "POST", Path: "/v1/vector/points/retrieve", Authentication: "session_bearer", Mutation: false},
	OperationVectorPointScroll:        {Method: "POST", Path: "/v1/vector/points/scroll", Authentication: "session_bearer", Mutation: false},
	OperationVectorSearch:             {Method: "POST", Path: "/v1/vector/search", Authentication: "session_bearer", Mutation: false},
}
