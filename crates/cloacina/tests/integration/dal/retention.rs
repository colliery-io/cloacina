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

//! Integration tests for run-history retention (CLOACINA-T-1023).
//!
//! The unit tests next to `RetentionDAL` cover the rules in detail on SQLite.
//! These run the same deletes on every enabled backend, PostgreSQL included,
//! where the foreign keys, the row locks and the `= ANY(...)` binds are real.
//! They use only the public DAL, so "old" is expressed by the cutoff: a cutoff
//! in the future makes every finished run old, a cutoff in the past none.

use crate::fixtures::get_all_fixtures;
use chrono::{Duration, Utc};
use cloacina::dal::DAL;
use cloacina::dal::unified::RunHistoryDeleted;
use cloacina::models::delivery_outbox::NewDeliveryOutbox;
use cloacina::models::execution_event::NewExecutionEvent;
use cloacina::models::recovery_event::NewRecoveryEvent;
use cloacina::models::schedule::{NewSchedule, NewScheduleExecution};
use cloacina::models::task_execution::NewTaskExecution;
use cloacina::models::task_execution_metadata::NewTaskExecutionMetadata;
use cloacina::models::workflow_execution::NewWorkflowExecution;
use cloacina::{Context, UniversalTimestamp, UniversalUuid};
use serde_json::json;

struct Run {
    execution: UniversalUuid,
    execution_context: UniversalUuid,
    task_context: UniversalUuid,
}

async fn new_context(dal: &DAL) -> UniversalUuid {
    let mut ctx = Context::<serde_json::Value>::new();
    ctx.insert("k", json!("v")).unwrap();
    dal.context()
        .create(&ctx)
        .await
        .unwrap()
        .expect("a non-empty context is stored")
}

/// A running execution with one task and a row in every dependent table.
async fn make_run(dal: &DAL) -> Run {
    let execution_context = new_context(dal).await;
    let task_context = new_context(dal).await;
    let execution = dal
        .workflow_execution()
        .create(NewWorkflowExecution {
            workflow_name: "retention-it".to_string(),
            workflow_version: "1.0".to_string(),
            status: "Running".to_string(),
            context_id: Some(execution_context),
        })
        .await
        .unwrap()
        .id;
    let task = dal
        .task_execution()
        .create(NewTaskExecution {
            workflow_execution_id: execution,
            task_name: "t1".to_string(),
            status: "Completed".to_string(),
            attempt: 1,
            max_attempts: 1,
            trigger_rules: json!({"type": "Always"}).to_string(),
            task_configuration: json!({}).to_string(),
        })
        .await
        .unwrap()
        .id;
    dal.task_execution_metadata()
        .create(NewTaskExecutionMetadata {
            task_execution_id: task,
            workflow_execution_id: execution,
            task_name: "t1".to_string(),
            context_id: Some(task_context),
        })
        .await
        .unwrap();
    dal.execution_event()
        .create(NewExecutionEvent {
            workflow_execution_id: execution,
            task_execution_id: Some(task),
            event_type: "task_completed".to_string(),
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
            workflow_execution_id: execution,
            task_execution_id: Some(task),
            recovery_type: "task_reset".to_string(),
            details: None,
        })
        .await
        .unwrap();
    let schedule = dal
        .schedule()
        .create(NewSchedule::cron(
            &format!("retention-it-{}", uuid::Uuid::new_v4()),
            "0 2 * * *",
            UniversalTimestamp::now(),
        ))
        .await
        .unwrap()
        .id;
    dal.schedule_execution()
        .create(NewScheduleExecution {
            schedule_id: schedule,
            workflow_execution_id: Some(execution),
            scheduled_time: None,
            claimed_at: None,
            context_hash: None,
        })
        .await
        .unwrap();
    Run {
        execution,
        execution_context,
        task_context,
    }
}

async fn context_exists(dal: &DAL, id: UniversalUuid) -> bool {
    dal.context().read::<serde_json::Value>(id).await.is_ok()
}

