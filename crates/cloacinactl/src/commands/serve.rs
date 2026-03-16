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

//! `cloacinactl serve` command — starts the Cloacina server.

use crate::config::ServerConfig;
use crate::routes::health::{self, AppState};
use anyhow::Result;
use axum::routing::get;
use axum::Router;
use std::sync::Arc;
use std::time::Instant;
use tracing::info;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

/// Server operational mode.
#[derive(Debug, Clone, Copy, PartialEq, clap::ValueEnum)]
pub enum ServeMode {
    /// Run all services: API server, scheduler, worker, cron, recovery.
    All,
    /// Run only the HTTP API server (no background services).
    Api,
    /// Run only the task executor/dispatcher (no API, no scheduler).
    Worker,
    /// Run only the scheduler, cron, and recovery services (no API, no executor).
    Scheduler,
}

impl std::fmt::Display for ServeMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServeMode::All => write!(f, "all"),
            ServeMode::Api => write!(f, "api"),
            ServeMode::Worker => write!(f, "worker"),
            ServeMode::Scheduler => write!(f, "scheduler"),
        }
    }
}

/// Arguments for the `serve` subcommand.
#[derive(Debug, clap::Args)]
pub struct ServeArgs {
    /// Server operational mode.
    #[arg(long, value_enum, default_value = "all")]
    pub mode: ServeMode,

    /// Path to the TOML configuration file.
    #[arg(long)]
    pub config: Option<String>,

    /// Bind address (overrides config file).
    #[arg(long, default_value = "0.0.0.0")]
    pub bind: String,

    /// Port to listen on (overrides config file).
    #[arg(long, default_value = "8080")]
    pub port: u16,
}

/// OpenAPI documentation struct.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Cloacina API",
        description = "Cloacina workflow orchestration server API",
    ),
    paths(health::health),
    components(schemas(health::HealthResponse)),
    tags(
        (name = "system", description = "System health and status"),
    )
)]
struct ApiDoc;

/// Build the axum Router with application state.
///
/// Separated from `run()` so it can be tested independently.
pub fn app(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health::health))
        .merge(SwaggerUi::new("/api-docs/").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .with_state(state)
}

/// Wait for a shutdown signal (SIGTERM or Ctrl+C).
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    info!("Shutdown signal received, stopping server...");
}

/// Build a DefaultRunnerConfig from the ServerConfig.
fn build_runner_config(
    config: &ServerConfig,
    mode: ServeMode,
) -> cloacina::runner::DefaultRunnerConfig {
    let mut builder = cloacina::runner::DefaultRunnerConfig::builder()
        .max_concurrent_tasks(config.worker.max_concurrent_tasks)
        .task_timeout(std::time::Duration::from_secs(
            config.worker.task_timeout_seconds,
        ))
        .scheduler_poll_interval(std::time::Duration::from_millis(
            config.scheduler.poll_interval_ms,
        ))
        .db_pool_size(config.database.pool_size);

    // Enable cron and triggers for scheduler and all modes
    let enable_scheduling = matches!(mode, ServeMode::All | ServeMode::Scheduler);
    builder = builder
        .enable_cron_scheduling(enable_scheduling)
        .enable_trigger_scheduling(enable_scheduling);

    // Enable continuous scheduling if configured
    if config.scheduler.enable_continuous && enable_scheduling {
        builder = builder.enable_continuous_scheduling(true);
    }

    builder.build()
}

