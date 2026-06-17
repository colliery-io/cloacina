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
// chain where each step sleeps and then records its completion in the
// context. It emits a visible *sequence* of task start/finish events — enough
// for the UI's live execution view to animate, and slow enough that a seed
// run is still "in flight" when an automated UAT snapshots the dashboard.
//
// Each task has its own *characteristic* duration with per-run jitter, rather
// than a single flat sleep, so the UI's runtime / Gantt / distribution views
// have a realistic spread to show: `transform` is the deliberate hot-spot,
// the rest are lighter. The pacing is still context-overridable —
// `context["step_seconds"]` pins every step to a fixed duration (CI
// determinism / harness speed control), bypassing the jitter.

use cloacina_workflow::{task, workflow, Context, TaskError};

cloacina_workflow_plugin::package!();

#[workflow(
    name = "demo_slow_workflow",
    description = "demo slow-streaming workflow — five chained steps with varied, jittered durations",
    author = "cloacina-ui-demo"
)]
pub mod demo_slow_workflow {
    use super::*;

    // Sleep `base_ms` ± `jitter_ms`, seeded from the wall clock so each task
    // invocation differs — this is what gives the distribution views real
    // spread. xorshift64 over the clock nanos, no `rand` dependency.
    //
    // Blocking sleep on purpose: the cdylib's statically linked tokio is a
    // different instance than the agent's runtime, so `tokio::time::sleep`
    // would panic ("no reactor running"). Parking the worker thread is
    // runtime-free and lets the agent keep heartbeating on its other workers
    // (same lesson as fleet-slow-rust).
    fn pause(context: &Context<serde_json::Value>, base_ms: u64, jitter_ms: u64) {
        // Context override pins every step to a flat duration (CI / harness).
        if let Some(secs) = context.get("step_seconds").and_then(|v| v.as_u64()) {
            std::thread::sleep(std::time::Duration::from_secs(secs));
            return;
        }
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(1)
            | 1;
        let mut x = seed;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        let span = jitter_ms.saturating_mul(2).saturating_add(1);
        let delta = x % span; // [0, 2*jitter]
        let ms = base_ms.saturating_sub(jitter_ms).saturating_add(delta);
        std::thread::sleep(std::time::Duration::from_millis(ms));
    }

    #[task(retry_attempts = 0)]
    pub async fn ingest(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        pause(context, 3_000, 1_500); // ~1.5–4.5s
        context.insert("ingest_done", serde_json::json!(true))?;
        Ok(())
    }

    #[task(dependencies = ["ingest"], retry_attempts = 0)]
    pub async fn validate(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        pause(context, 4_000, 2_000); // ~2–6s
        context.insert("validate_done", serde_json::json!(true))?;
        Ok(())
    }

    #[task(dependencies = ["validate"], retry_attempts = 0)]
    pub async fn transform(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        pause(context, 12_000, 5_000); // ~7–17s — the deliberate hot-spot
        context.insert("transform_done", serde_json::json!(true))?;
        Ok(())
    }

    #[task(dependencies = ["transform"], retry_attempts = 0)]
    pub async fn aggregate(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        pause(context, 5_000, 2_500); // ~2.5–7.5s
        context.insert("aggregate_done", serde_json::json!(true))?;
        Ok(())
    }

    #[task(dependencies = ["aggregate"], retry_attempts = 0)]
    pub async fn publish(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        pause(context, 3_000, 1_500); // ~1.5–4.5s
        context.insert("demo_slow_complete", serde_json::json!(true))?;
        Ok(())
    }
}
