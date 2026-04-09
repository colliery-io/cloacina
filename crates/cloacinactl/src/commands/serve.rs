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

//! API server mode — HTTP service backed by Postgres.
//!
//! Starts an axum HTTP server with health/ready/metrics endpoints.
//! Later tasks add auth, tenant management, workflow upload, and execution APIs.

use anyhow::{Context, Result};
use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::get, Json, Router};
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::{info, warn};
use tracing_appender::rolling;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use cloacina::computation_graph::registry::EndpointRegistry;
use cloacina::computation_graph::scheduler::ReactiveScheduler;
use cloacina::database::Database;
use cloacina::runner::{DefaultRunner, DefaultRunnerConfig};
use cloacina::security::SecurityConfig;

/// Shared application state accessible from all route handlers.
#[derive(Clone)]
pub struct AppState {
    pub database: Database,
    pub runner: Arc<DefaultRunner>,
    pub key_cache: Arc<crate::server::auth::KeyCache>,
    pub endpoint_registry: EndpointRegistry,
    pub reactive_scheduler: Arc<ReactiveScheduler>,
    pub security_config: SecurityConfig,
    /// Short-lived WebSocket auth tickets (single-use, TTL-based).
    pub ws_tickets: Arc<crate::server::auth::WsTicketStore>,
}

/// Run the API server.
pub async fn run(
    home: std::path::PathBuf,
    bind: SocketAddr,
    database_url: String,
    verbose: bool,
    bootstrap_key: Option<String>,
) -> Result<()> {
    // Set up logging (file + stderr, same as daemon)
    std::fs::create_dir_all(&home)
        .with_context(|| format!("Failed to create home: {}", home.display()))?;

    let logs_dir = home.join("logs");
    std::fs::create_dir_all(&logs_dir)
        .with_context(|| format!("Failed to create logs dir: {}", logs_dir.display()))?;

    let filter = if verbose {
        EnvFilter::new("debug")
    } else {
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))
    };

    let file_appender = rolling::daily(&logs_dir, "cloacina-server.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_writer(std::io::stderr))
        .with(fmt::layer().json().with_writer(non_blocking))
        .init();

    info!("Starting API server");
    info!("  Bind:     {}", bind);
    info!("  Database: {}", mask_db_url(&database_url));
    info!("  Home:     {}", home.display());
    warn!("Server running without TLS -- use a TLS-terminating reverse proxy (nginx, Caddy, Envoy) in production");

    // Connect to Postgres with DB-backed registry (so uploaded packages get compiled + loaded)
    let runner_config = DefaultRunnerConfig::builder()
        .registry_storage_backend("database")
        .build();

    let runner = DefaultRunner::with_config(&database_url, runner_config)
        .await
        .context("Failed to connect to database")?;

    info!("Connected to Postgres, migrations applied");

    let endpoint_registry = EndpointRegistry::new();
    let unified_dal = cloacina::dal::unified::DAL::new(runner.database().clone());
    let reactive_scheduler = Arc::new(ReactiveScheduler::with_dal(
        endpoint_registry.clone(),
        unified_dal,
    ));

    // Wire reactive scheduler into the runner so the reconciler can route CG packages
    runner
        .set_reactive_scheduler(reactive_scheduler.clone())
        .await;

    let state = AppState {
        database: runner.database().clone(),
        runner: Arc::new(runner),
        key_cache: Arc::new(crate::server::auth::KeyCache::default_cache()),
        endpoint_registry,
        reactive_scheduler,
        security_config: SecurityConfig::default(),
        ws_tickets: Arc::new(crate::server::auth::WsTicketStore::new(
            std::time::Duration::from_secs(60),
        )),
    };

    // Bootstrap: create initial admin key if none exist
    bootstrap_admin_key(&state, &home, bootstrap_key.as_deref()).await?;

    // Keep references for shutdown
    let scheduler_for_shutdown = state.reactive_scheduler.clone();
    let runner_for_shutdown = state.runner.clone();

    // Build router
    let app = build_router(state);

    // Start server with graceful shutdown
    let listener = tokio::net::TcpListener::bind(bind)
        .await
        .with_context(|| format!("Failed to bind to {}", bind))?;

    info!("");
    info!("API server is running on http://{}", bind);
    info!("  GET  /health     — liveness check");
    info!("  GET  /ready      — readiness check");
    info!("  GET  /metrics    — Prometheus metrics");
    info!("  POST /auth/keys  — create API key (auth required)");
    info!("  GET  /auth/keys  — list API keys (auth required)");
    info!("  DEL  /auth/keys/:id — revoke key (auth required)");
    info!("");

    // Shared shutdown signal — used by supervision loop and graceful shutdown.
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

    // Start supervision loop — auto-restart crashed accumulator/reactor tasks
    let _supervision_handle = scheduler_for_shutdown
        .start_supervision(shutdown_rx.clone(), std::time::Duration::from_secs(5));

    let scheduler_handle = {
        let scheduler = scheduler_for_shutdown.clone();
        let mut rx = shutdown_rx; // move, not clone — only consumer
        tokio::spawn(async move {
            let _ = rx.changed().await;
            info!("Shutting down reactive scheduler...");
            scheduler.shutdown_all().await;
            info!("Reactive scheduler shutdown complete");
        })
    };

    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            shutdown_signal().await;
            // Signal the reactive scheduler to shut down first
            let _ = shutdown_tx.send(true);
            // Wait for reactive scheduler to finish flushing/persisting
            let _ = scheduler_handle.await;
            // Shut down the workflow runner (scheduler loop, executor, stale claim sweeper)
            info!("Shutting down workflow runner...");
            match tokio::time::timeout(
                std::time::Duration::from_secs(30),
                runner_for_shutdown.shutdown(),
            )
            .await
            {
                Ok(Ok(())) => info!("Workflow runner shutdown complete"),
                Ok(Err(e)) => warn!("Workflow runner shutdown error: {}", e),
                Err(_) => warn!("Workflow runner shutdown timed out after 30s"),
            }
        })
        .await
        .context("Server error")?;

    info!("API server shutdown complete");
    Ok(())
}

