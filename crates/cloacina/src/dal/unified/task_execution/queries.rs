/*
 *  Copyright 2025-2026 Colliery Software
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
    /// Retrieves all pending (NotStarted) tasks for a specific workflow execution.
    pub async fn get_pending_tasks(
        &self,
        workflow_execution_id: UniversalUuid,
    ) -> Result<Vec<TaskExecution>, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.get_pending_tasks_postgres(workflow_execution_id).await,
            self.get_pending_tasks_sqlite(workflow_execution_id).await
        )
    }

    #[cfg(feature = "postgres")]
    async fn get_pending_tasks_postgres(
        &self,
        workflow_execution_id: UniversalUuid,
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
                    .filter(task_executions::workflow_execution_id.eq(workflow_execution_id))
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
        workflow_execution_id: UniversalUuid,
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
                    .filter(task_executions::workflow_execution_id.eq(workflow_execution_id))
                    .filter(task_executions::status.eq("NotStarted"))
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(tasks.into_iter().map(Into::into).collect())
    }

    /// Gets all pending tasks for multiple workflow executions in a single query.
    pub async fn get_pending_tasks_batch(
        &self,
        workflow_execution_ids: Vec<UniversalUuid>,
    ) -> Result<Vec<TaskExecution>, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.get_pending_tasks_batch_postgres(workflow_execution_ids)
                .await,
            self.get_pending_tasks_batch_sqlite(workflow_execution_ids)
                .await
        )
    }

    #[cfg(feature = "postgres")]
    async fn get_pending_tasks_batch_postgres(
        &self,
        workflow_execution_ids: Vec<UniversalUuid>,
    ) -> Result<Vec<TaskExecution>, ValidationError> {
        if workflow_execution_ids.is_empty() {
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
                    .filter(task_executions::workflow_execution_id.eq_any(&workflow_execution_ids))
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
        workflow_execution_ids: Vec<UniversalUuid>,
    ) -> Result<Vec<TaskExecution>, ValidationError> {
        if workflow_execution_ids.is_empty() {
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
                    .filter(task_executions::workflow_execution_id.eq_any(&workflow_execution_ids))
                    .filter(task_executions::status.eq_any(vec!["NotStarted", "Pending"]))
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(tasks.into_iter().map(Into::into).collect())
    }

    /// Count task executions currently in the `Running` state across all
    /// workflows. Used by the scheduler tick to re-seed
    /// `cloacina_active_tasks` from SQL, replacing the gauge-leak prone
    /// increment/decrement pattern (CLOACI-T-0589, mirrors T-0534).
    pub async fn count_running_tasks(&self) -> Result<i64, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.count_running_tasks_postgres().await,
            self.count_running_tasks_sqlite().await
        )
    }

    #[cfg(feature = "postgres")]
    async fn count_running_tasks_postgres(&self) -> Result<i64, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let n: i64 = conn
            .interact(move |conn| {
                task_executions::table
                    .filter(task_executions::status.eq("Running"))
                    .count()
                    .get_result(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(n)
    }

    #[cfg(feature = "sqlite")]
    async fn count_running_tasks_sqlite(&self) -> Result<i64, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let n: i64 = conn
            .interact(move |conn| {
                task_executions::table
                    .filter(task_executions::status.eq("Running"))
                    .count()
                    .get_result(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(n)
    }

    /// Checks if all tasks in a workflow execution have reached a terminal state.
    pub async fn check_workflow_completion(
        &self,
        workflow_execution_id: UniversalUuid,
    ) -> Result<bool, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.check_workflow_completion_postgres(workflow_execution_id)
                .await,
            self.check_workflow_completion_sqlite(workflow_execution_id)
                .await
        )
    }

    #[cfg(feature = "postgres")]
    async fn check_workflow_completion_postgres(
        &self,
        workflow_execution_id: UniversalUuid,
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
                    .filter(task_executions::workflow_execution_id.eq(workflow_execution_id))
                    .filter(task_executions::status.ne_all(vec!["Completed", "Failed", "Skipped"]))
                    .count()
                    .get_result(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(incomplete_count == 0)
    }

    #[cfg(feature = "sqlite")]
    async fn check_workflow_completion_sqlite(
        &self,
        workflow_execution_id: UniversalUuid,
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
                    .filter(task_executions::workflow_execution_id.eq(workflow_execution_id))
                    .filter(task_executions::status.ne_all(vec!["Completed", "Failed", "Skipped"]))
                    .count()
                    .get_result(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(incomplete_count == 0)
    }

    /// Gets the current status of a specific task in a workflow execution.
    pub async fn get_task_status(
        &self,
        workflow_execution_id: UniversalUuid,
        task_name: &str,
    ) -> Result<String, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.get_task_status_postgres(workflow_execution_id, task_name)
                .await,
            self.get_task_status_sqlite(workflow_execution_id, task_name)
                .await
        )
    }

    #[cfg(feature = "postgres")]
    async fn get_task_status_postgres(
        &self,
        workflow_execution_id: UniversalUuid,
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
                    .filter(task_executions::workflow_execution_id.eq(workflow_execution_id))
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
        workflow_execution_id: UniversalUuid,
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
                    .filter(task_executions::workflow_execution_id.eq(workflow_execution_id))
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
        workflow_execution_id: UniversalUuid,
        task_names: Vec<String>,
    ) -> Result<std::collections::HashMap<String, String>, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.get_task_statuses_batch_postgres(workflow_execution_id, task_names)
                .await,
            self.get_task_statuses_batch_sqlite(workflow_execution_id, task_names)
                .await
        )
    }

    /// Load every task's status for a set of workflow executions in ONE query,
    /// grouped by execution: `execution_id -> { task_name -> status }`
    /// (CLOACI-T-0745). The scheduler tick uses this to resolve task-dependency
    /// gating, status-based trigger conditions, AND workflow completion entirely
    /// in memory — replacing the previous O(active_executions × pending_tasks)
    /// per-task `get_by_id` + `get_task_statuses_batch` + per-execution
    /// `check_workflow_completion` round-trips that stalled the loop under load.
    pub async fn get_all_task_statuses_for_executions(
        &self,
        workflow_execution_ids: Vec<UniversalUuid>,
    ) -> Result<std::collections::HashMap<UniversalUuid, std::collections::HashMap<String, String>>, ValidationError>
    {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.get_all_task_statuses_for_executions_postgres(workflow_execution_ids)
                .await,
            self.get_all_task_statuses_for_executions_sqlite(workflow_execution_ids)
                .await
        )
    }

    #[cfg(feature = "postgres")]
    async fn get_all_task_statuses_for_executions_postgres(
        &self,
        workflow_execution_ids: Vec<UniversalUuid>,
    ) -> Result<std::collections::HashMap<UniversalUuid, std::collections::HashMap<String, String>>, ValidationError>
    {
        use std::collections::HashMap;
        if workflow_execution_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let rows: Vec<(UniversalUuid, String, String)> = conn
            .interact(move |conn| {
                task_executions::table
                    .filter(task_executions::workflow_execution_id.eq_any(&workflow_execution_ids))
                    .select((
                        task_executions::workflow_execution_id,
                        task_executions::task_name,
                        task_executions::status,
                    ))
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        let mut grouped: HashMap<UniversalUuid, HashMap<String, String>> = HashMap::new();
        for (exec_id, name, status) in rows {
            grouped.entry(exec_id).or_default().insert(name, status);
        }
        Ok(grouped)
    }

    #[cfg(feature = "sqlite")]
    async fn get_all_task_statuses_for_executions_sqlite(
        &self,
        workflow_execution_ids: Vec<UniversalUuid>,
    ) -> Result<std::collections::HashMap<UniversalUuid, std::collections::HashMap<String, String>>, ValidationError>
    {
        use std::collections::HashMap;
        if workflow_execution_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let rows: Vec<(UniversalUuid, String, String)> = conn
            .interact(move |conn| {
                task_executions::table
                    .filter(task_executions::workflow_execution_id.eq_any(&workflow_execution_ids))
                    .select((
                        task_executions::workflow_execution_id,
                        task_executions::task_name,
                        task_executions::status,
                    ))
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        let mut grouped: HashMap<UniversalUuid, HashMap<String, String>> = HashMap::new();
        for (exec_id, name, status) in rows {
            grouped.entry(exec_id).or_default().insert(name, status);
        }
        Ok(grouped)
    }

    #[cfg(feature = "postgres")]
    async fn get_task_statuses_batch_postgres(
        &self,
        workflow_execution_id: UniversalUuid,
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
                    .filter(task_executions::workflow_execution_id.eq(workflow_execution_id))
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
        workflow_execution_id: UniversalUuid,
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
                    .filter(task_executions::workflow_execution_id.eq(workflow_execution_id))
                    .filter(task_executions::task_name.eq_any(&task_names))
                    .select((task_executions::task_name, task_executions::status))
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(results.into_iter().collect())
    }
}