#[tokio::test]
async fn test_retention_removes_finished_runs_and_keeps_live_ones() {
    for (backend, fixture) in get_all_fixtures().await {
        tracing::info!("Running retention run-history test on {}", backend);

        let mut guard = fixture.lock().unwrap_or_else(|e| e.into_inner());
        guard.reset_database().await;
        guard.initialize().await;
        let dal = DAL::new(guard.get_database());

        let completed = make_run(&dal).await;
        let failed = make_run(&dal).await;
        let cancelled = make_run(&dal).await;
        let running = make_run(&dal).await;
        let paused = make_run(&dal).await;
        dal.workflow_execution()
            .mark_completed(completed.execution)
            .await
            .unwrap();
        dal.workflow_execution()
            .mark_failed(failed.execution, "boom")
            .await
            .unwrap();
        dal.workflow_execution()
            .cancel(cancelled.execution)
            .await
            .unwrap();
        dal.workflow_execution()
            .pause(paused.execution, Some("hold"))
            .await
            .unwrap();

        // A cutoff in the past: every run finished after it, nothing goes.
        let past = UniversalTimestamp::from(Utc::now() - Duration::hours(1));
        let (deleted, more) = dal
            .retention()
            .delete_finished_executions(past, 100, 10)
            .await
            .unwrap();
        assert_eq!(deleted, RunHistoryDeleted::default(), "[{backend}]");
        assert!(!more, "[{backend}]");

        // A cutoff in the future: the three finished runs are old.
        let future = UniversalTimestamp::from(Utc::now() + Duration::hours(1));
        let (deleted, more) = dal
            .retention()
            .delete_finished_executions(future, 2, 10)
            .await
            .unwrap();
        assert!(!more, "[{backend}]");
        assert_eq!(deleted.workflow_executions, 3, "[{backend}]");
        assert_eq!(deleted.task_executions, 3, "[{backend}]");
        assert_eq!(deleted.task_execution_metadata, 3, "[{backend}]");
        assert_eq!(deleted.contexts, 6, "[{backend}]");
        assert_eq!(deleted.recovery_events, 3, "[{backend}]");
        assert_eq!(deleted.schedule_executions, 3, "[{backend}]");
        assert_eq!(deleted.delivery_outbox, 3, "[{backend}]");
        assert!(deleted.execution_events >= 3, "[{backend}]");

        for gone in [&completed, &failed, &cancelled] {
            assert!(
                dal.workflow_execution()
                    .get_by_id(gone.execution)
                    .await
                    .is_err(),
                "[{backend}] a finished run is removed"
            );
            assert!(
                dal.task_execution()
                    .get_all_tasks_for_workflow(gone.execution)
                    .await
                    .unwrap()
                    .is_empty(),
                "[{backend}] its tasks are removed"
            );
            assert!(!context_exists(&dal, gone.execution_context).await);
            assert!(!context_exists(&dal, gone.task_context).await);
        }
        for live in [&running, &paused] {
            assert!(
                dal.workflow_execution()
                    .get_by_id(live.execution)
                    .await
                    .is_ok(),
                "[{backend}] a run that is not finished stays"
            );
            assert_eq!(
                dal.task_execution()
                    .get_all_tasks_for_workflow(live.execution)
                    .await
                    .unwrap()
                    .len(),
                1,
                "[{backend}] its task stays"
            );
            assert!(context_exists(&dal, live.execution_context).await);
            assert!(context_exists(&dal, live.task_context).await);
        }
        // The two live runs still have their event pushes in the outbox.
        assert_eq!(
            dal.delivery_outbox().count_open().await.unwrap(),
            2,
            "[{backend}]"
        );

        // Nothing is left to remove.
        let again = dal
            .retention()
            .delete_finished_executions_batch(future, 100)
            .await
            .unwrap();
        assert_eq!(again, RunHistoryDeleted::default(), "[{backend}]");
    }
}

#[tokio::test]
async fn test_retention_removes_acked_outbox_rows_only() {
    for (backend, fixture) in get_all_fixtures().await {
        tracing::info!("Running retention outbox test on {}", backend);

        let mut guard = fixture.lock().unwrap_or_else(|e| e.into_inner());
        guard.reset_database().await;
        guard.initialize().await;
        let dal = DAL::new(guard.get_database());
        let outbox = dal.delivery_outbox();

        let mut ids = Vec::new();
        for _ in 0..4 {
            let row = outbox
                .enqueue(NewDeliveryOutbox {
                    recipient: "agent:1".to_string(),
                    kind: "work_packet".to_string(),
                    tenant_id: None,
                    payload: vec![1, 2, 3],
                })
                .await
                .unwrap();
            ids.push(row.id);
        }
        outbox.mark_acked(ids[0]).await.unwrap();
        outbox.mark_acked(ids[1]).await.unwrap();
        outbox.mark_delivered(ids[2]).await.unwrap();

        let past = UniversalTimestamp::from(Utc::now() - Duration::hours(1));
        assert_eq!(
            outbox.delete_acked_older_than(past, 100).await.unwrap(),
            0,
            "[{backend}] rows acked after the cutoff stay"
        );

        let future = UniversalTimestamp::from(Utc::now() + Duration::hours(1));
        assert_eq!(
            outbox.delete_acked_older_than(future, 1).await.unwrap(),
            1,
            "[{backend}] the limit bounds one call"
        );
        assert_eq!(
            outbox.delete_acked_older_than(future, 100).await.unwrap(),
            1,
            "[{backend}]"
        );
        assert_eq!(
            outbox.delete_acked_older_than(future, 100).await.unwrap(),
            0,
            "[{backend}]"
        );

        // The delivered row and the pending row are untouched.
        let open: Vec<i64> = outbox
            .list_open_for_recipient("agent:1", 10)
            .await
            .unwrap()
            .iter()
            .map(|r| r.id)
            .collect();
        assert_eq!(open, vec![ids[2], ids[3]], "[{backend}]");
    }
}
