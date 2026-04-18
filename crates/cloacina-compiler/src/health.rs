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
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

type Registry = Arc<WorkflowRegistryImpl<UnifiedRegistryStorage>>;

pub(crate) async fn serve(bind: SocketAddr, registry: Registry, shutdown: CancellationToken) {
    let app = Router::new()
        .route("/health", get(health))
        .route("/v1/status", get(status))
        .with_state(registry);

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

async fn status(State(registry): State<Registry>) -> Json<serde_json::Value> {
    match registry.build_queue_stats().await {
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
