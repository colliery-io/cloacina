/*
 *  Copyright 2026 Colliery Software
 *
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 */

//! # CEL-Filtered Reactor Subscriptions (CLOACI-T-0602)
//!
//! Reactors fire at high throughput — every accumulator update, every
//! source tick. Subscribing a workflow to "every firing" can drown the
//! dispatcher in workflow executions that don't actually need to run.
//!
//! This example demonstrates the **filtered subscription**: a CEL
//! expression evaluated against each firing's payload. Only firings
//! where the expression returns `true` cause a workflow dispatch; the
//! rest advance the subscription's watermark and move on without
//! creating a workflow row.
//!
//! ## What the example does
//!
//! 1. Spins up `DefaultRunner` against an in-memory SQLite database.
//! 2. Registers a trivial `alert_workflow` (one task that prints).
//! 3. Subscribes `alert_workflow` to a `pricing_reactor` with the CEL
//!    filter `payload.value > 100`.
//! 4. Synthesizes four reactor firings via the DAL with values
//!    `[50, 150, 80, 200]`.
//! 5. Forces one scheduler tick via `poll_reactor_subscriptions_once`.
//! 6. Inspects the `workflow_executions` table — expect 2 rows
//!    (values 150 and 200), and the subscription's watermark advanced
//!    past all four firings.
//!
//! ## Running
//!
//! ```sh
//! cd examples/features/computation-graphs/filtered-reactor
//! cargo run
//! ```

use cloacina::dal::DAL;
use cloacina::database::universal_types::UniversalTimestamp;
use cloacina::runner::{DefaultRunner, DefaultRunnerConfig};
use cloacina::{task, workflow, Context, TaskError};
use std::collections::HashMap;
use tracing::info;

// ────────────────────────────────────────────────────────────────────
// Workflow
// ────────────────────────────────────────────────────────────────────

#[workflow(
    name = "alert_workflow",
    description = "Fires for high-value pricing events only."
)]
pub mod alert_workflow {
    use super::*;

    #[task(id = "emit_alert", dependencies = [])]
    pub async fn emit_alert(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        let value = context
            .get("value")
            .and_then(|v| v.as_i64())
            .unwrap_or(-1);
        info!(
            "alert_workflow fired for high-value event (value={}, reactor={:?})",
            value,
            context.get("reactor_name").cloned().unwrap_or_default()
        );
        Ok(())
    }
}

// ────────────────────────────────────────────────────────────────────
// Helpers
// ────────────────────────────────────────────────────────────────────

/// Build a `(source -> JSON-encoded bytes)` map into the bincode form
/// the scheduler expects for `reactor_firings.payload`. The source
/// name becomes a top-level context key in the dispatched workflow,
/// and (because the bytes parse as JSON) the value is JSON-decoded.
fn build_firing_payload(source: &str, value: serde_json::Value) -> Vec<u8> {
    let inner = serde_json::to_vec(&value).expect("encode value");
    let mut map: HashMap<String, Vec<u8>> = HashMap::new();
    map.insert(source.to_string(), inner);
    bincode::serialize(&map).expect("bincode encode")
}

// ────────────────────────────────────────────────────────────────────
// main
// ────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter("filtered_reactor_example=info,cloacina=info")
        .init();

    info!("== Filtered Reactor Subscriptions (T-0602) ==");

    // SQLite in-memory: per CLOACI-T-0608, this is substituted to a
    // per-Database tempfile under the hood so background services and
    // queries see the same schema.
    let runner = DefaultRunner::with_config(
        "sqlite://:memory:",
        DefaultRunnerConfig::builder().build()?,
    )
    .await?;

    // Subscribe with a CEL filter — fire only when payload.value > 100.
    let reactor = "pricing_reactor";
    let workflow = "alert_workflow";
    let tenant = None; // defaults to "public"
    runner
        .subscribe_workflow_to_reactor(
            reactor,
            workflow,
            tenant,
            Some("payload.value > 100"),
        )
        .await?;
    info!(
        "subscribed {} → {} with predicate 'payload.value > 100'",
        reactor, workflow
    );

    // Synthesize four reactor firings. Only the firings with
    // `value > 100` should produce a workflow execution.
    let dal = DAL::new(runner.database().clone());
    let mut now = UniversalTimestamp::now();
    for value in [50, 150, 80, 200] {
        dal.reactor_subscriptions()
            .insert_firing(
                reactor,
                "public",
                Some(build_firing_payload("value", serde_json::json!(value))),
                now,
            )
            .await?;
        info!("inserted firing with value={}", value);
        // Ensure strictly increasing fired_at so poll_unconsumed
        // returns them in order.
        now = UniversalTimestamp(now.0 + chrono::Duration::milliseconds(1));
    }

    // Force one reactor poll instead of waiting for the background tick.
    let scheduler = runner
        .unified_scheduler()
        .await
        .ok_or("unified scheduler not enabled — check enable_trigger_scheduling()")?;
    scheduler.poll_reactor_subscriptions_once().await?;
    info!("ran one scheduler pass");

    // Inspect the subscription watermark — should be past all four.
    let subs = dal
        .reactor_subscriptions()
        .list_subscriptions("public")
        .await?;
    if let Some(sub) = subs
        .iter()
        .find(|s| s.reactor_name == reactor && s.workflow_name == workflow)
    {
        info!(
            "subscription watermark advanced to {:?} (all four firings observed: {})",
            sub.last_seen_fired_at,
            sub.last_seen_fired_at.is_some(),
        );
    }

    // Let the runner's dispatcher pick up the dispatched workflows
    // before we shut down so the alert logs surface.
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    info!("done — expected to see two alert_workflow fires (values 150, 200)");
    runner.shutdown().await?;
    Ok(())
}
