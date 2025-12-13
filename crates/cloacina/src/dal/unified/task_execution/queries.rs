/*
 *  Copyright 2025 Colliery Software
 *
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS,
 *  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 */

//! Query operations for task executions.

use super::TaskExecutionDAL;
use crate::dal::unified::models::UnifiedTaskExecution;
use crate::database::schema::unified::task_executions;
use crate::database::universal_types::UniversalUuid;
use crate::error::ValidationError;
use crate::models::task_execution::TaskExecution;
use diesel::prelude::*;

impl<'a> TaskExecutionDAL<'a> {
    /// Retrieves all pending (NotStarted) tasks for a specific pipeline execution.
    pub async fn get_pending_tasks(
        &self,
        pipeline_execution_id: UniversalUuid,
    ) -> Result<Vec<TaskExecution>, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.get_pending_tasks_postgres(pipeline_execution_id).await,
            self.get_pending_tasks_sqlite(pipeline_execution_id).await
        )
    }

    #[cfg(feature = "postgres")]
    async fn get_pending_tasks_postgres(
        &self,
        pipeline_execution_id: UniversalUuid,
    ) -> Result<Vec<TaskExecution>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let tasks: Vec<UnifiedTaskExecution> = conn
            .interact(move |conn| {
                task_executions::table
                    .filter(task_executions::pipeline_execution_id.eq(pipeline_execution_id))
                    .filter(task_executions::status.eq("NotStarted"))
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(tasks.into_iter().map(Into::into).collect())
    }

    #[cfg(feature = "sqlite")]
    async fn get_pending_tasks_sqlite(
        &self,
        pipeline_execution_id: UniversalUuid,
    ) -> Result<Vec<TaskExecution>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let tasks: Vec<UnifiedTaskExecution> = conn
            .interact(move |conn| {
                task_executions::table
                    .filter(task_executions::pipeline_execution_id.eq(pipeline_execution_id))
                    .filter(task_executions::status.eq("NotStarted"))
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(tasks.into_iter().map(Into::into).collect())
    }

    /// Gets all pending tasks for multiple pipelines in a single query.
    pub async fn get_pending_tasks_batch(
        &self,
        pipeline_execution_ids: Vec<UniversalUuid>,
    ) -> Result<Vec<TaskExecution>, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.get_pending_tasks_batch_postgres(pipeline_execution_ids)
                .await,
            self.get_pending_tasks_batch_sqlite(pipeline_execution_ids)
                .await
        )
    }

    #[cfg(feature = "postgres")]
    async fn get_pending_tasks_batch_postgres(
        &self,
        pipeline_execution_ids: Vec<UniversalUuid>,
    ) -> Result<Vec<TaskExecution>, ValidationError> {
        if pipeline_execution_ids.is_empty() {
            return Ok(Vec::new());
        }

        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let tasks: Vec<UnifiedTaskExecution> = conn
            .interact(move |conn| {
                task_executions::table
                    .filter(task_executions::pipeline_execution_id.eq_any(&pipeline_execution_ids))
                    .filter(task_executions::status.eq_any(vec!["NotStarted", "Pending"]))
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(tasks.into_iter().map(Into::into).collect())
    }

    #[cfg(feature = "sqlite")]
    async fn get_pending_tasks_batch_sqlite(
        &self,
        pipeline_execution_ids: Vec<UniversalUuid>,
    ) -> Result<Vec<TaskExecution>, ValidationError> {
        if pipeline_execution_ids.is_empty() {
            return Ok(Vec::new());
        }

        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let tasks: Vec<UnifiedTaskExecution> = conn
            .interact(move |conn| {
                task_executions::table
                    .filter(task_executions::pipeline_execution_id.eq_any(&pipeline_execution_ids))
                    .filter(task_executions::status.eq_any(vec!["NotStarted", "Pending"]))
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(tasks.into_iter().map(Into::into).collect())
    }

    /// Checks if all tasks in a pipeline have reached a terminal state.
    pub async fn check_pipeline_completion(
        &self,
        pipeline_execution_id: UniversalUuid,
    ) -> Result<bool, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.check_pipeline_completion_postgres(pipeline_execution_id)
                .await,
            self.check_pipeline_completion_sqlite(pipeline_execution_id)
                .await
        )
    }

    #[cfg(feature = "postgres")]
    async fn check_pipeline_completion_postgres(
        &self,
        pipeline_execution_id: UniversalUuid,
    ) -> Result<bool, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let incomplete_count: i64 = conn
            .interact(move |conn| {
                task_executions::table
                    .filter(task_executions::pipeline_execution_id.eq(pipeline_execution_id))
                    .filter(task_executions::status.ne_all(vec!["Completed", "Failed", "Skipped"]))
                    .count()
                    .get_result(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(incomplete_count == 0)
    }

    #[cfg(feature = "sqlite")]
    async fn check_pipeline_completion_sqlite(
        &self,
        pipeline_execution_id: UniversalUuid,
    ) -> Result<bool, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let incomplete_count: i64 = conn
            .interact(move |conn| {
                task_executions::table
                    .filter(task_executions::pipeline_execution_id.eq(pipeline_execution_id))
                    .filter(task_executions::status.ne_all(vec!["Completed", "Failed", "Skipped"]))
                    .count()
                    .get_result(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(incomplete_count == 0)
    }

    /// Gets the current status of a specific task in a pipeline.
    pub async fn get_task_status(
        &self,
        pipeline_execution_id: UniversalUuid,
        task_name: &str,
    ) -> Result<String, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.get_task_status_postgres(pipeline_execution_id, task_name)
                .await,
            self.get_task_status_sqlite(pipeline_execution_id, task_name)
                .await
        )
    }

    #[cfg(feature = "postgres")]
    async fn get_task_status_postgres(
        &self,
        pipeline_execution_id: UniversalUuid,
        task_name: &str,
    ) -> Result<String, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let task_name_owned = task_name.to_string();
        let status: String = conn
            .interact(move |conn| {
                task_executions::table
                    .filter(task_executions::pipeline_execution_id.eq(pipeline_execution_id))
                    .filter(task_executions::task_name.eq(&task_name_owned))
                    .select(task_executions::status)
                    .first(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(status)
    }

    #[cfg(feature = "sqlite")]
    async fn get_task_status_sqlite(
        &self,
        pipeline_execution_id: UniversalUuid,
        task_name: &str,
    ) -> Result<String, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let task_name_owned = task_name.to_string();
        let status: String = conn
            .interact(move |conn| {
                task_executions::table
                    .filter(task_executions::pipeline_execution_id.eq(pipeline_execution_id))
                    .filter(task_executions::task_name.eq(&task_name_owned))
                    .select(task_executions::status)
                    .first(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(status)
    }

    /// Gets the status of multiple tasks in a single database query.
    pub async fn get_task_statuses_batch(
        &self,
        pipeline_execution_id: UniversalUuid,
        task_names: Vec<String>,
    ) -> Result<std::collections::HashMap<String, String>, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.get_task_statuses_batch_postgres(pipeline_execution_id, task_names)
                .await,
            self.get_task_statuses_batch_sqlite(pipeline_execution_id, task_names)
                .await
        )
    }

    #[cfg(feature = "postgres")]
    async fn get_task_statuses_batch_postgres(
        &self,
        pipeline_execution_id: UniversalUuid,
        task_names: Vec<String>,
    ) -> Result<std::collections::HashMap<String, String>, ValidationError> {
        use std::collections::HashMap;

        if task_names.is_empty() {
            return Ok(HashMap::new());
        }

        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let results: Vec<(String, String)> = conn
            .interact(move |conn| {
                task_executions::table
                    .filter(task_executions::pipeline_execution_id.eq(pipeline_execution_id))
                    .filter(task_executions::task_name.eq_any(&task_names))
                    .select((task_executions::task_name, task_executions::status))
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(results.into_iter().collect())
    }

    #[cfg(feature = "sqlite")]
    async fn get_task_statuses_batch_sqlite(
        &self,
        pipeline_execution_id: UniversalUuid,
        task_names: Vec<String>,
    ) -> Result<std::collections::HashMap<String, String>, ValidationError> {
        use std::collections::HashMap;

        if task_names.is_empty() {
            return Ok(HashMap::new());
        }

        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let results: Vec<(String, String)> = conn
            .interact(move |conn| {
                task_executions::table
                    .filter(task_executions::pipeline_execution_id.eq(pipeline_execution_id))
                    .filter(task_executions::task_name.eq_any(&task_names))
                    .select((task_executions::task_name, task_executions::status))
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(results.into_iter().collect())
    }
}