/// Build the axum router with all routes.
///
/// Public routes (health/ready/metrics) have no auth.
/// Authenticated routes use `route_layer` (not `layer`) so unmatched paths still 404.
fn build_router(state: AppState) -> Router {
    use axum::{extract::DefaultBodyLimit, middleware, routing::delete, routing::post};
    use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer};

    // Authenticated routes — behind auth middleware
    let auth_routes = Router::new()
        // Key management
        .route("/auth/keys", post(crate::server::keys::create_key))
        .route("/auth/keys", get(crate::server::keys::list_keys))
        .route(
            "/auth/keys/{key_id}",
            delete(crate::server::keys::revoke_key),
        )
        // WebSocket ticket exchange (single-use, short-lived)
        .route(
            "/auth/ws-ticket",
            post(crate::server::keys::create_ws_ticket),
        )
        // Tenant management
        .route("/tenants", post(crate::server::tenants::create_tenant))
        .route("/tenants", get(crate::server::tenants::list_tenants))
        .route(
            "/tenants/{schema_name}",
            delete(crate::server::tenants::remove_tenant),
        )
        // Tenant-scoped key creation (admin-only)
        .route(
            "/tenants/{tenant_id}/keys",
            post(crate::server::keys::create_tenant_key),
        )
        // Workflow packages (tenant-scoped)
        .route(
            "/tenants/{tenant_id}/workflows",
            post(crate::server::workflows::upload_workflow),
        )
        .route(
            "/tenants/{tenant_id}/workflows",
            get(crate::server::workflows::list_workflows),
        )
        .route(
            "/tenants/{tenant_id}/workflows/{name}",
            get(crate::server::workflows::get_workflow),
        )
        .route(
            "/tenants/{tenant_id}/workflows/{name}/{version}",
            delete(crate::server::workflows::delete_workflow),
        )
        // Trigger schedules (tenant-scoped, read-only)
        .route(
            "/tenants/{tenant_id}/triggers",
            get(crate::server::triggers::list_triggers),
        )
        .route(
            "/tenants/{tenant_id}/triggers/{name}",
            get(crate::server::triggers::get_trigger),
        )
        // Executions (tenant-scoped)
        .route(
            "/tenants/{tenant_id}/workflows/{name}/execute",
            post(crate::server::executions::execute_workflow),
        )
        .route(
            "/tenants/{tenant_id}/executions",
            get(crate::server::executions::list_executions),
        )
        .route(
            "/tenants/{tenant_id}/executions/{exec_id}",
            get(crate::server::executions::get_execution),
        )
        .route(
            "/tenants/{tenant_id}/executions/{exec_id}/events",
            get(crate::server::executions::get_execution_events),
        )
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            crate::server::auth::require_auth,
        ));

    // Reactive health routes — behind auth
    let reactive_health_routes = Router::new()
        .route(
            "/v1/health/accumulators",
            get(crate::server::health_reactive::list_accumulators),
        )
        .route(
            "/v1/health/reactors",
            get(crate::server::health_reactive::list_reactors),
        )
        .route(
            "/v1/health/reactors/{name}",
            get(crate::server::health_reactive::get_reactor),
        )
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            crate::server::auth::require_auth,
        ));

    // WebSocket routes — auth handled in the handler (before upgrade)
    let ws_routes = Router::new()
        .route(
            "/v1/ws/accumulator/{name}",
            get(crate::server::ws::accumulator_ws),
        )
        .route("/v1/ws/reactor/{name}", get(crate::server::ws::reactor_ws));

    // Rate limiting — per-IP, applied globally
    // 30 requests per second burst, replenishes 10/sec
    let rate_limit_config = GovernorConfigBuilder::default()
        .per_second(10)
        .burst_size(30)
        .finish()
        .expect("Failed to build rate limit config");
    let rate_limit_layer = GovernorLayer::new(std::sync::Arc::new(rate_limit_config));

    // Public routes — no auth
    Router::new()
        .route("/health", get(health))
        .route("/ready", get(ready))
        .route("/metrics", get(metrics))
        // All authenticated routes under /v1/
        .nest("/v1", auth_routes)
        .merge(reactive_health_routes)
        .merge(ws_routes)
        .fallback(fallback_404)
        // Body size limit: 100MB (matches PackageValidator)
        .layer(DefaultBodyLimit::max(100 * 1024 * 1024))
        // Rate limiting: per-IP
        .layer(rate_limit_layer)
        .with_state(state)
}

