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

//! Retention of run history (CLOACINA-T-1023).
//!
//! A finished workflow execution leaves rows in eight tables. Nothing removed
//! them, so every table grew with each run. [`RetentionDAL`] deletes finished
//! executions older than a cutoff, with every row that hangs off them, in
//! bounded batches.
//!
//! # What one batch removes
//!
//! One batch is one transaction. It picks up to `batch` executions whose
//! status is terminal (`Completed`, `Failed`, `Cancelled`) and whose
//! `completed_at` is before the cutoff (`updated_at` when `completed_at` was
//! never stamped), then deletes, in this order:
//!
//! 1. `execution_events` of those executions
//! 2. `recovery_events` of those executions
//! 3. `schedule_executions` that point at them
//! 4. open `delivery_outbox` rows addressed to `exec_events:<execution id>`
//!    (one is written per execution event and stays `pending` when nobody
//!    subscribed; acked rows belong to
//!    [`DeliveryOutboxDAL::delete_acked_older_than`](super::DeliveryOutboxDAL::delete_acked_older_than))
//! 5. `task_execution_metadata`
//! 6. `task_executions`
//! 7. `workflow_executions`
//! 8. `contexts` that the removed rows pointed at and that no remaining
//!    execution or metadata row references
//!
//! Children go first because only `execution_events` and
//! `schedule_executions` cascade on delete; the other foreign keys are plain
//! and would reject the delete of a parent. Each table is named explicitly, so
//! the row counts are known and the order does not depend on the cascades.
//!
//! # Guards
//!
//! * An execution that is not terminal is never selected. Inside the
//!   transaction a no-change `UPDATE` with the same filter takes the row locks
//!   (Postgres) and the ids are read again, so an execution that left the
//!   terminal state between the first read and the lock is skipped.
//! * A context is removed only after a check that no `workflow_executions` or
//!   `task_execution_metadata` row still references it. On Postgres the
//!   foreign keys back this up: a reference that appears concurrently makes the
//!   delete fail and the whole batch roll back.

use super::DAL;
use crate::database::schema::unified::{
    contexts, delivery_outbox, execution_events, recovery_events, schedule_executions,
    task_execution_metadata, task_executions, workflow_executions,
};
use crate::database::universal_types::{UniversalTimestamp, UniversalUuid};
use crate::error::ValidationError;
use diesel::prelude::*;
use std::collections::HashSet;

/// Statuses after which a workflow execution never changes again.
pub const TERMINAL_EXECUTION_STATUSES: [&str; 3] = ["Completed", "Failed", "Cancelled"];

/// Upper bound on executions per batch. Ids travel as bind parameters, so this
/// also keeps every statement well under SQLite's parameter limit.
pub const MAX_RUN_HISTORY_BATCH: i64 = 500;

/// How many context ids go into one statement.
const CONTEXT_CHUNK: usize = 500;

const OUTBOX_STATE_ACKED: &str = "acked";

/// Rows removed by run-history retention, per table.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct RunHistoryDeleted {
    pub workflow_executions: usize,
    pub task_executions: usize,
    pub task_execution_metadata: usize,
    pub contexts: usize,
    pub execution_events: usize,
    pub recovery_events: usize,
    pub schedule_executions: usize,
    pub delivery_outbox: usize,
}

impl RunHistoryDeleted {
    /// Adds another batch's counts to this one.
    pub fn add(&mut self, other: &RunHistoryDeleted) {
        self.workflow_executions += other.workflow_executions;
        self.task_executions += other.task_executions;
        self.task_execution_metadata += other.task_execution_metadata;
        self.contexts += other.contexts;
        self.execution_events += other.execution_events;
        self.recovery_events += other.recovery_events;
        self.schedule_executions += other.schedule_executions;
        self.delivery_outbox += other.delivery_outbox;
    }

