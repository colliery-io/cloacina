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

//! # Event Triggers
//!
//! User-defined event triggers, each written as an async function and
//! decorated with `#[trigger]`. The macro generates the underlying
//! `Trigger` impl + a zero-arg constructor, then submits the constructor
//! to the runtime inventory so `Runtime::new()` auto-registers it.
//!
//! Because the macro builds a unit struct, any state the poll body needs
//! has to live in module-level statics (atomics, `OnceLock<Mutex<…>>`,
//! etc.). That's intentional — triggers are conceptually singletons per
//! process, and the inventory-driven registry expects no constructor
//! args.
//!
//! ## Triggers demonstrated
//!
//! 1. **`file_watcher`** — every fifth poll, simulate a new file landing
//!    on disk and fire `process_file_workflow`.
//! 2. **`queue_monitor`** — fire `process_queue_workflow` whenever the
//!    synthetic queue depth crosses a threshold.
//! 3. **`service_health`** — fire `alert_workflow` after three
//!    consecutive simulated failures.

use cloacina::trigger;
use cloacina_workflow::Context;
use cloacina_workflow::{TriggerError, TriggerResult};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use tracing::{debug, info};

// ----------------------------------------------------------------------
// Module-level state.
//
// Each #[trigger] expands to a unit struct + a zero-arg factory the
// runtime registers via the inventory crate. There's no `self` to hang
// instance state from, so polling state lives here as plain atomics.
// ----------------------------------------------------------------------

/// Pseudo-counter for the file watcher's poll loop. We pretend a new
/// file lands on disk every fifth poll.
static FILE_POLL_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Pseudo-counter for the queue depth simulation.
static QUEUE_TICK: AtomicUsize = AtomicUsize::new(0);

/// Latest queue depth — exposed for log output, not strictly needed.
static QUEUE_DEPTH: AtomicUsize = AtomicUsize::new(0);

/// Synthetic service-health state. Flipped by the health-check trigger
/// based on a tick counter.
static SERVICE_HEALTHY: AtomicBool = AtomicBool::new(true);

/// Consecutive-failure counter for the health check. Reset on first
/// successful poll after a fire and on each successful poll.
static HEALTH_FAILURE_STREAK: AtomicUsize = AtomicUsize::new(0);

// ----------------------------------------------------------------------
// Tuning knobs.
//
// In a real demo these would come from config; hardcoding keeps the
// example focused on the trigger model rather than configuration
// scaffolding.
// ----------------------------------------------------------------------

/// Synthetic queue threshold — fire `process_queue_workflow` whenever the
/// simulated depth crosses this value.
const QUEUE_THRESHOLD: usize = 10;

/// Synthetic health-check threshold — fire `alert_workflow` after this
/// many consecutive simulated failures.
const HEALTH_FAILURE_THRESHOLD: usize = 3;

// ============================================================================
// 1. File-watcher trigger
// ============================================================================
//
// Pretend a new file lands in `/data/inbox` every fifth poll. Real
// production code would `std::fs::read_dir` or call an object-storage
// API; the macro form is identical either way.

#[trigger(
    on = "file_processing_workflow",
    poll_interval = "2s",
    allow_concurrent = false,
    name = "file_watcher"
)]
async fn file_watcher() -> Result<TriggerResult, TriggerError> {
    let count = FILE_POLL_COUNTER.fetch_add(1, Ordering::SeqCst);
    if count % 5 != 4 {
        debug!("file_watcher: no new files (poll #{})", count);
        return Ok(TriggerResult::Skip);
    }

    let filename = format!("data_file_{}.csv", chrono::Utc::now().timestamp());
    info!(
        "file_watcher: found new file '{}' in /data/inbox",
        filename
    );

    let mut ctx = Context::new();
    ctx.insert("filename", serde_json::json!(filename))
        .map_err(|e| TriggerError::PollError {
            message: format!("insert filename: {}", e),
        })?;
    ctx.insert("watch_path", serde_json::json!("/data/inbox"))
        .map_err(|e| TriggerError::PollError {
            message: format!("insert watch_path: {}", e),
        })?;
    ctx.insert(
        "discovered_at",
        serde_json::json!(chrono::Utc::now().to_rfc3339()),
    )
    .map_err(|e| TriggerError::PollError {
        message: format!("insert discovered_at: {}", e),
    })?;

    Ok(TriggerResult::Fire(Some(ctx)))
}

