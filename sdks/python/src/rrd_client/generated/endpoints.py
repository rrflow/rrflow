# Generated from rrd-contract; do not edit.
# OpenAPI SHA-256: 6a8d3e490eeb0e13a10465b752dee165223c7097e18ccd08acb79b38934e4650
from typing import Final, Literal, TypedDict

ENDPOINT_OPENAPI_DOCUMENT_SHA256: Final[str] = (
    "6a8d3e490eeb0e13a10465b752dee165223c7097e18ccd08acb79b38934e4650"
)

OperationId = Literal[
    "audit-export",
    "audit-read",
    "backup-create",
    "backup-list",
    "capabilities-read",
    "changefeed-follow",
    "changefeed-read",
    "context-assemble",
    "data-schema-read",
    "diagnostics-read",
    "diagnostics-storage-cursor",
    "diagnostics-storage-read",
    "endpoint-catalogue",
    "estate-read",
    "flight-launch",
    "flight-list",
    "flight-read",
    "health-live",
    "health-ready",
    "openapi-read",
    "query-execute",
    "query-export",
    "query-index-ensure",
    "query-index-list",
    "query-live-poll",
    "restore-create",
    "session-close",
    "session-create",
    "session-renew",
    "subscription-close",
    "subscription-open",
    "task-close",
    "task-list",
    "task-open",
    "task-read",
    "transaction-abort",
    "transaction-begin",
    "transaction-commit",
    "transaction-preview",
    "vector-collection-ensure",
    "vector-collection-list",
    "vector-point-retrieve",
    "vector-point-scroll",
    "vector-search",
]


class Endpoint(TypedDict):
    method: str
    path: str
    authentication: str
    mutation: bool


