/*
 *  Copyright 2025 Colliery Software
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
//! This module defines custom triggers that poll for conditions and fire
//! workflows when those conditions are met.
//!
//! ## Triggers Demonstrated
//!
//! 1. **FileWatcherTrigger** - Polls for new files in a directory
//! 2. **QueueDepthTrigger** - Fires when a queue exceeds a threshold
//! 3. **HealthCheckTrigger** - Fires when a service becomes unhealthy

use async_trait::async_trait;
use cloacina::trigger::{Trigger, TriggerError, TriggerResult};
use cloacina::Context;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info};

/// Counter for simulating file arrivals
static FILE_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Counter for simulating queue depth
static QUEUE_DEPTH: AtomicUsize = AtomicUsize::new(0);

/// Flag for simulating service health
static SERVICE_HEALTHY: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);

// ============================================================================
// File Watcher Trigger
// ============================================================================

/// A trigger that polls for new files in a simulated directory.
///
/// In a real application, this would check a filesystem directory or
/// cloud storage bucket for new files.
#[derive(Debug, Clone)]
pub struct FileWatcherTrigger {
    name: String,
    poll_interval: Duration,
    watch_path: String,
}

impl FileWatcherTrigger {
    /// Creates a new file watcher trigger.
    pub fn new(name: &str, watch_path: &str, poll_interval: Duration) -> Self {
        Self {
            name: name.to_string(),
            poll_interval,
            watch_path: watch_path.to_string(),
        }
    }

    /// Simulates checking for new files.
    /// In production, this would use std::fs or an async filesystem library.
    async fn check_for_new_files(&self) -> Option<String> {
        // Simulate file arrival every few polls
        let count = FILE_COUNTER.fetch_add(1, Ordering::SeqCst);
        if count % 5 == 4 {
            // Every 5th poll, "find" a new file
            let filename = format!("data_file_{}.csv", chrono::Utc::now().timestamp());
            info!(
                "FileWatcherTrigger: Found new file '{}' in '{}'",
                filename, self.watch_path
            );
            Some(filename)
        } else {
            debug!(
                "FileWatcherTrigger: No new files in '{}' (poll #{})",
                self.watch_path, count
            );
            None
        }
    }
}

#[async_trait]
impl Trigger for FileWatcherTrigger {
    fn name(&self) -> &str {
        &self.name
    }

    fn poll_interval(&self) -> Duration {
        self.poll_interval
    }

    fn allow_concurrent(&self) -> bool {
        // Don't allow concurrent processing of the same file
        false
    }

    async fn poll(&self) -> Result<TriggerResult, TriggerError> {
        if let Some(filename) = self.check_for_new_files().await {
            // Create context with file information
            let mut ctx = Context::new();
            ctx.insert("filename", serde_json::json!(filename))
                .map_err(|e| TriggerError::PollError {
                    message: format!("Failed to insert filename: {}", e),
                })?;
            ctx.insert("watch_path", serde_json::json!(self.watch_path.clone()))
                .map_err(|e| TriggerError::PollError {
                    message: format!("Failed to insert watch_path: {}", e),
                })?;
            ctx.insert(
                "discovered_at",
                serde_json::json!(chrono::Utc::now().to_rfc3339()),
            )
            .map_err(|e| TriggerError::PollError {
                message: format!("Failed to insert discovered_at: {}", e),
            })?;

            Ok(TriggerResult::Fire(Some(ctx)))
        } else {
            Ok(TriggerResult::Skip)
        }
    }
}

// ============================================================================
// Queue Depth Trigger
// ============================================================================

/// A trigger that fires when a queue exceeds a depth threshold.
///
/// This demonstrates monitoring patterns where workflows are triggered
/// based on system metrics rather than time.
#[derive(Debug, Clone)]
pub struct QueueDepthTrigger {
    name: String,
    poll_interval: Duration,
    queue_name: String,
    threshold: usize,
}

impl QueueDepthTrigger {
    /// Creates a new queue depth trigger.
    pub fn new(name: &str, queue_name: &str, threshold: usize, poll_interval: Duration) -> Self {
        Self {
            name: name.to_string(),
            poll_interval,
            queue_name: queue_name.to_string(),
            threshold,
        }
    }

    /// Simulates checking queue depth.
    /// In production, this would query a message queue like RabbitMQ, SQS, etc.
    async fn get_queue_depth(&self) -> usize {
        // Simulate varying queue depth
        let base = QUEUE_DEPTH.fetch_add(3, Ordering::SeqCst);
        // Oscillate between 0 and 20
        let depth = (base % 21).min(20 - (base % 21).min(20));
        debug!(
            "QueueDepthTrigger: Queue '{}' depth = {}",
            self.queue_name, depth
        );
        depth
    }
}

#[async_trait]
impl Trigger for QueueDepthTrigger {
    fn name(&self) -> &str {
        &self.name
    }

    fn poll_interval(&self) -> Duration {
        self.poll_interval
    }

    fn allow_concurrent(&self) -> bool {
        // Allow concurrent executions for queue processing
        true
    }

    async fn poll(&self) -> Result<TriggerResult, TriggerError> {
        let depth = self.get_queue_depth().await;

        if depth >= self.threshold {
            info!(
                "QueueDepthTrigger: Queue '{}' depth ({}) exceeds threshold ({})",
                self.queue_name, depth, self.threshold
            );

            let mut ctx = Context::new();
            ctx.insert("queue_name", serde_json::json!(self.queue_name.clone()))
                .map_err(|e| TriggerError::PollError {
                    message: format!("Failed to insert queue_name: {}", e),
                })?;
            ctx.insert("queue_depth", serde_json::json!(depth))
                .map_err(|e| TriggerError::PollError {
                    message: format!("Failed to insert queue_depth: {}", e),
                })?;
            ctx.insert("threshold", serde_json::json!(self.threshold))
                .map_err(|e| TriggerError::PollError {
                    message: format!("Failed to insert threshold: {}", e),
                })?;

            Ok(TriggerResult::Fire(Some(ctx)))
        } else {
            Ok(TriggerResult::Skip)
        }
    }
}

// ============================================================================
// Health Check Trigger
// ============================================================================

/// A trigger that fires when a service becomes unhealthy.
///
/// This demonstrates reactive patterns where workflows are triggered
/// in response to failures or anomalies.
#[derive(Debug, Clone)]
pub struct HealthCheckTrigger {
    name: String,
    poll_interval: Duration,
    service_name: String,
    consecutive_failures: Arc<AtomicUsize>,
    failure_threshold: usize,
}

impl HealthCheckTrigger {
    /// Creates a new health check trigger.
    pub fn new(
        name: &str,
        service_name: &str,
        failure_threshold: usize,
        poll_interval: Duration,
    ) -> Self {
        Self {
            name: name.to_string(),
            poll_interval,
            service_name: service_name.to_string(),
            consecutive_failures: Arc::new(AtomicUsize::new(0)),
            failure_threshold,
        }
    }

    /// Simulates checking service health.
    /// In production, this would make HTTP health check requests.
    async fn check_service_health(&self) -> bool {
        // Toggle health every 10 polls for demonstration
        let counter = FILE_COUNTER.load(Ordering::SeqCst);
        let healthy = counter % 15 < 10;
        SERVICE_HEALTHY.store(healthy, Ordering::SeqCst);
        healthy
    }
}

#[async_trait]
impl Trigger for HealthCheckTrigger {
    fn name(&self) -> &str {
        &self.name
    }

    fn poll_interval(&self) -> Duration {
        self.poll_interval
    }

    fn allow_concurrent(&self) -> bool {
        // Don't allow concurrent recovery workflows
        false
    }

    async fn poll(&self) -> Result<TriggerResult, TriggerError> {
        let healthy = self.check_service_health().await;

        if healthy {
            // Reset failure counter on success
            self.consecutive_failures.store(0, Ordering::SeqCst);
            debug!(
                "HealthCheckTrigger: Service '{}' is healthy",
                self.service_name
            );
            return Ok(TriggerResult::Skip);
        }

        // Increment failure counter
        let failures = self.consecutive_failures.fetch_add(1, Ordering::SeqCst) + 1;
        debug!(
            "HealthCheckTrigger: Service '{}' unhealthy ({} consecutive failures)",
            self.service_name, failures
        );

        if failures >= self.failure_threshold {
            info!(
                "HealthCheckTrigger: Service '{}' has {} consecutive failures (threshold: {})",
                self.service_name, failures, self.failure_threshold
            );

            // Reset counter after triggering
            self.consecutive_failures.store(0, Ordering::SeqCst);

            let mut ctx = Context::new();
            ctx.insert("service_name", serde_json::json!(self.service_name.clone()))
                .map_err(|e| TriggerError::PollError {
                    message: format!("Failed to insert service_name: {}", e),
                })?;
            ctx.insert("consecutive_failures", serde_json::json!(failures))
                .map_err(|e| TriggerError::PollError {
                    message: format!("Failed to insert consecutive_failures: {}", e),
                })?;
            ctx.insert(
                "detected_at",
                serde_json::json!(chrono::Utc::now().to_rfc3339()),
            )
            .map_err(|e| TriggerError::PollError {
                message: format!("Failed to insert detected_at: {}", e),
            })?;

            Ok(TriggerResult::Fire(Some(ctx)))
        } else {
            Ok(TriggerResult::Skip)
        }
    }
}

// ============================================================================
// Constructor Functions
// ============================================================================

/// Creates the file watcher trigger for the file processing workflow.
pub fn create_file_watcher_trigger() -> FileWatcherTrigger {
    FileWatcherTrigger::new(
        "file_watcher",
        "/data/inbox",
        Duration::from_secs(2), // Poll every 2 seconds for demo
    )
}

/// Creates the queue depth trigger for the queue processing workflow.
pub fn create_queue_depth_trigger() -> QueueDepthTrigger {
    QueueDepthTrigger::new(
        "queue_monitor",
        "order_queue",
        10, // Fire when queue depth exceeds 10
        Duration::from_secs(3),
    )
}

/// Creates the health check trigger for the recovery workflow.
pub fn create_health_check_trigger() -> HealthCheckTrigger {
    HealthCheckTrigger::new(
        "service_health",
        "payment_service",
        3, // Fire after 3 consecutive failures
        Duration::from_secs(2),
    )
}