/// GET /health — liveness check (no auth, no DB)
async fn health() -> impl IntoResponse {
    Json(serde_json::json!({"status": "ok"}))
}

/// GET /ready — readiness check (verifies DB connection pool is healthy)
async fn ready(State(state): State<AppState>) -> impl IntoResponse {
    // Verify we can acquire a connection from the pool
    let db_ready = state.database.get_postgres_connection().await.is_ok();

    // Check if any computation graphs have crashed
    let graphs = state.reactive_scheduler.list_graphs().await;
    let crashed_graphs: Vec<&str> = graphs
        .iter()
        .filter(|g| !g.running)
        .map(|g| g.name.as_str())
        .collect();

    if db_ready && crashed_graphs.is_empty() {
        (StatusCode::OK, Json(serde_json::json!({"status": "ready"})))
    } else if !db_ready {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"status": "not ready", "reason": "database unreachable"})),
        )
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "status": "not ready",
                "reason": "crashed computation graphs",
                "crashed_graphs": crashed_graphs,
            })),
        )
    }
}

/// GET /metrics — Prometheus metrics (placeholder for now)
async fn metrics() -> impl IntoResponse {
    // TODO: Wire in prometheus metrics in a future task
    (
        StatusCode::OK,
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; version=0.0.4",
        )],
        "# HELP cloacina_up Server is running\n# TYPE cloacina_up gauge\ncloacina_up 1\n",
    )
}

/// Fallback for unmatched routes — returns 404 JSON
async fn fallback_404() -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({"error": "not found"})),
    )
}

/// Wait for shutdown signal (SIGINT or SIGTERM)
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to listen for ctrl+c");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to listen for SIGTERM")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => info!("Received SIGINT — shutting down"),
        _ = terminate => info!("Received SIGTERM — shutting down"),
    }
}