// ============================================================================
// 2. Queue-depth trigger
// ============================================================================
//
// Synthesizes a depth that oscillates between 0 and 20. Whenever the
// depth meets or exceeds the threshold we fire. `allow_concurrent = true`
// because draining a queue is generally safe in parallel.

#[trigger(
    on = "queue_processing_workflow",
    poll_interval = "3s",
    allow_concurrent = true,
    name = "queue_monitor"
)]
async fn queue_monitor() -> Result<TriggerResult, TriggerError> {
    // Generate a depth that bounces between 0 and 20 over time.
    let tick = QUEUE_TICK.fetch_add(3, Ordering::SeqCst);
    let raw = tick % 21;
    let depth = raw.min(20usize.saturating_sub(raw));
    QUEUE_DEPTH.store(depth, Ordering::SeqCst);

    debug!("queue_monitor: order_queue depth = {}", depth);

    if depth < QUEUE_THRESHOLD {
        return Ok(TriggerResult::Skip);
    }

    info!(
        "queue_monitor: order_queue depth ({}) exceeds threshold ({})",
        depth, QUEUE_THRESHOLD
    );

    let mut ctx = Context::new();
    ctx.insert("queue_name", serde_json::json!("order_queue"))
        .map_err(|e| TriggerError::PollError {
            message: format!("insert queue_name: {}", e),
        })?;
    ctx.insert("queue_depth", serde_json::json!(depth))
        .map_err(|e| TriggerError::PollError {
            message: format!("insert queue_depth: {}", e),
        })?;
    ctx.insert("threshold", serde_json::json!(QUEUE_THRESHOLD))
        .map_err(|e| TriggerError::PollError {
            message: format!("insert threshold: {}", e),
        })?;

    Ok(TriggerResult::Fire(Some(ctx)))
}

// ============================================================================
// 3. Health-check trigger
// ============================================================================
//
// Polls a synthetic service. Service goes unhealthy for 5 ticks every 15.
// Fires `alert_workflow` after `HEALTH_FAILURE_THRESHOLD` consecutive
// failures, then resets the streak.

#[trigger(
    on = "service_recovery_workflow",
    poll_interval = "2s",
    allow_concurrent = false,
    name = "service_health"
)]
async fn service_health() -> Result<TriggerResult, TriggerError> {
    // Reuse file_watcher's tick so the health pattern lines up with the
    // demo's overall timeline; in production each trigger would have its
    // own clock or call out to a real probe.
    let tick = FILE_POLL_COUNTER.load(Ordering::SeqCst);
    let healthy = tick % 15 < 10;
    SERVICE_HEALTHY.store(healthy, Ordering::SeqCst);

    if healthy {
        HEALTH_FAILURE_STREAK.store(0, Ordering::SeqCst);
        debug!("service_health: payment_service healthy");
        return Ok(TriggerResult::Skip);
    }

    let failures = HEALTH_FAILURE_STREAK.fetch_add(1, Ordering::SeqCst) + 1;
    debug!(
        "service_health: payment_service unhealthy (streak={})",
        failures
    );

    if failures < HEALTH_FAILURE_THRESHOLD {
        return Ok(TriggerResult::Skip);
    }

    info!(
        "service_health: payment_service has {} consecutive failures (threshold {})",
        failures, HEALTH_FAILURE_THRESHOLD
    );
    HEALTH_FAILURE_STREAK.store(0, Ordering::SeqCst);

    let mut ctx = Context::new();
    ctx.insert("service_name", serde_json::json!("payment_service"))
        .map_err(|e| TriggerError::PollError {
            message: format!("insert service_name: {}", e),
        })?;
    ctx.insert("consecutive_failures", serde_json::json!(failures))
        .map_err(|e| TriggerError::PollError {
            message: format!("insert consecutive_failures: {}", e),
        })?;
    ctx.insert(
        "detected_at",
        serde_json::json!(chrono::Utc::now().to_rfc3339()),
    )
    .map_err(|e| TriggerError::PollError {
        message: format!("insert detected_at: {}", e),
    })?;

    Ok(TriggerResult::Fire(Some(ctx)))
}
