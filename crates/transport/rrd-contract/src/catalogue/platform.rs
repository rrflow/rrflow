//! Endpoint descriptors for the platform operations; `endpoint_catalogue` sorts the union.

use crate::*;

operations! {
    AuditExport = "audit-export",
        Post "/v1/audit/export",
        SessionBearer,
        false,
        AuditExport,
        ExportAudit => AuditExport,
        Read;
    AuditRead = "audit-read",
        Post "/v1/audit/read",
        SessionBearer,
        false,
        AuditRead,
        ReadAudit => AuditPage,
        Read;
    CapabilitiesRead = "capabilities-read",
        Get "/v1/capabilities",
        Public,
        false,
        ServiceInspect,
        Empty => ServiceCapabilities,
        Read;
    DiagnosticsStorageCursor = "diagnostics-storage-cursor",
        Post "/v1/diagnostics/storage/cursor",
        SessionBearer,
        false,
        DiagnosticsRead,
        InspectStorageCursor => StorageCursorSnapshot,
        Read;
    DiagnosticsStorageRead = "diagnostics-storage-read",
        Post "/v1/diagnostics/storage/read",
        SessionBearer,
        false,
        DiagnosticsRead,
        ReadStorageDiagnostics => DiagnosticStorageSnapshot,
        Read;
    DiagnosticsRead = "diagnostics-read",
        Post "/v1/diagnostics/read",
        SessionBearer,
        false,
        DiagnosticsRead,
        ReadDiagnosticSnapshot => DiagnosticSnapshot,
        Read;
    HealthLive = "health-live",
        Get "/v1/health/live",
        Public,
        false,
        ServiceInspect,
        Empty => Liveness,
        Read;
    HealthReady = "health-ready",
        Get "/v1/health/ready",
        Public,
        false,
        ServiceInspect,
        Empty => Readiness,
        Read;
    OpenapiRead = "openapi-read",
        Get "/v1/schema/openapi",
        Public,
        false,
        ServiceInspect,
        Empty => OpenApiDocument,
        Read;
}
