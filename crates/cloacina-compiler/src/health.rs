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

use axum::{Json, Router, extract::State, routing::get};
use cloacina::dal::unified::workflow_registry_storage::UnifiedRegistryStorage;
use cloacina::registry::workflow_registry::WorkflowRegistryImpl;
use metrics_exporter_prometheus::PrometheusHandle;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

type Registry = Arc<WorkflowRegistryImpl<UnifiedRegistryStorage>>;

/// Combined HTTP state — registry powers `/v1/status`, the Prometheus
/// handle renders `/metrics`. Both endpoints sit on the same listener
/// to avoid a second bound port for a local service, unless
/// `--metrics-bind` moves `/metrics` to its own listener (CLOACINA-T-1025).
#[derive(Clone)]
struct HttpState {
    registry: Registry,
    metrics_handle: PrometheusHandle,
}

pub(crate) async fn serve(
    bind: SocketAddr,
    registry: Registry,
    metrics_handle: PrometheusHandle,
    serve_metrics: bool,
    shutdown: CancellationToken,
) {
    let state = HttpState {
        registry,
        metrics_handle,
    };
    let mut app = Router::new()
        .route("/health", get(health))
        .route("/v1/status", get(status));
    if serve_metrics {
        app = app.route("/metrics", get(metrics));
    }
    let app = app.with_state(state);

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

/// The metrics listener (`--metrics-bind`): `/metrics` and nothing else.
pub(crate) async fn serve_metrics(
    bind: SocketAddr,
    metrics_handle: PrometheusHandle,
    shutdown: CancellationToken,
) {
    let listener = match tokio::net::TcpListener::bind(bind).await {
        Ok(l) => l,
        Err(e) => {
            warn!(%e, %bind, "failed to bind compiler metrics endpoint");
            return;
        }
    };
    info!(%bind, "compiler metrics endpoint listening");
    serve_metrics_on(listener, metrics_handle, shutdown).await;
}

async fn serve_metrics_on(
    listener: tokio::net::TcpListener,
    metrics_handle: PrometheusHandle,
    shutdown: CancellationToken,
) {
    let app = Router::new().route(
        "/metrics",
        get(move || {
            let handle = metrics_handle.clone();
            async move { render_metrics(&handle) }
        }),
    );
    let shutdown_fut = async move {
        shutdown.cancelled().await;
    };
    if let Err(e) = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_fut)
        .await
    {
        warn!(%e, "compiler metrics endpoint exited with error");
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
            // CLOACINA-T-1074: packages in service with no metadata document.
            "undescribed": stats.undescribed,
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
async fn metrics(State(state): State<HttpState>) -> axum::response::Response {
    render_metrics(&state.metrics_handle)
}

fn render_metrics(handle: &PrometheusHandle) -> axum::response::Response {
    use axum::response::IntoResponse;
    let body = handle.render();
    (
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; version=0.0.4",
        )],
        body,
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    /// One HTTP/1.1 GET over a plain socket; returns the status line.
    async fn get_status_line(addr: SocketAddr, path: &str) -> String {
        let mut stream = tokio::net::TcpStream::connect(addr).await.expect("connect");
        let request = format!("GET {path} HTTP/1.1\r\nHost: test\r\nConnection: close\r\n\r\n");
        stream.write_all(request.as_bytes()).await.expect("write");
        let mut response = Vec::new();
        stream.read_to_end(&mut response).await.expect("read");
        String::from_utf8_lossy(&response)
            .lines()
            .next()
            .unwrap_or("")
            .to_string()
    }

    /// CLOACINA-T-1025: the metrics listener serves `/metrics` and answers 404
    /// for the health listener's routes. No database.
    #[tokio::test]
    async fn metrics_listener_serves_only_metrics() {
        let handle = metrics_exporter_prometheus::PrometheusBuilder::new()
            .build_recorder()
            .handle();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let addr = listener.local_addr().expect("local addr");
        let shutdown = CancellationToken::new();
        let server = tokio::spawn(serve_metrics_on(listener, handle, shutdown.clone()));

        assert!(get_status_line(addr, "/metrics").await.contains(" 200 "));
        assert!(get_status_line(addr, "/health").await.contains(" 404 "));
        assert!(get_status_line(addr, "/v1/status").await.contains(" 404 "));

        shutdown.cancel();
        server.await.expect("metrics listener task");
    }
}
