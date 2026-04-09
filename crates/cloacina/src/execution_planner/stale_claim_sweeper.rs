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

//! Stale claim sweeper — background service for expired claim recovery.
//!
//! Periodically scans for tasks with stale heartbeats (crashed runners),
//! releases their claims, and resets them to Ready for re-execution.
//!
//! ## Startup Grace Period
//!
//! In a distributed system, the scheduler may restart while runners are
//! still executing tasks. The sweeper records when it became ready and
//! ignores all claims during its first `stale_threshold` duration after
//! startup. This prevents false positives where tasks look stale simply
//! because the sweeper wasn't running to observe their heartbeats.

use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::watch;
use tracing::{debug, info, warn};

use crate::dal::DAL;

/// Configuration for the stale claim sweeper.
#[derive(Debug, Clone)]
pub struct StaleClaimSweeperConfig {
    /// How often to run the sweep (default 30s).
    pub sweep_interval: Duration,
    /// How old a heartbeat must be to consider the claim stale (default 60s).
    /// Must be greater than the heartbeat interval.
    pub stale_threshold: Duration,
}

impl Default for StaleClaimSweeperConfig {
    fn default() -> Self {
        Self {
            sweep_interval: Duration::from_secs(30),
            stale_threshold: Duration::from_secs(60),
        }
    }
}

/// Background service that sweeps for stale task claims.
pub struct StaleClaimSweeper {
    dal: Arc<DAL>,
    config: StaleClaimSweeperConfig,
    shutdown_rx: watch::Receiver<bool>,
    /// When the sweeper became ready. Used for the startup grace period.
    ready_at: Instant,
}

impl StaleClaimSweeper {
    /// Create a new stale claim sweeper.
    pub fn new(
        dal: Arc<DAL>,
        config: StaleClaimSweeperConfig,
        shutdown_rx: watch::Receiver<bool>,
    ) -> Self {
        Self {
            dal,
            config,
            shutdown_rx,
            ready_at: Instant::now(),
        }
    }

    /// Run the sweep loop.
    pub async fn run(&mut self) {
        info!(
            "Starting stale claim sweeper (interval: {}s, threshold: {}s, grace period: {}s)",
            self.config.sweep_interval.as_secs(),
            self.config.stale_threshold.as_secs(),
            self.config.stale_threshold.as_secs(),
        );

        let mut interval = tokio::time::interval(self.config.sweep_interval);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    self.sweep().await;
                }
                _ = self.shutdown_rx.changed() => {
                    if *self.shutdown_rx.borrow() {
                        info!("Stale claim sweeper shutting down");
                        break;
                    }
                }
            }
        }
    }

    /// Perform a single sweep pass.
    pub async fn sweep(&self) {
        // Startup grace period: don't sweep until we've been running for
        // at least one full stale_threshold duration. This prevents false
        // positives when the scheduler restarts — tasks that were being
        // executed by healthy runners look "stale" simply because the
        // sweeper wasn't running to see their heartbeats.
        let uptime = self.ready_at.elapsed();
        if uptime < self.config.stale_threshold {
            debug!(
                "Stale claim sweeper in grace period ({:.1}s / {}s) — skipping sweep",
                uptime.as_secs_f64(),
                self.config.stale_threshold.as_secs()
            );
            return;
        }

        // Find tasks with stale heartbeats
        let stale_claims = match self
            .dal
            .task_execution()
            .find_stale_claims(self.config.stale_threshold)
            .await
        {
            Ok(claims) => claims,
            Err(e) => {
                warn!("Stale claim sweep failed: {}", e);
                return;
            }
        };

        if stale_claims.is_empty() {
            debug!("Stale claim sweep: no stale claims found");
            return;
        }

        info!(
            "Stale claim sweep found {} stale claims",
            stale_claims.len()
        );

        for claim in &stale_claims {
            let age = chrono::Utc::now() - claim.heartbeat_at;

            // Release the claim
            if let Err(e) = self
                .dal
                .task_execution()
                .release_runner_claim(claim.task_id)
                .await
            {
                warn!(
                    "Failed to release stale claim on task {}: {}",
                    claim.task_id, e
                );
                continue;
            }

            // Reset task status to Ready for re-execution
            if let Err(e) = self.dal.task_execution().mark_ready(claim.task_id).await {
                warn!(
                    "Failed to reset task {} to Ready after stale claim release: {}",
                    claim.task_id, e
                );
                continue;
            }

            info!(
                "Released stale claim: task {} (runner {}, last heartbeat {}s ago)",
                claim.task_id,
                claim.claimed_by,
                age.num_seconds()
            );
        }

        info!(
            "Stale claim sweep complete: {} claims released",
            stale_claims.len()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_defaults() {
        let config = StaleClaimSweeperConfig::default();
        assert_eq!(config.sweep_interval, Duration::from_secs(30));
        assert_eq!(config.stale_threshold, Duration::from_secs(60));
    }

    #[test]
    fn config_custom_values() {
        let config = StaleClaimSweeperConfig {
            sweep_interval: Duration::from_secs(10),
            stale_threshold: Duration::from_secs(120),
        };
        assert_eq!(config.sweep_interval, Duration::from_secs(10));
        assert_eq!(config.stale_threshold, Duration::from_secs(120));
    }

    #[test]
    fn config_clone() {
        let config = StaleClaimSweeperConfig::default();
        let cloned = config.clone();
        assert_eq!(config.sweep_interval, cloned.sweep_interval);
        assert_eq!(config.stale_threshold, cloned.stale_threshold);
    }
}
