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
//! `cloacinactl compiler status` / `health` (T-0525).

use std::net::SocketAddr;

use axum::{routing::get, Json, Router};
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

pub(crate) async fn serve(bind: SocketAddr, shutdown: CancellationToken) {
    let app = Router::new()
        .route("/health", get(health))
        .route("/v1/status", get(status));

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

async fn status() -> Json<serde_json::Value> {
    // T-0525 wires real queue-depth + last-build telemetry.
    Json(serde_json::json!({
        "status": "ok",
        "pending": null,
        "building": null,
        "last_success_at": null,
        "last_failure_at": null,
        "heartbeat_at": null,
    }))
}
