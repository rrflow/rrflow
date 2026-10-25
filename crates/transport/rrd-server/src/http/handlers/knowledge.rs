use super::super::*;

impl AppState {
    pub(in crate::http) fn list_knowledge_trees(
        &self,
        headers: &HeaderMap,
        body: &[u8],
        now: u64,
    ) -> HttpResponse {
        self.with_authenticated_envelope::<ListKnowledgeTrees, _, _>(
            headers,
            body,
            now,
            RrdOperation::KnowledgeTreeRead,
            None,
            |envelope, session, token| {
                self.service
                    .list_knowledge_trees(
                        session,
                        token,
                        &envelope.payload,
                        now,
                        envelope.context.request_id.as_str(),
                        envelope.context.operation_id.as_str(),
                    )
                    .map_err(api_error)
            },
        )
    }

    pub(in crate::http) fn replace_knowledge_trees(
        &self,
        headers: &HeaderMap,
        body: &[u8],
        now: u64,
    ) -> HttpResponse {
        self.with_authenticated_envelope::<ReplaceKnowledgeTrees, _, _>(
            headers,
            body,
            now,
            RrdOperation::KnowledgeTreeWrite,
            None,
            |envelope, session, token| {
                self.service
                    .replace_knowledge_trees(
                        session,
                        token,
                        &envelope.payload,
                        &envelope.context,
                        now,
                    )
                    .map_err(api_error)
            },
        )
    }
}