    /// `(table name, rows removed)` for every table, for metrics and logs.
    pub fn per_table(&self) -> [(&'static str, usize); 8] {
        [
            ("workflow_executions", self.workflow_executions),
            ("task_executions", self.task_executions),
            ("task_execution_metadata", self.task_execution_metadata),
            ("contexts", self.contexts),
            ("execution_events", self.execution_events),
            ("recovery_events", self.recovery_events),
            ("schedule_executions", self.schedule_executions),
            ("delivery_outbox", self.delivery_outbox),
        ]
    }

    /// Total rows removed across all tables.
    pub fn total(&self) -> usize {
        self.per_table().iter().map(|(_, n)| n).sum()
    }
}

/// The "finished before the cutoff" predicate on `workflow_executions`. A
/// macro, not a function: the expression type differs per use and the body is
/// compiled once per backend.
macro_rules! finished_before {
    ($cutoff:expr) => {
        workflow_executions::status
            .eq_any(TERMINAL_EXECUTION_STATUSES.to_vec())
            .and(
                workflow_executions::completed_at
                    .lt($cutoff)
                    .or(workflow_executions::completed_at
                        .is_null()
                        .and(workflow_executions::updated_at.lt($cutoff))),
            )
    };
}

/// Data access layer for run-history retention, both backends.
#[derive(Clone)]
pub struct RetentionDAL<'a> {
    dal: &'a DAL,
}

impl<'a> RetentionDAL<'a> {
    pub fn new(dal: &'a DAL) -> Self {
        Self { dal }
    }

    /// Deletes one batch of finished executions older than `cutoff`, with
    /// their dependent rows. See the module docs for the order and the guards.
    ///
    /// `batch` is clamped to `1..=MAX_RUN_HISTORY_BATCH`. Returns the rows
    /// removed per table; `workflow_executions == 0` means nothing is left to
    /// remove for this cutoff.
    pub async fn delete_finished_executions_batch(
        &self,
        cutoff: UniversalTimestamp,
        batch: i64,
    ) -> Result<RunHistoryDeleted, ValidationError> {
        use diesel::connection::Connection;

        let batch = batch.clamp(1, MAX_RUN_HISTORY_BATCH);

        let deleted: RunHistoryDeleted = crate::interact_on_backend!(self.dal, |conn| {
            conn.transaction::<_, diesel::result::Error, _>(|conn| {
                let mut out = RunHistoryDeleted::default();

                // Candidates: terminal and older than the cutoff.
                let candidates: Vec<UniversalUuid> = workflow_executions::table
                    .filter(finished_before!(cutoff))
                    .select(workflow_executions::id)
                    .limit(batch)
                    .load(conn)?;
                if candidates.is_empty() {
                    return Ok(out);
                }

                // Lock the candidates that are still finished (a no-change
                // UPDATE takes the row locks on Postgres), then read back
                // exactly those. From here to commit they cannot change state.
                diesel::update(
                    workflow_executions::table
                        .filter(workflow_executions::id.eq_any(candidates.clone()))
                        .filter(finished_before!(cutoff)),
                )
                .set(workflow_executions::status.eq(workflow_executions::status))
                .execute(conn)?;

                let locked: Vec<(UniversalUuid, Option<UniversalUuid>)> =
                    workflow_executions::table
                        .filter(workflow_executions::id.eq_any(candidates))
                        .filter(finished_before!(cutoff))
                        .select((workflow_executions::id, workflow_executions::context_id))
                        .load(conn)?;
                if locked.is_empty() {
                    return Ok(out);
                }
                let ids: Vec<UniversalUuid> = locked.iter().map(|(id, _)| *id).collect();

                // Contexts these executions and their task metadata point at.
                let mut context_ids: HashSet<UniversalUuid> =
                    locked.iter().filter_map(|(_, ctx)| *ctx).collect();
                let metadata_contexts: Vec<Option<UniversalUuid>> = task_execution_metadata::table
                    .filter(task_execution_metadata::workflow_execution_id.eq_any(ids.clone()))
                    .filter(task_execution_metadata::context_id.is_not_null())
                    .select(task_execution_metadata::context_id)
                    .load(conn)?;
                context_ids.extend(metadata_contexts.into_iter().flatten());

                // Children first.
                out.execution_events = diesel::delete(
                    execution_events::table
                        .filter(execution_events::workflow_execution_id.eq_any(ids.clone())),
                )
                .execute(conn)?;

                out.recovery_events = diesel::delete(
                    recovery_events::table
                        .filter(recovery_events::workflow_execution_id.eq_any(ids.clone())),
                )
                .execute(conn)?;

                out.schedule_executions = diesel::delete(
                    schedule_executions::table
                        .filter(schedule_executions::workflow_execution_id.eq_any(ids.clone())),
                )
                .execute(conn)?;

                // Event pushes nobody acked. The `<> 'acked'` filter matches
                // the partial index on (recipient, id).
                let recipients: Vec<String> = ids
                    .iter()
                    .map(|id| format!("exec_events:{}", id.0))
                    .collect();
                out.delivery_outbox = diesel::delete(
                    delivery_outbox::table
                        .filter(delivery_outbox::recipient.eq_any(recipients))
                        .filter(delivery_outbox::delivery_state.ne(OUTBOX_STATE_ACKED)),
                )
                .execute(conn)?;

                out.task_execution_metadata = diesel::delete(
                    task_execution_metadata::table
                        .filter(task_execution_metadata::workflow_execution_id.eq_any(ids.clone())),
                )
                .execute(conn)?;

                out.task_executions = diesel::delete(
                    task_executions::table
                        .filter(task_executions::workflow_execution_id.eq_any(ids.clone())),
                )
                .execute(conn)?;

                out.workflow_executions = diesel::delete(
                    workflow_executions::table.filter(workflow_executions::id.eq_any(ids)),
                )
                .execute(conn)?;

                // Contexts last, and only those nothing references any more.
                let context_ids: Vec<UniversalUuid> = context_ids.into_iter().collect();
                for chunk in context_ids.chunks(CONTEXT_CHUNK) {
                    let mut still_used: HashSet<UniversalUuid> = HashSet::new();
                    let by_execution: Vec<Option<UniversalUuid>> = workflow_executions::table
                        .filter(workflow_executions::context_id.eq_any(chunk.to_vec()))
                        .select(workflow_executions::context_id)
                        .load(conn)?;
                    still_used.extend(by_execution.into_iter().flatten());
                    let by_metadata: Vec<Option<UniversalUuid>> = task_execution_metadata::table
                        .filter(task_execution_metadata::context_id.eq_any(chunk.to_vec()))
                        .select(task_execution_metadata::context_id)
                        .load(conn)?;
                    still_used.extend(by_metadata.into_iter().flatten());

                    let free: Vec<UniversalUuid> = chunk
                        .iter()
                        .filter(|id| !still_used.contains(*id))
                        .copied()
                        .collect();
                    if free.is_empty() {
                        continue;
                    }
                    out.contexts +=
                        diesel::delete(contexts::table.filter(contexts::id.eq_any(free)))
                            .execute(conn)?;
                }

                Ok(out)
            })
        })?;

        Ok(deleted)
    }

