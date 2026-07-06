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

use crate::fixtures::get_or_init_fixture;
use chrono::Utc;
use cloacina::cron_evaluator::CronEvaluator;
use cloacina::database::universal_types::UniversalTimestamp;
use cloacina::models::schedule::{NewSchedule, NewScheduleExecution};
use cloacina::runner::{DefaultRunner, DefaultRunnerConfig};
use serial_test::serial;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
#[serial]
async fn test_cron_evaluator_basic() {
    let evaluator = CronEvaluator::new("*/5 * * * *", "UTC").unwrap(); // Every 5 minutes (no seconds)

    let now = Utc::now();
    let next = evaluator.next_execution(now).unwrap();

    // Should be in the future
    assert!(next > now);

    // Should be within the next 5 minutes
    let diff = next - now;
    assert!(diff <= chrono::Duration::try_minutes(5).unwrap());
}

#[tokio::test]
#[serial]
async fn test_cron_schedule_creation() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());
    fixture.initialize().await;
    let dal = fixture.get_dal();

    let schedule = NewSchedule::cron(
        "test-workflow",
        "0 0 * * *",
        cloacina::database::UniversalTimestamp(Utc::now()),
    );

    let created_schedule = dal.schedule().create(schedule).await.unwrap();
    assert!(!created_schedule.id.to_string().is_empty());
}

#[tokio::test]
#[serial]
async fn test_default_runner_cron_integration() {
    // Get test fixture and initialize it
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());

    // Reset the database to ensure a clean state
    fixture.reset_database().await;
    fixture.initialize().await;

    // Use the same database URL as the fixture
    let database_url = fixture.get_database_url();

    // Create a runner with cron enabled
    let config = DefaultRunnerConfig::builder()
        .enable_cron_scheduling(true)
        .build()
        .unwrap();
    let runner = DefaultRunner::with_config(&database_url, config)
        .await
        .unwrap();

    // Register a cron workflow that won't be due immediately
    runner
        .register_cron_workflow(
            "test-workflow",
            "0 * * * *", // Run at the start of every hour
            "UTC",
        )
        .await
        .expect("Failed to register cron workflow");

    // Let the cron scheduler initialize
    sleep(Duration::from_millis(100)).await;

    // Test the new cron management methods
    let stats = runner
        .get_cron_execution_stats(Utc::now() - chrono::Duration::try_hours(1).unwrap())
        .await
        .unwrap();
    assert_eq!(stats.total_executions, 0); // No executions yet

    // Ensure proper cleanup by explicitly shutting down
    runner.shutdown().await.unwrap();
}

#[tokio::test]
#[serial]
async fn test_cron_scheduler_startup_shutdown() {
    // Get test fixture to determine database URL
    let fixture = get_or_init_fixture().await;
    let fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());
    let database_url = fixture.get_database_url();
    drop(fixture);

    // Create and start a runner with cron enabled
    let config = DefaultRunnerConfig::builder()
        .enable_cron_scheduling(true)
        .build()
        .unwrap();
    let runner = DefaultRunner::with_config(&database_url, config)
        .await
        .unwrap();

    // Let it run briefly (the runner starts background services automatically)
    sleep(Duration::from_millis(100)).await;

    // Shutdown gracefully
    runner.shutdown().await.unwrap();
}

#[tokio::test]
#[serial]
async fn test_cron_missed_executions_catchup_count() {
    // Verify that executions_between correctly calculates missed executions
    // This tests the catchup logic used by the scheduler
    let evaluator = CronEvaluator::new("*/5 * * * *", "UTC").unwrap(); // Every 5 minutes

    let start = chrono::Utc::now() - chrono::Duration::try_hours(1).unwrap();
    let end = chrono::Utc::now();

    let missed = evaluator.executions_between(start, end, 100).unwrap();
    // 1 hour / 5 minutes = 12 executions (approximately, depending on alignment)
    assert!(
        missed.len() >= 11 && missed.len() <= 13,
        "Expected ~12 missed executions in 1 hour, got {}",
        missed.len()
    );
}

#[tokio::test]
#[serial]
async fn test_cron_catchup_respects_max_limit() {
    let evaluator = CronEvaluator::new("* * * * *", "UTC").unwrap(); // Every minute

    let start = chrono::Utc::now() - chrono::Duration::try_hours(24).unwrap();
    let end = chrono::Utc::now();

    // Should cap at 10 even though there are 1440 missed
    let missed = evaluator.executions_between(start, end, 10).unwrap();
    assert_eq!(missed.len(), 10);
}

#[tokio::test]
#[serial]
async fn test_cron_schedule_with_recovery_config() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());
    fixture.reset_database().await;
    fixture.initialize().await;

    let database_url = fixture.get_database_url();

    // Create runner with cron + recovery enabled
    let config = DefaultRunnerConfig::builder()
        .enable_cron_scheduling(true)
        .cron_recovery_interval(Duration::from_secs(60))
        .cron_max_catchup_executions(5)
        .build()
        .unwrap();

    let runner = DefaultRunner::with_config(&database_url, config)
        .await
        .unwrap();

    // Register a cron workflow
    runner
        .register_cron_workflow("recovery-test-wf", "0 * * * *", "UTC")
        .await
        .expect("Failed to register cron workflow");

    sleep(Duration::from_millis(100)).await;

    // Verify schedule was created
    let stats = runner
        .get_cron_execution_stats(chrono::Utc::now() - chrono::Duration::try_hours(1).unwrap())
        .await
        .unwrap();
    assert_eq!(stats.total_executions, 0);

    runner.shutdown().await.unwrap();
}