/// Bootstrap: create an admin API key on first startup if none exist.
///
/// Writes the plaintext key to `~/.cloacina/bootstrap-key` with mode 0600.
/// The key is never logged.
async fn bootstrap_admin_key(
    state: &AppState,
    home: &std::path::Path,
    provided_key: Option<&str>,
) -> Result<()> {
    let dal = cloacina::dal::DAL::new(state.database.clone());
    let has_keys = dal.api_keys().has_any_keys().await.unwrap_or(false);

    if has_keys {
        info!("API keys exist — skipping bootstrap");
        return Ok(());
    }

    info!("No API keys found — creating bootstrap admin key");

    let (plaintext, hash) = if let Some(key) = provided_key {
        // Use provided key
        let hash = cloacina::security::api_keys::hash_api_key(key);
        (key.to_string(), hash)
    } else {
        // Auto-generate
        cloacina::security::api_keys::generate_api_key()
    };

    dal.api_keys()
        .create_key(&hash, "bootstrap-admin", None, true, "admin")
        .await
        .context("Failed to create bootstrap admin key")?;

    // Write plaintext to file (never log it)
    let key_path = home.join("bootstrap-key");
    std::fs::write(&key_path, &plaintext)
        .with_context(|| format!("Failed to write bootstrap key to {}", key_path.display()))?;

    // Set file permissions to owner-only (Unix)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&key_path, std::fs::Permissions::from_mode(0o600))
            .with_context(|| format!("Failed to set permissions on {}", key_path.display()))?;
    }

    info!(
        "Bootstrap admin key written to {} (mode 0600)",
        key_path.display()
    );

    Ok(())
}

