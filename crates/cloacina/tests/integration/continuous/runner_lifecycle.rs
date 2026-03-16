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

//! Integration tests for continuous scheduling wired into DefaultRunner.

use crate::fixtures;
use cloacina::runner::{DefaultRunner, DefaultRunnerConfig};
use serial_test::serial;
use std::time::Duration;

/// Test: continuous scheduling enabled with empty graph starts and stops cleanly.
///
/// The continuous scheduler with an empty graph polls an empty ledger harmlessly.
/// This verifies the full lifecycle: config → start_background_services →
/// start_continuous_scheduler → shutdown.
#[tokio::test]
#[serial]
async fn test_continuous_scheduler_empty_graph_lifecycle() {
    let fixture = fixtures::get_or_init_fixture().await;
    let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());
    fixture.reset_database().await;
    fixture.initialize().await;

    let database_url = fixture.get_database_url();
    let schema = fixture.get_schema();

    let config = DefaultRunnerConfig::builder()
        .enable_continuous_scheduling(true)
        .build();

    let runner = DefaultRunner::builder()
        .database_url(&database_url)
        .schema(&schema)
        .with_config(config)
        .build()
        .await
        .expect("runner should start with continuous scheduling enabled");

    // Let the continuous scheduler poll at least once
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Clean shutdown
    runner
        .shutdown()
        .await
        .expect("shutdown should complete cleanly");
}

/// Test: continuous scheduling disabled (default) starts and stops without
/// spawning a continuous scheduler.
#[tokio::test]
#[serial]
async fn test_continuous_scheduler_disabled_by_default() {
    let fixture = fixtures::get_or_init_fixture().await;
    let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());
    fixture.reset_database().await;
    fixture.initialize().await;

    let database_url = fixture.get_database_url();
    let schema = fixture.get_schema();

    // Default config — continuous scheduling is off
    let runner = DefaultRunner::builder()
        .database_url(&database_url)
        .schema(&schema)
        .build()
        .await
        .expect("runner should start with default config");

    // Shutdown should be instant since no continuous scheduler was spawned
    runner
        .shutdown()
        .await
        .expect("shutdown should complete cleanly");
}
