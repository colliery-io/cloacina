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

//! Recovery Sweeper — periodic orphan detection and task re-queuing.
//!
//! Scans for tasks stuck in "Running" with stale heartbeats and resets
//! them to "Ready" for re-execution. Works for all execution modes
//! (server, daemon, continuous).
//!
//! Two modes:
//! - **Startup mode** (first `startup_grace` seconds): Only recovers tasks
//!   stale before this instance started — avoids false positives during restart.
//! - **Normal mode**: Real-time orphan detection for tasks that go stale.

use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::watch;
use tracing::{debug, error, info, warn};

use crate::dal::DAL;
use crate::database::universal_types::UniversalTimestamp;
use crate::executor::PipelineError;

/// Configuration for the recovery sweeper.
#[derive(Debug, Clone)]
pub struct RecoverySweepConfig {
    /// How often to scan for orphans.
    pub sweep_interval: Duration,
    /// How old a heartbeat must be to consider a task orphaned.
    pub orphan_threshold: Duration,
    /// Grace period after startup before switching to live detection.
    pub startup_grace: Duration,
    /// Maximum recovery attempts before abandoning a task.
    pub max_recovery_attempts: usize,
}

impl Default for RecoverySweepConfig {
    fn default() -> Self {
        Self {
            sweep_interval: Duration::from_secs(30),
            orphan_threshold: Duration::from_secs(60),
            startup_grace: Duration::from_secs(120),
            max_recovery_attempts: 3,
        }
    }
}

/// Background service that detects orphaned tasks and resets them for re-execution.
pub struct RecoverySweepService {
    dal: Arc<DAL>,
    config: RecoverySweepConfig,
    shutdown: watch::Receiver<bool>,
    started_at: Instant,
    started_at_utc: chrono::DateTime<chrono::Utc>,
}

impl RecoverySweepService {
    pub fn new(
        dal: Arc<DAL>,
        config: RecoverySweepConfig,
        shutdown: watch::Receiver<bool>,
    ) -> Self {
        Self {
            dal,
            config,
            shutdown,
            started_at: Instant::now(),
            started_at_utc: chrono::Utc::now(),
        }
    }

    /// Main loop — runs until shutdown signal.
    pub async fn run_sweep_loop(&mut self) -> Result<(), PipelineError> {
        info!(
            "Starting recovery sweep service (interval: {:?}, orphan_threshold: {:?}, startup_grace: {:?})",
            self.config.sweep_interval, self.config.orphan_threshold, self.config.startup_grace
        );

        let mut interval = tokio::time::interval(self.config.sweep_interval);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if let Err(e) = self.perform_sweep().await {
                        error!("Error in recovery sweep: {}", e);
                    }
                }
                _ = self.shutdown.changed() => {
                    if *self.shutdown.borrow() {
                        info!("Recovery sweep service received shutdown signal");
                        break;
                    }
                }
            }
        }

        info!("Recovery sweep service stopped");
        Ok(())
    }

    /// Single sweep iteration.
    async fn perform_sweep(&self) -> Result<(), PipelineError> {
        // Determine cutoff based on startup vs normal mode
        let in_startup_mode = self.started_at.elapsed() < self.config.startup_grace;
        let cutoff = if in_startup_mode {
            // Only recover tasks stale BEFORE this instance started
            let threshold = chrono::Duration::from_std(self.config.orphan_threshold)
                .unwrap_or(chrono::Duration::seconds(60));
            UniversalTimestamp(self.started_at_utc - threshold)
        } else {
            // Real-time: tasks stale relative to NOW
            let threshold = chrono::Duration::from_std(self.config.orphan_threshold)
                .unwrap_or(chrono::Duration::seconds(60));
            UniversalTimestamp(chrono::Utc::now() - threshold)
        };

        let mode = if in_startup_mode { "startup" } else { "normal" };
        debug!("Recovery sweep ({} mode, cutoff: {})", mode, cutoff.0);

        // Find orphaned tasks
        let orphaned = self
            .dal
            .task_execution()
            .find_stale_heartbeats(cutoff)
            .await
            .map_err(|e| PipelineError::ExecutionFailed {
                message: format!("Failed to find orphaned tasks: {}", e),
            })?;

        if orphaned.is_empty() {
            debug!("No orphaned tasks found");
            return Ok(());
        }

        info!(
            "Found {} orphaned task(s) with stale heartbeats ({} mode)",
            orphaned.len(),
            mode
        );

        let mut recovered = 0;
        let mut abandoned = 0;

        for task in &orphaned {
            let attempts = task.recovery_attempts as usize;

            if attempts >= self.config.max_recovery_attempts {
                // Abandon — too many recovery attempts
                warn!(
                    "Abandoning task {} (pipeline {}) after {} recovery attempts",
                    task.task_name, task.pipeline_execution_id, attempts
                );
                if let Err(e) = self
                    .dal
                    .task_execution()
                    .mark_failed(
                        task.id,
                        &format!(
                            "ABANDONED: task orphaned {} times, exceeded max recovery attempts ({})",
                            attempts, self.config.max_recovery_attempts
                        ),
                    )
                    .await
                {
                    error!("Failed to abandon task {}: {}", task.id, e);
                }
                abandoned += 1;
            } else {
                // Recover — reset to Ready (mark_ready auto-inserts outbox)
                info!(
                    "Recovering task {} (pipeline {}, attempt {})",
                    task.task_name,
                    task.pipeline_execution_id,
                    attempts + 1
                );
                if let Err(e) = self
                    .dal
                    .task_execution()
                    .reset_task_for_recovery(task.id)
                    .await
                {
                    error!("Failed to recover task {}: {}", task.id, e);
                } else {
                    recovered += 1;
                }
            }
        }

        if recovered > 0 || abandoned > 0 {
            info!(
                "Recovery sweep complete: {} recovered, {} abandoned",
                recovered, abandoned
            );
        }

        Ok(())
    }
}

impl Clone for RecoverySweepService {
    fn clone(&self) -> Self {
        Self {
            dal: self.dal.clone(),
            config: self.config.clone(),
            shutdown: self.shutdown.clone(),
            started_at: self.started_at,
            started_at_utc: self.started_at_utc,
        }
    }
}