/// Regression test for CLOACI-T-0572.
///
/// Before the fix, the cron success path created a `schedule_executions` row
/// with `started_at` populated but never called `.complete()`. The recovery
/// service therefore treated every successfully-completed cron firing as
/// "lost" and rescheduled it on every tick, producing ~37x execution
/// amplification.
///
/// This test pins the DAL contract that the cron path now relies on:
///   - A row whose `completed_at` is NULL is returned by `find_lost_executions`.
///   - After `complete()` populates `completed_at`, the same row is excluded.
#[tokio::test]
#[serial]
async fn test_completed_schedule_executions_excluded_from_lost_recovery() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());
    fixture.reset_database().await;
    fixture.initialize().await;
    let dal = fixture.get_dal();

    // Create a cron schedule that the execution row will reference.
    let schedule = dal
        .schedule()
        .create(NewSchedule::cron(
            "cloaci-t-0572-fixture",
            "*/15 * * * *",
            UniversalTimestamp(Utc::now()),
        ))
        .await
        .expect("create schedule");

    // Mimic what the cron success path does: create a started execution row
    // (started_at is set automatically; completed_at is NULL).
    let execution = dal
        .schedule_execution()
        .create(NewScheduleExecution {
            schedule_id: schedule.id,
            workflow_execution_id: None,
            scheduled_time: Some(UniversalTimestamp(Utc::now())),
            claimed_at: Some(UniversalTimestamp(Utc::now())),
            context_hash: None,
        })
        .await
        .expect("create schedule_execution");

    // Use a negative threshold so the cutoff lies in the future and any row
    // whose started_at < cutoff is returned, regardless of how recently it
    // was created. This isolates the test from wall-clock timing.
    let lost_before_complete = dal
        .schedule_execution()
        .find_lost_executions(-1)
        .await
        .expect("find_lost_executions before complete");
    assert!(
        lost_before_complete.iter().any(|e| e.id == execution.id),
        "row with NULL completed_at should be reported as lost (this is the \
         predicate that drove CLOACI-T-0572's infinite recovery loop)"
    );

    // The fix: cron success / failure paths now call .complete() on the audit
    // row. After this call, recovery must not pick the row up again.
    dal.schedule_execution()
        .complete(execution.id, Utc::now())
        .await
        .expect("complete schedule_execution");

    let after = dal
        .schedule_execution()
        .get_by_id(execution.id)
        .await
        .expect("re-read schedule_execution");
    assert!(
        after.completed_at.is_some(),
        "complete() must populate completed_at"
    );

    let lost_after_complete = dal
        .schedule_execution()
        .find_lost_executions(-1)
        .await
        .expect("find_lost_executions after complete");
    assert!(
        lost_after_complete.iter().all(|e| e.id != execution.id),
        "completed row must be excluded from find_lost_executions, otherwise \
         cron_recovery will reschedule it on every tick"
    );
}

/// CLOACI-I-0116: named parameterized instances — full persistence round-trip
/// through the runner API: register (params + name persisted on the schedule
/// row), look up by name, uniqueness enforced, unregister by name. The
/// fire-time delivery of the bound params is covered by the
/// `workflow_instance` merge unit tests + the cron/trigger path wiring.
#[tokio::test]
#[serial]
async fn test_workflow_instance_register_roundtrip() {
    use cloacina::workflow_instance::WorkflowInstance;
    use cloacina_api_types::InputSlot;

    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());
    fixture.reset_database().await;
    fixture.initialize().await;
    let database_url = fixture.get_database_url();
    drop(fixture);

    let config = DefaultRunnerConfig::builder()
        .enable_cron_scheduling(true)
        .build()
        .unwrap();
    let runner = DefaultRunner::with_config(&database_url, config)
        .await
        .unwrap();

    // Build a validated instance: required param supplied, default snapshotted.
    let declared = vec![
        InputSlot::required("source", serde_json::json!({"type": "string"})),
        InputSlot {
            name: "mode".into(),
            schema: serde_json::json!({"type": "string"}),
            required: false,
            default: Some(serde_json::json!("copy")),
        },
    ];
    let instance = WorkflowInstance::builder("sync-file")
        .param("source", "/prod")
        .unwrap()
        .build(&declared)
        .unwrap();

    // Register under a human name.
    runner
        .register_cron_workflow_instance(&instance, "sync_prod", "0 * * * *", "UTC")
        .await
        .expect("instance registration");

    // Round-trip by name: params + instance_name persisted on the row.
    let row = runner
        .get_workflow_instance("sync-file", "sync_prod")
        .await
        .unwrap()
        .expect("instance row exists");
    assert_eq!(row.instance_name.as_deref(), Some("sync_prod"));
    let params: serde_json::Value =
        serde_json::from_str(row.params.as_deref().unwrap()).unwrap();
    assert_eq!(params["source"], serde_json::json!("/prod"));
    assert_eq!(params["mode"], serde_json::json!("copy")); // default snapshotted

    // Uniqueness: same (workflow, instance_name) again must fail.
    let dup = runner
        .register_cron_workflow_instance(&instance, "sync_prod", "0 * * * *", "UTC")
        .await;
    assert!(dup.is_err(), "duplicate instance name must be rejected");

    // A second DIFFERENT name is fine (stamp out many copies).
    runner
        .register_cron_workflow_instance(&instance, "sync_staging", "0 3 * * *", "UTC")
        .await
        .expect("second named instance");

    // Unregister by name; lookup then misses.
    assert!(runner
        .unregister_workflow_instance("sync-file", "sync_prod")
        .await
        .unwrap());
    assert!(runner
        .get_workflow_instance("sync-file", "sync_prod")
        .await
        .unwrap()
        .is_none());

    runner.shutdown().await.unwrap();
}
