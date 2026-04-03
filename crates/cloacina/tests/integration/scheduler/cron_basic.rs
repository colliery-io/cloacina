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
use cloacina::models::schedule::NewSchedule;
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
    assert!(created_schedule.id.to_string().len() > 0);
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
        .build();
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
        .build();
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
        .build();

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