/// Mask password in database URL for logging
/// Re-export from cloacina::logging for backward compat in tests.
fn mask_db_url(url: &str) -> String {
    cloacina::logging::mask_db_url(url)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use http_body_util::BodyExt;
    use serial_test::serial;
    use std::io::Write;
    use tower::ServiceExt;

    const TEST_DB_URL: &str = "postgres://cloacina:cloacina@localhost:5432/cloacina";

    /// Create a test AppState with a real Postgres connection.
    async fn test_state() -> AppState {
        let runner_config = cloacina::runner::DefaultRunnerConfig::builder()
            .registry_storage_backend("database")
            .build();

        let runner = cloacina::runner::DefaultRunner::with_config(TEST_DB_URL, runner_config)
            .await
            .expect("Failed to connect to test database");

        AppState {
            database: runner.database().clone(),
            runner: Arc::new(runner),
            key_cache: Arc::new(crate::server::auth::KeyCache::default_cache()),
            endpoint_registry: EndpointRegistry::new(),
            reactive_scheduler: Arc::new(ReactiveScheduler::new(EndpointRegistry::new())),
            security_config: SecurityConfig::default(),
            ws_tickets: Arc::new(crate::server::auth::WsTicketStore::new(
                std::time::Duration::from_secs(60),
            )),
        }
    }

    /// Create a bootstrap API key and return the plaintext token.
    async fn create_test_api_key(state: &AppState) -> String {
        let (plaintext, hash) = cloacina::security::api_keys::generate_api_key();
        let dal = cloacina::dal::DAL::new(state.database.clone());
        dal.api_keys()
            .create_key(&hash, "test-key", None, true, "admin")
            .await
            .expect("Failed to create test API key");
        plaintext
    }

    /// Send a request to the router and return (status, body as serde_json::Value).
    async fn send_request(
        app: Router,
        request: axum::http::Request<Body>,
    ) -> (StatusCode, serde_json::Value) {
        let response = app.oneshot(request).await.expect("request failed");
        let status = response.status();
        let body_bytes = response
            .into_body()
            .collect()
            .await
            .expect("failed to read body")
            .to_bytes();
        let body: serde_json::Value =
            serde_json::from_slice(&body_bytes).unwrap_or(serde_json::json!({}));
        (status, body)
    }

    // ── Health / Ready / Metrics ──────────────────────────────────────

    #[tokio::test]
    #[serial]
    async fn test_health_returns_200() {
        let state = test_state().await;
        let app = build_router(state);

        let req = axum::http::Request::builder()
            .uri("/health")
            .body(Body::empty())
            .unwrap();

        let (status, body) = send_request(app, req).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["status"], "ok");
    }

    #[tokio::test]
    #[serial]
    async fn test_ready_returns_200_with_db() {
        let state = test_state().await;
        let app = build_router(state);

        let req = axum::http::Request::builder()
            .uri("/ready")
            .body(Body::empty())
            .unwrap();

        let (status, body) = send_request(app, req).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["status"], "ready");
    }

    #[tokio::test]
    #[serial]
    async fn test_metrics_returns_200() {
        let state = test_state().await;
        let app = build_router(state);

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/metrics")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("request failed");

        assert_eq!(response.status(), StatusCode::OK);
        let body_bytes = response
            .into_body()
            .collect()
            .await
            .expect("failed to read body")
            .to_bytes();
        let text = String::from_utf8_lossy(&body_bytes);
        assert!(text.contains("cloacina_up"));
    }

    // ── Auth middleware ───────────────────────────────────────────────

    #[tokio::test]
    #[serial]
    async fn test_auth_no_token_returns_401() {
        let state = test_state().await;
        let app = build_router(state);

        let req = axum::http::Request::builder()
            .uri("/auth/keys")
            .body(Body::empty())
            .unwrap();

        let (status, body) = send_request(app, req).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert!(body["error"].as_str().unwrap().contains("Authorization"));
    }

    #[tokio::test]
    #[serial]
    async fn test_auth_invalid_token_returns_401() {
        let state = test_state().await;
        let app = build_router(state);

        let req = axum::http::Request::builder()
            .uri("/auth/keys")
            .header("Authorization", "Bearer clk_totally_invalid_key_12345678")
            .body(Body::empty())
            .unwrap();

        let (status, body) = send_request(app, req).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert!(body["error"].as_str().unwrap().contains("invalid"));
    }

    #[tokio::test]
    #[serial]
    async fn test_auth_valid_token_passes() {
        let state = test_state().await;
        let token = create_test_api_key(&state).await;
        let app = build_router(state);

        let req = axum::http::Request::builder()
            .uri("/auth/keys")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        let (status, _body) = send_request(app, req).await;
        assert_eq!(status, StatusCode::OK);
    }

    #[tokio::test]
    #[serial]
    async fn test_auth_malformed_header_returns_401() {
        let state = test_state().await;
        let app = build_router(state);

        // "Basic" instead of "Bearer"
        let req = axum::http::Request::builder()
            .uri("/auth/keys")
            .header("Authorization", "Basic abc123")
            .body(Body::empty())
            .unwrap();

        let (status, _) = send_request(app, req).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    // ── Key management ───────────────────────────────────────────────

    #[tokio::test]
    #[serial]
    async fn test_create_key_returns_201() {
        let state = test_state().await;
        let token = create_test_api_key(&state).await;
        let app = build_router(state);

        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/auth/keys")
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"name": "new-test-key"}"#))
            .unwrap();

        let (status, body) = send_request(app, req).await;
        assert_eq!(status, StatusCode::CREATED);
        assert!(body["key"].as_str().unwrap().starts_with("clk_"));
        assert_eq!(body["name"], "new-test-key");
        assert!(body["id"].as_str().is_some());
    }

    #[tokio::test]
    #[serial]
    async fn test_create_key_missing_name_returns_422() {
        let state = test_state().await;
        let token = create_test_api_key(&state).await;
        let app = build_router(state);

        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/auth/keys")
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{}"#))
            .unwrap();

        let (status, _) = send_request(app, req).await;
        // axum returns 422 for deserialization failures
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    #[serial]
    async fn test_list_keys_returns_list() {
        let state = test_state().await;
        let token = create_test_api_key(&state).await;
        let app = build_router(state);

        let req = axum::http::Request::builder()
            .uri("/auth/keys")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        let (status, body) = send_request(app, req).await;
        assert_eq!(status, StatusCode::OK);
        assert!(body["keys"].as_array().is_some());
        assert!(!body["keys"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    #[serial]
    async fn test_revoke_key_valid() {
        let state = test_state().await;
        let token = create_test_api_key(&state).await;

        // Create a second key to revoke
        let (_, hash2) = cloacina::security::api_keys::generate_api_key();
        let dal = cloacina::dal::DAL::new(state.database.clone());
        let info2 = dal
            .api_keys()
            .create_key(&hash2, "to-revoke", None, false, "admin")
            .await
            .expect("create key");

        let app = build_router(state);

        let req = axum::http::Request::builder()
            .method("DELETE")
            .uri(format!("/auth/keys/{}", info2.id))
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        let (status, body) = send_request(app, req).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["status"], "revoked");
    }

    #[tokio::test]
    #[serial]
    async fn test_revoke_key_nonexistent_returns_404() {
        let state = test_state().await;
        let token = create_test_api_key(&state).await;
        let app = build_router(state);

        let fake_id = uuid::Uuid::new_v4();
        let req = axum::http::Request::builder()
            .method("DELETE")
            .uri(format!("/auth/keys/{}", fake_id))
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        let (status, _) = send_request(app, req).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    #[serial]
    async fn test_revoke_key_invalid_uuid_returns_400() {
        let state = test_state().await;
        let token = create_test_api_key(&state).await;
        let app = build_router(state);

        let req = axum::http::Request::builder()
            .method("DELETE")
            .uri("/auth/keys/not-a-uuid")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        let (status, _) = send_request(app, req).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    // ── Tenants ──────────────────────────────────────────────────────

    #[tokio::test]
    #[serial]
    async fn test_create_tenant_returns_201() {
        let state = test_state().await;
        let token = create_test_api_key(&state).await;
        let app = build_router(state);

        let schema = format!(
            "test_{}",
            uuid::Uuid::new_v4().to_string().replace('-', "_")
        );
        let body_json = serde_json::json!({
            "schema_name": schema,
            "username": schema,
            "password": "testpass123"
        });

        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/tenants")
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .body(Body::from(serde_json::to_string(&body_json).unwrap()))
            .unwrap();

        let (status, body) = send_request(app, req).await;
        assert_eq!(status, StatusCode::CREATED, "body: {:?}", body);
        assert_eq!(body["schema_name"], schema);
    }

    #[tokio::test]
    #[serial]
    async fn test_list_tenants() {
        let state = test_state().await;
        let token = create_test_api_key(&state).await;
        let app = build_router(state);

        let req = axum::http::Request::builder()
            .uri("/tenants")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        let (status, body) = send_request(app, req).await;
        assert_eq!(status, StatusCode::OK);
        assert!(body["tenants"].as_array().is_some());
    }

    #[tokio::test]
    #[serial]
    async fn test_remove_tenant_nonexistent_succeeds() {
        let state = test_state().await;
        let token = create_test_api_key(&state).await;
        let app = build_router(state);

        let req = axum::http::Request::builder()
            .method("DELETE")
            .uri("/tenants/nonexistent_schema_xyz")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        let (status, body) = send_request(app, req).await;
        // DROP SCHEMA IF EXISTS is idempotent — succeeds even if schema doesn't exist
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["status"], "removed");
    }

    #[tokio::test]
    #[serial]
    async fn test_create_then_delete_tenant() {
        let state = test_state().await;
        let token = create_test_api_key(&state).await;

        let schema = format!(
            "test_{}",
            uuid::Uuid::new_v4().to_string().replace('-', "_")
        );
        let body_json = serde_json::json!({
            "schema_name": schema,
            "username": schema,
            "password": "testpass123"
        });

        // Create
        let app = build_router(state.clone());
        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/tenants")
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .body(Body::from(serde_json::to_string(&body_json).unwrap()))
            .unwrap();
        let (status, _) = send_request(app, req).await;
        assert_eq!(status, StatusCode::CREATED);

        // Delete
        let app = build_router(state);
        let req = axum::http::Request::builder()
            .method("DELETE")
            .uri(format!("/tenants/{}", schema))
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();
        let (status, body) = send_request(app, req).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["status"], "removed");
    }

    #[tokio::test]
    #[serial]
    async fn test_create_tenant_missing_fields_returns_422() {
        let state = test_state().await;
        let token = create_test_api_key(&state).await;
        let app = build_router(state);

        // Missing required schema_name and username
        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/tenants")
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{}"#))
            .unwrap();
        let (status, _) = send_request(app, req).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    }

    // ── Workflows ────────────────────────────────────────────────────

    #[tokio::test]
    #[serial]
    async fn test_list_workflows_returns_list() {
        let state = test_state().await;
        let token = create_test_api_key(&state).await;
        let app = build_router(state);

        let req = axum::http::Request::builder()
            .uri("/tenants/default/workflows")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        let (status, body) = send_request(app, req).await;
        assert_eq!(status, StatusCode::OK);
        assert!(body["workflows"].as_array().is_some());
    }

    #[tokio::test]
    #[serial]
    async fn test_get_workflow_nonexistent_returns_404() {
        let state = test_state().await;
        let token = create_test_api_key(&state).await;
        let app = build_router(state);

        let req = axum::http::Request::builder()
            .uri("/tenants/default/workflows/nonexistent_workflow")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        let (status, _) = send_request(app, req).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    #[serial]
    async fn test_upload_workflow_empty_file_returns_400() {
        let state = test_state().await;
        let token = create_test_api_key(&state).await;
        let app = build_router(state);

        // Build a multipart body with an empty file
        let boundary = "----testboundary";
        let multipart_body = format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"test.cloacina\"\r\nContent-Type: application/octet-stream\r\n\r\n\r\n--{boundary}--\r\n"
        );

        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/tenants/default/workflows")
            .header("Authorization", format!("Bearer {}", token))
            .header(
                "Content-Type",
                format!("multipart/form-data; boundary={}", boundary),
            )
            .body(Body::from(multipart_body))
            .unwrap();

        let (status, body) = send_request(app, req).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "body: {:?}", body);
    }

    #[tokio::test]
    #[serial]
    async fn test_upload_workflow_no_file_field_returns_400() {
        let state = test_state().await;
        let token = create_test_api_key(&state).await;
        let app = build_router(state);

        // Multipart body with wrong field name
        let boundary = "----testboundary";
        let multipart_body = format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"data\"; filename=\"test.txt\"\r\nContent-Type: application/octet-stream\r\n\r\nsome data\r\n--{boundary}--\r\n"
        );

        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/tenants/default/workflows")
            .header("Authorization", format!("Bearer {}", token))
            .header(
                "Content-Type",
                format!("multipart/form-data; boundary={}", boundary),
            )
            .body(Body::from(multipart_body))
            .unwrap();

        let (status, body) = send_request(app, req).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "body: {:?}", body);
    }

    /// Path to test fixture directory (relative to workspace root).
    fn fixture_path(name: &str) -> std::path::PathBuf {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        std::path::PathBuf::from(manifest_dir)
            .join("test-fixtures")
            .join(name)
    }

    /// Build a multipart request body with a file field.
    fn multipart_file_body(data: &[u8]) -> (String, Vec<u8>) {
        let boundary = "----TestBoundary9876543210";
        let mut body = Vec::new();
        write!(
            body,
            "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"pkg.cloacina\"\r\nContent-Type: application/octet-stream\r\n\r\n"
        )
        .unwrap();
        body.extend_from_slice(data);
        write!(body, "\r\n--{boundary}--\r\n").unwrap();
        (boundary.to_string(), body)
    }

    /// Delete a workflow by name/version if it exists (cleanup for idempotent tests).
    async fn delete_workflow_if_exists(state: &AppState, token: &str, name: &str, version: &str) {
        let app = build_router(state.clone());
        let req = axum::http::Request::builder()
            .method("DELETE")
            .uri(format!("/tenants/default/workflows/{}/{}", name, version))
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();
        // Ignore result — may 404 on first run
        let _ = app.oneshot(req).await;
    }

    #[tokio::test]
    #[serial]
    async fn test_upload_valid_python_workflow_returns_201() {
        let state = test_state().await;
        let token = create_test_api_key(&state).await;

        // Clean up from any prior run
        delete_workflow_if_exists(&state, &token, "test-fixture-python", "1.0.0").await;

        let app = build_router(state);
        let package_data = std::fs::read(fixture_path("python-workflow.cloacina"))
            .expect("fixture file not found");
        let (boundary, body_bytes) = multipart_file_body(&package_data);

        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/tenants/default/workflows")
            .header("Authorization", format!("Bearer {}", token))
            .header(
                "Content-Type",
                format!("multipart/form-data; boundary={}", boundary),
            )
            .body(Body::from(body_bytes))
            .unwrap();

        let (status, body) = send_request(app, req).await;
        assert_eq!(status, StatusCode::CREATED, "body: {:?}", body);
        assert!(body["package_id"].as_str().is_some());
    }

    #[tokio::test]
    #[serial]
    async fn test_upload_valid_rust_workflow_returns_201() {
        let state = test_state().await;
        let token = create_test_api_key(&state).await;

        // Clean up from any prior run
        delete_workflow_if_exists(&state, &token, "test-fixture-rust", "1.0.0").await;

        let app = build_router(state);
        let package_data =
            std::fs::read(fixture_path("rust-workflow.cloacina")).expect("fixture file not found");
        let (boundary, body_bytes) = multipart_file_body(&package_data);

        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/tenants/default/workflows")
            .header("Authorization", format!("Bearer {}", token))
            .header(
                "Content-Type",
                format!("multipart/form-data; boundary={}", boundary),
            )
            .body(Body::from(body_bytes))
            .unwrap();

        let (status, body) = send_request(app, req).await;
        assert_eq!(status, StatusCode::CREATED, "body: {:?}", body);
        assert!(body["package_id"].as_str().is_some());
    }

    #[tokio::test]
    #[serial]
    async fn test_upload_corrupt_package_returns_400() {
        let state = test_state().await;
        let token = create_test_api_key(&state).await;
        let app = build_router(state);

        let (boundary, body_bytes) = multipart_file_body(b"this is not a valid bzip2 archive");

        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/tenants/default/workflows")
            .header("Authorization", format!("Bearer {}", token))
            .header(
                "Content-Type",
                format!("multipart/form-data; boundary={}", boundary),
            )
            .body(Body::from(body_bytes))
            .unwrap();

        let (status, body) = send_request(app, req).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "body: {:?}", body);
    }

    // ── Executions ───────────────────────────────────────────────────

    #[tokio::test]
    #[serial]
    async fn test_list_executions_returns_list() {
        let state = test_state().await;
        let token = create_test_api_key(&state).await;
        let app = build_router(state);

        let req = axum::http::Request::builder()
            .uri("/tenants/default/executions")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        let (status, body) = send_request(app, req).await;
        assert_eq!(status, StatusCode::OK);
        assert!(body["executions"].as_array().is_some());
    }

    #[tokio::test]
    #[serial]
    async fn test_get_execution_invalid_uuid_returns_400() {
        let state = test_state().await;
        let token = create_test_api_key(&state).await;
        let app = build_router(state);

        let req = axum::http::Request::builder()
            .uri("/tenants/default/executions/not-a-uuid")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        let (status, _) = send_request(app, req).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    #[serial]
    async fn test_get_execution_nonexistent_returns_404() {
        let state = test_state().await;
        let token = create_test_api_key(&state).await;
        let app = build_router(state);

        let fake_id = uuid::Uuid::new_v4();
        let req = axum::http::Request::builder()
            .uri(format!("/tenants/default/executions/{}", fake_id))
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        let (status, _) = send_request(app, req).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    #[serial]
    async fn test_get_execution_events_invalid_uuid_returns_400() {
        let state = test_state().await;
        let token = create_test_api_key(&state).await;
        let app = build_router(state);

        let req = axum::http::Request::builder()
            .uri("/tenants/default/executions/not-a-uuid/events")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        let (status, _) = send_request(app, req).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    #[serial]
    async fn test_execute_nonexistent_workflow_returns_error() {
        let state = test_state().await;
        let token = create_test_api_key(&state).await;
        let app = build_router(state);

        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/tenants/default/workflows/nonexistent_wf/execute")
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"context": {}}"#))
            .unwrap();

        let (status, _) = send_request(app, req).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    #[serial]
    async fn test_get_execution_events_valid_uuid_no_events() {
        let state = test_state().await;
        let token = create_test_api_key(&state).await;
        let app = build_router(state);

        // Valid UUID format but no matching execution — should return
        // an empty events list (the DAL returns Ok([]) for missing pipelines)
        let fake_id = uuid::Uuid::new_v4();
        let req = axum::http::Request::builder()
            .uri(format!("/tenants/default/executions/{}/events", fake_id))
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        let (status, body) = send_request(app, req).await;
        assert_eq!(status, StatusCode::OK);
        assert!(body["events"].as_array().is_some());
        assert!(body["events"].as_array().unwrap().is_empty());
    }

    // ── Triggers ─────────────────────────────────────────────────────

    #[tokio::test]
    #[serial]
    async fn test_list_triggers_returns_list() {
        let state = test_state().await;
        let token = create_test_api_key(&state).await;
        let app = build_router(state);

        let req = axum::http::Request::builder()
            .uri("/tenants/default/triggers")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        let (status, body) = send_request(app, req).await;
        assert_eq!(status, StatusCode::OK);
        assert!(body["schedules"].as_array().is_some());
    }

    #[tokio::test]
    #[serial]
    async fn test_get_trigger_nonexistent_returns_404() {
        let state = test_state().await;
        let token = create_test_api_key(&state).await;
        let app = build_router(state);

        let req = axum::http::Request::builder()
            .uri("/tenants/default/triggers/nonexistent_trigger")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        let (status, _) = send_request(app, req).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    // ── Fallback / 404 ──────────────────────────────────────────────

    #[tokio::test]
    #[serial]
    async fn test_unknown_route_returns_404() {
        let state = test_state().await;
        let app = build_router(state);

        let req = axum::http::Request::builder()
            .uri("/nonexistent/route")
            .body(Body::empty())
            .unwrap();

        let (status, body) = send_request(app, req).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body["error"], "not found");
    }
}
