// Generated from rrd-contract; do not edit.
// OpenAPI SHA-256: 06cd649d80ec2de5fe152da49daa00533d2e5dbf56e0321fea4d86f7b432a7b3
export const ENDPOINT_OPENAPI_DOCUMENT_SHA256 = "06cd649d80ec2de5fe152da49daa00533d2e5dbf56e0321fea4d86f7b432a7b3" as const;

export const endpoints = {
  "audit-export": {
    "method": "POST",
    "path": "/v1/audit/export",
    "authentication": "session_bearer",
    "mutation": false
  },
  "audit-read": {
    "method": "POST",
    "path": "/v1/audit/read",
    "authentication": "session_bearer",
    "mutation": false
  },
  "backup-create": {
    "method": "POST",
    "path": "/v1/backups",
    "authentication": "session_bearer",
    "mutation": true
  },
  "backup-list": {
    "method": "POST",
    "path": "/v1/backups/list",
    "authentication": "session_bearer",
    "mutation": false
  },
  "capabilities-read": {
    "method": "GET",
    "path": "/v1/capabilities",
    "authentication": "public",
    "mutation": false
  },
  "changefeed-follow": {
    "method": "POST",
    "path": "/v1/changes/follow",
    "authentication": "session_bearer",
    "mutation": false
  },
  "changefeed-read": {
    "method": "POST",
    "path": "/v1/changes/read",
    "authentication": "session_bearer",
    "mutation": false
  },
  "endpoint-configuration-apply": {
    "method": "POST",
    "path": "/v1/configuration/endpoints/apply",
    "authentication": "session_bearer",
    "mutation": true
  },
  "endpoint-configuration-preview": {
    "method": "POST",
    "path": "/v1/configuration/endpoints/preview",
    "authentication": "session_bearer",
    "mutation": false
  },
  "endpoint-configuration-read": {
    "method": "POST",
    "path": "/v1/configuration/endpoints/read",
    "authentication": "session_bearer",
    "mutation": false
  },
  "context-assemble": {
    "method": "POST",
    "path": "/v1/context/assemble",
    "authentication": "session_bearer",
    "mutation": false
  },
  "data-schema-read": {
    "method": "POST",
    "path": "/v1/data/schema",
    "authentication": "session_bearer",
    "mutation": false
  },
  "diagnostics-read": {
    "method": "POST",
    "path": "/v1/diagnostics/read",
    "authentication": "session_bearer",
    "mutation": false
  },
  "diagnostics-storage-cursor": {
    "method": "POST",
    "path": "/v1/diagnostics/storage/cursor",
    "authentication": "session_bearer",
    "mutation": false
  },
  "diagnostics-storage-read": {
    "method": "POST",
    "path": "/v1/diagnostics/storage/read",
    "authentication": "session_bearer",
    "mutation": false
  },
  "embedding-builtin-activate": {
    "method": "POST",
    "path": "/v1/embedding/builtin/activate",
    "authentication": "session_bearer",
    "mutation": true
  },
  "estate-read": {
    "method": "POST",
    "path": "/v1/estates/{estate}/read",
    "authentication": "session_bearer",
    "mutation": false
  },
  "flight-launch": {
    "method": "POST",
    "path": "/v1/flights",
    "authentication": "session_bearer",
    "mutation": true
  },
  "flight-list": {
    "method": "POST",
    "path": "/v1/flights/list",
    "authentication": "session_bearer",
    "mutation": false
  },
  "flight-read": {
    "method": "POST",
    "path": "/v1/flights/read",
    "authentication": "session_bearer",
    "mutation": false
  },
  "health-live": {
    "method": "GET",
    "path": "/v1/health/live",
    "authentication": "public",
    "mutation": false
  },
  "health-ready": {
    "method": "GET",
    "path": "/v1/health/ready",
    "authentication": "public",
    "mutation": false
  },
  "knowledge-tree-list": {
    "method": "POST",
    "path": "/v1/knowledge/trees/list",
    "authentication": "session_bearer",
    "mutation": false
  },
  "knowledge-tree-replace": {
    "method": "POST",
    "path": "/v1/knowledge/trees/replace",
    "authentication": "session_bearer",
    "mutation": true
  },
  "query-execute": {
    "method": "POST",
    "path": "/v1/query",
    "authentication": "session_bearer",
    "mutation": false
  },
  "query-export": {
    "method": "POST",
    "path": "/v1/query/export",
    "authentication": "session_bearer",
    "mutation": false
  },
  "query-index-ensure": {
    "method": "POST",
    "path": "/v1/query/indexes/ensure",
    "authentication": "session_bearer",
    "mutation": true
  },
  "query-index-list": {
    "method": "POST",
    "path": "/v1/query/indexes/list",
    "authentication": "session_bearer",
    "mutation": false
  },
  "query-live-poll": {
    "method": "POST",
    "path": "/v1/query/live/poll",
    "authentication": "session_bearer",
    "mutation": false
  },
  "restore-create": {
    "method": "POST",
    "path": "/v1/restores",
    "authentication": "session_bearer",
    "mutation": true
  },
  "endpoint-catalogue": {
    "method": "GET",
    "path": "/v1/schema/endpoints",
    "authentication": "public",
    "mutation": false
  },
  "openapi-read": {
    "method": "GET",
    "path": "/v1/schema/openapi",
    "authentication": "public",
    "mutation": false
  },
  "session-create": {
    "method": "POST",
    "path": "/v1/sessions",
    "authentication": "api_key",
    "mutation": true
  },
  "session-close": {
    "method": "DELETE",
    "path": "/v1/sessions/{session}",
    "authentication": "session_bearer",
    "mutation": true
  },
  "session-renew": {
    "method": "POST",
    "path": "/v1/sessions/{session}/renew",
    "authentication": "session_bearer",
    "mutation": true
  },
  "subscription-close": {
    "method": "POST",
    "path": "/v1/subscriptions/close",
    "authentication": "session_bearer",
    "mutation": true
  },
  "subscription-open": {
    "method": "POST",
    "path": "/v1/subscriptions/open",
    "authentication": "session_bearer",
    "mutation": true
  },
  "task-open": {
    "method": "POST",
    "path": "/v1/tasks",
    "authentication": "session_bearer",
    "mutation": true
  },
  "task-attempt-read": {
    "method": "POST",
    "path": "/v1/tasks/attempt/read",
    "authentication": "session_bearer",
    "mutation": false
  },
  "task-budget-configure": {
    "method": "POST",
    "path": "/v1/tasks/budget/configure",
    "authentication": "session_bearer",
    "mutation": true
  },
  "task-budget-read": {
    "method": "POST",
    "path": "/v1/tasks/budget/read",
    "authentication": "session_bearer",
    "mutation": false
  },
  "task-budget-stop": {
    "method": "POST",
    "path": "/v1/tasks/budget/stop",
    "authentication": "session_bearer",
    "mutation": true
  },
  "task-close": {
    "method": "POST",
    "path": "/v1/tasks/close",
    "authentication": "session_bearer",
    "mutation": true
  },
  "task-embedding-execute": {
    "method": "POST",
    "path": "/v1/tasks/embedding/execute",
    "authentication": "session_bearer",
    "mutation": true
  },
  "task-list": {
    "method": "POST",
    "path": "/v1/tasks/list",
    "authentication": "session_bearer",
    "mutation": false
  },
  "task-read": {
    "method": "POST",
    "path": "/v1/tasks/read",
    "authentication": "session_bearer",
    "mutation": false
  },
  "transaction-begin": {
    "method": "POST",
    "path": "/v1/transactions",
    "authentication": "session_bearer",
    "mutation": true
  },
  "transaction-abort": {
    "method": "DELETE",
    "path": "/v1/transactions/{transaction}",
    "authentication": "session_bearer",
    "mutation": true
  },
  "transaction-commit": {
    "method": "POST",
    "path": "/v1/transactions/{transaction}/commit",
    "authentication": "session_bearer",
    "mutation": true
  },
  "transaction-preview": {
    "method": "POST",
    "path": "/v1/transactions/{transaction}/preview",
    "authentication": "session_bearer",
    "mutation": true
  },
  "vector-collection-ensure": {
    "method": "POST",
    "path": "/v1/vector/collections/ensure",
    "authentication": "session_bearer",
    "mutation": true
  },
  "vector-collection-list": {
    "method": "POST",
    "path": "/v1/vector/collections/list",
    "authentication": "session_bearer",
    "mutation": false
  },
  "vector-point-retrieve": {
    "method": "POST",
    "path": "/v1/vector/points/retrieve",
    "authentication": "session_bearer",
    "mutation": false
  },
  "vector-point-scroll": {
    "method": "POST",
    "path": "/v1/vector/points/scroll",
    "authentication": "session_bearer",
    "mutation": false
  },
  "vector-search": {
    "method": "POST",
    "path": "/v1/vector/search",
    "authentication": "session_bearer",
    "mutation": false
  }
} as const;

export type OperationId = keyof typeof endpoints;
