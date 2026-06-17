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

//! Operational-metrics publisher (CLOACI-T-0718).
//!
//! Gathers a snapshot of the control plane's own health (server / compiler /
//! fleet / reconciler) on a fixed tick and pushes it to the Operations UI over
//! the WS substrate — but **directly to the in-memory delivery sink**, NOT the
//! durable `delivery_outbox`. Ops metrics are ephemeral latest-snapshot data;
//! routing them through the durable path would accrue rows with no retention.
//! Publishing is gated on a connected subscriber, so nothing is gathered when
//! no Operations page is open.

use std::sync::atomic::{AtomicI64, Ordering};
use std::time::{Duration, Instant};

use cloacina_api_types::operations::{OpsMetricsEvent, ReconcilerStatus, ServerHealthLite};
use cloacina_api_types::{AgentInfo, CompilerStatus, ServerMessage};
use tokio::sync::watch;
use tracing::warn;

use crate::AppState;

/// Recipient the Operations UI subscribes to. Tenant scope is `None` (admin):
/// admin keys are `tenant_id = None`, and the delivery sink matches
/// `(recipient, tenant)` exactly, so only admin connections receive these.
const OPS_RECIPIENT: &str = "ops_metrics:global";

/// Publish cadence.
const PUBLISH_INTERVAL: Duration = Duration::from_secs(5);

/// Spawn the background ops-metrics publisher.
pub fn spawn(state: AppState, mut shutdown: watch::Receiver<bool>) {
    tokio::spawn(async move {
        // Monotonic push id (NOT a DB row id) for the SDK's dedup window.
        let id_counter = AtomicI64::new(1);
        let mut ticker = tokio::time::interval(PUBLISH_INTERVAL);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                changed = shutdown.changed() => {
                    if changed.is_err() || *shutdown.borrow() {
                        break;
                    }
                }
                _ = ticker.tick() => {
                    // Nobody listening → don't even gather.
                    if !state.delivery_sink.has_recipient(OPS_RECIPIENT, None) {
                        continue;
                    }
                    let event = gather(&state).await;
                    match serde_json::to_vec(&event) {
                        Ok(bytes) => {
                            let id = id_counter.fetch_add(1, Ordering::Relaxed);
                            let msg =
                                ServerMessage::push(id, "ops_metrics", OPS_RECIPIENT, None, &bytes);
                            state.delivery_sink.push_direct(OPS_RECIPIENT, None, msg);
                        }
                        Err(e) => warn!(error = %e, "ops_metrics: serialize failed"),
                    }
                }
            }
        }
    });
}

/// Gather one operational-metrics snapshot. Mirrors the four former REST
/// pollers: `/health`+`/ready`, `GET /v1/compiler/status`, `GET /v1/agents`,
/// and the reconciler/package-availability counts.
async fn gather(state: &AppState) -> OpsMetricsEvent {
    // ── Server health (the /ready rollup). ──
    let db_ready = state.database.get_postgres_connection().await.is_ok();
    let graphs = state.graph_scheduler.list_graphs().await;
    let crashed: Vec<String> = graphs
        .iter()
        .filter(|g| !g.running)
        .map(|g| g.name.clone())
        .collect();
    let ready = db_ready && crashed.is_empty();
    let reason = if !db_ready {
        Some("database unreachable".to_string())
    } else if !crashed.is_empty() {
        Some(format!(
            "crashed computation graphs: {}",
            crashed.join(", ")
        ))
    } else {
        None
    };
    let server = ServerHealthLite {
        alive: true,
        ready,
        reason,
    };

    // ── Compiler / build pipeline (same mapping as GET /v1/compiler/status). ──
    let compiler =
        match cloacina::registry::workflow_registry::build_queue_stats(&state.database).await {
            Ok(s) => {
                let status = if s.building > 0 {
                    "building"
                } else if s.pending > 0 {
                    "backlogged"
                } else {
                    "idle"
                };
                CompilerStatus {
                    status: status.to_string(),
                    pending: s.pending,
                    building: s.building,
                    seconds_since_heartbeat: s
                        .heartbeat_at
                        .map(|h| (chrono::Utc::now() - h).num_seconds().max(0) as u64),
                    last_success_at: s.last_success_at.map(|t| t.to_rfc3339()),
                    last_failure_at: s.last_failure_at.map(|t| t.to_rfc3339()),
                }
            }
            Err(e) => {
                warn!(error = %e, "ops_metrics: build_queue_stats failed");
                CompilerStatus {
                    status: "idle".to_string(),
                    pending: 0,
                    building: 0,
                    seconds_since_heartbeat: None,
                    last_success_at: None,
                    last_failure_at: None,
                }
            }
        };

    // ── Fleet roster (same mapping as GET /v1/agents). ──
    let now = Instant::now();
    let fleet: Vec<AgentInfo> = state
        .agent_registry
        .snapshot()
        .into_iter()
        .map(|r| AgentInfo {
            agent_id: r.agent_id,
            target_triple: r.target_triple,
            max_concurrency: r.max_concurrency,
            in_flight: r.in_flight,
            available_capacity: r.available_capacity,
            seconds_since_heartbeat: now.duration_since(r.last_heartbeat).as_secs(),
            capabilities: r.capabilities,
            tenant_id: r.tenant_id,
        })
        .collect();

    // ── Reconciler / package availability. ──
    let reconciler =
        match cloacina::registry::workflow_registry::reconciler_stats(&state.database).await {
            Ok(s) => ReconcilerStatus {
                status: if s.failed > 0 { "errors" } else { "ok" }.to_string(),
                built: s.built,
                failed: s.failed,
                last_built_at: s.last_built_at.map(|t| t.to_rfc3339()),
            },
            Err(e) => {
                warn!(error = %e, "ops_metrics: reconciler_stats failed");
                ReconcilerStatus {
                    status: "ok".to_string(),
                    built: 0,
                    failed: 0,
                    last_built_at: None,
                }
            }
        };

    OpsMetricsEvent {
        server,
        compiler,
        fleet,
        reconciler,
        ts: chrono::Utc::now().to_rfc3339(),
    }
}
