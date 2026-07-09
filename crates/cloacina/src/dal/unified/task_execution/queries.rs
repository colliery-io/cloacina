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
        let tasks: Vec<UnifiedTaskExecution> = crate::interact_on_backend!(self.dal, |conn| {
            task_executions::table
                .filter(task_executions::workflow_execution_id.eq(workflow_execution_id))
                .filter(task_executions::status.eq("NotStarted"))
                .load(conn)
        })?;

        Ok(tasks.into_iter().map(Into::into).collect())
    }

    /// Gets all pending tasks for multiple workflow executions in a single query.
    pub async fn get_pending_tasks_batch(
        &self,
        workflow_execution_ids: Vec<UniversalUuid>,
    ) -> Result<Vec<TaskExecution>, ValidationError> {
        if workflow_execution_ids.is_empty() {
            return Ok(Vec::new());
        }

        let tasks: Vec<UnifiedTaskExecution> = crate::interact_on_backend!(self.dal, |conn| {
            task_executions::table
                .filter(task_executions::workflow_execution_id.eq_any(&workflow_execution_ids))
                .filter(task_executions::status.eq_any(vec!["NotStarted", "Pending"]))
                .load(conn)
        })?;

        Ok(tasks.into_iter().map(Into::into).collect())
    }

    /// Count task executions currently in the `Running` state across all
    /// workflows. Used by the scheduler tick to re-seed
    /// `cloacina_active_tasks` from SQL, replacing the gauge-leak prone
    /// increment/decrement pattern (CLOACI-T-0589, mirrors T-0534).
    pub async fn count_running_tasks(&self) -> Result<i64, ValidationError> {
        let n: i64 = crate::interact_on_backend!(self.dal, |conn| {
            task_executions::table
                .filter(task_executions::status.eq("Running"))
                .count()
                .get_result(conn)
        })?;

        Ok(n)
    }

    /// Checks if all tasks in a workflow execution have reached a terminal state.
    pub async fn check_workflow_completion(
        &self,
        workflow_execution_id: UniversalUuid,
    ) -> Result<bool, ValidationError> {
        let incomplete_count: i64 = crate::interact_on_backend!(self.dal, |conn| {
            task_executions::table
                .filter(task_executions::workflow_execution_id.eq(workflow_execution_id))
                .filter(task_executions::status.ne_all(vec!["Completed", "Failed", "Skipped"]))
                .count()
                .get_result(conn)
        })?;

        Ok(incomplete_count == 0)
    }

    /// Gets the current status of a specific task in a workflow execution.
    pub async fn get_task_status(
        &self,
        workflow_execution_id: UniversalUuid,
        task_name: &str,
    ) -> Result<String, ValidationError> {
        let task_name_owned = task_name.to_string();
        let status: String = crate::interact_on_backend!(self.dal, |conn| {
            task_executions::table
                .filter(task_executions::workflow_execution_id.eq(workflow_execution_id))
                .filter(task_executions::task_name.eq(&task_name_owned))
                .select(task_executions::status)
                .first(conn)
        })?;

        Ok(status)
    }

    /// Gets the status of multiple tasks in a single database query.
    pub async fn get_task_statuses_batch(
        &self,
        workflow_execution_id: UniversalUuid,
        task_names: Vec<String>,
    ) -> Result<std::collections::HashMap<String, String>, ValidationError> {
        use std::collections::HashMap;

        if task_names.is_empty() {
            return Ok(HashMap::new());
        }

        let results: Vec<(String, String)> = crate::interact_on_backend!(self.dal, |conn| {
            task_executions::table
                .filter(task_executions::workflow_execution_id.eq(workflow_execution_id))
                .filter(task_executions::task_name.eq_any(&task_names))
                .select((task_executions::task_name, task_executions::status))
                .load(conn)
        })?;

        Ok(results.into_iter().collect())
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
    ) -> Result<
        std::collections::HashMap<UniversalUuid, std::collections::HashMap<String, String>>,
        ValidationError,
    > {
        use std::collections::HashMap;
        if workflow_execution_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let rows: Vec<(UniversalUuid, String, String)> =
            crate::interact_on_backend!(self.dal, |conn| {
                task_executions::table
                    .filter(task_executions::workflow_execution_id.eq_any(&workflow_execution_ids))
                    .select((
                        task_executions::workflow_execution_id,
                        task_executions::task_name,
                        task_executions::status,
                    ))
                    .load(conn)
            })?;

        let mut grouped: HashMap<UniversalUuid, HashMap<String, String>> = HashMap::new();
        for (exec_id, name, status) in rows {
            grouped.entry(exec_id).or_default().insert(name, status);
        }
        Ok(grouped)
    }
}