/// Run the serve command.
pub async fn run(args: &ServeArgs) -> Result<()> {
    let config = crate::config::load_config(args.config.as_deref(), args)?;
    let mode = args.mode;
    let needs_api = matches!(mode, ServeMode::All | ServeMode::Api);
    let needs_runner = matches!(
        mode,
        ServeMode::All | ServeMode::Worker | ServeMode::Scheduler
    );

    // Start DefaultRunner if this mode needs background services
    let runner = if needs_runner && !config.database.url.is_empty() {
        let runner_config = build_runner_config(&config, mode);
        info!("Starting background services (mode: {})", mode);
        match cloacina::runner::DefaultRunner::with_config(&config.database.url, runner_config)
            .await
        {
            Ok(r) => {
                info!("Background services started");
                Some(r)
            }
            Err(e) => {
                tracing::error!("Failed to start background services: {}", e);
                return Err(e.into());
            }
        }
    } else {
        if needs_runner && config.database.url.is_empty() {
            tracing::warn!("No database URL configured — background services disabled");
        }
        None
    };

    // Start HTTP server if this mode needs API
    if needs_api {
        let state = Arc::new(AppState {
            startup_instant: Instant::now(),
            mode: config.server.mode.clone(),
        });

        let bind_addr = format!("{}:{}", config.server.bind, config.server.port);
        let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
        let local_addr = listener.local_addr()?;

        info!(
            "Cloacina server listening on {} (mode: {})",
            local_addr, config.server.mode
        );

        axum::serve(listener, app(state))
            .with_graceful_shutdown(shutdown_signal())
            .await?;
    } else {
        // No API — just wait for shutdown signal
        info!("Running in {} mode (no HTTP server)", mode);
        shutdown_signal().await;
    }

    // Shutdown runner
    if let Some(runner) = runner {
        info!("Stopping background services...");
        if let Err(e) = runner.shutdown().await {
            tracing::error!("Error during runner shutdown: {}", e);
        }
    }

    info!("Server stopped cleanly");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_serve_health_endpoint_lifecycle() {
        // Create app state
        let state = Arc::new(AppState {
            startup_instant: Instant::now(),
            mode: "api".to_string(),
        });

        // Bind to port 0 (OS assigns random available port)
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let base_url = format!("http://{}", addr);

        // Start server in background task
        let server_handle = tokio::spawn(async move {
            axum::serve(listener, app(state))
                .with_graceful_shutdown(async {
                    // Shutdown after 5 seconds max (safety net)
                    tokio::time::sleep(Duration::from_secs(5)).await;
                })
                .await
                .unwrap();
        });

        // Wait for server to be ready
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Hit /health
        let client = reqwest::Client::new();
        let resp = client
            .get(format!("{}/health", base_url))
            .send()
            .await
            .expect("health request failed");

        assert_eq!(resp.status(), 200);

        let body: serde_json::Value = resp.json().await.expect("invalid JSON");
        assert_eq!(body["status"], "ok");
        assert!(body["version"].is_string());
        assert_eq!(body["mode"], "api");
        assert!(body["uptime_seconds"].is_number());

        // Hit /api-docs/openapi.json
        let resp = client
            .get(format!("{}/api-docs/openapi.json", base_url))
            .send()
            .await
            .expect("openapi request failed");

        assert_eq!(resp.status(), 200);
        let spec: serde_json::Value = resp.json().await.expect("invalid OpenAPI JSON");
        assert!(spec["openapi"].is_string());
        assert!(spec["paths"]["/health"].is_object());

        // Server will shut down when the safety-net timeout fires
        let _ = tokio::time::timeout(Duration::from_secs(6), server_handle).await;
    }

    #[tokio::test]
    async fn test_health_returns_correct_mode() {
        let state = Arc::new(AppState {
            startup_instant: Instant::now(),
            mode: "scheduler".to_string(),
        });

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server_handle = tokio::spawn(async move {
            axum::serve(listener, app(state))
                .with_graceful_shutdown(async {
                    tokio::time::sleep(Duration::from_secs(5)).await;
                })
                .await
                .unwrap();
        });

        tokio::time::sleep(Duration::from_millis(100)).await;

        let resp = reqwest::get(format!("http://{}/health", addr))
            .await
            .unwrap();
        let body: serde_json::Value = resp.json().await.unwrap();
        assert_eq!(body["mode"], "scheduler");

        let _ = tokio::time::timeout(Duration::from_secs(6), server_handle).await;
    }

    #[tokio::test]
    async fn test_unknown_route_returns_404() {
        let state = Arc::new(AppState {
            startup_instant: Instant::now(),
            mode: "api".to_string(),
        });

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server_handle = tokio::spawn(async move {
            axum::serve(listener, app(state))
                .with_graceful_shutdown(async {
                    tokio::time::sleep(Duration::from_secs(5)).await;
                })
                .await
                .unwrap();
        });

        tokio::time::sleep(Duration::from_millis(100)).await;

        let resp = reqwest::get(format!("http://{}/nonexistent", addr))
            .await
            .unwrap();
        assert_eq!(resp.status(), 404);

        let _ = tokio::time::timeout(Duration::from_secs(6), server_handle).await;
    }
}