    /// Runs [`delete_finished_executions_batch`](Self::delete_finished_executions_batch)
    /// until nothing is left or `max_batches` batches ran. Returns the summed
    /// counts and whether more work remains (the last batch was full).
    pub async fn delete_finished_executions(
        &self,
        cutoff: UniversalTimestamp,
        batch: i64,
        max_batches: usize,
    ) -> Result<(RunHistoryDeleted, bool), ValidationError> {
        let batch = batch.clamp(1, MAX_RUN_HISTORY_BATCH);
        let mut total = RunHistoryDeleted::default();
        for _ in 0..max_batches {
            let one = self.delete_finished_executions_batch(cutoff, batch).await?;
            total.add(&one);
            if (one.workflow_executions as i64) < batch {
                return Ok((total, false));
            }
        }
        Ok((total, true))
    }
}

#[cfg(all(test, feature = "sqlite"))]
mod tests {
    use super::*;
    use crate::database::Database;
    use crate::models::delivery_outbox::NewDeliveryOutbox;
    use crate::models::execution_event::NewExecutionEvent;
    use crate::models::recovery_event::NewRecoveryEvent;
    use crate::models::schedule::{NewSchedule, NewScheduleExecution};
    use crate::models::task_execution::NewTaskExecution;
    use crate::models::task_execution_metadata::NewTaskExecutionMetadata;
    use crate::models::workflow_execution::NewWorkflowExecution;
    use chrono::{Duration, Utc};

    async fn unique_dal() -> DAL {
        let url = format!(
            "file:retention_test_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4()
        );
        let db = Database::new(&url, "", 5);
        db.run_migrations()
            .await
            .expect("migrations should succeed");
        DAL::new(db)
    }

    async fn new_context(dal: &DAL) -> UniversalUuid {
        let mut ctx = crate::Context::<serde_json::Value>::new();
        ctx.insert("k", serde_json::json!("v")).unwrap();
        dal.context()
            .create(&ctx)
            .await
            .unwrap()
            .expect("a non-empty context is stored")
    }

