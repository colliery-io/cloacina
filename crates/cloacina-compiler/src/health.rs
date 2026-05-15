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

//! Local HTTP endpoint for /health and /v1/status. Consumed by
//! `cloacinactl compiler status` / `health`.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::{extract::State, routing::get, Json, Router};
use cloacina::dal::unified::workflow_registry_storage::UnifiedRegistryStorage;
use cloacina::registry::workflow_registry::WorkflowRegistryImpl;
use metrics_exporter_prometheus::PrometheusHandle;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

type Registry = Arc<WorkflowRegistryImpl<UnifiedRegistryStorage>>;

/// Combined HTTP state — registry powers `/v1/status`, the Prometheus
/// handle renders `/metrics`. Both endpoints sit on the same listener
/// to avoid a second bound port for a local service.
#[derive(Clone)]
struct HttpState {
    registry: Registry,
    metrics_handle: PrometheusHandle,
}

pub(crate) async fn serve(
    bind: SocketAddr,
    registry: Registry,
    metrics_handle: PrometheusHandle,
    shutdown: CancellationToken,
) {
    let state = HttpState {
        registry,
        metrics_handle,
    };
    let app = Router::new()
        .route("/health", get(health))
        .route("/v1/status", get(status))
        .route("/metrics", get(metrics))
        .with_state(state);

    let listener = match tokio::net::TcpListener::bind(bind).await {
        Ok(l) => l,
        Err(e) => {
            warn!(%e, %bind, "failed to bind compiler health endpoint");
            return;
        }
    };
    info!(%bind, "compiler health endpoint listening");

    let shutdown_fut = async move {
        shutdown.cancelled().await;
    };

    if let Err(e) = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_fut)
        .await
    {
        warn!(%e, "compiler health endpoint exited with error");
    }
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok" }))
}

async fn status(State(state): State<HttpState>) -> Json<serde_json::Value> {
    match state.registry.build_queue_stats().await {
        Ok(stats) => Json(serde_json::json!({
            "status": "ok",
            "pending": stats.pending,
            "building": stats.building,
            "last_success_at": stats.last_success_at.map(|t| t.to_rfc3339()),
            "last_failure_at": stats.last_failure_at.map(|t| t.to_rfc3339()),
            "heartbeat_at": stats.heartbeat_at.map(|t| t.to_rfc3339()),
        })),
        Err(e) => Json(serde_json::json!({
            "status": "degraded",
            "error": format!("{}", e),
        })),
    }
}

/// GET /metrics — Prometheus text exposition. Matches the
/// `cloacina-server` endpoint: public (no auth) so a Prometheus
/// scraper can poll it without credential management.
async fn metrics(State(state): State<HttpState>) -> impl axum::response::IntoResponse {
    let body = state.metrics_handle.render();
    (
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; version=0.0.4",
        )],
        body,
    )
}
