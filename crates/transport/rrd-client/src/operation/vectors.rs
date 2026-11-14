//! Vector collection, index, quantization, search and point operations.

use super::*;

impl RrdClient {
    pub async fn search_vectors(
        &self,
        session: &Session,
        request: SearchVectors,
        options: RequestOptions,
    ) -> Result<VectorSearchResult> {
        self.session_call(
            Method::POST,
            "/v1/vector/search",
            session,
            request,
            options,
            false,
        )
        .await
    }

    pub async fn ensure_vector_collection(
        &self,
        session: &Session,
        request: EnsureVectorCollection,
        options: RequestOptions,
    ) -> Result<EnsureVectorCollectionResult> {
        self.session_call(
            Method::POST,
            "/v1/vector/collections/ensure",
            session,
            request,
            options,
            true,
        )
        .await
    }

    /// Builds or advances the graph index of one named dense vector, and
    /// publishes it for approximate search. The request needs an idempotency
    /// key: the same key and request return the generation that key built.
    pub async fn ensure_vector_index(
        &self,
        session: &Session,
        request: EnsureVectorIndex,
        options: RequestOptions,
    ) -> Result<EnsureVectorIndexResult> {
        self.session_call(
            Method::POST,
            "/v1/vector/indexes/ensure",
            session,
            request,
            options,
            true,
        )
        .await
    }

    /// Declares a typed payload index, so a filter can use that property.
    pub async fn ensure_vector_payload_index(
        &self,
        session: &Session,
        request: EnsureVectorPayloadIndex,
        options: RequestOptions,
    ) -> Result<EnsureVectorPayloadIndexResult> {
        self.session_call(
            Method::POST,
            "/v1/vector/payload-indexes/ensure",
            session,
            request,
            options,
            true,
        )
        .await
    }

    /// Deletes a payload index, refused while an active generation filters on
    /// it.
    pub async fn delete_vector_payload_index(
        &self,
        session: &Session,
        request: DeleteVectorPayloadIndex,
        options: RequestOptions,
    ) -> Result<DeleteVectorPayloadIndexResult> {
        self.session_call(
            Method::POST,
            "/v1/vector/payload-indexes/delete",
            session,
            request,
            options,
            true,
        )
        .await
    }

    pub async fn list_vector_collections(
        &self,
        session: &Session,
        request: ListVectorCollections,
        options: RequestOptions,
    ) -> Result<VectorCollectionCatalogueSnapshot> {
        self.session_call(
            Method::POST,
            "/v1/vector/collections/list",
            session,
            request,
            options,
            false,
        )
        .await
    }

    /// Builds the next generation of one quantization artifact. The request
    /// needs an idempotency key: the same key and request replay the
    /// recorded generation.
    pub async fn build_vector_quantization(
        &self,
        session: &Session,
        request: BuildVectorQuantizationArtifact,
        options: RequestOptions,
    ) -> Result<BuildVectorQuantizationArtifactResult> {
        self.session_call(
            Method::POST,
            "/v1/vector/quantizations/build",
            session,
            request,
            options,
            true,
        )
        .await
    }

    pub async fn list_vector_quantizations(
        &self,
        session: &Session,
        request: ListVectorQuantizationArtifacts,
        options: RequestOptions,
    ) -> Result<ListVectorQuantizationArtifactsResult> {
        self.session_call(
            Method::POST,
            "/v1/vector/quantizations/list",
            session,
            request,
            options,
            false,
        )
        .await
    }

    /// Activates one built generation for approximate search.
    pub async fn activate_vector_quantization(
        &self,
        session: &Session,
        request: ActivateVectorQuantizationArtifact,
        options: RequestOptions,
    ) -> Result<ActivateVectorQuantizationArtifactResult> {
        self.session_call(
            Method::POST,
            "/v1/vector/quantizations/activate",
            session,
            request,
            options,
            true,
        )
        .await
    }

    /// Retires one generation from approximate search.
    pub async fn retire_vector_quantization(
        &self,
        session: &Session,
        request: RetireVectorQuantizationArtifact,
        options: RequestOptions,
    ) -> Result<RetireVectorQuantizationArtifactResult> {
        self.session_call(
            Method::POST,
            "/v1/vector/quantizations/retire",
            session,
            request,
            options,
            true,
        )
        .await
    }

    pub async fn scroll_vector_points(
        &self,
        session: &Session,
        request: ScrollVectorPoints,
        options: RequestOptions,
    ) -> Result<VectorPointPage> {
        self.session_call(
            Method::POST,
            "/v1/vector/points/scroll",
            session,
            request,
            options,
            false,
        )
        .await
    }

    pub async fn retrieve_vector_points(
        &self,
        session: &Session,
        request: RetrieveVectorPoints,
        options: RequestOptions,
    ) -> Result<VectorPointBatch> {
        self.session_call(
            Method::POST,
            "/v1/vector/points/retrieve",
            session,
            request,
            options,
            false,
        )
        .await
    }
}