    /// Row counts of the eight tables, in `RunHistoryDeleted::per_table` order.
    async fn counts(dal: &DAL) -> [i64; 8] {
        crate::interact_on_backend!(dal, |conn| {
            Ok::<_, diesel::result::Error>([
                workflow_executions::table.count().get_result(conn)?,
                task_executions::table.count().get_result(conn)?,
                task_execution_metadata::table.count().get_result(conn)?,
                contexts::table.count().get_result(conn)?,
                execution_events::table.count().get_result(conn)?,
                recovery_events::table.count().get_result(conn)?,
                schedule_executions::table.count().get_result(conn)?,
                delivery_outbox::table.count().get_result(conn)?,
            ])
        })
        .unwrap()
    }

    /// A run with one task and rows in every dependent table: 2 contexts
    /// (execution + task), 3 events (the DAL writes one when the execution is
    /// created and one when the task is; the third is explicit and also writes
    /// 1 outbox row), 1 recovery event, 1 schedule execution. Left in `status`.
    async fn make_run(dal: &DAL, status: &str) -> UniversalUuid {
        let exec_ctx = new_context(dal).await;
        let task_ctx = new_context(dal).await;
        let wf = dal
            .workflow_execution()
            .create(NewWorkflowExecution {
                workflow_name: "retention-test".into(),
                workflow_version: "1.0".into(),
                status: "Running".into(),
                context_id: Some(exec_ctx),
            })
            .await
            .unwrap()
            .id;
        let task = dal
            .task_execution()
            .create(NewTaskExecution {
                workflow_execution_id: wf,
                task_name: "t1".into(),
                status: "Completed".into(),
                attempt: 1,
                max_attempts: 1,
                trigger_rules: "{}".into(),
                task_configuration: "{}".into(),
            })
            .await
            .unwrap()
            .id;
        dal.task_execution_metadata()
            .create(NewTaskExecutionMetadata {
                task_execution_id: task,
                workflow_execution_id: wf,
                task_name: "t1".into(),
                context_id: Some(task_ctx),
                context_size_bytes: None,
            })
            .await
            .unwrap();
        dal.execution_event()
            .create(NewExecutionEvent {
                workflow_execution_id: wf,
                task_execution_id: Some(task),
                event_type: "task_completed".into(),
                event_data: None,
                worker_id: None,
                request_id: None,
                runner_id: None,
                tenant_id: None,
            })
            .await
            .unwrap();
        dal.recovery_event()
            .create(NewRecoveryEvent {
                workflow_execution_id: wf,
                task_execution_id: Some(task),
                recovery_type: "task_reset".into(),
                details: None,
            })
            .await
            .unwrap();
        let schedule = dal
            .schedule()
            .create(NewSchedule::cron(
                &format!("retention-test-{}", uuid::Uuid::new_v4()),
                "0 2 * * *",
                UniversalTimestamp::now(),
            ))
            .await
            .unwrap()
            .id;
        dal.schedule_execution()
            .create(NewScheduleExecution {
                schedule_id: schedule,
                workflow_execution_id: Some(wf),
                scheduled_time: None,
                claimed_at: None,
                context_hash: None,
            })
            .await
            .unwrap();
        set_state(dal, wf, status, None).await;
        wf
    }

    /// Sets the status directly (no event, so the row counts stay as made)
    /// and, with `finished_ago`, stamps `completed_at`/`updated_at` that far
    /// in the past.
    async fn set_state(dal: &DAL, wf: UniversalUuid, status: &str, finished_ago: Option<Duration>) {
        let status = status.to_string();
        let terminal = TERMINAL_EXECUTION_STATUSES.contains(&status.as_str());
        let at = UniversalTimestamp::from(Utc::now() - finished_ago.unwrap_or_else(Duration::zero));
        crate::interact_on_backend!(dal, |conn| {
            diesel::update(workflow_executions::table.find(wf))
                .set((
                    workflow_executions::status.eq(status),
                    workflow_executions::completed_at.eq(terminal.then_some(at)),
                    workflow_executions::updated_at.eq(at),
                ))
                .execute(conn)
        })
        .unwrap();
    }

    fn cutoff(period: Duration) -> UniversalTimestamp {
        UniversalTimestamp::from(Utc::now() - period)
    }

