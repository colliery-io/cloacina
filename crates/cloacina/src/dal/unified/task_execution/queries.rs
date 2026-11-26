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

    /// Count the tasks that wait for capacity and find the one that has
    /// waited longest (CLOACINA-T-1024, CLOACINA-T-1036).
    ///
    /// A waiting task has status `Ready` and a `waiting_since` that is not in
    /// the future. `waiting_since` is set when the task becomes `Ready` and
    /// cleared when it starts or ends, so:
    ///
    /// - A task whose dispatch is put off for capacity
    ///   ([`defer_dispatch`](Self::defer_dispatch)) is counted the whole
    ///   time: a deferral moves `retry_at`, not `waiting_since`.
    /// - A task that an executor holds a claim on and has not started (a
    ///   dispatch try on a fleet, a wait for a slot in process) is counted.
    /// - A retry in backoff is not counted: its `waiting_since` is its
    ///   `retry_at`, in the future until the retry is due.
    ///
    /// A `Ready` row with no `waiting_since` (written by a replica that runs
    /// a version from before the column) is counted when its `retry_at` is
    /// empty or past, and adds nothing to the age.
    ///
    /// Reads no task rows: one count, and a one-column `ORDER BY … LIMIT 1`
    /// read for the earliest `waiting_since`.
    pub async fn waiting_task_stats(&self) -> Result<WaitingTasks, ValidationError> {
        let now = UniversalTimestamp::now();
        let (count, oldest): (i64, Option<UniversalTimestamp>) =
            crate::interact_on_backend!(self.dal, |conn| {
                let count: i64 = task_executions::table
                    .filter(task_executions::status.eq("Ready"))
                    .filter(
                        task_executions::waiting_since
                            .le(now)
                            .or(task_executions::waiting_since.is_null().and(
                                task_executions::retry_at
                                    .is_null()
                                    .or(task_executions::retry_at.le(now)),
                            )),
                    )
                    .count()
                    .get_result(conn)?;
                if count == 0 {
                    return Ok((0, None));
                }
                let oldest: Option<UniversalTimestamp> = task_executions::table
                    .filter(task_executions::status.eq("Ready"))
                    .filter(task_executions::waiting_since.le(now))
                    .order(task_executions::waiting_since.asc())
                    .select(task_executions::waiting_since)
                    .first::<Option<UniversalTimestamp>>(conn)
                    .optional()?
                    .flatten();
                Ok::<_, diesel::result::Error>((count, oldest))
            })?;

        Ok(WaitingTasks {
            count,
            oldest_waiting_since: oldest.map(|t| t.0),
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

    /// Move `waiting_since` of a task into the past, as if it became Ready
    /// `seconds` ago.
    async fn age_waiting_since(dal: &DAL, task_id: UniversalUuid, seconds: i64) {
        use crate::database::schema::unified::task_executions;
        use diesel::prelude::*;
        let since = UniversalTimestamp(Utc::now() - Duration::seconds(seconds));
        crate::interact_on_backend!(dal, |conn| {
            diesel::update(task_executions::table.find(task_id))
                .set(task_executions::waiting_since.eq(Some(since)))
                .execute(conn)
        })
        .unwrap();
    }

    async fn waiting_since(dal: &DAL, task_id: UniversalUuid) -> Option<UniversalTimestamp> {
        dal.task_execution()
            .get_by_id(task_id)
            .await
            .unwrap()
            .waiting_since
    }

    #[tokio::test]
    async fn waiting_task_stats_is_empty_with_no_ready_task() {
        let dal = unique_dal().await;
        let wf = create_workflow(&dal).await;
        create_task(&dal, wf, "not_started", "NotStarted").await;
        create_task(&dal, wf, "running", "Running").await;

        let stats = dal.task_execution().waiting_task_stats().await.unwrap();
        assert_eq!(stats.count, 0);
        assert_eq!(stats.oldest_waiting_since, None);
        assert_eq!(stats.oldest_age_seconds(Utc::now()), 0.0);
    }

    #[tokio::test]
    async fn waiting_task_stats_counts_ready_tasks_that_wait_for_capacity() {
        let dal = unique_dal().await;
        let wf = create_workflow(&dal).await;
        let before = Utc::now();

        // Counted: Ready, just created.
        create_task(&dal, wf, "fresh", "Ready").await;
        // Counted: a retry that became due 120 s ago. This is the oldest.
        let overdue = create_task(&dal, wf, "overdue", "Ready").await;
        let overdue_at = before - Duration::seconds(120);
        dal.task_execution()
            .schedule_retry(overdue, UniversalTimestamp(overdue_at), 2)
            .await
            .unwrap();
        // Not counted: a retry in backoff.
        let backoff = create_task(&dal, wf, "backoff", "Ready").await;
        dal.task_execution()
            .schedule_retry(
                backoff,
                UniversalTimestamp(before + Duration::seconds(600)),
                2,
            )
            .await
            .unwrap();
        // Counted: dispatch put off for capacity.
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
        // Counted: Ready with a claim and not started (a dispatch try, or a
        // wait for a slot).
        let claimed = create_task(&dal, wf, "claimed", "Ready").await;
        dal.task_execution()
            .claim_for_runner(claimed, UniversalUuid::new_v4())
            .await
            .unwrap();
        // Not counted: other states.
        create_task(&dal, wf, "not_started", "NotStarted").await;
        create_task(&dal, wf, "running", "Running").await;
        create_task(&dal, wf, "done", "Completed").await;

        let stats = dal.task_execution().waiting_task_stats().await.unwrap();
        assert_eq!(stats.count, 4);

        let oldest = stats.oldest_waiting_since.expect("a task waits");
        assert!(
            (oldest - overdue_at).num_milliseconds().abs() < 1000,
            "oldest waiting_since {oldest} should be the overdue retry_at {overdue_at}"
        );
        let age = stats.oldest_age_seconds(Utc::now());
        assert!((119.0..180.0).contains(&age), "age was {age}");
    }

    #[tokio::test]
    async fn waiting_since_is_set_when_a_task_becomes_ready() {
        let dal = unique_dal().await;
        let wf = create_workflow(&dal).await;

        let before = Utc::now() - Duration::seconds(1);
        let created_ready = create_task(&dal, wf, "created_ready", "Ready").await;
        let task = create_task(&dal, wf, "task", "NotStarted").await;
        assert_eq!(waiting_since(&dal, task).await, None);
        dal.task_execution().mark_ready(task).await.unwrap();
        let after = Utc::now() + Duration::seconds(1);

        for id in [created_ready, task] {
            let since = waiting_since(&dal, id).await.expect("set when Ready").0;
            assert!(since >= before && since <= after, "waiting_since {since}");
        }
        let stats = dal.task_execution().waiting_task_stats().await.unwrap();
        assert_eq!(stats.count, 2);
    }

    /// CLOACINA-T-1036: a fleet task with no free agent is deferred again and
    /// again. It is one waiting task the whole time and its age grows.
    #[tokio::test]
    async fn a_task_deferred_three_times_is_counted_once_and_its_age_grows() {
        let dal = unique_dal().await;
        let wf = create_workflow(&dal).await;
        let runner = UniversalUuid::new_v4();
        let task = create_task(&dal, wf, "waits_for_an_agent", "NotStarted").await;
        dal.task_execution().mark_ready(task).await.unwrap();
        let ready_at = waiting_since(&dal, task).await.expect("set by mark_ready");

        let mut last_age = 0.0;
        for round in 1..=3 {
            // The fleet executor claims the task for a dispatch try.
            dal.task_execution()
                .claim_for_runner(task, runner)
                .await
                .unwrap();
            let stats = dal.task_execution().waiting_task_stats().await.unwrap();
            assert_eq!(stats.count, 1, "claimed for dispatch, round {round}");

            // No agent is free: the try is put off 2 s and the claim released.
            let not_before = UniversalTimestamp(Utc::now() + Duration::seconds(2));
            assert!(
                dal.task_execution()
                    .defer_dispatch(task, not_before)
                    .await
                    .unwrap()
            );
            dal.task_execution()
                .release_runner_claim(task)
                .await
                .unwrap();

            // Between tries the task is not due for dispatch.
            let due = dal.task_execution().get_ready_for_retry().await.unwrap();
            assert!(due.is_empty(), "deferred, round {round}");

            // It still waits, since the time it became Ready.
            assert_eq!(waiting_since(&dal, task).await, Some(ready_at));
            tokio::time::sleep(std::time::Duration::from_millis(30)).await;
            let stats = dal.task_execution().waiting_task_stats().await.unwrap();
            assert_eq!(stats.count, 1, "deferred, round {round}");
            assert_eq!(stats.oldest_waiting_since, Some(ready_at.0));
            let age = stats.oldest_age_seconds(Utc::now());
            assert!(age > last_age, "age {age} did not grow from {last_age}");
            last_age = age;
        }

        // An agent takes the task: the wait is over.
        dal.task_execution()
            .claim_for_runner(task, runner)
            .await
            .unwrap();
        dal.task_execution().mark_started(task).await.unwrap();
        assert_eq!(waiting_since(&dal, task).await, None);
        let stats = dal.task_execution().waiting_task_stats().await.unwrap();
        assert_eq!(stats.count, 0);
        assert_eq!(stats.oldest_age_seconds(Utc::now()), 0.0);
    }

    #[tokio::test]
    async fn a_claimed_deferred_task_is_counted_with_its_full_age() {
        let dal = unique_dal().await;
        let wf = create_workflow(&dal).await;
        let task = create_task(&dal, wf, "waits", "Ready").await;
        age_waiting_since(&dal, task, 300).await;

        // Deferred, and its `retry_at` has not passed.
        dal.task_execution()
            .defer_dispatch(
                task,
                UniversalTimestamp(Utc::now() + Duration::seconds(600)),
            )
            .await
            .unwrap();
        // The fleet executor holds the claim for the next dispatch try.
        dal.task_execution()
            .claim_for_runner(task, UniversalUuid::new_v4())
            .await
            .unwrap();

        let stats = dal.task_execution().waiting_task_stats().await.unwrap();
        assert_eq!(stats.count, 1);
        let age = stats.oldest_age_seconds(Utc::now());
        assert!((299.0..360.0).contains(&age), "age was {age}");
    }

    #[tokio::test]
    async fn a_retry_in_backoff_is_not_counted_until_it_is_due() {
        let dal = unique_dal().await;
        let wf = create_workflow(&dal).await;
        let task = create_task(&dal, wf, "retried", "Running").await;

        // The backoff ends in 300 ms.
        let due_at = UniversalTimestamp(Utc::now() + Duration::milliseconds(300));
        dal.task_execution()
            .schedule_retry(task, due_at, 2)
            .await
            .unwrap();
        assert_eq!(waiting_since(&dal, task).await, Some(due_at));
        let stats = dal.task_execution().waiting_task_stats().await.unwrap();
        assert_eq!(stats.count, 0, "in backoff");
        assert_eq!(stats.oldest_waiting_since, None);

        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        let stats = dal.task_execution().waiting_task_stats().await.unwrap();
        assert_eq!(stats.count, 1, "the retry is due");
        // The wait starts at the due time, not at the failure.
        assert_eq!(stats.oldest_waiting_since, Some(due_at.0));
        let age = stats.oldest_age_seconds(Utc::now());
        assert!((0.1..5.0).contains(&age), "age was {age}");

        // A deferral of the due retry keeps the wait.
        dal.task_execution()
            .defer_dispatch(
                task,
                UniversalTimestamp(Utc::now() + Duration::seconds(600)),
            )
            .await
            .unwrap();
        let stats = dal.task_execution().waiting_task_stats().await.unwrap();
        assert_eq!(stats.count, 1);
        assert_eq!(stats.oldest_waiting_since, Some(due_at.0));
    }

    #[tokio::test]
    async fn waiting_since_is_cleared_when_a_task_starts_or_ends() {
        let dal = unique_dal().await;
        let wf = create_workflow(&dal).await;
        let te = dal.task_execution();

        let started = create_task(&dal, wf, "started", "Ready").await;
        te.mark_started(started).await.unwrap();
        let completed = create_task(&dal, wf, "completed", "Ready").await;
        te.mark_completed(completed, None).await.unwrap();
        let failed = create_task(&dal, wf, "failed", "Ready").await;
        te.mark_failed(failed, "boom", None).await.unwrap();
        let skipped = create_task(&dal, wf, "skipped", "Ready").await;
        te.mark_skipped(skipped, "not needed").await.unwrap();
        let abandoned = create_task(&dal, wf, "abandoned", "Ready").await;
        te.mark_abandoned(abandoned, "gave up").await.unwrap();

        for id in [started, completed, failed, skipped, abandoned] {
            assert_eq!(waiting_since(&dal, id).await, None);
        }
        let stats = te.waiting_task_stats().await.unwrap();
        assert_eq!(stats.count, 0);
    }

    #[tokio::test]
    async fn recovery_keeps_the_wait_of_a_ready_task_and_restarts_a_running_one() {
        let dal = unique_dal().await;
        let wf = create_workflow(&dal).await;
        let te = dal.task_execution();

        // The runner died between the claim and the start.
        let ready = create_task(&dal, wf, "ready", "Ready").await;
        age_waiting_since(&dal, ready, 300).await;
        let ready_since = waiting_since(&dal, ready).await;
        te.reset_task_for_recovery(ready).await.unwrap();
        assert_eq!(waiting_since(&dal, ready).await, ready_since);

        // The runner died while the task ran.
        let running = create_task(&dal, wf, "running", "Ready").await;
        te.mark_started(running).await.unwrap();
        let before = Utc::now() - Duration::seconds(1);
        te.reset_task_for_recovery(running).await.unwrap();
        let since = waiting_since(&dal, running).await.expect("waits again").0;
        assert!(since >= before, "waiting_since {since}");

        let stats = te.waiting_task_stats().await.unwrap();
        assert_eq!(stats.count, 2);
    }

    /// A `Ready` row from before the column (or from a replica of an older
    /// version) has no `waiting_since`.
    #[tokio::test]
    async fn a_ready_row_with_no_waiting_since_is_counted_when_due_with_no_age() {
        use crate::database::schema::unified::task_executions;
        use diesel::prelude::*;

        let dal = unique_dal().await;
        let wf = create_workflow(&dal).await;
        let due = create_task(&dal, wf, "due", "Ready").await;
        let later = create_task(&dal, wf, "later", "Ready").await;
        let in_future = UniversalTimestamp(Utc::now() + Duration::seconds(600));
        crate::interact_on_backend!(&dal, |conn| {
            diesel::update(task_executions::table.find(due))
                .set(task_executions::waiting_since.eq(None::<UniversalTimestamp>))
                .execute(conn)?;
            diesel::update(task_executions::table.find(later))
                .set((
                    task_executions::waiting_since.eq(None::<UniversalTimestamp>),
                    task_executions::retry_at.eq(Some(in_future)),
                ))
                .execute(conn)
        })
        .unwrap();

        let stats = dal.task_execution().waiting_task_stats().await.unwrap();
        assert_eq!(stats.count, 1);
        assert_eq!(stats.oldest_waiting_since, None);
        assert_eq!(stats.oldest_age_seconds(Utc::now()), 0.0);
    }

    /// Migration 057 on a database with rows: `Ready` rows get a value, a
    /// retry gets its due time, other rows stay empty.
    #[tokio::test]
    async fn migration_057_backfills_ready_rows() {
        use crate::database::schema::unified::task_executions;
        use diesel::prelude::*;

        let dal = unique_dal().await;
        let wf = create_workflow(&dal).await;
        let fresh = create_task(&dal, wf, "fresh", "Ready").await;
        let retry = create_task(&dal, wf, "retry", "Ready").await;
        let running = create_task(&dal, wf, "running", "Running").await;
        let retry_at = UniversalTimestamp(Utc::now() + Duration::seconds(600));
        let backfill =
            include_str!("../../../database/migrations/sqlite/057_add_task_waiting_since/up.sql")
                .split(';')
                .map(|statement| {
                    statement
                        .lines()
                        .filter(|line| !line.trim_start().starts_with("--"))
                        .collect::<Vec<_>>()
                        .join("\n")
                })
                .find(|statement| statement.trim_start().starts_with("UPDATE"))
                .expect("the migration has an UPDATE statement");
        // The rows as the version before the column left them.
        crate::interact_on_backend!(&dal, |conn| {
            diesel::update(task_executions::table)
                .set(task_executions::waiting_since.eq(None::<UniversalTimestamp>))
                .execute(conn)?;
            diesel::update(task_executions::table.find(retry))
                .set(task_executions::retry_at.eq(Some(retry_at)))
                .execute(conn)?;
            // The data statement of the migration file.
            diesel::sql_query(backfill.clone()).execute(conn)
        })
        .unwrap();

        let fresh_row = dal.task_execution().get_by_id(fresh).await.unwrap();
        assert_eq!(fresh_row.waiting_since, Some(fresh_row.updated_at));
        assert_eq!(waiting_since(&dal, retry).await, Some(retry_at));
        assert_eq!(waiting_since(&dal, running).await, None);

        let stats = dal.task_execution().waiting_task_stats().await.unwrap();
        assert_eq!(stats.count, 1, "the retry in backoff is not counted");
    }
}
