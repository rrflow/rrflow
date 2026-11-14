//! Backup, estate, audit and diagnostic operations.

use super::*;

impl RrdClient {
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
}