    #[tokio::test]
    async fn removes_a_finished_run_and_every_dependent_row() {
        let dal = unique_dal().await;
        let wf = make_run(&dal, "Completed").await;
        set_state(&dal, wf, "Completed", Some(Duration::seconds(10))).await;
        assert_eq!(counts(&dal).await, [1, 1, 1, 2, 3, 1, 1, 1]);

        // Short period: 2 seconds. The run finished 10 seconds ago.
        let deleted = dal
            .retention()
            .delete_finished_executions_batch(cutoff(Duration::seconds(2)), 100)
            .await
            .unwrap();

        assert_eq!(
            deleted,
            RunHistoryDeleted {
                workflow_executions: 1,
                task_executions: 1,
                task_execution_metadata: 1,
                contexts: 2,
                execution_events: 3,
                recovery_events: 1,
                schedule_executions: 1,
                delivery_outbox: 1,
            }
        );
        assert_eq!(deleted.total(), 11);
        assert_eq!(counts(&dal).await, [0; 8]);
    }

    #[tokio::test]
    async fn keeps_a_finished_run_inside_the_period() {
        let dal = unique_dal().await;
        let wf = make_run(&dal, "Completed").await;
        set_state(&dal, wf, "Completed", Some(Duration::seconds(1))).await;
        let before = counts(&dal).await;

        let deleted = dal
            .retention()
            .delete_finished_executions_batch(cutoff(Duration::hours(1)), 100)
            .await
            .unwrap();

        assert_eq!(deleted, RunHistoryDeleted::default());
        assert_eq!(counts(&dal).await, before);
    }

    #[tokio::test]
    async fn never_removes_a_run_that_is_not_terminal() {
        let dal = unique_dal().await;
        // Old by every timestamp, but still live.
        for status in ["Pending", "Running", "Paused"] {
            let wf = make_run(&dal, status).await;
            set_state(&dal, wf, status, Some(Duration::days(400))).await;
        }
        let before = counts(&dal).await;
        assert_eq!(before[0], 3);

        let (deleted, more) = dal
            .retention()
            .delete_finished_executions(cutoff(Duration::seconds(1)), 100, 10)
            .await
            .unwrap();

        assert_eq!(deleted, RunHistoryDeleted::default());
        assert!(!more);
        assert_eq!(counts(&dal).await, before);
    }

    #[tokio::test]
    async fn removes_failed_and_cancelled_runs_and_leaves_the_live_one() {
        let dal = unique_dal().await;
        let failed = make_run(&dal, "Failed").await;
        let cancelled = make_run(&dal, "Cancelled").await;
        let running = make_run(&dal, "Running").await;
        for (wf, status) in [
            (failed, "Failed"),
            (cancelled, "Cancelled"),
            (running, "Running"),
        ] {
            set_state(&dal, wf, status, Some(Duration::seconds(30))).await;
        }

        let deleted = dal
            .retention()
            .delete_finished_executions_batch(cutoff(Duration::seconds(5)), 100)
            .await
            .unwrap();

        assert_eq!(deleted.workflow_executions, 2);
        // Exactly the live run's rows remain.
        assert_eq!(counts(&dal).await, [1, 1, 1, 2, 3, 1, 1, 1]);
        assert!(
            dal.workflow_execution().get_by_id(running).await.is_ok(),
            "the running execution is untouched"
        );
    }

    #[tokio::test]
    async fn a_terminal_run_with_no_completed_at_uses_updated_at() {
        let dal = unique_dal().await;
        let wf = make_run(&dal, "Failed").await;
        let old = UniversalTimestamp::from(Utc::now() - Duration::seconds(30));
        crate::interact_on_backend!(&dal, |conn| {
            diesel::update(workflow_executions::table.find(wf))
                .set((
                    workflow_executions::completed_at.eq(None::<UniversalTimestamp>),
                    workflow_executions::updated_at.eq(old),
                ))
                .execute(conn)
        })
        .unwrap();

        let deleted = dal
            .retention()
            .delete_finished_executions_batch(cutoff(Duration::seconds(5)), 100)
            .await
            .unwrap();
        assert_eq!(deleted.workflow_executions, 1);
    }

