//! Task, budget and attempt operations.

use super::*;

impl RrdClient {
    pub async fn configure_task_budget(
        &self,
        session: &Session,
        request: rrd_contract::ConfigureTaskBudget,
        options: RequestOptions,
    ) -> Result<rrd_contract::TaskBudgetView> {
        let result: rrd_contract::TaskBudgetView = self
            .session_call(
                Method::POST,
                "/v1/tasks/budget/configure",
                session,
                request,
                options,
                true,
            )
            .await?;
        result.validate().map_err(contract)?;
        Ok(result)
    }

    pub async fn read_task_budget(
        &self,
        session: &Session,
        request: rrd_contract::ReadTaskBudget,
        options: RequestOptions,
    ) -> Result<rrd_contract::TaskBudgetView> {
        let result: rrd_contract::TaskBudgetView = self
            .session_call(
                Method::POST,
                "/v1/tasks/budget/read",
                session,
                request,
                options,
                false,
            )
            .await?;
        result.validate().map_err(contract)?;
        Ok(result)
    }

    pub async fn stop_task_budget(
        &self,
        session: &Session,
        request: rrd_contract::StopTaskBudget,
        options: RequestOptions,
    ) -> Result<rrd_contract::TaskBudgetView> {
        let result: rrd_contract::TaskBudgetView = self
            .session_call(
                Method::POST,
                "/v1/tasks/budget/stop",
                session,
                request,
                options,
                true,
            )
            .await?;
        result.validate().map_err(contract)?;
        Ok(result)
    }

    pub async fn execute_task_embedding(
        &self,
        session: &Session,
        request: rrd_contract::ExecuteTaskEmbedding,
        options: RequestOptions,
    ) -> Result<rrd_contract::GenerateEmbeddingsResult> {
        let result: rrd_contract::GenerateEmbeddingsResult = self
            .session_call(
                Method::POST,
                "/v1/tasks/embedding/execute",
                session,
                request,
                options,
                true,
            )
            .await?;
        result.validate().map_err(contract)?;
        Ok(result)
    }

    pub async fn read_task_attempt(
        &self,
        session: &Session,
        request: rrd_contract::ReadTaskAttempt,
        options: RequestOptions,
    ) -> Result<rrd_contract::TaskAttemptView> {
        let result: rrd_contract::TaskAttemptView = self
            .session_call(
                Method::POST,
                "/v1/tasks/attempt/read",
                session,
                request,
                options,
                false,
            )
            .await?;
        result.validate().map_err(contract)?;
        Ok(result)
    }

    pub async fn open_task(
        &self,
        session: &Session,
        request: OpenTask,
        options: RequestOptions,
    ) -> Result<TaskResult> {
        let result: TaskResult = self
            .session_call(Method::POST, "/v1/tasks", session, request, options, true)
            .await?;
        result.validate().map_err(contract)?;
        Ok(result)
    }

    pub async fn close_task(
        &self,
        session: &Session,
        request: CloseTask,
        options: RequestOptions,
    ) -> Result<TaskResult> {
        let result: TaskResult = self
            .session_call(
                Method::POST,
                "/v1/tasks/close",
                session,
                request,
                options,
                true,
            )
            .await?;
        result.validate().map_err(contract)?;
        Ok(result)
    }

    pub async fn list_tasks(
        &self,
        session: &Session,
        request: ListTasks,
        options: RequestOptions,
    ) -> Result<TaskPage> {
        let page: TaskPage = self
            .session_call(
                Method::POST,
                "/v1/tasks/list",
                session,
                request,
                options,
                false,
            )
            .await?;
        page.validate().map_err(contract)?;
        Ok(page)
    }

    pub async fn read_task(
        &self,
        session: &Session,
        request: ReadTask,
        options: RequestOptions,
    ) -> Result<TaskReadResult> {
        let result: TaskReadResult = self
            .session_call(
                Method::POST,
                "/v1/tasks/read",
                session,
                request,
                options,
                false,
            )
            .await?;
        result.validate().map_err(contract)?;
        Ok(result)
    }
}
