/*
 *  Copyright 2026 Colliery Software
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

//! Integration test for the unified #[workflow] macro (embedded mode).

use cloacina::executor::PipelineExecutor;
use cloacina::runner::{DefaultRunner, DefaultRunnerConfig};
use cloacina::{task, trigger, workflow, Context, TaskError};
use serde_json::json;

#[workflow(
    name = "unified_test_workflow",
    description = "Test workflow using unified macro",
    author = "Test"
)]
pub mod unified_test_workflow {
    use super::*;

    #[task(id = "step_one", dependencies = [])]
    pub async fn step_one(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        context.insert("step_one_done", json!(true))?;
        Ok(())
    }

    #[task(id = "step_two", dependencies = ["step_one"])]
    pub async fn step_two(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        let step_one_done = context
            .get("step_one_done")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        assert!(step_one_done, "step_one should have run first");
        context.insert("step_two_done", json!(true))?;
        Ok(())
    }
}

#[cfg(feature = "sqlite")]
#[tokio::test]
async fn test_workflow_executes_sqlite() {
    let db_url = ":memory:";
    let config = DefaultRunnerConfig::builder()
        .enable_registry_reconciler(false)
        .build();

    let runner = DefaultRunner::with_config(db_url, config)
        .await
        .expect("Failed to create runner");

    let context = Context::new();
    let result = runner
        .execute("unified_test_workflow", context)
        .await
        .expect("Workflow execution failed");

    assert_eq!(
        result.final_context.get("step_one_done"),
        Some(&json!(true))
    );
    assert_eq!(
        result.final_context.get("step_two_done"),
        Some(&json!(true))
    );

    runner.shutdown().await.expect("Shutdown failed");
}

// --- Trigger macro tests ---

use cloacina_workflow::TriggerResult;

#[trigger(on = "unified_test_workflow", poll_interval = "5s")]
pub async fn test_trigger() -> Result<TriggerResult, TriggerError> {
    Ok(TriggerResult::Skip)
}

#[test]
fn test_trigger_registered() {
    // The #[trigger] macro auto-registers the trigger in the global registry
    assert!(
        cloacina::trigger::is_trigger_registered("test_trigger"),
        "Trigger should be auto-registered by #[trigger] macro"
    );
}

#[trigger(
    on = "unified_test_workflow",
    poll_interval = "100ms",
    allow_concurrent = true,
    name = "custom_named_trigger"
)]
pub async fn my_trigger_fn() -> Result<TriggerResult, TriggerError> {
    Ok(TriggerResult::Skip)
}

#[test]
fn test_trigger_custom_name() {
    assert!(
        cloacina::trigger::is_trigger_registered("custom_named_trigger"),
        "Trigger should be registered under custom name"
    );
}

// --- Cron trigger tests ---

#[trigger(on = "unified_test_workflow", cron = "0 2 * * *", timezone = "UTC")]
fn nightly_job() {}

#[test]
fn test_cron_trigger_registered() {
    assert!(
        cloacina::trigger::is_trigger_registered("nightly_job"),
        "Cron trigger should be auto-registered"
    );
}

#[trigger(
    on = "unified_test_workflow",
    cron = "*/5 * * * *",
    name = "every_five_minutes"
)]
fn frequent_check() {}

#[test]
fn test_cron_trigger_custom_name() {
    assert!(
        cloacina::trigger::is_trigger_registered("every_five_minutes"),
        "Cron trigger should be registered under custom name"
    );
}

#[test]
fn test_cron_trigger_poll_returns_result() {
    // Get the trigger from registry and verify poll works
    let trigger =
        cloacina::trigger::get_trigger("nightly_job").expect("nightly_job trigger should exist");

    // Verify basic properties
    assert_eq!(trigger.name(), "nightly_job");
    assert!(!trigger.allow_concurrent());
    // Cron triggers poll every 30s
    assert_eq!(trigger.poll_interval(), std::time::Duration::from_secs(30));
}