    #[tokio::test]
    async fn keeps_a_context_a_live_run_still_references() {
        let dal = unique_dal().await;
        let old = make_run(&dal, "Completed").await;
        set_state(&dal, old, "Completed", Some(Duration::seconds(30))).await;

        // A live run shares the old run's execution context, and a live
        // run's task metadata shares the old run's task context.
        let (old_exec_ctx, old_task_ctx): (Option<UniversalUuid>, Option<UniversalUuid>) =
            crate::interact_on_backend!(&dal, |conn| {
                let e: Option<UniversalUuid> = workflow_executions::table
                    .find(old)
                    .select(workflow_executions::context_id)
                    .first(conn)?;
                let t: Option<UniversalUuid> = task_execution_metadata::table
                    .filter(task_execution_metadata::workflow_execution_id.eq(old))
                    .select(task_execution_metadata::context_id)
                    .first(conn)?;
                Ok::<_, diesel::result::Error>((e, t))
            })
            .unwrap();
        let live = dal
            .workflow_execution()
            .create(NewWorkflowExecution {
                workflow_name: "retention-test".into(),
                workflow_version: "1.0".into(),
                status: "Running".into(),
                context_id: old_exec_ctx,
            })
            .await
            .unwrap()
            .id;
        let live_task = dal
            .task_execution()
            .create(NewTaskExecution {
                workflow_execution_id: live,
                task_name: "t1".into(),
                status: "Running".into(),
                attempt: 1,
                max_attempts: 1,
                trigger_rules: "{}".into(),
                task_configuration: "{}".into(),
            })
            .await
            .unwrap()
            .id;
        dal.task_execution_metadata()
            .create(NewTaskExecutionMetadata {
                task_execution_id: live_task,
                workflow_execution_id: live,
                task_name: "t1".into(),
                context_id: old_task_ctx,
                context_size_bytes: None,
            })
            .await
            .unwrap();

        let deleted = dal
            .retention()
            .delete_finished_executions_batch(cutoff(Duration::seconds(5)), 100)
            .await
            .unwrap();

        assert_eq!(deleted.workflow_executions, 1);
        assert_eq!(deleted.contexts, 0, "both contexts are still referenced");
        let left = counts(&dal).await;
        assert_eq!(left[0], 1, "the live execution stays");
        assert_eq!(left[3], 2, "both shared contexts stay");
    }

    #[tokio::test]
    async fn leaves_outbox_rows_of_other_recipients() {
        let dal = unique_dal().await;
        let wf = make_run(&dal, "Completed").await;
        set_state(&dal, wf, "Completed", Some(Duration::seconds(30))).await;
        // Fleet work for an agent: not this run's event push.
        dal.delivery_outbox()
            .enqueue(NewDeliveryOutbox {
                recipient: "agent:1".into(),
                kind: "work_packet".into(),
                tenant_id: None,
                payload: vec![1, 2, 3],
            })
            .await
            .unwrap();

        let deleted = dal
            .retention()
            .delete_finished_executions_batch(cutoff(Duration::seconds(5)), 100)
            .await
            .unwrap();

        assert_eq!(deleted.delivery_outbox, 1);
        assert_eq!(dal.delivery_outbox().count_open().await.unwrap(), 1);
    }

    #[tokio::test]
    async fn works_in_bounded_batches() {
        let dal = unique_dal().await;
        for _ in 0..5 {
            let wf = make_run(&dal, "Completed").await;
            set_state(&dal, wf, "Completed", Some(Duration::seconds(30))).await;
        }
        let cut = cutoff(Duration::seconds(5));

        // One batch of 2 removes 2 runs.
        let one = dal
            .retention()
            .delete_finished_executions_batch(cut, 2)
            .await
            .unwrap();
        assert_eq!(one.workflow_executions, 2);
        assert_eq!(counts(&dal).await[0], 3);

        // A cap of 1 batch stops early and says more remains.
        let (capped, more) = dal
            .retention()
            .delete_finished_executions(cut, 2, 1)
            .await
            .unwrap();
        assert_eq!(capped.workflow_executions, 2);
        assert!(more);

        // No cap in the way: the rest goes and nothing remains.
        let (rest, more) = dal
            .retention()
            .delete_finished_executions(cut, 2, 10)
            .await
            .unwrap();
        assert_eq!(rest.workflow_executions, 1);
        assert!(!more);
        assert_eq!(counts(&dal).await, [0; 8]);
    }
}
