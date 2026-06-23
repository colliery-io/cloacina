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
use std::collections::HashSet;
use std::time::{Duration, Instant};

use cloacina_api_types::operations::{OpsMetricsEvent, ReconcilerStatus, ServerHealthLite};
use cloacina_api_types::{AgentInfo, CompilerStatus, ServerMessage};
use tokio::sync::watch;
use tracing::warn;

use crate::AppState;

/// Recipient the Operations UI subscribes to. The delivery sink matches
/// `(recipient, tenant)` exactly; CLOACI-T-0779 publishes a SEPARATE snapshot per
/// tenant under this recipient, so each connection (admin = `None`, a scoped key
/// = its tenant) receives its OWN tenant-scoped operational state.
const OPS_RECIPIENT: &str = "ops_metrics:global";

/// Publish cadence.
const PUBLISH_INTERVAL: Duration = Duration::from_secs(5);

/// Spawn the background ops-metrics publisher.
pub fn spawn(state: AppState, mut shutdown: watch::Receiver<bool>) {
    tokio::spawn(async move {
        // Monotonic push id (NOT a DB row id) for the SDK's dedup window.
        let id_counter = AtomicI64::new(1);
        // Poll for subscribers every second (cheap — just a recipient check) so a
        // newly-connected UI gets its first snapshot within ~1s instead of waiting
        // up to PUBLISH_INTERVAL for the next global tick. That wait was the
        // cold-start that made live pages look "down" on open (CLOACI-T-0774).
        // Snapshots are still only gathered + pushed on the PUBLISH_INTERVAL
        // cadence once a subscriber is connected, plus once immediately when one
        // first connects.
        const CHECK: Duration = Duration::from_secs(1);
        let mut ticker = tokio::time::interval(CHECK);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        // CLOACI-T-0779: per-tenant ops. Each connection subscribes to OPS under
        // its own tenant (admin = None; a scoped key = its tenant), and the
        // delivery sink matches (recipient, tenant) exactly — so we publish a
        // SEPARATE, tenant-scoped snapshot to each connected view. A tenant sees
        // its own build queue / reconciler / fleet / graph health, never another's.
        let mut last_listening: HashSet<Option<String>> = HashSet::new();
        let mut since_publish = PUBLISH_INTERVAL; // force a publish on first connect
        loop {
            tokio::select! {
                changed = shutdown.changed() => {
                    if changed.is_err() || *shutdown.borrow() {
                        break;
                    }
                }
                _ = ticker.tick() => {
                    // Candidate views: admin (None), the public realm, and every
                    // tenant schema. We only gather for views with a live
                    // subscriber, so the cost is a single list query + one
                    // has_recipient check per candidate when nobody's watching.
                    let mut views: Vec<Option<String>> = vec![None, Some("public".to_string())];
                    if let Ok(schemas) =
                        cloacina::database::DatabaseAdmin::new(state.database.clone())
                            .list_tenant_schemas()
                            .await
                    {
                        views.extend(schemas.into_iter().map(Some));
                    }
                    let listening: HashSet<Option<String>> = views
                        .into_iter()
                        .filter(|v| state.delivery_sink.has_recipient(OPS_RECIPIENT, v.as_deref()))
                        .collect();
                    if listening.is_empty() {
                        last_listening.clear();
                        since_publish = PUBLISH_INTERVAL;
                        continue;
                    }
                    since_publish += CHECK;
                    let due = since_publish >= PUBLISH_INTERVAL;
                    for view in &listening {
                        // Publish on the interval, plus immediately when a view
                        // first connects (so its page populates within ~1s).
                        if !due && last_listening.contains(view) {
                            continue;
                        }
                        let event = gather(&state, view.as_deref()).await;
                        match serde_json::to_vec(&event) {
                            Ok(bytes) => {
                                let id = id_counter.fetch_add(1, Ordering::Relaxed);
                                let msg = ServerMessage::push(
                                    id,
                                    "ops_metrics",
                                    OPS_RECIPIENT,
                                    view.clone(),
                                    &bytes,
                                );
                                state
                                    .delivery_sink
                                    .push_direct(OPS_RECIPIENT, view.as_deref(), msg);
                            }
                            Err(e) => warn!(error = %e, "ops_metrics: serialize failed"),
                        }
                    }
                    if due {
                        since_publish = Duration::ZERO;
                    }
                    last_listening = listening;
                }
            }
        }
    });
}

/// Gather one operational-metrics snapshot. Mirrors the four former REST
/// pollers: `/health`+`/ready`, `GET /v1/compiler/status`, `GET /v1/agents`,
/// and the reconciler/package-availability counts.
/// Does an item owned by `item_tenant` belong to the ops view scoped to
/// `view`? CLOACI-T-0779: `None` view = admin (sees everything); `"public"` view
/// = the null-tenant realm (public tasks/agents/graphs carry tenant_id = None);
/// any other view = exact tenant match. So a tenant sees only its own
/// operational state.
fn in_view(item_tenant: Option<&str>, view: Option<&str>) -> bool {
    match view {
        None => true,
        Some("public") => item_tenant.is_none(),
        Some(t) => item_tenant == Some(t),
    }
}

/// Gather one operational-metrics snapshot SCOPED to a view tenant
/// (CLOACI-T-0779). `view` = `None` for the admin/global view, or a tenant for a
/// tenant-scoped Operations page — so each tenant sees its own build queue,
/// reconciler, fleet, and graph health, never another tenant's.
async fn gather(state: &AppState, view: Option<&str>) -> OpsMetricsEvent {
    // The database for this view: the tenant's schema, or admin for the global
    // view. `resolve("public")` returns the admin (public-schema) db.
    let db = match view {
        Some(t) => state
            .tenant_databases
            .resolve(t, &state.database)
            .await
            .unwrap_or_else(|_| state.database.clone()),
        None => state.database.clone(),
    };

    // ── Server health (the /ready rollup), scoped to this view. ──
    let db_ready = db.get_postgres_connection().await.is_ok();
    let graphs = state.graph_scheduler.list_graphs().await;
    let crashed: Vec<String> = graphs
        .iter()
        .filter(|g| in_view(g.tenant_id.as_deref(), view) && !g.running)
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

    // ── Compiler / build pipeline (same mapping as GET /v1/compiler/status),
    //    scoped to this view's schema (its own pending/building/heartbeat). ──
    let compiler =
        match cloacina::registry::workflow_registry::build_queue_stats(&db).await {
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
        .filter(|r| in_view(r.tenant_id.as_deref(), view))
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

    // ── Reconciler / package availability, scoped to this view's schema. ──
    let reconciler =
        match cloacina::registry::workflow_registry::reconciler_stats(&db).await {
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
