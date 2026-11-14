//! Knowledge-tree, context, prompt-flight and embedding operations.

use super::*;

impl RrdClient {
    pub async fn list_knowledge_trees(
        &self,
        session: &Session,
        request: ListKnowledgeTrees,
        options: RequestOptions,
    ) -> Result<KnowledgeTreeCatalogue> {
        request.validate().map_err(contract)?;
        let catalogue: KnowledgeTreeCatalogue = self
            .session_call(
                Method::POST,
                "/v1/knowledge/trees/list",
                session,
                request,
                options,
                false,
            )
            .await?;
        catalogue.validate().map_err(contract)?;
        Ok(catalogue)
    }

    pub async fn replace_knowledge_trees(
        &self,
        session: &Session,
        request: ReplaceKnowledgeTrees,
        options: RequestOptions,
    ) -> Result<ReplaceKnowledgeTreesResult> {
        request.validate().map_err(contract)?;
        let result: ReplaceKnowledgeTreesResult = self
            .session_call(
                Method::POST,
                "/v1/knowledge/trees/replace",
                session,
                request,
                options,
                true,
            )
            .await?;
        result.catalogue.validate().map_err(contract)?;
        Ok(result)
    }

    pub async fn assemble_context(
        &self,
        session: &Session,
        request: AssembleContext,
        options: RequestOptions,
    ) -> Result<ContextPacket> {
        let packet: ContextPacket = self
            .session_call(
                Method::POST,
                "/v1/context/assemble",
                session,
                request,
                options,
                false,
            )
            .await?;
        packet.validate().map_err(contract)?;
        Ok(packet)
    }

    pub async fn launch_prompt_flight(
        &self,
        session: &Session,
        request: LaunchPromptFlight,
        options: RequestOptions,
    ) -> Result<LaunchPromptFlightResult> {
        let result: LaunchPromptFlightResult = self
            .session_call(Method::POST, "/v1/flights", session, request, options, true)
            .await?;
        result.validate().map_err(contract)?;
        Ok(result)
    }

    pub async fn list_prompt_flights(
        &self,
        session: &Session,
        request: ListPromptFlights,
        options: RequestOptions,
    ) -> Result<PromptFlightPage> {
        let page: PromptFlightPage = self
            .session_call(
                Method::POST,
                "/v1/flights/list",
                session,
                request,
                options,
                false,
            )
            .await?;
        page.validate().map_err(contract)?;
        Ok(page)
    }

    pub async fn read_prompt_flight(
        &self,
        session: &Session,
        request: ReadPromptFlight,
        options: RequestOptions,
    ) -> Result<PromptFlightReadResult> {
        let result: PromptFlightReadResult = self
            .session_call(
                Method::POST,
                "/v1/flights/read",
                session,
                request,
                options,
                false,
            )
            .await?;
        result.validate().map_err(contract)?;
        Ok(result)
    }

    pub async fn activate_builtin_embedding(
        &self,
        session: &Session,
        request: rrd_contract::ActivateBuiltinEmbedding,
        options: RequestOptions,
    ) -> Result<rrd_contract::EmbeddingModelCatalogue> {
        let result: rrd_contract::EmbeddingModelCatalogue = self
            .session_call(
                Method::POST,
                "/v1/embedding/builtin/activate",
                session,
                request,
                options,
                true,
            )
            .await?;
        Ok(result)
    }
}