ENDPOINTS: Final[dict[OperationId, Endpoint]] = {
    "audit-export": {
        "authentication": "session_bearer",
        "method": "POST",
        "mutation": False,
        "path": "/v1/audit/export",
    },
    "audit-read": {
        "authentication": "session_bearer",
        "method": "POST",
        "mutation": False,
        "path": "/v1/audit/read",
    },
    "backup-create": {
        "authentication": "session_bearer",
        "method": "POST",
        "mutation": True,
        "path": "/v1/backups",
    },
    "backup-list": {
        "authentication": "session_bearer",
        "method": "POST",
        "mutation": False,
        "path": "/v1/backups/list",
    },
    "capabilities-read": {
        "authentication": "public",
        "method": "GET",
        "mutation": False,
        "path": "/v1/capabilities",
    },
    "changefeed-follow": {
        "authentication": "session_bearer",
        "method": "POST",
        "mutation": False,
        "path": "/v1/changes/follow",
    },
    "changefeed-read": {
        "authentication": "session_bearer",
        "method": "POST",
        "mutation": False,
        "path": "/v1/changes/read",
    },
    "context-assemble": {
        "authentication": "session_bearer",
        "method": "POST",
        "mutation": False,
        "path": "/v1/context/assemble",
    },
    "data-schema-read": {
        "authentication": "session_bearer",
        "method": "POST",
        "mutation": False,
        "path": "/v1/data/schema",
    },
    "diagnostics-read": {
        "authentication": "session_bearer",
        "method": "POST",
        "mutation": False,
        "path": "/v1/diagnostics/read",
    },
    "diagnostics-storage-cursor": {
        "authentication": "session_bearer",
        "method": "POST",
        "mutation": False,
        "path": "/v1/diagnostics/storage/cursor",
    },
    "diagnostics-storage-read": {
        "authentication": "session_bearer",
        "method": "POST",
        "mutation": False,
        "path": "/v1/diagnostics/storage/read",
    },
    "endpoint-catalogue": {
        "authentication": "public",
        "method": "GET",
        "mutation": False,
        "path": "/v1/schema/endpoints",
    },
    "estate-read": {
        "authentication": "session_bearer",
        "method": "POST",
        "mutation": False,
        "path": "/v1/estates/{estate}/read",
    },
    "flight-launch": {
        "authentication": "session_bearer",
        "method": "POST",
        "mutation": True,
        "path": "/v1/flights",
    },
    "flight-list": {
        "authentication": "session_bearer",
        "method": "POST",
        "mutation": False,
        "path": "/v1/flights/list",
    },
    "flight-read": {
        "authentication": "session_bearer",
        "method": "POST",
        "mutation": False,
        "path": "/v1/flights/read",
    },
    "health-live": {
        "authentication": "public",
        "method": "GET",
        "mutation": False,
        "path": "/v1/health/live",
    },
    "health-ready": {
        "authentication": "public",
        "method": "GET",
        "mutation": False,
        "path": "/v1/health/ready",
    },
    "openapi-read": {
        "authentication": "public",
        "method": "GET",
        "mutation": False,
        "path": "/v1/schema/openapi",
    },
    "query-execute": {
        "authentication": "session_bearer",
        "method": "POST",
        "mutation": False,
        "path": "/v1/query",
    },
    "query-export": {
        "authentication": "session_bearer",
        "method": "POST",
        "mutation": False,
        "path": "/v1/query/export",
    },
    "query-index-ensure": {
        "authentication": "session_bearer",
        "method": "POST",
        "mutation": True,
        "path": "/v1/query/indexes/ensure",
    },
    "query-index-list": {
        "authentication": "session_bearer",
        "method": "POST",
        "mutation": False,
        "path": "/v1/query/indexes/list",
    },
    "query-live-poll": {
        "authentication": "session_bearer",
        "method": "POST",
        "mutation": False,
        "path": "/v1/query/live/poll",
    },
    "restore-create": {
        "authentication": "session_bearer",
        "method": "POST",
        "mutation": True,
        "path": "/v1/restores",
    },
    "session-close": {
        "authentication": "session_bearer",
        "method": "DELETE",
        "mutation": True,
        "path": "/v1/sessions/{session}",
    },
    "session-create": {
        "authentication": "api_key",
        "method": "POST",
        "mutation": True,
        "path": "/v1/sessions",
    },
    "session-renew": {
        "authentication": "session_bearer",
        "method": "POST",
        "mutation": True,
        "path": "/v1/sessions/{session}/renew",
    },
    "subscription-close": {
        "authentication": "session_bearer",
        "method": "POST",
        "mutation": True,
        "path": "/v1/subscriptions/close",
    },
    "subscription-open": {
        "authentication": "session_bearer",
        "method": "POST",
        "mutation": True,
        "path": "/v1/subscriptions/open",
    },
    "task-close": {
        "authentication": "session_bearer",
        "method": "POST",
        "mutation": True,
        "path": "/v1/tasks/close",
    },
    "task-list": {
        "authentication": "session_bearer",
        "method": "POST",
        "mutation": False,
        "path": "/v1/tasks/list",
    },
    "task-open": {
        "authentication": "session_bearer",
        "method": "POST",
        "mutation": True,
        "path": "/v1/tasks",
    },
    "task-read": {
        "authentication": "session_bearer",
        "method": "POST",
        "mutation": False,
        "path": "/v1/tasks/read",
    },
    "transaction-abort": {
        "authentication": "session_bearer",
        "method": "DELETE",
        "mutation": True,
        "path": "/v1/transactions/{transaction}",
    },
    "transaction-begin": {
        "authentication": "session_bearer",
        "method": "POST",
        "mutation": True,
        "path": "/v1/transactions",
    },
    "transaction-commit": {
        "authentication": "session_bearer",
        "method": "POST",
        "mutation": True,
        "path": "/v1/transactions/{transaction}/commit",
    },
    "transaction-preview": {
        "authentication": "session_bearer",
        "method": "POST",
        "mutation": True,
        "path": "/v1/transactions/{transaction}/preview",
    },
    "vector-collection-ensure": {
        "authentication": "session_bearer",
        "method": "POST",
        "mutation": True,
        "path": "/v1/vector/collections/ensure",
    },
    "vector-collection-list": {
        "authentication": "session_bearer",
        "method": "POST",
        "mutation": False,
        "path": "/v1/vector/collections/list",
    },
    "vector-point-retrieve": {
        "authentication": "session_bearer",
        "method": "POST",
        "mutation": False,
        "path": "/v1/vector/points/retrieve",
    },
    "vector-point-scroll": {
        "authentication": "session_bearer",
        "method": "POST",
        "mutation": False,
        "path": "/v1/vector/points/scroll",
    },
    "vector-search": {
        "authentication": "session_bearer",
        "method": "POST",
        "mutation": False,
        "path": "/v1/vector/search",
    },
}
