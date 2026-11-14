//! Endpoint descriptors for the vector operations; `endpoint_catalogue` sorts the union.

use crate::*;

operations! {
    VectorCollectionEnsure = "vector-collection-ensure",
        Post "/v1/vector/collections/ensure",
        SessionBearer,
        true,
        VectorCollectionEnsure,
        EnsureVectorCollection => EnsureVectorCollectionResult,
        KeyedMutation;
    VectorCollectionList = "vector-collection-list",
        Post "/v1/vector/collections/list",
        SessionBearer,
        false,
        VectorCollectionList,
        ListVectorCollections => VectorCollectionCatalogueSnapshot,
        Read;
    VectorIndexEnsure = "vector-index-ensure",
        Post "/v1/vector/indexes/ensure",
        SessionBearer,
        true,
        VectorCollectionEnsure,
        EnsureVectorIndex => EnsureVectorIndexResult,
        KeyedMutation;
    VectorPayloadIndexDelete = "vector-payload-index-delete",
        Post "/v1/vector/payload-indexes/delete",
        SessionBearer,
        true,
        VectorPayloadIndexDelete,
        DeleteVectorPayloadIndex => DeleteVectorPayloadIndexResult,
        KeyedMutation;
    VectorPayloadIndexEnsure = "vector-payload-index-ensure",
        Post "/v1/vector/payload-indexes/ensure",
        SessionBearer,
        true,
        VectorPayloadIndexEnsure,
        EnsureVectorPayloadIndex => EnsureVectorPayloadIndexResult,
        KeyedMutation;
    VectorPointRetrieve = "vector-point-retrieve",
        Post "/v1/vector/points/retrieve",
        SessionBearer,
        false,
        VectorPointRetrieve,
        RetrieveVectorPoints => VectorPointBatch,
        Read;
    VectorPointScroll = "vector-point-scroll",
        Post "/v1/vector/points/scroll",
        SessionBearer,
        false,
        VectorPointScroll,
        ScrollVectorPoints => VectorPointPage,
        Read;
    VectorQuantizationActivate = "vector-quantization-activate",
        Post "/v1/vector/quantizations/activate",
        SessionBearer,
        true,
        VectorCollectionEnsure,
        ActivateVectorQuantizationArtifact => ActivateVectorQuantizationArtifactResult,
        KeyedMutation;
    VectorQuantizationBuild = "vector-quantization-build",
        Post "/v1/vector/quantizations/build",
        SessionBearer,
        true,
        VectorCollectionEnsure,
        BuildVectorQuantizationArtifact => BuildVectorQuantizationArtifactResult,
        KeyedMutation;
    VectorQuantizationList = "vector-quantization-list",
        Post "/v1/vector/quantizations/list",
        SessionBearer,
        false,
        VectorCollectionList,
        ListVectorQuantizationArtifacts => ListVectorQuantizationArtifactsResult,
        Read;
    VectorQuantizationRetire = "vector-quantization-retire",
        Post "/v1/vector/quantizations/retire",
        SessionBearer,
        true,
        VectorCollectionEnsure,
        RetireVectorQuantizationArtifact => RetireVectorQuantizationArtifactResult,
        KeyedMutation;
    VectorSearch = "vector-search",
        Post "/v1/vector/search",
        SessionBearer,
        false,
        VectorSearch,
        SearchVectors => VectorSearchResult,
        Read;
}
