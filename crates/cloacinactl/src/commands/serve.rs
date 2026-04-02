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
use tokio::sync::watch;
use tracing::{error, info};
use tracing_appender::rolling;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use cloacina::database::Database;
use cloacina::runner::{DefaultRunner, DefaultRunnerConfig};

/// Shared application state accessible from all route handlers.
#[derive(Clone)]
pub struct AppState {
    pub database: Database,
    pub runner: Arc<DefaultRunner>,
    pub key_cache: Arc<crate::server::auth::KeyCache>,
}

/// Run the API server.
pub async fn run(
    home: std::path::PathBuf,
    bind: SocketAddr,
    database_url: String,
    verbose: bool,
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

    // Connect to Postgres
    let runner_config = DefaultRunnerConfig::builder().build();

    let runner = DefaultRunner::with_config(&database_url, runner_config)
        .await
        .context("Failed to connect to database")?;

    info!("Connected to Postgres, migrations applied");

    let state = AppState {
        database: runner.database().clone(),
        runner: Arc::new(runner),
        key_cache: Arc::new(crate::server::auth::KeyCache::default_cache()),
    };

    // Bootstrap: create initial admin key if none exist
    bootstrap_admin_key(&state, &home).await?;

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

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
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
    use axum::{middleware, routing::delete, routing::post};

    // Authenticated routes — behind auth middleware
    let auth_routes = Router::new()
        .route("/auth/keys", post(crate::server::keys::create_key))
        .route("/auth/keys", get(crate::server::keys::list_keys))
        .route(
            "/auth/keys/{key_id}",
            delete(crate::server::keys::revoke_key),
        )
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            crate::server::auth::require_auth,
        ));

    // Public routes — no auth
    Router::new()
        .route("/health", get(health))
        .route("/ready", get(ready))
        .route("/metrics", get(metrics))
        .merge(auth_routes)
        .fallback(fallback_404)
        .with_state(state)
}

/// GET /health — liveness check (no auth, no DB)
async fn health() -> impl IntoResponse {
    Json(serde_json::json!({"status": "ok"}))
}

/// GET /ready — readiness check (verifies DB connection pool is healthy)
async fn ready(State(state): State<AppState>) -> impl IntoResponse {
    // Verify we can acquire a connection from the pool
    let is_ready = state.database.get_postgres_connection().await.is_ok();

    if is_ready {
        (StatusCode::OK, Json(serde_json::json!({"status": "ready"})))
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"status": "not ready", "reason": "database unreachable"})),
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
async fn bootstrap_admin_key(state: &AppState, home: &std::path::Path) -> Result<()> {
    let dal = cloacina::dal::DAL::new(state.database.clone());
    let has_keys = dal.api_keys().has_any_keys().await.unwrap_or(false);

    if has_keys {
        info!("API keys exist — skipping bootstrap");
        return Ok(());
    }

    info!("No API keys found — creating bootstrap admin key");

    let (plaintext, hash) = cloacina::security::api_keys::generate_api_key();
    dal.api_keys()
        .create_key(&hash, "bootstrap-admin")
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
    info!("Use this key to authenticate API requests, then create additional keys via POST /auth/keys");

    Ok(())
}

/// Mask password in database URL for logging
fn mask_db_url(url: &str) -> String {
    if let Some(at_pos) = url.find('@') {
        if let Some(colon_pos) = url[..at_pos].rfind(':') {
            let prefix = &url[..colon_pos + 1];
            let suffix = &url[at_pos..];
            return format!("{}****{}", prefix, suffix);
        }
    }
    url.to_string()
}
