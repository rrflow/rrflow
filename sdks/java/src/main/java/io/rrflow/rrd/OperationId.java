// Generated from rrd-contract; do not edit.
// OpenAPI SHA-256: 8c3b29a2a5af942de85d39126bab05c0fb479a3947ed11ac8c8faecf4c304bae
package io.rrflow.rrd;

public enum OperationId {
    ATTUNEMENT_CANCEL("attunement-cancel", "POST", "/v1/attunement/cancel", Authentication.SESSION_BEARER, true),
    ATTUNEMENT_JOB_LIST("attunement-job-list", "POST", "/v1/attunement/jobs/list", Authentication.SESSION_BEARER, false),
    ATTUNEMENT_PAUSE("attunement-pause", "POST", "/v1/attunement/pause", Authentication.SESSION_BEARER, true),
    ATTUNEMENT_READ("attunement-read", "POST", "/v1/attunement/read", Authentication.SESSION_BEARER, false),
    ATTUNEMENT_RECEIPT_READ("attunement-receipt-read", "POST", "/v1/attunement/receipt/read", Authentication.SESSION_BEARER, false),
    ATTUNEMENT_RECOVER_EXPIRED("attunement-recover-expired", "POST", "/v1/attunement/recover-expired", Authentication.SESSION_BEARER, true),
    ATTUNEMENT_REPLAN_ACCEPT("attunement-replan-accept", "POST", "/v1/attunement/replan/accept", Authentication.SESSION_BEARER, true),
    ATTUNEMENT_REPLAN_PREVIEW("attunement-replan-preview", "POST", "/v1/attunement/replan/preview", Authentication.SESSION_BEARER, false),
    ATTUNEMENT_REPLAN_RECEIPT_READ("attunement-replan-receipt-read", "POST", "/v1/attunement/replan/receipt/read", Authentication.SESSION_BEARER, false),
    ATTUNEMENT_RESUME("attunement-resume", "POST", "/v1/attunement/resume", Authentication.SESSION_BEARER, true),
    ATTUNEMENT_START("attunement-start", "POST", "/v1/attunement/start", Authentication.SESSION_BEARER, true),
    ATTUNEMENT_STEP("attunement-step", "POST", "/v1/attunement/step", Authentication.SESSION_BEARER, true),
    AUDIT_EXPORT("audit-export", "POST", "/v1/audit/export", Authentication.SESSION_BEARER, false),
    AUDIT_READ("audit-read", "POST", "/v1/audit/read", Authentication.SESSION_BEARER, false),
    BACKUP_CREATE("backup-create", "POST", "/v1/backups", Authentication.SESSION_BEARER, true),
    BACKUP_LIST("backup-list", "POST", "/v1/backups/list", Authentication.SESSION_BEARER, false),
    CAPABILITIES_READ("capabilities-read", "GET", "/v1/capabilities", Authentication.PUBLIC, false),
    CHANGEFEED_FOLLOW("changefeed-follow", "POST", "/v1/changes/follow", Authentication.SESSION_BEARER, false),
    CHANGEFEED_READ("changefeed-read", "POST", "/v1/changes/read", Authentication.SESSION_BEARER, false),
    COLD_HISTORY_READ("cold-history-read", "POST", "/v1/storage/cold-history/read", Authentication.SESSION_BEARER, false),
    CONTEXT_ASSEMBLE("context-assemble", "POST", "/v1/context/assemble", Authentication.SESSION_BEARER, false),
    DATA_SCHEMA_READ("data-schema-read", "POST", "/v1/data/schema", Authentication.SESSION_BEARER, false),
    DIAGNOSTICS_READ("diagnostics-read", "POST", "/v1/diagnostics/read", Authentication.SESSION_BEARER, false),
    DIAGNOSTICS_STORAGE_CURSOR("diagnostics-storage-cursor", "POST", "/v1/diagnostics/storage/cursor", Authentication.SESSION_BEARER, false),
    DIAGNOSTICS_STORAGE_READ("diagnostics-storage-read", "POST", "/v1/diagnostics/storage/read", Authentication.SESSION_BEARER, false),
    EMBEDDING_BUILTIN_ACTIVATE("embedding-builtin-activate", "POST", "/v1/embedding/builtin/activate", Authentication.SESSION_BEARER, true),
    ENDPOINT_CATALOGUE("endpoint-catalogue", "GET", "/v1/schema/endpoints", Authentication.PUBLIC, false),
    ENDPOINT_CONFIGURATION_APPLY("endpoint-configuration-apply", "POST", "/v1/configuration/endpoints/apply", Authentication.SESSION_BEARER, true),
    ENDPOINT_CONFIGURATION_PREVIEW("endpoint-configuration-preview", "POST", "/v1/configuration/endpoints/preview", Authentication.SESSION_BEARER, false),
    ENDPOINT_CONFIGURATION_READ("endpoint-configuration-read", "POST", "/v1/configuration/endpoints/read", Authentication.SESSION_BEARER, false),
    ESTATE_READ("estate-read", "POST", "/v1/estates/{estate}/read", Authentication.SESSION_BEARER, false),
    FLIGHT_LAUNCH("flight-launch", "POST", "/v1/flights", Authentication.SESSION_BEARER, true),
    FLIGHT_LIST("flight-list", "POST", "/v1/flights/list", Authentication.SESSION_BEARER, false),
    FLIGHT_READ("flight-read", "POST", "/v1/flights/read", Authentication.SESSION_BEARER, false),
    HEALTH_LIVE("health-live", "GET", "/v1/health/live", Authentication.PUBLIC, false),
    HEALTH_READY("health-ready", "GET", "/v1/health/ready", Authentication.PUBLIC, false),
    KNOWLEDGE_TREE_LIST("knowledge-tree-list", "POST", "/v1/knowledge/trees/list", Authentication.SESSION_BEARER, false),
    KNOWLEDGE_TREE_REPLACE("knowledge-tree-replace", "POST", "/v1/knowledge/trees/replace", Authentication.SESSION_BEARER, true),
    OBSERVATION_DELIVER("observation-deliver", "POST", "/v1/observations/deliver", Authentication.SESSION_BEARER, true),
    OBSERVATION_PREPARE("observation-prepare", "POST", "/v1/observations/prepare", Authentication.SESSION_BEARER, true),
    OBSERVATION_READ("observation-read", "POST", "/v1/observations/read", Authentication.SESSION_BEARER, false),
    OPENAPI_READ("openapi-read", "GET", "/v1/schema/openapi", Authentication.PUBLIC, false),
    QUERY_EXECUTE("query-execute", "POST", "/v1/query", Authentication.SESSION_BEARER, false),
    QUERY_EXPORT("query-export", "POST", "/v1/query/export", Authentication.SESSION_BEARER, false),
    QUERY_INDEX_ENSURE("query-index-ensure", "POST", "/v1/query/indexes/ensure", Authentication.SESSION_BEARER, true),
    QUERY_INDEX_LIST("query-index-list", "POST", "/v1/query/indexes/list", Authentication.SESSION_BEARER, false),
    QUERY_LIVE_POLL("query-live-poll", "POST", "/v1/query/live/poll", Authentication.SESSION_BEARER, false),
    RESTORE_CREATE("restore-create", "POST", "/v1/restores", Authentication.SESSION_BEARER, true),
    SESSION_CLOSE("session-close", "DELETE", "/v1/sessions/{session}", Authentication.SESSION_BEARER, true),
    SESSION_CREATE("session-create", "POST", "/v1/sessions", Authentication.API_KEY, true),
    SESSION_RENEW("session-renew", "POST", "/v1/sessions/{session}/renew", Authentication.SESSION_BEARER, true),
    SUBSCRIPTION_CLOSE("subscription-close", "POST", "/v1/subscriptions/close", Authentication.SESSION_BEARER, true),
    SUBSCRIPTION_OPEN("subscription-open", "POST", "/v1/subscriptions/open", Authentication.SESSION_BEARER, true),
    TASK_ASSESSMENT_EXECUTE("task-assessment-execute", "POST", "/v1/tasks/assessment/execute", Authentication.SESSION_BEARER, true),
    TASK_ASSESSMENT_READ("task-assessment-read", "POST", "/v1/tasks/assessment/read", Authentication.SESSION_BEARER, false),
    TASK_ATTEMPT_READ("task-attempt-read", "POST", "/v1/tasks/attempt/read", Authentication.SESSION_BEARER, false),
    TASK_BUDGET_CONFIGURE("task-budget-configure", "POST", "/v1/tasks/budget/configure", Authentication.SESSION_BEARER, true),
    TASK_BUDGET_READ("task-budget-read", "POST", "/v1/tasks/budget/read", Authentication.SESSION_BEARER, false),
    TASK_BUDGET_RESUME("task-budget-resume", "POST", "/v1/tasks/budget/resume", Authentication.SESSION_BEARER, true),
    TASK_BUDGET_STOP("task-budget-stop", "POST", "/v1/tasks/budget/stop", Authentication.SESSION_BEARER, true),
    TASK_CLOSE("task-close", "POST", "/v1/tasks/close", Authentication.SESSION_BEARER, true),
    TASK_EMBEDDING_EXECUTE("task-embedding-execute", "POST", "/v1/tasks/embedding/execute", Authentication.SESSION_BEARER, true),
    TASK_ENDPOINT_PROPOSE("task-endpoint-propose", "POST", "/v1/tasks/endpoint/propose", Authentication.SESSION_BEARER, true),
    TASK_LIST("task-list", "POST", "/v1/tasks/list", Authentication.SESSION_BEARER, false),
    TASK_OPEN("task-open", "POST", "/v1/tasks", Authentication.SESSION_BEARER, true),
    TASK_READ("task-read", "POST", "/v1/tasks/read", Authentication.SESSION_BEARER, false),
    TASK_WORKER_EXECUTE("task-worker-execute", "POST", "/v1/tasks/worker/execute", Authentication.SESSION_BEARER, true),
    TRANSACTION_ABORT("transaction-abort", "DELETE", "/v1/transactions/{transaction}", Authentication.SESSION_BEARER, true),
    TRANSACTION_BEGIN("transaction-begin", "POST", "/v1/transactions", Authentication.SESSION_BEARER, true),
    TRANSACTION_COMMIT("transaction-commit", "POST", "/v1/transactions/{transaction}/commit", Authentication.SESSION_BEARER, true),
    TRANSACTION_PREVIEW("transaction-preview", "POST", "/v1/transactions/{transaction}/preview", Authentication.SESSION_BEARER, true),
    VECTOR_COLLECTION_ENSURE("vector-collection-ensure", "POST", "/v1/vector/collections/ensure", Authentication.SESSION_BEARER, true),
    VECTOR_COLLECTION_LIST("vector-collection-list", "POST", "/v1/vector/collections/list", Authentication.SESSION_BEARER, false),
    VECTOR_INDEX_ENSURE("vector-index-ensure", "POST", "/v1/vector/indexes/ensure", Authentication.SESSION_BEARER, true),
    VECTOR_PAYLOAD_INDEX_DELETE("vector-payload-index-delete", "POST", "/v1/vector/payload-indexes/delete", Authentication.SESSION_BEARER, true),
    VECTOR_PAYLOAD_INDEX_ENSURE("vector-payload-index-ensure", "POST", "/v1/vector/payload-indexes/ensure", Authentication.SESSION_BEARER, true),
    VECTOR_POINT_RETRIEVE("vector-point-retrieve", "POST", "/v1/vector/points/retrieve", Authentication.SESSION_BEARER, false),
    VECTOR_POINT_SCROLL("vector-point-scroll", "POST", "/v1/vector/points/scroll", Authentication.SESSION_BEARER, false),
    VECTOR_QUANTIZATION_ACTIVATE("vector-quantization-activate", "POST", "/v1/vector/quantizations/activate", Authentication.SESSION_BEARER, true),
    VECTOR_QUANTIZATION_BUILD("vector-quantization-build", "POST", "/v1/vector/quantizations/build", Authentication.SESSION_BEARER, true),
    VECTOR_QUANTIZATION_LIST("vector-quantization-list", "POST", "/v1/vector/quantizations/list", Authentication.SESSION_BEARER, false),
    VECTOR_QUANTIZATION_RETIRE("vector-quantization-retire", "POST", "/v1/vector/quantizations/retire", Authentication.SESSION_BEARER, true),
    VECTOR_SEARCH("vector-search", "POST", "/v1/vector/search", Authentication.SESSION_BEARER, false);

    static final String OPENAPI_DOCUMENT_SHA256 = "8c3b29a2a5af942de85d39126bab05c0fb479a3947ed11ac8c8faecf4c304bae";

    public enum Authentication { PUBLIC, API_KEY, SESSION_BEARER }

    private final String wireName;
    private final String method;
    private final String path;
    private final Authentication authentication;
    private final boolean mutation;

    OperationId(String wireName, String method, String path, Authentication authentication, boolean mutation) {
        this.wireName = wireName;
        this.method = method;
        this.path = path;
        this.authentication = authentication;
        this.mutation = mutation;
    }

    public String wireName() { return wireName; }
    public String method() { return method; }
    public String path() { return path; }
    public Authentication authentication() { return authentication; }
    public boolean mutation() { return mutation; }
}
