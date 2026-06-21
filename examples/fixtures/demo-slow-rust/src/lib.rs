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

// Demo slow-streaming fixture (CLOACI-I-0117 / T-0660). A *branching* slow chain
// where each step sleeps and records its completion in the context. It emits a
// visible sequence of task start/finish events — enough for the UI's live
// execution view to animate, and slow enough that a seed run is still "in
// flight" when an automated UAT snapshots the dashboard.
//
//   ingest ──┬─▶ validate ─▶ transform ─┐
//            │                          ├─▶ aggregate ─▶ publish
//            └─▶ enrich (SKIPPED) ──────┘
//
// `ingest` always sets `do_enrich = false`, so `enrich`'s trigger rule (wants
// == true) never fires → the scheduler marks it Skipped (deps satisfied, rule
// fails). `aggregate` fans in on `transform` + `enrich`; a Skipped dependency
// counts as resolved, so it still runs. Net per run: a branch, a dashed skipped
// node, and a fan-in — the execution DAG is non-linear on every run (T-0719).
//
// Each task has its own *characteristic* duration with per-run jitter so the
// UI's runtime / Gantt / distribution views have a realistic spread:
// `transform` is the deliberate hot-spot, the rest are lighter. Pacing is
// context-overridable — `context["step_seconds"]` pins every step to a fixed
// duration (CI determinism / harness speed control), bypassing the jitter.

use cloacina_workflow::{task, workflow, Context, TaskError};

cloacina_workflow_plugin::package!();

#[workflow(
    name = "demo_slow_workflow",
    description = "demo slow-streaming workflow — a branching chain with a skipped path and varied, jittered durations",
    author = "cloacina-ui-demo",
    // CLOACI-T-0768: declared injectors — typed execute-time params. Surfaced as
    // JSON-Schema InputSlots in the Inputs card + the Run-workflow form.
    params(
        source_id: String,
        batch_size: u32 = 100,
        dry_run: bool = false,
    )
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
                                      // Gate the enrich branch off → it skips every run (deps met, rule fails).
        context.insert("do_enrich", serde_json::json!(false))?;
        context.insert("ingest_done", serde_json::json!(true))?;
        Ok(())
    }

    #[task(dependencies = ["ingest"], retry_attempts = 0)]
    pub async fn validate(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        pause(context, 4_000, 2_000); // ~2–6s
        context.insert("validate_done", serde_json::json!(true))?;
        Ok(())
    }

    // Branch sibling of `validate`, gated off so it lands in the Skipped state —
    // the dashed node on the DAG. Kept so the graph has the branch + fan-in.
    #[task(
        dependencies = ["ingest"],
        retry_attempts = 0,
        trigger_rules = context_value("do_enrich", equals, true)
    )]
    pub async fn enrich(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        pause(context, 2_000, 1_000);
        context.insert("enrich_done", serde_json::json!(true))?;
        Ok(())
    }

    #[task(dependencies = ["validate"], retry_attempts = 0)]
    pub async fn transform(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        pause(context, 12_000, 5_000); // ~7–17s — the deliberate hot-spot
        context.insert("transform_done", serde_json::json!(true))?;
        Ok(())
    }

    // Fan-in: both the transform path and the (skipped) enrich path converge
    // here. A Skipped dependency counts as resolved, so aggregate still runs.
    #[task(dependencies = ["transform", "enrich"], retry_attempts = 0)]
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
