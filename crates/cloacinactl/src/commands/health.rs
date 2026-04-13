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

//! Daemon health observability — shared state, Unix socket listener, and log pulse.

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

/// Health response served over the Unix socket.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonHealth {
    pub status: String,
    pub pid: u32,
    pub uptime_seconds: u64,
    pub database: DatabaseHealth,
    pub reconciler: ReconcilerHealth,
    pub active_pipelines: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseHealth {
    pub connected: bool,
    pub backend: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconcilerHealth {
    pub packages_loaded: usize,
    pub last_run_at: Option<String>,
}

/// Mutable state updated by the daemon's main loop.
pub struct SharedDaemonState {
    pub start_time: Instant,
    pub packages_loaded: AtomicUsize,
    pub last_reconciliation: Mutex<Option<chrono::DateTime<chrono::Utc>>>,
}

impl SharedDaemonState {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            packages_loaded: AtomicUsize::new(0),
            last_reconciliation: Mutex::new(None),
        }
    }

    pub fn set_packages_loaded(&self, count: usize) {
        self.packages_loaded.store(count, Ordering::Relaxed);
    }

    pub async fn set_last_reconciliation(&self, time: chrono::DateTime<chrono::Utc>) {
        *self.last_reconciliation.lock().await = Some(time);
    }
}

/// Build a health snapshot by querying DB and reading shared state.
pub async fn build_health(
    dal: &cloacina::dal::DAL,
    state: &SharedDaemonState,
    db_backend: &str,
) -> DaemonHealth {
    // Test DB connectivity
    let db_connected = dal
        .workflow_execution()
        .get_active_executions()
        .await
        .is_ok();

    // Count active pipelines
    let active_pipelines = dal
        .workflow_execution()
        .get_active_executions()
        .await
        .map(|v| v.len() as u64)
        .unwrap_or(0);

    let packages_loaded = state.packages_loaded.load(Ordering::Relaxed);
    let last_reconciliation = state.last_reconciliation.lock().await;
    let uptime = state.start_time.elapsed().as_secs();

    let status = if !db_connected {
        "unhealthy"
    } else {
        "healthy"
    };

    DaemonHealth {
        status: status.to_string(),
        pid: std::process::id(),
        uptime_seconds: uptime,
        database: DatabaseHealth {
            connected: db_connected,
            backend: db_backend.to_string(),
        },
        reconciler: ReconcilerHealth {
            packages_loaded,
            last_run_at: last_reconciliation.map(|t| t.to_rfc3339()),
        },
        active_pipelines,
    }
}

/// Accept connections on a Unix domain socket and serve health JSON.
///
/// Each connection receives the full JSON response, then the connection is closed.
/// No request is needed — connecting to the socket is the request.
pub async fn run_health_socket(
    socket_path: PathBuf,
    dal: cloacina::dal::DAL,
    state: Arc<SharedDaemonState>,
    db_backend: String,
    mut shutdown_rx: tokio::sync::watch::Receiver<bool>,
) {
    // Remove stale socket from a previous run
    if socket_path.exists() {
        warn!("Removing stale health socket at {}", socket_path.display());
        let _ = std::fs::remove_file(&socket_path);
    }

    let listener = match tokio::net::UnixListener::bind(&socket_path) {
        Ok(l) => {
            info!("Health socket listening at {}", socket_path.display());
            l
        }
        Err(e) => {
            warn!(
                "Failed to bind health socket at {}: {}",
                socket_path.display(),
                e
            );
            return;
        }
    };

    loop {
        tokio::select! {
            result = listener.accept() => {
                match result {
                    Ok((mut stream, _)) => {
                        let health = build_health(&dal, &state, &db_backend).await;
                        match serde_json::to_vec_pretty(&health) {
                            Ok(json) => {
                                let _ = stream.write_all(&json).await;
                                let _ = stream.shutdown().await;
                            }
                            Err(e) => {
                                debug!("Failed to serialize health: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        debug!("Failed to accept health connection: {}", e);
                    }
                }
            }
            _ = shutdown_rx.changed() => {
                debug!("Health socket shutting down");
                break;
            }
        }
    }

    // Cleanup socket file
    let _ = std::fs::remove_file(&socket_path);
    info!("Health socket removed");
}

/// Emit a periodic structured health log line.
pub async fn run_health_pulse(
    dal: cloacina::dal::DAL,
    state: Arc<SharedDaemonState>,
    db_backend: String,
    interval: Duration,
    mut shutdown_rx: tokio::sync::watch::Receiver<bool>,
) {
    let mut ticker = tokio::time::interval(interval);
    // Skip the immediate first tick
    ticker.tick().await;

    loop {
        tokio::select! {
            _ = ticker.tick() => {
                let health = build_health(&dal, &state, &db_backend).await;
                info!(
                    target: "cloacina::health",
                    status = %health.status,
                    uptime_s = health.uptime_seconds,
                    db_connected = health.database.connected,
                    active_pipelines = health.active_pipelines,
                    packages_loaded = health.reconciler.packages_loaded,
                    "health pulse"
                );
            }
            _ = shutdown_rx.changed() => break,
        }
    }
}
