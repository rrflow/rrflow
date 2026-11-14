//! Schema, query, transaction, cold-history and changefeed operations.

use super::*;

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

    /// Read the durable cold-history head or a verified published generation.
    pub async fn read_cold_history(
        &self,
        session: &Session,
        request: rrd_contract::ReadColdHistory,
        options: RequestOptions,
    ) -> Result<rrd_contract::ColdHistoryReadResult> {
        self.call_operation::<generated::ColdHistoryRead>(session, request, options)
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
}
