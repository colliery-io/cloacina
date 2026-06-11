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

// Demo slow-streaming fixture (CLOACI-I-0117 / T-0660): a five-task linear
// chain where every step sleeps for `context["step_seconds"]` (default 4s)
// and then records its completion in the context. Run end-to-end it takes
// ~20s and emits a visible *sequence* of task start/finish events — enough
// for the UI's live execution view to animate, and slow enough that a seed
// run is still "in flight" when an automated UAT snapshots the dashboard.
//
// The pacing is context-driven so the harness can speed it up for CI or
// slow it down for a human demo.

use cloacina_workflow::{task, workflow, Context, TaskError};

cloacina_workflow_plugin::package!();

#[workflow(
    name = "demo_slow_workflow",
    description = "demo slow-streaming workflow — five chained steps that each sleep",
    author = "cloacina-ui-demo"
)]
pub mod demo_slow_workflow {
    use super::*;

    // Per-step pause. Blocking sleep on purpose: the cdylib's statically
    // linked tokio is a different instance than the agent's runtime, so
    // `tokio::time::sleep` would panic ("no reactor running"). Parking the
    // worker thread is runtime-free and lets the agent keep heartbeating on
    // its other workers (same lesson as fleet-slow-rust).
    fn pause(context: &Context<serde_json::Value>) {
        let secs = context
            .get("step_seconds")
            .and_then(|v| v.as_u64())
            .unwrap_or(4);
        std::thread::sleep(std::time::Duration::from_secs(secs));
    }

    #[task(id = "ingest", dependencies = [], retry_attempts = 0)]
    pub async fn ingest(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        pause(context);
        context.insert("ingest_done", serde_json::json!(true))?;
        Ok(())
    }

    #[task(id = "validate", dependencies = ["ingest"], retry_attempts = 0)]
    pub async fn validate(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        pause(context);
        context.insert("validate_done", serde_json::json!(true))?;
        Ok(())
    }

    #[task(id = "transform", dependencies = ["validate"], retry_attempts = 0)]
    pub async fn transform(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        pause(context);
        context.insert("transform_done", serde_json::json!(true))?;
        Ok(())
    }

    #[task(id = "aggregate", dependencies = ["transform"], retry_attempts = 0)]
    pub async fn aggregate(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        pause(context);
        context.insert("aggregate_done", serde_json::json!(true))?;
        Ok(())
    }

    #[task(id = "publish", dependencies = ["aggregate"], retry_attempts = 0)]
    pub async fn publish(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        pause(context);
        context.insert("demo_slow_complete", serde_json::json!(true))?;
        Ok(())
    }
}
