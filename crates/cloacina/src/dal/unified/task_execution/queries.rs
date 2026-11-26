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

use super::{TaskExecutionDAL, WaitingTasks};
use crate::dal::unified::models::UnifiedTaskExecution;
use crate::database::schema::unified::task_executions;
use crate::database::universal_types::{UniversalTimestamp, UniversalUuid};
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

    /// Count the tasks that wait to be dispatched and find the earliest time
    /// one of them became due (CLOACINA-T-1024).
    ///
    /// A waiting task matches the predicate of
    /// [`get_ready_for_retry`](Self::get_ready_for_retry): status `Ready`,
    /// `retry_at` NULL or not in the future, and `claimed_by` NULL. A task
    /// that waits out a retry backoff, or a dispatch that was put off for
    /// capacity (`defer_dispatch`), is not due and is not counted until its
    /// `retry_at` passes.
    ///
    /// Reads no task rows: one count, and two one-column `ORDER BY … LIMIT 1`
    /// reads for the earliest due time (`retry_at` when set, else
    /// `updated_at`, the time the task was marked `Ready`).
    pub async fn waiting_task_stats(&self) -> Result<WaitingTasks, ValidationError> {
        let now = UniversalTimestamp::now();
        let (count, oldest_ready, oldest_retry): (
            i64,
            Option<UniversalTimestamp>,
            Option<UniversalTimestamp>,
        ) = crate::interact_on_backend!(self.dal, |conn| {
            let count: i64 = task_executions::table
                .filter(task_executions::status.eq("Ready"))
                .filter(
                    task_executions::retry_at
                        .is_null()
                        .or(task_executions::retry_at.le(now)),
                )
                .filter(task_executions::claimed_by.is_null())
                .count()
                .get_result(conn)?;
            if count == 0 {
                return Ok((0, None, None));
            }
            let oldest_ready: Option<UniversalTimestamp> = task_executions::table
                .filter(task_executions::status.eq("Ready"))
                .filter(task_executions::retry_at.is_null())
                .filter(task_executions::claimed_by.is_null())
                .order(task_executions::updated_at.asc())
                .select(task_executions::updated_at)
                .first::<UniversalTimestamp>(conn)
                .optional()?;
            let oldest_retry: Option<UniversalTimestamp> = task_executions::table
                .filter(task_executions::status.eq("Ready"))
                .filter(task_executions::retry_at.le(now))
                .filter(task_executions::claimed_by.is_null())
                .order(task_executions::retry_at.asc())
                .select(task_executions::retry_at)
                .first::<Option<UniversalTimestamp>>(conn)
                .optional()?
                .flatten();
            Ok::<_, diesel::result::Error>((count, oldest_ready, oldest_retry))
        })?;

        let oldest_due_at = match (oldest_ready, oldest_retry) {
            (Some(a), Some(b)) => Some(a.0.min(b.0)),
            (a, b) => a.or(b).map(|t| t.0),
        };
        Ok(WaitingTasks {
            count,
            oldest_due_at,
        })
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

#[cfg(all(test, feature = "sqlite"))]
mod waiting_task_stats_tests {
    use crate::dal::DAL;
    use crate::database::Database;
    use crate::database::universal_types::{UniversalTimestamp, UniversalUuid};
    use crate::models::task_execution::NewTaskExecution;
    use crate::models::workflow_execution::NewWorkflowExecution;
    use chrono::{Duration, Utc};

    async fn unique_dal() -> DAL {
        let url = format!(
            "file:waiting_stats_test_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4()
        );
        let db = Database::new(&url, "", 5);
        db.run_migrations()
            .await
            .expect("migrations should succeed");
        DAL::new(db)
    }

    async fn create_workflow(dal: &DAL) -> UniversalUuid {
        dal.workflow_execution()
            .create(NewWorkflowExecution {
                workflow_name: "waiting_workflow".into(),
                workflow_version: "1.0".into(),
                status: "Running".into(),
                context_id: None,
            })
            .await
            .unwrap()
            .id
    }

    async fn create_task(
        dal: &DAL,
        workflow_id: UniversalUuid,
        name: &str,
        status: &str,
    ) -> UniversalUuid {
        dal.task_execution()
            .create(NewTaskExecution {
                workflow_execution_id: workflow_id,
                task_name: name.into(),
                status: status.into(),
                attempt: 1,
                max_attempts: 3,
                trigger_rules: r#"{"type":"Always"}"#.into(),
                task_configuration: "{}".into(),
            })
            .await
            .unwrap()
            .id
    }

    #[tokio::test]
    async fn waiting_task_stats_is_empty_with_no_ready_task() {
        let dal = unique_dal().await;
        let wf = create_workflow(&dal).await;
        create_task(&dal, wf, "not_started", "NotStarted").await;
        create_task(&dal, wf, "running", "Running").await;

        let stats = dal.task_execution().waiting_task_stats().await.unwrap();
        assert_eq!(stats.count, 0);
        assert_eq!(stats.oldest_due_at, None);
        assert_eq!(stats.oldest_age_seconds(Utc::now()), 0.0);
    }

    #[tokio::test]
    async fn waiting_task_stats_counts_only_due_unclaimed_ready_tasks() {
        let dal = unique_dal().await;
        let wf = create_workflow(&dal).await;
        let before = Utc::now();

        // Counted: Ready, no retry_at, no claim.
        create_task(&dal, wf, "fresh", "Ready").await;
        // Counted: Ready, retry_at in the past. This is the oldest.
        let overdue = create_task(&dal, wf, "overdue", "Ready").await;
        let overdue_at = before - Duration::seconds(120);
        dal.task_execution()
            .schedule_retry(overdue, UniversalTimestamp(overdue_at), 2)
            .await
            .unwrap();
        // Not counted: retry_at in the future (retry backoff).
        let backoff = create_task(&dal, wf, "backoff", "Ready").await;
        dal.task_execution()
            .schedule_retry(
                backoff,
                UniversalTimestamp(before + Duration::seconds(600)),
                2,
            )
            .await
            .unwrap();
        // Not counted: dispatch put off for capacity.
        let deferred = create_task(&dal, wf, "deferred", "Ready").await;
        assert!(
            dal.task_execution()
                .defer_dispatch(
                    deferred,
                    UniversalTimestamp(before + Duration::seconds(600))
                )
                .await
                .unwrap()
        );
        // Not counted: Ready with a claim (a dispatch is in flight).
        let claimed = create_task(&dal, wf, "claimed", "Ready").await;
        dal.task_execution()
            .claim_for_runner(claimed, UniversalUuid::new_v4())
            .await
            .unwrap();
        // Not counted: other states.
        create_task(&dal, wf, "running", "Running").await;
        create_task(&dal, wf, "done", "Completed").await;

        let stats = dal.task_execution().waiting_task_stats().await.unwrap();
        assert_eq!(stats.count, 2);
        // The same set that the dispatcher reads.
        let ready = dal.task_execution().get_ready_for_retry().await.unwrap();
        assert_eq!(ready.len() as i64, stats.count);

        let oldest = stats.oldest_due_at.expect("a task waits");
        assert!(
            (oldest - overdue_at).num_milliseconds().abs() < 1000,
            "oldest due time {oldest} should be the overdue retry_at {overdue_at}"
        );
        let age = stats.oldest_age_seconds(Utc::now());
        assert!((119.0..180.0).contains(&age), "age was {age}");
    }

    #[tokio::test]
    async fn waiting_task_stats_uses_updated_at_when_retry_at_is_null() {
        let dal = unique_dal().await;
        let wf = create_workflow(&dal).await;
        let before = Utc::now();
        create_task(&dal, wf, "fresh", "Ready").await;
        let after = Utc::now();

        let stats = dal.task_execution().waiting_task_stats().await.unwrap();
        assert_eq!(stats.count, 1);
        let oldest = stats.oldest_due_at.expect("a task waits");
        assert!(oldest >= before - Duration::seconds(1) && oldest <= after + Duration::seconds(1));
    }
}
